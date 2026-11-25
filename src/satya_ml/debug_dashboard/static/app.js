(() => {
  "use strict";

  const state = {
    topology: { ai_gm: { nodes: [], edges: [] }, ai_player: { nodes: [], edges: [] } },
    defaults: {},
    activeAgent: "ai_gm",
    runsById: new Map(),
    runOrder: [],
    selectedRunId: null,
    selectedStepIndex: null,
    breakpoints: { ai_gm: new Set(), ai_player: new Set() },
    scenarios: [],
    lastScenarioResult: null,
    sse: null,
  };

  const $ = (id) => document.getElementById(id);

  function escapeHtml(str) {
    return String(str ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function statusPillClass(status) {
    if (status === "completed") return "pill-ok";
    if (status === "paused") return "pill-warn";
    if (status === "running") return "pill-run";
    if (status === "error" || status === "aborted") return "pill-err";
    return "pill-muted";
  }

  async function apiJson(url, opts = {}) {
    const resp = await fetch(url, {
      headers: { "Content-Type": "application/json", ...(opts.headers || {}) },
      ...opts,
    });
    const body = await resp.json().catch(() => ({}));
    if (!resp.ok) {
      throw new Error(body.detail || `HTTP ${resp.status}`);
    }
    return body;
  }

  function upsertRunSummary(summary) {
    if (!summary || !summary.run_id) return;
    const prev = state.runsById.get(summary.run_id);
    if (!prev) {
      state.runOrder.unshift(summary.run_id);
      state.runsById.set(summary.run_id, { ...summary, steps: [], messages: [], tool_invocations: [], turn_logs: [] });
    } else {
      Object.assign(prev, summary);
    }
  }

  async function loadRunDetail(runId) {
    if (!runId) return;
    try {
      const data = await apiJson(`/debug/api/runs/${encodeURIComponent(runId)}`);
      if (data.run) {
        state.runsById.set(runId, data.run);
        if (!state.runOrder.includes(runId)) {
          state.runOrder.unshift(runId);
        }
        renderAll();
      }
    } catch (_err) {
      // Ignore transient lookup failures while run is finishing
    }
  }

  function selectRun(runId) {
    state.selectedRunId = runId;
    state.selectedStepIndex = null;
    const run = state.runsById.get(runId);
    if (run && run.agent_kind && run.agent_kind !== state.activeAgent) {
      setActiveAgent(run.agent_kind, false);
    }
    renderAll();
    loadRunDetail(runId);
  }

  function setActiveAgent(agentKind, rerender = true) {
    state.activeAgent = agentKind;
    $("btn-agent-gm").classList.toggle("active", agentKind === "ai_gm");
    $("btn-agent-player").classList.toggle("active", agentKind === "ai_player");
    $("graph-title").textContent = agentKind === "ai_gm" ? "AI GM StateGraph (7 Nodes)" : "AI Player StateGraph (4 Nodes)";
    if (rerender) renderAll();
  }

  function layoutNodes(agentKind, topology) {
    const nodes = topology.nodes || [];
    const customPositions = {
      ai_gm: {
        __start__: [42, 130],
        sync_session_context: [148, 130],
        party_turn_gate_node: [288, 130],
        scenario_director_node: [435, 76],
        npc_action_resolution_node: [585, 76],
        gm_supervisor_node: [728, 130],
        tool_execution_node: [728, 228],
        publish_gm_response: [865, 130],
        __end__: [945, 130],
      },
      ai_player: {
        __start__: [65, 125],
        sync_player_context: [235, 125],
        player_decision_node: [460, 125],
        player_tool_node: [460, 225],
        publish_player_turn: [700, 125],
        __end__: [890, 125],
      },
    };
    const preset = customPositions[agentKind] || {};
    const out = new Map();
    const fallbackGap = 900 / Math.max(1, nodes.length);
    nodes.forEach((n, idx) => {
      const pos = preset[n.id] || [60 + idx * fallbackGap, 130];
      out.set(n.id, { ...n, x: pos[0], y: pos[1] });
    });
    return out;
  }

  function renderGraphSvg() {
    const svg = $("graph-svg");
    const topo = state.topology[state.activeAgent] || { nodes: [], edges: [] };
    const posMap = layoutNodes(state.activeAgent, topo);
    const run = state.runsById.get(state.selectedRunId);
    const matchesAgent = run && run.agent_kind === state.activeAgent;

    const stepsByNode = new Map();
    let runningNode = null;
    if (matchesAgent) {
      for (const s of run.steps || []) {
        if (!stepsByNode.has(s.node_name)) stepsByNode.set(s.node_name, []);
        stepsByNode.get(s.node_name).push(s);
        if (s.status === "running") runningNode = s.node_name;
      }
    }
    const pausedNext = new Set(matchesAgent && run.status === "paused" ? (run.next_nodes || []) : []);
    const selectedStep =
      matchesAgent && state.selectedStepIndex !== null
        ? (run.steps || []).find((s) => s.step_index === state.selectedStepIndex)
        : null;

    const parts = [
      `<defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M 0 1 L 10 5 L 0 9 z" fill="#64748b"></path>
        </marker>
        <marker id="arrow-active" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M 0 1 L 10 5 L 0 9 z" fill="#60a5fa"></path>
        </marker>
      </defs>`,
    ];

    for (const edge of topo.edges || []) {
      const src = posMap.get(edge.source);
      const dst = posMap.get(edge.target);
      if (!src || !dst) continue;
      let d = "";
      if (src.x === dst.x && src.y !== dst.y) {
        const offset = dst.y > src.y ? 34 : -34;
        d = `M ${src.x + offset} ${src.y} Q ${src.x + offset * 2.1} ${(src.y + dst.y) / 2} ${dst.x + offset} ${dst.y}`;
      } else if (edge.source === "party_turn_gate_node" && edge.target === "publish_gm_response") {
        d = `M ${src.x} ${src.y + 22} Q ${(src.x + dst.x) / 2} 255 ${dst.x} ${dst.y + 22}`;
      } else {
        const dx = dst.x - src.x;
        const dy = dst.y - src.y;
        const len = Math.hypot(dx, dy) || 1;
        const padSrc = src.kind === "node" ? 56 : 18;
        const padDst = dst.kind === "node" ? 58 : 18;
        const x1 = src.x + (dx / len) * padSrc;
        const y1 = src.y + (dy / len) * (src.kind === "node" ? 20 : 12);
        const x2 = dst.x - (dx / len) * padDst;
        const y2 = dst.y - (dy / len) * (dst.kind === "node" ? 20 : 12);
        d = `M ${x1.toFixed(1)} ${y1.toFixed(1)} L ${x2.toFixed(1)} ${y2.toFixed(1)}`;
      }
      const visitedEdge =
        stepsByNode.has(edge.source) && (stepsByNode.has(edge.target) || edge.target === "__end__");
      const stroke = visitedEdge ? "#60a5fa" : "#475569";
      const dash = edge.conditional ? 'stroke-dasharray="5 4"' : "";
      const marker = visitedEdge ? "url(#arrow-active)" : "url(#arrow)";
      parts.push(
        `<path d="${d}" fill="none" stroke="${stroke}" stroke-width="${visitedEdge ? 2 : 1.4}" ${dash} marker-end="${marker}" />`
      );
    }

    const bps = state.breakpoints[state.activeAgent] || new Set();
    for (const node of posMap.values()) {
      if (node.kind !== "node") {
        parts.push(
          `<g transform="translate(${node.x},${node.y})">
            <circle r="15" fill="#1e293b" stroke="#475569" stroke-width="1.5"></circle>
            <text text-anchor="middle" dy="4" fill="#cbd5e1" font-size="9" font-weight="700">${escapeHtml(node.label)}</text>
          </g>`
        );
        continue;
      }

      const nodeSteps = stepsByNode.get(node.id) || [];
      const hasErr = nodeSteps.some((s) => s.status === "error");
      let cls = "node-idle";
      if (runningNode === node.id) cls = "node-running";
      else if (pausedNext.has(node.id)) cls = "node-paused";
      else if (hasErr) cls = "node-error";
      else if (nodeSteps.length > 0) cls = "node-done";
      else if (matchesAgent && run.status === "completed") cls = "node-skipped";

      if (selectedStep && selectedStep.node_name === node.id) {
        cls += " node-selected";
      }

      const totalMs = nodeSteps.reduce((acc, s) => acc + (s.duration_ms || 0), 0);
      const countText = nodeSteps.length > 1 ? ` ×${nodeSteps.length}` : "";
      const subLabel =
        runningNode === node.id
          ? "running..."
          : pausedNext.has(node.id)
          ? "PAUSED NEXT"
          : nodeSteps.length > 0
          ? `${totalMs}ms${countText}`
          : "idle";

      const shortName = node.id.replace(/_node$/, "");
      const hasBp = bps.has(node.id);
      parts.push(
        `<g transform="translate(${node.x - 58},${node.y - 22})" data-node-id="${escapeHtml(node.id)}">
          <rect class="node-box ${cls}" width="116" height="44" rx="7"></rect>
          <text x="58" y="18" text-anchor="middle" fill="#e2e8f0" font-size="10.5" font-weight="700">${escapeHtml(shortName)}</text>
          <text x="58" y="34" text-anchor="middle" fill="#94a3b8" font-size="9.5" font-family="monospace">${escapeHtml(subLabel)}</text>
          <circle class="bp-dot" data-bp-node="${escapeHtml(node.id)}" cx="107" cy="9" r="5"
            fill="${hasBp ? "#ef4444" : "#0b1017"}" stroke="${hasBp ? "#fca5a5" : "#64748b"}" stroke-width="1.5">
            <title>Toggle breakpoint before ${escapeHtml(node.id)}</title>
          </circle>
        </g>`
      );
    }

    svg.innerHTML = parts.join("");
  }

  function renderRunList() {
    const filterAgent = $("run-filter-agent").value;
    const list = $("run-list");
    const items = state.runOrder
      .map((id) => state.runsById.get(id))
      .filter((r) => r && (!filterAgent || r.agent_kind === filterAgent));

    if (!items.length) {
      list.innerHTML = `<div class="empty-state">No turns observed yet.</div>`;
      return;
    }

    list.innerHTML = items
      .map((r) => {
        const selected = r.run_id === state.selectedRunId ? "selected" : "";
        const pill = statusPillClass(r.status);
        const label = r.agent_kind === "ai_player" ? "PLAYER" : "GM";
        return `
          <div class="run-card ${selected}" data-run-id="${escapeHtml(r.run_id)}">
            <div class="run-card-top">
              <span><strong>${label}</strong> · ${escapeHtml(r.session_id)}</span>
              <span class="pill ${pill}">${escapeHtml(r.status)}</span>
            </div>
            <div class="run-summary">${escapeHtml(r.trigger_summary || r.final_reply || "(no summary)")}</div>
            <div class="hint">${r.duration_ms || 0}ms · ${r.step_count ?? (r.steps || []).length} steps · ${r.tool_count ?? (r.tool_invocations || []).length} tools</div>
          </div>`;
      })
      .join("");
  }

  function renderTimelineAndBanner() {
    const run = state.runsById.get(state.selectedRunId);
    const bpSet = state.breakpoints[state.activeAgent] || new Set();
    $("bp-summary").textContent = bpSet.size
      ? `Breakpoints: ${Array.from(bpSet).join(", ")}`
      : "No breakpoints";

    $("btn-download-trace").disabled = !run;

    const banner = $("pause-banner");
    if (run && run.status === "paused") {
      banner.classList.remove("hidden");
      $("pause-next-nodes").textContent = (run.next_nodes || []).join(", ") || "(unknown)";
    } else {
      banner.classList.add("hidden");
    }

    const badge = $("active-run-badge");
    if (run) {
      badge.className = `pill ${statusPillClass(run.status)}`;
      badge.textContent = `${run.agent_kind}: ${run.status} (${run.duration_ms || 0}ms)`;
    } else {
      badge.className = "pill pill-muted";
      badge.textContent = "Idle";
    }

    const timeline = $("step-timeline");
    const steps = (run && run.steps) || [];
    if (!steps.length) {
      timeline.innerHTML = `<span class="hint">Waiting for execution...</span>`;
      return;
    }
    timeline.innerHTML = steps
      .map((s) => {
        const active = s.step_index === state.selectedStepIndex ? "active" : "";
        const err = s.status === "error" ? "error" : "";
        return `<button class="step-chip ${active} ${err}" data-step-idx="${s.step_index}">
          #${s.step_index + 1} ${escapeHtml(s.node_name)} (${s.duration_ms || 0}ms)
        </button>`;
      })
      .join("");
  }

  function renderInspector() {
    const run = state.runsById.get(state.selectedRunId);
    if (!run) {
      $("insp-delta").innerHTML = `<div class="empty-state">Select a run to inspect node deltas.</div>`;
      $("insp-messages").innerHTML = `<div class="empty-state">No messages captured yet.</div>`;
      $("insp-tools").innerHTML = `<div class="empty-state">No tools captured yet.</div>`;
      $("logs-view").innerHTML = `<div class="empty-state">No turn logs yet.</div>`;
      return;
    }

    const steps = run.steps || [];
    const targetStep =
      state.selectedStepIndex !== null
        ? steps.find((s) => s.step_index === state.selectedStepIndex)
        : steps[steps.length - 1];

    if (targetStep) {
      $("insp-delta").innerHTML = `
        <div><strong>Step #${targetStep.step_index + 1}:</strong> <code>${escapeHtml(targetStep.node_name)}</code>
          <span class="pill ${statusPillClass(targetStep.status)}">${escapeHtml(targetStep.status)}</span>
          <span class="hint">${targetStep.duration_ms || 0}ms</span>
        </div>
        ${targetStep.error ? `<pre class="json-block">${escapeHtml(targetStep.error)}</pre>` : ""}
        <pre class="json-block">${escapeHtml(JSON.stringify(targetStep.state_delta || {}, null, 2))}</pre>`;
    } else {
      $("insp-delta").innerHTML = `<pre class="json-block">${escapeHtml(JSON.stringify(run.state_preview || {}, null, 2))}</pre>`;
    }

    const msgs = run.messages || [];
    const replyHtml = run.final_reply
      ? `<div class="msg-card"><div class="msg-role role-ai">Final Reply (${escapeHtml(run.final_channel || "PUBLIC")})</div><div class="msg-content">${escapeHtml(run.final_reply)}</div></div>`
      : "";
    $("insp-messages").innerHTML =
      replyHtml +
      (msgs.length
        ? msgs
            .map((m) => {
              const role = String(m.role || "message");
              const body = typeof m.content === "string" ? m.content : JSON.stringify(m.content, null, 2);
              const calls = m.tool_calls ? `\nTool Calls:\n${JSON.stringify(m.tool_calls, null, 2)}` : "";
              return `<div class="msg-card">
                <div class="msg-role role-${escapeHtml(role)}">${escapeHtml(role)}${m.name ? ` (${escapeHtml(m.name)})` : ""}</div>
                <div class="msg-content">${escapeHtml(body + calls)}</div>
              </div>`;
            })
            .join("")
        : `<div class="empty-state">No serialized messages on this run.</div>`);

    const tools = run.tool_invocations || [];
    const packsHtml = (run.loaded_tool_packs || []).length
      ? `<div><strong>Loaded Packs:</strong> ${run.loaded_tool_packs.map((p) => `<span class="pill pill-muted">${escapeHtml(p)}</span>`).join(" ")}</div>`
      : "";
    $("insp-tools").innerHTML =
      packsHtml +
      (tools.length
        ? tools
            .map(
              (t) => `<div class="tool-card">
                <div><strong>#${t.step_index} ${escapeHtml(t.tool_name)}</strong>
                  <span class="pill ${t.is_error ? "pill-err" : "pill-ok"}">${t.is_error ? "ERR" : "OK"}</span>
                  <span class="hint">${t.duration_ms}ms · ${escapeHtml(t.mcp_tier || "")}</span>
                </div>
                <pre class="json-block">args: ${escapeHtml(t.arguments_json || "{}")}\nresult: ${escapeHtml(t.result_summary_json || "")}</pre>
              </div>`
            )
            .join("")
        : `<div class="empty-state">No tools invoked in this turn.</div>`);

    const logs = run.turn_logs || [];
    $("logs-view").innerHTML = logs.length
      ? `<pre class="json-block">${escapeHtml(logs.map((l) => l.log_line || JSON.stringify(l)).join("\n"))}</pre>`
      : `<div class="empty-state">No INFO+ buffered logs captured for this turn.</div>`;
  }

  function renderAll() {
    renderRunList();
    renderGraphSvg();
    renderTimelineAndBanner();
    renderInspector();
  }

  function connectSse() {
    if (state.sse) state.sse.close();
    const es = new EventSource("/debug/api/stream");
    state.sse = es;

    es.onopen = () => {
      $("sse-pill").className = "pill pill-ok";
      $("sse-pill").textContent = "SSE: Live";
    };

    es.onerror = () => {
      $("sse-pill").className = "pill pill-warn";
      $("sse-pill").textContent = "SSE: Reconnecting";
    };

    es.onmessage = (ev) => {
      let msg;
      try {
        msg = JSON.parse(ev.data);
      } catch (_e) {
        return;
      }
      const { type, data } = msg;
      if (type === "hello") {
        for (const r of [...(data.runs || [])].reverse()) {
          upsertRunSummary(r);
        }
        if (!state.selectedRunId && state.runOrder.length) {
          selectRun(state.runOrder[0]);
        } else {
          renderAll();
        }
      } else if (type === "turn_started" || type === "turn_paused" || type === "turn_completed") {
        const r = data.run;
        if (r) {
          upsertRunSummary(r);
          if (type === "turn_started") {
            state.selectedRunId = r.run_id;
            state.selectedStepIndex = null;
            if (r.agent_kind && r.agent_kind !== state.activeAgent) {
              setActiveAgent(r.agent_kind, false);
            }
          } else {
            loadRunDetail(r.run_id);
          }
          renderAll();
        }
      } else if (type === "node_started" || type === "node_completed" || type === "node_error" || type === "node_delta") {
        const run = state.runsById.get(data.run_id);
        const step = data.step;
        if (run && step) {
          run.steps = run.steps || [];
          const idx = run.steps.findIndex((s) => s.step_index === step.step_index);
          if (idx >= 0) run.steps[idx] = step;
          else run.steps.push(step);
          renderAll();
        }
      } else if (type === "traces_cleared") {
        state.runsById.clear();
        state.runOrder = [];
        state.selectedRunId = null;
        state.selectedStepIndex = null;
        renderAll();
      }
    };
  }

  async function init() {
    try {
      const [topoData, scData] = await Promise.all([
        apiJson("/debug/api/topology"),
        apiJson("/debug/api/scenarios"),
      ]);
      state.topology = topoData.graphs || state.topology;
      state.defaults = topoData.defaults || {};
      const sigUrl = topoData.signaling_url || "";
      $("signaling-pill").textContent = sigUrl ? `Signaling: ${sigUrl}` : "Signaling: OFFLINE";
      $("signaling-pill").className = sigUrl ? "pill pill-ok" : "pill pill-muted";

      state.scenarios = scData.scenarios || [];
      $("scenario-select").innerHTML = state.scenarios
        .map((s) => `<option value="${escapeHtml(s.id)}">${escapeHtml(s.name)}</option>`)
        .join("");
      syncScenarioEditor();
    } catch (err) {
      $("sse-pill").className = "pill pill-err";
      $("sse-pill").textContent = `Init Error: ${err.message}`;
    }

    renderAll();
    connectSse();
  }

  function syncScenarioEditor() {
    const id = $("scenario-select").value;
    const spec = state.scenarios.find((s) => s.id === id);
    if (spec) {
      $("scenario-desc").textContent = spec.description || "";
      $("scenario-json").value = JSON.stringify(spec, null, 2);
    }
  }

  // Event Listeners
  $("btn-agent-gm").addEventListener("click", () => setActiveAgent("ai_gm"));
  $("btn-agent-player").addEventListener("click", () => setActiveAgent("ai_player"));
  $("run-filter-agent").addEventListener("change", renderRunList);

  $("run-list").addEventListener("click", (e) => {
    const card = e.target.closest("[data-run-id]");
    if (card) selectRun(card.dataset.runId);
  });

  $("graph-svg").addEventListener("click", (e) => {
    const bp = e.target.closest("[data-bp-node]");
    if (bp) {
      e.stopPropagation();
      const nodeId = bp.dataset.bpNode;
      const set = state.breakpoints[state.activeAgent];
      if (set.has(nodeId)) set.delete(nodeId);
      else set.add(nodeId);
      renderTimelineAndBanner();
      renderGraphSvg();
      return;
    }
    const nodeEl = e.target.closest("[data-node-id]");
    if (nodeEl) {
      const nodeId = nodeEl.dataset.nodeId;
      const run = state.runsById.get(state.selectedRunId);
      if (run) {
        const matches = (run.steps || []).filter((s) => s.node_name === nodeId);
        if (matches.length) {
          state.selectedStepIndex = matches[matches.length - 1].step_index;
          renderAll();
        }
      }
    }
  });

  $("step-timeline").addEventListener("click", (e) => {
    const btn = e.target.closest("[data-step-idx]");
    if (btn) {
      state.selectedStepIndex = Number(btn.dataset.stepIdx);
      renderAll();
    }
  });

  $("btn-clear-bps").addEventListener("click", () => {
    state.breakpoints[state.activeAgent].clear();
    renderAll();
  });

  $("btn-download-trace").addEventListener("click", () => {
    if (state.selectedRunId) {
      window.location.href = `/debug/api/runs/${encodeURIComponent(state.selectedRunId)}/download`;
    }
  });

  $("btn-clear-runs").addEventListener("click", async () => {
    await apiJson("/debug/api/traces", { method: "DELETE" });
  });

  $("btn-seed-session").addEventListener("click", async () => {
    const sid = $("session-input").value.trim() || "debug-session-1";
    await apiJson(`/debug/api/sessions/${encodeURIComponent(sid)}/seed`, {
      method: "POST",
      body: JSON.stringify({ seed_config: { seed_default_character: true, seed_default_map: true } }),
    });
    await refreshSessionState();
  });

  $("btn-reset-session").addEventListener("click", async () => {
    const sid = $("session-input").value.trim() || "debug-session-1";
    await apiJson(`/debug/api/sessions/${encodeURIComponent(sid)}/reset`, { method: "POST" });
    await refreshSessionState();
  });

  async function refreshSessionState() {
    const sid = $("session-input").value.trim() || "debug-session-1";
    const snap = await apiJson(`/debug/api/sessions/${encodeURIComponent(sid)}/state`);
    $("session-state-view").textContent = JSON.stringify(snap, null, 2);
  }

  $("btn-refresh-state").addEventListener("click", refreshSessionState);

  $("btn-refresh-ollama").addEventListener("click", async () => {
    const data = await apiJson("/debug/api/ollama-logs?lines=120");
    $("logs-view").innerHTML = `<pre class="json-block">${escapeHtml((data.lines || []).join("\n") || "No Ollama logs buffered.")}</pre>`;
  });

  document.querySelectorAll("[data-workbench]").forEach((btn) => {
    btn.addEventListener("click", () => {
      document.querySelectorAll("[data-workbench]").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      const target = btn.dataset.workbench;
      $("wb-prompt").classList.toggle("hidden", target !== "prompt");
      $("wb-scenarios").classList.toggle("hidden", target !== "scenarios");
    });
  });

  document.querySelectorAll("[data-inspector]").forEach((btn) => {
    btn.addEventListener("click", () => {
      document.querySelectorAll("[data-inspector]").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      const target = btn.dataset.inspector;
      ["delta", "messages", "tools", "state", "logs"].forEach((tab) => {
        $(`insp-${tab}`).classList.toggle("hidden", tab !== target);
      });
      if (target === "state") refreshSessionState();
    });
  });

  $("btn-send-prompt").addEventListener("click", async () => {
    const btn = $("btn-send-prompt");
    btn.disabled = true;
    try {
      const target = $("prompt-target").value;
      const sid = $("session-input").value.trim() || "debug-session-1";
      const offline = $("chk-offline-mode").checked;
      const provider = $("prompt-provider").value || null;
      const modelId = $("prompt-model").value.trim() || null;
      const prompt = $("prompt-text").value;
      const actor = $("prompt-actor").value.trim() || "Mateo Vance";

      let res;
      if (target === "player") {
        setActiveAgent("ai_player");
        res = await apiJson("/debug/api/invoke/player", {
          method: "POST",
          body: JSON.stringify({
            session_id: sid,
            scene_prompt: prompt,
            player_display_name: actor,
            offline_mode: offline,
            provider,
            model_id: modelId,
            interrupt_before: Array.from(state.breakpoints.ai_player),
          }),
        });
      } else if (target === "scenario_beat") {
        setActiveAgent("ai_gm");
        res = await apiJson("/debug/api/invoke/scenario-beat", {
          method: "POST",
          body: JSON.stringify({
            session_id: sid,
            custom_context: prompt,
            operating_mode: $("prompt-op-mode").value,
            offline_mode: offline,
            provider,
            model_id: modelId,
            interrupt_before: Array.from(state.breakpoints.ai_gm),
          }),
        });
      } else {
        setActiveAgent("ai_gm");
        res = await apiJson("/debug/api/invoke/gm", {
          method: "POST",
          body: JSON.stringify({
            session_id: sid,
            prompt,
            actor_display_name: actor,
            operating_mode: $("prompt-op-mode").value,
            channel_context: $("prompt-channel").value,
            offline_mode: offline,
            provider,
            model_id: modelId,
            interrupt_before: Array.from(state.breakpoints.ai_gm),
          }),
        });
      }
      if (res && res.run) {
        state.runsById.set(res.run.run_id, res.run);
        if (!state.runOrder.includes(res.run.run_id)) state.runOrder.unshift(res.run.run_id);
        state.selectedRunId = res.run.run_id;
        renderAll();
      }
    } catch (err) {
      alert(`Invoke failed: ${err.message}`);
    } finally {
      btn.disabled = false;
    }
  });

  async function resumeRun(stepOnly, patch = null) {
    if (!state.selectedRunId) return;
    const run = state.runsById.get(state.selectedRunId);
    const bps = run ? Array.from(state.breakpoints[run.agent_kind] || []) : [];
    const res = await apiJson(`/debug/api/runs/${encodeURIComponent(state.selectedRunId)}/resume`, {
      method: "POST",
      body: JSON.stringify({
        step_only: stepOnly,
        interrupt_before: stepOnly ? null : bps,
        state_patch: patch,
      }),
    });
    if (res && res.run) {
      state.runsById.set(res.run.run_id, res.run);
      renderAll();
    }
  }

  $("btn-step-node").addEventListener("click", () => resumeRun(true));
  $("btn-continue-run").addEventListener("click", () => resumeRun(false));
  $("btn-toggle-patch").addEventListener("click", () => $("patch-drawer").classList.toggle("hidden"));
  $("btn-apply-patch-resume").addEventListener("click", async () => {
    try {
      const parsed = JSON.parse($("patch-json").value || "{}");
      await resumeRun(false, parsed);
      $("patch-drawer").classList.add("hidden");
    } catch (err) {
      alert(`Invalid JSON patch: ${err.message}`);
    }
  });
  $("btn-abort-run").addEventListener("click", async () => {
    if (!state.selectedRunId) return;
    const res = await apiJson(`/debug/api/runs/${encodeURIComponent(state.selectedRunId)}/abort`, { method: "POST" });
    if (res && res.run) {
      state.runsById.set(res.run.run_id, res.run);
      renderAll();
    }
  });

  $("scenario-select").addEventListener("change", syncScenarioEditor);

  $("btn-run-scenario").addEventListener("click", async () => {
    const btn = $("btn-run-scenario");
    btn.disabled = true;
    $("scenario-results").innerHTML = `<div class="empty-state">Running scenario...</div>`;
    try {
      const spec = JSON.parse($("scenario-json").value);
      const prov = $("scenario-provider").value;
      const res = await apiJson("/debug/api/scenarios/run", {
        method: "POST",
        body: JSON.stringify({
          scenario: spec,
          provider: prov || null,
          offline_mode: $("chk-offline-mode").checked,
        }),
      });
      state.lastScenarioResult = res;
      $("btn-export-scenario").disabled = false;

      const statusBadge = res.passed
        ? `<span class="pill pill-ok">PASS</span>`
        : `<span class="pill pill-err">FAIL</span>`;
      const rows = (res.assertion_results || [])
        .map(
          (a) => `<div class="assertion-row">
            <span><strong>${escapeHtml(a.kind)}</strong>: ${escapeHtml(a.detail)}</span>
            <span class="pill ${a.passed ? "pill-ok" : "pill-err"}">${a.passed ? "OK" : "FAIL"}</span>
          </div>`
        )
        .join("");
      const transcriptRows = (res.transcript || [])
        .map(
          (t) => `<div class="msg-card">
            <div class="msg-role">${escapeHtml(t.speaker)} (${escapeHtml(t.agent_kind)})</div>
            <div class="msg-content">${escapeHtml(t.reply)}</div>
          </div>`
        )
        .join("");

      $("scenario-results").innerHTML = `
        <div>${statusBadge} <strong>${escapeHtml(res.name)}</strong> <span class="hint">(${res.duration_ms}ms)</span></div>
        ${rows}
        ${transcriptRows}`;

      if ((res.run_ids || []).length) {
        await loadRunDetail(res.run_ids[res.run_ids.length - 1]);
      }
    } catch (err) {
      $("scenario-results").innerHTML = `<div class="pill pill-err">Error: ${escapeHtml(err.message)}</div>`;
    } finally {
      btn.disabled = false;
    }
  });

  $("btn-export-scenario").addEventListener("click", () => {
    if (!state.lastScenarioResult) return;
    const blob = new Blob([JSON.stringify(state.lastScenarioResult, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `scenario-${state.lastScenarioResult.scenario_id || "report"}.json`;
    a.click();
    URL.revokeObjectURL(url);
  });

  init();
})();
