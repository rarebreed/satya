(function () {
    const state = {
        authToken: null,
        user: null,
        sessionId: null,
        sessionMode: "TTRPG",
        joinToken: null,
        selfParticipant: null,
        roster: [],
        ws: null,
        localStream: null,
        audioMuted: false,
        videoMuted: false,
        peers: {}, // user_id -> { pc, dc }
        typingStartedAt: null,
        rmsPeak: 0.0,
        rmsSum: 0.0,
        rmsSamples: 0,
    };

    const rtcConfig = {
        iceServers: [{ urls: "stun:stun.l.google.com:19302" }]
    };

    function setFeedback(id, text, isError) {
        const el = document.getElementById(id);
        if (!el) return;
        el.textContent = text;
        el.style.color = isError ? "#f43f5e" : "#38bdf8";
    }

    window.satyaFillDemo = function (username, password, displayName) {
        document.getElementById("auth-username").value = username;
        document.getElementById("auth-password").value = password;
        document.getElementById("auth-display-name").value = displayName;
        if (username.startsWith("gm_")) {
            document.getElementById("session-mode-select").value = "TTRPG";
            document.getElementById("session-role-select").value = "GM";
        } else if (username.startsWith("host_")) {
            document.getElementById("session-mode-select").value = "MEETING";
            document.getElementById("session-role-select").value = "HOST";
        } else {
            document.getElementById("session-role-select").value = "PLAYER";
        }
    };

    window.satyaModeChanged = function () {
        const mode = document.getElementById("session-mode-select").value;
        const roleSelect = document.getElementById("session-role-select");
        const ttrpgDrawer = document.getElementById("ttrpg-drawer");
        if (mode === "MEETING") {
            roleSelect.value = "HOST";
            if (ttrpgDrawer) ttrpgDrawer.style.display = "none";
        } else {
            roleSelect.value = "GM";
            if (ttrpgDrawer) ttrpgDrawer.style.display = "block";
        }
    };

    window.satyaRegister = async function () {
        const username = document.getElementById("auth-username").value.trim();
        const password = document.getElementById("auth-password").value;
        const display_name = document.getElementById("auth-display-name").value.trim() || username;
        try {
            const res = await fetch("/api/auth/register", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ username, password, display_name })
            });
            const data = await res.json();
            if (!res.ok) throw new Error(data.error || "Registration failed");
            applyAuthSuccess(data);
            setFeedback("auth-feedback", `Registered & authenticated user_id: ${data.user_id}`, false);
        } catch (err) {
            setFeedback("auth-feedback", err.message, true);
        }
    };

    window.satyaLogin = async function () {
        const username = document.getElementById("auth-username").value.trim();
        const password = document.getElementById("auth-password").value;
        try {
            const res = await fetch("/api/auth/login", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({ username, password })
            });
            const data = await res.json();
            if (!res.ok) throw new Error(data.error || "Login failed");
            applyAuthSuccess(data);
            setFeedback("auth-feedback", `Authenticated as ${data.username} (user_id: ${data.user_id})`, false);
        } catch (err) {
            setFeedback("auth-feedback", err.message, true);
        }
    };

    function applyAuthSuccess(data) {
        state.authToken = data.auth_token;
        state.user = data;
        const pill = document.getElementById("auth-status-pill");
        if (pill) {
            pill.textContent = `User: ${data.username} (${data.user_id.slice(0, 8)}...)`;
            pill.className = "pill pill-active";
        }
    }

    window.satyaCreateSession = async function () {
        if (!state.authToken) {
            await window.satyaLogin();
            if (!state.authToken) return;
        }
        const session_id = document.getElementById("session-id-input").value.trim();
        const session_password = document.getElementById("session-password-input").value;
        const session_mode = document.getElementById("session-mode-select").value;
        const role = document.getElementById("session-role-select").value;
        const enable_ai_gm = document.getElementById("enable-ai-gm").checked;
        const display_name = document.getElementById("auth-display-name").value.trim();

        try {
            const res = await fetch("/api/sessions/create", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    "Authorization": `Bearer ${state.authToken}`
                },
                body: JSON.stringify({
                    session_id,
                    session_password,
                    session_mode,
                    role,
                    display_name: display_name || null,
                    enable_ai_gm
                })
            });
            const data = await res.json();
            if (!res.ok) throw new Error(data.error || "Could not create session");
            enterSession(data);
            setFeedback("session-feedback", `Created session '${data.session_id}' (${data.session_mode})`, false);
        } catch (err) {
            setFeedback("session-feedback", err.message, true);
        }
    };

    window.satyaJoinSession = async function () {
        if (!state.authToken) {
            await window.satyaLogin();
            if (!state.authToken) return;
        }
        const session_id = document.getElementById("session-id-input").value.trim();
        const session_password = document.getElementById("session-password-input").value;
        const role = document.getElementById("session-role-select").value;
        const display_name = document.getElementById("auth-display-name").value.trim();

        try {
            const res = await fetch("/api/sessions/join", {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                    "Authorization": `Bearer ${state.authToken}`
                },
                body: JSON.stringify({
                    session_id,
                    session_password,
                    role,
                    display_name: display_name || null
                })
            });
            const data = await res.json();
            if (!res.ok) throw new Error(data.error || "Could not join session");
            enterSession(data);
            setFeedback("session-feedback", `Joined session '${data.session_id}' as ${data.self_participant.role}`, false);
        } catch (err) {
            setFeedback("session-feedback", err.message, true);
        }
    };

    function enterSession(data) {
        state.sessionId = data.session_id;
        state.sessionMode = data.session_mode;
        state.joinToken = data.join_token;
        state.selfParticipant = data.self_participant;
        state.roster = data.roster || [];

        const pill = document.getElementById("session-status-pill");
        if (pill) {
            pill.textContent = `Session: ${state.sessionId} [${state.sessionMode} / ${state.selfParticipant.role}]`;
            pill.className = "pill pill-active";
        }
        const localLabel = document.getElementById("local-participant-label");
        if (localLabel) {
            localLabel.textContent = `${state.selfParticipant.display_name} (${state.selfParticipant.role})`;
        }

        renderRoster();
        connectSignalingWebSocket();
        window.satyaRefreshMetrics();
    }

    function renderRoster() {
        const listEl = document.getElementById("roster-list");
        const recipEl = document.getElementById("recipient-checkboxes");
        if (!listEl || !recipEl) return;

        listEl.innerHTML = "";
        recipEl.innerHTML = "";

        state.roster.forEach((p) => {
            const chip = document.createElement("span");
            chip.className = "participant-chip" + (p.participant_kind === "AI_AGENT" ? " ai" : "");
            chip.textContent = `${p.display_name} [${p.role}] (${p.user_id.slice(0, 8)})`;
            listEl.appendChild(chip);

            if (!state.selfParticipant || p.user_id !== state.selfParticipant.user_id) {
                const label = document.createElement("label");
                label.className = "recip-option";
                const cb = document.createElement("input");
                cb.type = "checkbox";
                cb.value = p.user_id;
                cb.dataset.username = p.username;
                cb.checked = true;
                label.appendChild(cb);
                const span = document.createElement("span");
                span.textContent = `${p.display_name} (${p.role})`;
                label.appendChild(span);
                recipEl.appendChild(label);
            }
        });
    }

    function connectSignalingWebSocket() {
        if (state.ws) {
            state.ws.close();
        }
        const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
        const url = `${proto}//${window.location.host}/ws/session?join_token=${encodeURIComponent(state.joinToken)}`;
        const ws = new WebSocket(url);
        state.ws = ws;

        ws.onopen = () => {
            appendSystemNotice(`Connected to WebRTC Signaling & Apache Iggy Event Hub for '${state.sessionId}'.`);
            initiateWebRtcMesh();
        };

        ws.onmessage = async (ev) => {
            try {
                const msg = JSON.parse(ev.data);
                await handleServerPacket(msg);
            } catch (e) {
                console.error("Failed to parse WS packet:", e);
            }
        };
    }

    async function handleServerPacket(pkt) {
        switch (pkt.type) {
            case "ROSTER_SYNC":
                state.roster = pkt.roster || [];
                renderRoster();
                initiateWebRtcMesh();
                window.satyaRefreshMetrics();
                break;
            case "CHAT_EVENT":
                renderChatEvent(pkt.event);
                window.satyaRefreshMetrics();
                break;
            case "TTRPG_MECHANICS_EVENT":
                renderMechanicsEvent(pkt.event);
                window.satyaRefreshMetrics();
                break;
            case "WEBRTC_SIGNAL":
                await handleWebRtcSignal(pkt);
                break;
            default:
                break;
        }
    }

    // ========================================================================
    // WebRTC P2P Audio/Video + DataChannel Mesh
    // ========================================================================
    window.satyaStartCamera = async function () {
        try {
            const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: true });
            state.localStream = stream;
            const videoEl = document.getElementById("local-video");
            if (videoEl) videoEl.srcObject = stream;
            startAudioProsodyMonitor(stream);
            initiateWebRtcMesh(true);
        } catch (err) {
            appendSystemNotice(`Camera/Mic access unavailable (${err.message}); continuing in DataChannel & signaling mode.`);
        }
    };

    window.satyaToggleAudio = function () {
        state.audioMuted = !state.audioMuted;
        if (state.localStream) {
            state.localStream.getAudioTracks().forEach(t => t.enabled = !state.audioMuted);
        }
        const btn = document.getElementById("btn-mute-audio");
        if (btn) btn.textContent = state.audioMuted ? "Unmute Mic" : "Mute Mic";
        sendWs({
            type: "WEBRTC_TELEMETRY",
            event_kind: "TRACK_MUTE_CHANGE",
            audio_muted: state.audioMuted,
            video_muted: state.videoMuted
        });
    };

    window.satyaToggleVideo = function () {
        state.videoMuted = !state.videoMuted;
        if (state.localStream) {
            state.localStream.getVideoTracks().forEach(t => t.enabled = !state.videoMuted);
        }
        const btn = document.getElementById("btn-mute-video");
        if (btn) btn.textContent = state.videoMuted ? "Unmute Video" : "Mute Video";
        sendWs({
            type: "WEBRTC_TELEMETRY",
            event_kind: "TRACK_MUTE_CHANGE",
            audio_muted: state.audioMuted,
            video_muted: state.videoMuted
        });
    };

    function startAudioProsodyMonitor(stream) {
        try {
            const AudioCtx = window.AudioContext || window.webkitAudioContext;
            if (!AudioCtx) return;
            const ctx = new AudioCtx();
            const source = ctx.createMediaStreamSource(stream);
            const analyser = ctx.createAnalyser();
            analyser.fftSize = 512;
            source.connect(analyser);
            const buf = new Float32Array(analyser.fftSize);

            setInterval(() => {
                analyser.getFloatTimeDomainData(buf);
                let sumSq = 0;
                for (let i = 0; i < buf.length; i++) {
                    sumSq += buf[i] * buf[i];
                }
                const rms = Math.sqrt(sumSq / buf.length);
                state.rmsSum += rms;
                state.rmsSamples += 1;
                if (rms > state.rmsPeak) state.rmsPeak = rms;
                const el = document.getElementById("vad-energy-readout");
                if (el) el.textContent = `RMS: ${rms.toFixed(3)}`;
            }, 250);
        } catch (_) {}
    }

    window.satyaEmitProsodySample = function () {
        if (!state.ws) return;
        const mean = state.rmsSamples > 0 ? (state.rmsSum / state.rmsSamples) : 0.18;
        const peak = state.rmsPeak > 0 ? state.rmsPeak : 0.44;
        state.rmsSum = 0;
        state.rmsSamples = 0;
        state.rmsPeak = 0;

        sendWs({
            type: "INTERACTION_DYNAMICS",
            turn_type: "SPEECH_END",
            duration_ms: 2450,
            rms_energy_mean: mean,
            rms_energy_peak: peak,
            pitch_hz_estimate: 148.0,
            preceding_silence_ms: 620
        });
        appendSystemNotice(`Logged InteractionDynamicsEvent (RMS mean=${mean.toFixed(2)}, peak=${peak.toFixed(2)}) to Apache Iggy.`);
        setTimeout(() => window.satyaRefreshMetrics(), 200);
    };

    function ensurePeerConnection(remoteUserId, remoteDisplayName, isInitiator) {
        if (state.peers[remoteUserId]) return state.peers[remoteUserId];

        const pc = new RTCPeerConnection(rtcConfig);
        const peerEntry = { pc, dc: null };
        state.peers[remoteUserId] = peerEntry;

        if (state.localStream) {
            state.localStream.getTracks().forEach(track => pc.addTrack(track, state.localStream));
        }

        pc.onicecandidate = (ev) => {
            if (ev.candidate) {
                sendWs({
                    type: "WEBRTC_SIGNAL",
                    target_user_id: remoteUserId,
                    signal_kind: "ICE_CANDIDATE",
                    payload: JSON.stringify(ev.candidate)
                });
            }
        };

        pc.ontrack = (ev) => {
            let tile = document.getElementById(`video-tile-${remoteUserId}`);
            if (!tile) {
                const grid = document.getElementById("video-grid");
                tile = document.createElement("div");
                tile.id = `video-tile-${remoteUserId}`;
                tile.className = "video-tile";
                tile.innerHTML = `
                    <video id="remote-video-${remoteUserId}" autoplay playsinline></video>
                    <div class="tile-overlay">
                        <span class="role-tag">${remoteDisplayName}</span>
                        <span class="telemetry-mini">P2P WebRTC</span>
                    </div>
                `;
                grid.appendChild(tile);
            }
            const vid = document.getElementById(`remote-video-${remoteUserId}`);
            if (vid && ev.streams && ev.streams[0]) {
                vid.srcObject = ev.streams[0];
            }
        };

        if (isInitiator) {
            const dc = pc.createDataChannel("satya-chat-dc");
            setupDataChannel(dc, remoteUserId);
            peerEntry.dc = dc;
            pc.createOffer().then(offer => {
                pc.setLocalDescription(offer);
                sendWs({
                    type: "WEBRTC_SIGNAL",
                    target_user_id: remoteUserId,
                    signal_kind: "SIGNALING_OFFER",
                    payload: JSON.stringify(offer)
                });
            }).catch(() => {});
        } else {
            pc.ondatachannel = (ev) => {
                peerEntry.dc = ev.channel;
                setupDataChannel(ev.channel, remoteUserId);
            };
        }

        return peerEntry;
    }

    function setupDataChannel(dc, remoteUserId) {
        dc.onopen = () => {
            sendWs({
                type: "WEBRTC_TELEMETRY",
                target_user_id: remoteUserId,
                event_kind: "DATA_CHANNEL_OPEN",
                audio_muted: state.audioMuted,
                video_muted: state.videoMuted
            });
        };
    }

    function initiateWebRtcMesh(renegotiate) {
        if (!state.selfParticipant) return;
        state.roster.forEach(p => {
            if (p.user_id !== state.selfParticipant.user_id && p.participant_kind === "HUMAN") {
                if (renegotiate && state.peers[p.user_id]) {
                    state.peers[p.user_id].pc.close();
                    delete state.peers[p.user_id];
                }
                const shouldInitiate = state.selfParticipant.user_id < p.user_id;
                ensurePeerConnection(p.user_id, p.display_name, shouldInitiate);
            }
        });
    }

    async function handleWebRtcSignal(pkt) {
        const fromUser = pkt.from_participant;
        if (!fromUser) return;
        const peer = ensurePeerConnection(fromUser.user_id, fromUser.display_name, false);
        const data = JSON.parse(pkt.payload);

        if (pkt.signal_kind === "SIGNALING_OFFER") {
            await peer.pc.setRemoteDescription(new RTCSessionDescription(data));
            const answer = await peer.pc.createAnswer();
            await peer.pc.setLocalDescription(answer);
            sendWs({
                type: "WEBRTC_SIGNAL",
                target_user_id: fromUser.user_id,
                signal_kind: "SIGNALING_ANSWER",
                payload: JSON.stringify(answer)
            });
        } else if (pkt.signal_kind === "SIGNALING_ANSWER") {
            await peer.pc.setRemoteDescription(new RTCSessionDescription(data));
        } else if (pkt.signal_kind === "ICE_CANDIDATE") {
            try {
                await peer.pc.addIceCandidate(new RTCIceCandidate(data));
            } catch (_) {}
        }
    }

    // ========================================================================
    // Public & Private Targeted Chat + TTRPG Task Roller
    // ========================================================================
    window.satyaVisibilityChanged = function () {
        const vis = document.getElementById("chat-visibility").value;
        const picker = document.getElementById("private-recipient-picker");
        if (picker) {
            picker.style.display = vis === "PRIVATE" ? "block" : "none";
        }
    };

    window.satyaChatKeydown = function (ev) {
        if (!state.typingStartedAt) {
            state.typingStartedAt = Date.now();
        }
        if (ev.key === "Enter") {
            window.satyaSendChat();
        }
    };

    window.satyaSendChat = function () {
        const input = document.getElementById("chat-input");
        const body = input.value.trim();
        if (!body || !state.ws) return;

        const visibility = document.getElementById("chat-visibility").value;
        const channel_context = document.getElementById("chat-context").value;
        const composition_time_ms = state.typingStartedAt ? (Date.now() - state.typingStartedAt) : 450;
        state.typingStartedAt = null;

        const recipient_user_ids = [];
        if (visibility === "PRIVATE") {
            document.querySelectorAll("#recipient-checkboxes input[type='checkbox']:checked").forEach(cb => {
                recipient_user_ids.push(cb.value);
            });
        }

        sendWs({
            type: "SEND_CHAT",
            visibility,
            recipient_user_ids,
            channel_context,
            body,
            composition_time_ms
        });
        input.value = "";
    };

    window.satyaRollTask = function () {
        if (!state.ws) return;
        const skill_name = document.getElementById("ttrpg-skill").value.trim() || "Athletics";
        const attribute_tn = parseInt(document.getElementById("ttrpg-tn").value, 10) || 12;
        const pool_size = parseInt(document.getElementById("ttrpg-pool").value, 10) || 3;
        const difficulty_rating = parseInt(document.getElementById("ttrpg-diff").value, 10) || 2;
        const stanceVal = document.getElementById("ttrpg-stance").value;
        const fatigue_level = parseInt(document.getElementById("ttrpg-fatigue").value, 10) || 0;

        sendWs({
            type: "ROLL_TTRPG_TASK",
            skill_name,
            attribute_tn,
            pool_size,
            difficulty_rating,
            combat_stance: stanceVal ? stanceVal : null,
            fatigue_level
        });
    };

    function renderChatEvent(ev) {
        const log = document.getElementById("chat-log");
        if (!log) return;
        const div = document.createElement("div");
        const isPrivate = ev.visibility !== "PUBLIC";
        const isAi = ev.sender.participant_kind === "AI_AGENT";
        div.className = "chat-msg" + (isPrivate ? " private" : "") + (isAi ? " ai-gm" : "");

        const recipNames = (ev.recipients || []).map(r => r.display_name).join(", ");
        const visLabel = ev.visibility === "PUBLIC"
            ? "PUBLIC"
            : `${ev.visibility} -> [${recipNames}]`;

        div.innerHTML = `
            <div class="chat-meta">
                <span><strong>${ev.sender.display_name}</strong> (${ev.sender.role}) · ${ev.channel_context}</span>
                <span>${visLabel} · schema v${ev.metadata.schema_version}</span>
            </div>
            <div>${escapeHtml(ev.body)}</div>
        `;
        log.appendChild(div);
        log.scrollTop = log.scrollHeight;
    }

    function renderMechanicsEvent(ev) {
        const log = document.getElementById("chat-log");
        if (!log) return;
        const div = document.createElement("div");
        div.className = "chat-msg mechanics";
        const rolls = (ev.dice_rolled_d20 || []).join(", ");
        const stanceInfo = ev.combat_stance ? ` · Stance: ${ev.combat_stance} (Action Cost: ${ev.action_count_cost})` : "";
        div.innerHTML = `
            <div class="chat-meta">
                <span><strong>[SATYA MECHANICS] ${ev.actor.display_name}</strong> · ${ev.skill_name || "Action"}</span>
                <span>Outcome: ${ev.outcome_tier}</span>
            </div>
            <div>Rolled d20s: [${rolls}] vs TN ${ev.attribute_tn} (Diff ${ev.difficulty_rating}) => <strong>${ev.successes_counted} successes</strong>${stanceInfo}</div>
        `;
        log.appendChild(div);
        log.scrollTop = log.scrollHeight;
    }

    function appendSystemNotice(text) {
        const log = document.getElementById("chat-log");
        if (!log) return;
        const div = document.createElement("div");
        div.className = "chat-msg";
        div.innerHTML = `<div class="chat-meta"><span>SYSTEM</span><span>LOCAL</span></div><div>${escapeHtml(text)}</div>`;
        log.appendChild(div);
        log.scrollTop = log.scrollHeight;
    }

    function sendWs(obj) {
        if (state.ws && state.ws.readyState === WebSocket.OPEN) {
            state.ws.send(JSON.stringify(obj));
        }
    }

    // ========================================================================
    // Apache Iggy Topic & Avro Schema Inspector
    // ========================================================================
    window.satyaRefreshMetrics = async function () {
        try {
            const res = await fetch("/api/storage/metrics");
            const data = await res.json();
            renderMetricsTable(data.topics || []);
        } catch (_) {}
    };

    window.satyaFlushBatches = async function () {
        try {
            const res = await fetch("/api/storage/flush", { method: "POST" });
            const data = await res.json();
            appendSystemNotice(`Flushed ${data.flushed_files.length} offset-tracked Avro container batch(es) for PyLance ingestion.`);
            await window.satyaRefreshMetrics();
        } catch (_) {}
    };

    function renderMetricsTable(topics) {
        const wrap = document.getElementById("topic-metrics-table");
        if (!wrap) return;
        let rows = topics.map(t => `
            <tr>
                <td><strong>${t.topic}</strong></td>
                <td><span class="mono">v${t.active_schema_version} (${t.active_schema_fingerprint_hex.slice(0, 10)}...)</span></td>
                <td>${t.storage_backend.toUpperCase()} (<span class="mono">${t.destination_uri}</span>)</td>
                <td>${t.total_messages_ingested}</td>
                <td>${t.buffered_messages}</td>
                <td>${t.flushed_batches_count} (last offset: ${t.last_flushed_offset !== null ? t.last_flushed_offset : "-"})</td>
            </tr>
        `).join("");

        wrap.innerHTML = `
            <table class="metrics-table">
                <thead>
                    <tr>
                        <th>Apache Iggy Topic</th>
                        <th>Schema Ver & CRC-64</th>
                        <th>Configurable Storage Sink</th>
                        <th>Ingested Events</th>
                        <th>Buffered</th>
                        <th>Flushed .avro Batches</th>
                    </tr>
                </thead>
                <tbody>${rows}</tbody>
            </table>
        `;
    }

    function escapeHtml(str) {
        return String(str)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
    }

    document.addEventListener("DOMContentLoaded", () => {
        window.satyaVisibilityChanged();
        window.satyaRefreshMetrics();
    });
})();
