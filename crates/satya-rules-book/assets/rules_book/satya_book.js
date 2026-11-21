/* ============================================================================
 * Satya TTRPG Rules Book — Deterministic 2-Column Paginator & Book Controller
 * ============================================================================ */

(function () {
  "use strict";

  const THEME_KEY = "satya.book.theme";
  const VIEW_KEY = "satya.book.view";
  const TOC_KEY = "satya.book.toc";
  const VALID_THEMES = new Set(["sangha-robes", "chan-sanctuary", "tactical-hud"]);
  const VALID_VIEWS = new Set(["spread", "single", "scroll"]);
  const MAX_PAGES_PER_CHAPTER = 120;

  let bookPages = [];
  let currentPageIdx = 0;
  let headingPageMap = new Map();
  let widgetEl = null;
  let widgetPlaceholder = null;
  let widgetPageBand = null;

  function meta() {
    const ds = document.body.dataset;
    return {
      partIndex: ds.partIndex || "1",
      partLabel: ds.partLabel || "Part I",
      chapterNum: ds.chapterNum || "0",
      chapterTitle: ds.chapterTitle || document.title,
      chapterId: ds.chapterId || "index",
    };
  }

  function resolveAssetUrls(root) {
    if (!root) return;
    const isHttpBook =
      (window.location.protocol === "http:" || window.location.protocol === "https:") &&
      window.location.pathname.includes("/book");
    const isFile = window.location.protocol === "file:";
    const imgs = root.querySelectorAll("img");
    for (const img of imgs) {
      img.removeAttribute("loading");
      img.loading = "eager";
      const raw = (img.getAttribute("src") || "").trim();
      if (!raw) continue;
      if (isHttpBook && raw.startsWith("../assets/")) {
        img.setAttribute("src", "/docs-assets/" + raw.slice("../assets/".length));
      } else if (isHttpBook && raw.startsWith("../../assets/")) {
        img.setAttribute("src", "/docs-assets/" + raw.slice("../../assets/".length));
      } else if (isFile && raw.startsWith("/docs-assets/")) {
        img.setAttribute("src", "../assets/" + raw.slice("/docs-assets/".length));
      }
    }
  }

  function waitForImages(root, timeoutMs) {
    const imgs = Array.from(root.querySelectorAll("img"));
    if (imgs.length === 0) return Promise.resolve();
    const recordDims = (img, w, h) => {
      if (w > 0 && h > 0) {
        img.dataset.natW = String(w);
        img.dataset.natH = String(h);
        img.setAttribute("width", String(w));
        img.setAttribute("height", String(h));
      }
    };
    const waitOne = (img) =>
      new Promise((resolve) => {
        if (img.complete && img.naturalWidth > 0) {
          recordDims(img, img.naturalWidth, img.naturalHeight);
          resolve();
          return;
        }
        const probe = new Image();
        let settled = false;
        const finish = () => {
          if (settled) return;
          settled = true;
          probe.onload = null;
          probe.onerror = null;
          recordDims(img, probe.naturalWidth || img.naturalWidth, probe.naturalHeight || img.naturalHeight);
          resolve();
        };
        probe.onload = finish;
        probe.onerror = finish;
        probe.src = img.currentSrc || img.src;
        if (probe.complete && probe.naturalWidth > 0) {
          finish();
        }
      });
    const timeout = new Promise((resolve) => window.setTimeout(resolve, timeoutMs || 2500));
    return Promise.race([Promise.all(imgs.map(waitOne)), timeout]);
  }

  function applyAutoImageLayouts(root) {
    const figures = root.querySelectorAll("figure.rules-figure");
    for (const fig of figures) {
      if (fig.dataset.layout) continue;
      const img = fig.querySelector("img");
      let layout = "half";
      const w = img ? Number(img.dataset.natW) || img.naturalWidth : 0;
      const h = img ? Number(img.dataset.natH) || img.naturalHeight : 0;
      if (w > 0 && h > 0) {
        const ratio = w / h;
        if (ratio <= 0.65) {
          layout = "column";
        } else if (ratio <= 1.2) {
          layout = "quarter";
        } else {
          layout = "half";
        }
      }
      fig.dataset.layout = layout;
      fig.classList.add("rules-figure--" + layout);
    }
  }

  function collectFlowNodes(sourceEl) {
    const out = [];
    for (const child of Array.from(sourceEl.children)) {
      const tag = child.tagName;
      if (tag === "SCRIPT" || tag === "STYLE") continue;
      if (tag === "ARTICLE" && child.classList.contains("satya-rules-article")) {
        for (const inner of Array.from(child.children)) {
          if (inner.tagName === "SCRIPT" || inner.tagName === "STYLE") continue;
          out.push(inner);
        }
      } else {
        out.push(child);
      }
    }
    return out;
  }

  function buildPageShell(extraClass, pageInfo) {
    const page = document.createElement("article");
    page.className = "book-page" + (extraClass ? " " + extraClass : "");

    const tab = document.createElement("div");
    tab.className = "page-part-tab";
    tab.textContent = "PART " + romanNumeral(pageInfo.partIndex);
    page.appendChild(tab);

    const header = document.createElement("header");
    header.className = "page-header";
    const hLeft = document.createElement("span");
    hLeft.className = "page-header-left";
    const hRight = document.createElement("span");
    hRight.className = "page-header-right";
    header.appendChild(hLeft);
    header.appendChild(hRight);
    page.appendChild(header);

    const body = document.createElement("div");
    body.className = "page-body";
    page.appendChild(body);

    const footer = document.createElement("footer");
    footer.className = "page-footer";
    const fLeft = document.createElement("span");
    fLeft.className = "page-footer-left";
    const folio = document.createElement("span");
    folio.className = "page-folio";
    const fRight = document.createElement("span");
    fRight.className = "page-footer-right";
    footer.appendChild(fLeft);
    footer.appendChild(folio);
    footer.appendChild(fRight);
    page.appendChild(footer);

    return {
      el: page,
      body,
      headerLeft: hLeft,
      headerRight: hRight,
      footerLeft: fLeft,
      footerRight: fRight,
      folio,
      colsBand: null,
      colLeft: null,
      colRight: null,
      activeCol: 0,
      hasContent: false,
      maxBottom: 0,
    };
  }

  function romanNumeral(val) {
    const map = { "1": "I", "2": "II", "3": "III", "4": "IV", "5": "V", "6": "VI" };
    return map[String(val)] || String(val);
  }

  function finalizeHeadersAndFolios(pages, info) {
    pages.forEach((p, idx) => {
      const num = idx + 1;
      const isVerso = idx % 2 === 0;
      p.el.classList.remove("book-page--verso", "book-page--recto");
      p.el.classList.add(isVerso ? "book-page--verso" : "book-page--recto");
      p.el.dataset.pageNumber = String(num);
      p.folio.textContent = String(num).padStart(2, "0");

      const chPrefix = info.chapterNum === "0" ? "INDEX" : "CH. " + info.chapterNum;
      if (isVerso) {
        p.headerLeft.textContent = "SATYA // CORE RULES";
        p.headerRight.textContent = info.partLabel;
        p.footerLeft.textContent = "GROUNDED 2035 CARPG";
        p.footerRight.textContent = chPrefix;
      } else {
        p.headerLeft.textContent = chPrefix;
        p.headerRight.textContent = info.chapterTitle;
        p.footerLeft.textContent = info.partLabel;
        p.footerRight.textContent = "SATYA // RULES BOOK";
      }
    });
  }

  function getMaxBottom(pageObj) {
    const rect = pageObj.body.getBoundingClientRect();
    return rect.top + pageObj.body.clientHeight - 10;
  }

  function getContainerContentBottom(container) {
    const last = container.lastElementChild;
    if (!last) {
      return container.getBoundingClientRect().top;
    }
    return last.getBoundingClientRect().bottom;
  }

  function getActiveColumn(pageObj) {
    return pageObj.activeCol === 0 ? pageObj.colLeft : pageObj.colRight;
  }

  function isSpanElement(node, colWidthPx) {
    const tag = node.tagName;
    if (tag === "H1") return true;
    if (node.classList.contains("book-interactive-widget")) return true;
    if (node.classList.contains("rules-mermaid")) return true;
    if (tag === "FIGURE") {
      const l = node.dataset.layout || "";
      if (l === "half" || l === "half-top" || l === "half-bottom") return true;
    }
    if (node.classList.contains("rules-table-wrap")) {
      const table = node.querySelector("table");
      if (!table) return false;
      const firstRow = table.querySelector("tr");
      const colCount = firstRow ? firstRow.children.length : 0;
      if (colCount >= 5) return true;
      if (colWidthPx > 0 && table.scrollWidth > colWidthPx + 8) return true;
      if (colCount >= 4) {
        const textLen = (table.textContent || "").length;
        const rowCount = Math.max(1, table.querySelectorAll("tr").length);
        if (textLen / (rowCount * colCount) > 16) return true;
      }
    }
    return false;
  }

  function cloneElementForFlow(node) {
    if (node.classList.contains("book-interactive-widget")) {
      return node;
    }
    return node.cloneNode(true);
  }

  function createArtPlatePage(figNode, kind, info, host) {
    const plateClass =
      kind === "full"
        ? "book-page--full-art"
        : kind === "spread-left"
          ? "book-page--spread-left"
          : "book-page--spread-right";
    const pageObj = buildPageShell(plateClass, info);
    const plate = document.createElement("div");
    plate.className = "page-art-plate";

    const origImg = figNode.querySelector("img");
    if (origImg) {
      const img = document.createElement("img");
      img.src = origImg.getAttribute("src") || origImg.src;
      img.alt = origImg.getAttribute("alt") || "";
      plate.appendChild(img);
    }

    const origCap = figNode.querySelector("figcaption");
    if (origCap && kind !== "spread-left") {
      const cap = document.createElement("div");
      cap.className = "page-art-caption";
      cap.innerHTML = origCap.innerHTML;
      plate.appendChild(cap);
    }

    pageObj.body.appendChild(plate);
    pageObj.hasContent = true;
    host.appendChild(pageObj.el);
    return pageObj;
  }

  function createFieldNotesPage(info, host) {
    const pageObj = buildPageShell("book-page--field-notes", info);
    const banner = document.createElement("div");
    banner.className = "field-notes-banner";
    banner.innerHTML =
      "<span>Operations Field Log // Margin Notes</span><span>" +
      info.chapterTitle.replace(/</g, "&lt;") +
      "</span>";
    pageObj.body.appendChild(banner);
    pageObj.hasContent = true;
    host.appendChild(pageObj.el);
    return pageObj;
  }

  function classifyTableWrap(wrapNode) {
    const table = wrapNode ? wrapNode.querySelector("table") : null;
    if (!table) return "normal";
    const firstRow = table.querySelector("tr");
    const colCount = firstRow ? firstRow.children.length : 0;
    const tier = colCount >= 11 ? "ultra" : colCount >= 8 ? "wide" : "normal";
    wrapNode.dataset.cols = tier;
    return tier;
  }

  function fitTableZoomIfNeeded(wrapNode) {
    const tbl = wrapNode ? wrapNode.querySelector("table") : null;
    if (!tbl) return;
    tbl.style.zoom = "";
    const availW = wrapNode.clientWidth;
    if (availW > 0 && tbl.scrollWidth > availW + 2) {
      const z = Math.max(0.62, (availW - 2) / tbl.scrollWidth);
      tbl.style.zoom = z.toFixed(3);
    }
  }

  function buildTableChunk(origWrap, headerRow, bodyRows) {
    const wrap = document.createElement("div");
    wrap.className = origWrap.className;
    const tbl = document.createElement("table");
    tbl.className = "rules-table";
    if (headerRow) {
      const thead = document.createElement("thead");
      thead.appendChild(headerRow.cloneNode(true));
      tbl.appendChild(thead);
    }
    const tbody = document.createElement("tbody");
    for (const r of bodyRows) {
      tbody.appendChild(r.cloneNode(true));
    }
    tbl.appendChild(tbody);
    wrap.appendChild(tbl);
    classifyTableWrap(wrap);
    return wrap;
  }

  function splitParagraphNode(pClone, container, maxBottom) {
    const children = Array.from(pClone.childNodes);
    if (children.length === 0) return null;

    const firstPart = document.createElement("p");
    firstPart.className = pClone.className;
    const secondPart = document.createElement("p");
    container.appendChild(firstPart);

    let splitDone = false;
    for (let i = 0; i < children.length; i++) {
      const ch = children[i];
      if (splitDone) {
        secondPart.appendChild(ch.cloneNode(true));
        continue;
      }
      if (ch.nodeType === Node.TEXT_NODE) {
        const txt = ch.textContent || "";
        const testText = document.createTextNode(txt);
        firstPart.appendChild(testText);
        if (firstPart.getBoundingClientRect().bottom <= maxBottom) {
          continue;
        }
        firstPart.removeChild(testText);
        const tokens = txt.split(/(\s+)/);
        let lo = 0;
        let hi = tokens.length;
        let best = 0;
        while (lo <= hi) {
          const mid = Math.floor((lo + hi) / 2);
          const candidate = document.createTextNode(tokens.slice(0, mid).join(""));
          firstPart.appendChild(candidate);
          const fits = firstPart.getBoundingClientRect().bottom <= maxBottom;
          firstPart.removeChild(candidate);
          if (fits) {
            best = mid;
            lo = mid + 1;
          } else {
            hi = mid - 1;
          }
        }
        const headText = tokens.slice(0, best).join("").trimEnd();
        const tailText = tokens.slice(best).join("").trimStart();
        if (headText) {
          firstPart.appendChild(document.createTextNode(headText));
        }
        if (tailText) {
          secondPart.appendChild(document.createTextNode(tailText));
        }
        splitDone = true;
      } else {
        const cloneCh = ch.cloneNode(true);
        firstPart.appendChild(cloneCh);
        if (firstPart.getBoundingClientRect().bottom > maxBottom) {
          firstPart.removeChild(cloneCh);
          secondPart.appendChild(ch.cloneNode(true));
          splitDone = true;
        }
      }
    }

    if (!firstPart.textContent || firstPart.textContent.trim().length === 0) {
      container.removeChild(firstPart);
      return null;
    }
    if (!secondPart.hasChildNodes() || !secondPart.textContent.trim()) {
      return { placed: firstPart, remainder: null };
    }
    return { placed: firstPart, remainder: secondPart };
  }

  function paginateBook() {
    const sourceEl = document.getElementById("book-source");
    const stageEl = document.getElementById("book-stage");
    if (!sourceEl || !stageEl) return;

    // Preserve live interactive calculator widget across re-paginations
    if (!widgetEl) {
      widgetEl = sourceEl.querySelector(".book-interactive-widget");
      if (widgetEl && widgetEl.parentElement) {
        widgetPlaceholder = document.createElement("div");
        widgetPlaceholder.id = "book-widget-anchor";
        widgetPlaceholder.style.display = "none";
        widgetEl.parentElement.insertBefore(widgetPlaceholder, widgetEl);
      }
    }

    const info = meta();
    const rawNodes = collectFlowNodes(sourceEl);
    if (rawNodes.length === 0) return;

    // Mark first body paragraph after h1 for drop-cap treatment
    let seenH1 = false;
    let markedDropcap = false;
    for (const node of rawNodes) {
      if (node.tagName === "H1") {
        seenH1 = true;
        continue;
      }
      if (seenH1 && !markedDropcap && node.tagName === "P") {
        const txt = (node.textContent || "").trim();
        if (txt.length > 40 && /^[A-Za-z]/.test(txt)) {
          node.classList.add("chapter-dropcap");
          markedDropcap = true;
        }
      }
    }

    const host = document.createElement("div");
    host.className = "book-pages-host book-pages-host--measuring";
    document.body.appendChild(host);

    const pages = [];
    const pendingPlates = [];
    let pageObj = null;

    function closeColsBandIfOpen() {
      if (!pageObj || !pageObj.colsBand) return;
      const leftEmpty = pageObj.colLeft.children.length === 0;
      const rightEmpty = pageObj.colRight.children.length === 0;
      if (leftEmpty && rightEmpty) {
        pageObj.body.removeChild(pageObj.colsBand);
      }
      pageObj.colsBand = null;
      pageObj.colLeft = null;
      pageObj.colRight = null;
    }

    function newPage() {
      if (pages.length >= MAX_PAGES_PER_CHAPTER) {
        return pageObj || pages[pages.length - 1];
      }
      if (pageObj && !pageObj.hasContent) {
        return pageObj;
      }
      if (pages.length > 0) {
        flushPendingPlates();
      }
      pageObj = buildPageShell("", info);
      host.appendChild(pageObj.el);
      pageObj.maxBottom = getMaxBottom(pageObj);
      pages.push(pageObj);
      return pageObj;
    }

    function flushPendingPlates() {
      while (pendingPlates.length > 0 && pages.length < MAX_PAGES_PER_CHAPTER) {
        const item = pendingPlates.shift();
        if (item.kind === "full") {
          const p = createArtPlatePage(item.node, "full", info, host);
          pages.push(p);
        } else if (item.kind === "spread") {
          // Ensure left spread page starts on an even 0-based index (verso)
          if (pages.length % 2 === 1 && pages.length + 2 < MAX_PAGES_PER_CHAPTER) {
            const spacer = createFieldNotesPage(info, host);
            pages.push(spacer);
          }
          pages.push(createArtPlatePage(item.node, "spread-left", info, host));
          pages.push(createArtPlatePage(item.node, "spread-right", info, host));
        }
      }
    }

    function ensureColsBand() {
      if (!pageObj) newPage();
      if (!pageObj.colsBand) {
        const rem = pageObj.maxBottom - getContainerContentBottom(pageObj.body);
        if (pageObj.hasContent && rem < 78) {
          newPage();
        }
        const band = document.createElement("div");
        band.className = "page-cols-band";
        const left = document.createElement("div");
        left.className = "page-col-left";
        const right = document.createElement("div");
        right.className = "page-col-right";
        band.appendChild(left);
        band.appendChild(right);
        pageObj.body.appendChild(band);
        pageObj.colsBand = band;
        pageObj.colLeft = left;
        pageObj.colRight = right;
        pageObj.activeCol = 0;
      }
    }

    function advanceColumn() {
      if (!pageObj) {
        ensureColsBand();
        return;
      }
      if (pageObj.colsBand && pageObj.activeCol === 0) {
        pageObj.activeCol = 1;
      } else {
        pageObj = null;
        ensureColsBand();
      }
    }

    function placeSpanNode(cloneNode) {
      if (!pageObj) newPage();
      closeColsBandIfOpen();

      // Handle table splitting for full-width tables
      if (cloneNode.classList && cloneNode.classList.contains("rules-table-wrap")) {
        placeTableAcrossContainer(cloneNode, true);
        return;
      }

      const spanBand = document.createElement("div");
      spanBand.className = "page-span-band";
      spanBand.appendChild(cloneNode);
      if (cloneNode.classList && cloneNode.classList.contains("book-interactive-widget")) {
        widgetPageBand = spanBand;
      }
      const hadPriorContent = pageObj.hasContent;
      pageObj.body.appendChild(spanBand);

      const bottom = spanBand.getBoundingClientRect().bottom;
      let movedToFreshPage = false;
      if (bottom > pageObj.maxBottom && hadPriorContent) {
        pageObj.body.removeChild(spanBand);
        newPage();
        pageObj.body.appendChild(spanBand);
        movedToFreshPage = true;
      }
      pageObj.hasContent = true;

      if (
        cloneNode.dataset &&
        cloneNode.dataset.layout === "half-bottom" &&
        hadPriorContent &&
        !movedToFreshPage
      ) {
        spanBand.style.marginTop = "auto";
        pageObj = null;
      }
    }

    function placeTableAcrossContainer(wrapNode, isSpan) {
      classifyTableWrap(wrapNode);
      const origTable = wrapNode.querySelector("table");
      if (!origTable) return;
      const headRow = origTable.querySelector("thead tr");
      const allRows = Array.from(origTable.querySelectorAll("tbody tr"));
      if (allRows.length <= 1) {
        if (isSpan) {
          const holder = document.createElement("div");
          holder.className = "page-span-band";
          holder.appendChild(wrapNode);
          pageObj.body.appendChild(holder);
          fitTableZoomIfNeeded(wrapNode);
          if (holder.getBoundingClientRect().bottom > pageObj.maxBottom && pageObj.hasContent) {
            pageObj.body.removeChild(holder);
            newPage();
            pageObj.body.appendChild(holder);
            fitTableZoomIfNeeded(wrapNode);
          }
          pageObj.hasContent = true;
        } else {
          placeSimpleFlowNode(wrapNode);
          fitTableZoomIfNeeded(wrapNode);
        }
        return;
      }

      let rowIdx = 0;
      let guard = 0;
      while (rowIdx < allRows.length && guard < MAX_PAGES_PER_CHAPTER * 3) {
        guard++;
        let targetParent;
        if (isSpan) {
          if (!pageObj) newPage();
          closeColsBandIfOpen();
          const spanBand = document.createElement("div");
          spanBand.className = "page-span-band";
          pageObj.body.appendChild(spanBand);
          targetParent = spanBand;
        } else {
          ensureColsBand();
          targetParent = getActiveColumn(pageObj);
        }

        const chunkWrap = buildTableChunk(wrapNode, headRow, []);
        const tbody = chunkWrap.querySelector("tbody");
        targetParent.appendChild(chunkWrap);
        fitTableZoomIfNeeded(chunkWrap);

        let added = 0;
        while (rowIdx < allRows.length) {
          const rowClone = allRows[rowIdx].cloneNode(true);
          tbody.appendChild(rowClone);
          fitTableZoomIfNeeded(chunkWrap);
          if (chunkWrap.getBoundingClientRect().bottom > pageObj.maxBottom) {
            tbody.removeChild(rowClone);
            fitTableZoomIfNeeded(chunkWrap);
            break;
          }
          added++;
          rowIdx++;
        }

        if (added === 0) {
          targetParent.removeChild(chunkWrap);
          const containerEmpty = isSpan
            ? !pageObj.hasContent
            : targetParent.children.length === 0;
          if (containerEmpty) {
            // Single row taller than empty column/page: force-place 1 row so we always advance
            const single = buildTableChunk(wrapNode, headRow, [allRows[rowIdx]]);
            targetParent.appendChild(single);
            fitTableZoomIfNeeded(single);
            pageObj.hasContent = true;
            rowIdx++;
          }
          if (isSpan) {
            if (targetParent.children.length === 0 && targetParent.parentElement) {
              targetParent.parentElement.removeChild(targetParent);
            }
            pageObj = null;
          } else {
            advanceColumn();
          }
        } else {
          pageObj.hasContent = true;
          if (rowIdx < allRows.length) {
            if (isSpan) {
              pageObj = null;
            } else {
              advanceColumn();
            }
          }
        }
      }
    }

    function placeListAcrossFlow(listNode) {
      const items = Array.from(listNode.children).filter((el) => el.tagName === "LI");
      if (items.length <= 1) {
        placeSimpleFlowNode(listNode);
        return;
      }
      const isOrdered = listNode.tagName === "OL";
      let startNum = parseInt(listNode.getAttribute("start") || "1", 10) || 1;
      let idx = 0;
      let guard = 0;
      while (idx < items.length && guard < MAX_PAGES_PER_CHAPTER * 3) {
        guard++;
        ensureColsBand();
        const col = getActiveColumn(pageObj);
        const subList = document.createElement(listNode.tagName.toLowerCase());
        subList.className = listNode.className;
        if (isOrdered && startNum > 1) {
          subList.setAttribute("start", String(startNum));
        }
        col.appendChild(subList);

        let added = 0;
        while (idx < items.length) {
          const liClone = items[idx].cloneNode(true);
          subList.appendChild(liClone);
          if (subList.getBoundingClientRect().bottom > pageObj.maxBottom) {
            subList.removeChild(liClone);
            break;
          }
          added++;
          idx++;
          startNum++;
        }

        if (added === 0) {
          col.removeChild(subList);
          if (col.children.length === 0) {
            const forced = document.createElement(listNode.tagName.toLowerCase());
            forced.className = listNode.className;
            if (isOrdered && startNum > 1) forced.setAttribute("start", String(startNum));
            forced.appendChild(items[idx].cloneNode(true));
            col.appendChild(forced);
            pageObj.hasContent = true;
            idx++;
            startNum++;
          }
          advanceColumn();
        } else {
          pageObj.hasContent = true;
          if (idx < items.length) {
            advanceColumn();
          }
        }
      }
    }

    function placeSimpleFlowNode(node) {
      ensureColsBand();
      let col = getActiveColumn(pageObj);

      // Orphan guard for section headings (`H2`, `H3`, `H4`)
      if (/^H[2-4]$/.test(node.tagName)) {
        const rem = pageObj.maxBottom - getContainerContentBottom(col);
        if (rem < 98 && col.children.length > 0) {
          advanceColumn();
          col = getActiveColumn(pageObj);
        }
      }

      col.appendChild(node);
      if (node.getBoundingClientRect().bottom <= pageObj.maxBottom) {
        pageObj.hasContent = true;
        return;
      }

      // Overflowed active column
      col.removeChild(node);
      const remaining = pageObj.maxBottom - getContainerContentBottom(col);

      // Split tall paragraphs cleanly when ample space remains or paragraph exceeds full column
      if (node.tagName === "P" && (remaining > 135 || col.children.length === 0)) {
        const splitRes = splitParagraphNode(node, col, pageObj.maxBottom);
        if (splitRes && splitRes.placed) {
          pageObj.hasContent = true;
          if (splitRes.remainder) {
            advanceColumn();
            placeSimpleFlowNode(splitRes.remainder);
          }
          return;
        }
      }

      if (col.children.length === 0) {
        // Already at top of an empty column: keep node rather than dropping content
        col.appendChild(node);
        pageObj.hasContent = true;
        advanceColumn();
        return;
      }

      advanceColumn();
      col = getActiveColumn(pageObj);
      col.appendChild(node);
      if (node.getBoundingClientRect().bottom > pageObj.maxBottom && node.tagName === "P") {
        col.removeChild(node);
        const splitRes = splitParagraphNode(node, col, pageObj.maxBottom);
        if (splitRes && splitRes.placed) {
          pageObj.hasContent = true;
          if (splitRes.remainder) {
            advanceColumn();
            placeSimpleFlowNode(splitRes.remainder);
          }
          return;
        }
        col.appendChild(node);
      }
      pageObj.hasContent = true;
    }

    newPage();

    const probeColWidth = (() => {
      ensureColsBand();
      const w = pageObj.colLeft.getBoundingClientRect().width;
      pageObj.body.removeChild(pageObj.colsBand);
      pageObj.colsBand = null;
      pageObj.colLeft = null;
      pageObj.colRight = null;
      return w;
    })();

    for (const raw of rawNodes) {
      if (pages.length >= MAX_PAGES_PER_CHAPTER) break;

      if (raw.classList.contains("rules-page-break")) {
        if (pageObj && pageObj.hasContent) {
          newPage();
        }
        continue;
      }
      if (raw.classList.contains("rules-col-break")) {
        advanceColumn();
        continue;
      }

      if (raw.tagName === "FIGURE") {
        const layout = raw.dataset.layout || "";
        if (layout === "full" || layout === "spread") {
          if (pageObj && !pageObj.hasContent && pages.length > 0 && pages[pages.length - 1] === pageObj) {
            // Replace empty trailing page immediately with the art plate(s)
            host.removeChild(pageObj.el);
            pages.pop();
            pageObj = null;
            pendingPlates.push({ kind: layout, node: raw });
            flushPendingPlates();
            newPage();
          } else {
            pendingPlates.push({ kind: layout, node: raw });
          }
          continue;
        }
      }

      const clone = cloneElementForFlow(raw);
      if (isSpanElement(raw, probeColWidth)) {
        placeSpanNode(clone);
      } else if (clone.tagName === "UL" || clone.tagName === "OL") {
        placeListAcrossFlow(clone);
      } else if (clone.classList && clone.classList.contains("rules-table-wrap")) {
        placeTableAcrossContainer(clone, false);
      } else {
        placeSimpleFlowNode(clone);
      }
    }

    // Drop empty trailing placeholder page if nothing went on it
    if (pageObj && !pageObj.hasContent && pages.length > 1) {
      host.removeChild(pageObj.el);
      pages.pop();
      pageObj = pages[pages.length - 1];
    }
    flushPendingPlates();

    finalizeHeadersAndFolios(pages, info);

    // Build heading-ID -> 0-based page index map
    headingPageMap = new Map();
    pages.forEach((p, idx) => {
      const anchored = p.el.querySelectorAll("[id]");
      for (const el of anchored) {
        if (el.id && !headingPageMap.has(el.id)) {
          headingPageMap.set(el.id, idx);
        }
      }
    });

    // Mount completed pages inside .book-spread-viewport > .book-spread
    stageEl.innerHTML = "";
    const viewport = document.createElement("div");
    viewport.className = "book-spread-viewport";
    const spread = document.createElement("div");
    spread.className = "book-spread";
    for (const p of pages) {
      spread.appendChild(p.el);
    }
    viewport.appendChild(spread);
    stageEl.appendChild(viewport);
    if (host.parentElement) host.parentElement.removeChild(host);

    bookPages = pages.map((p) => p.el);
    if (currentPageIdx >= bookPages.length) {
      currentPageIdx = 0;
    }

    if (typeof window.satyaBookRecalc === "function") {
      window.satyaBookRecalc();
    }
  }

  function restoreWidgetToSource() {
    if (!widgetEl || !widgetPlaceholder || !widgetPlaceholder.parentElement) return;
    if (widgetEl.parentElement !== widgetPlaceholder.parentElement) {
      widgetPlaceholder.parentElement.insertBefore(widgetEl, widgetPlaceholder.nextSibling);
    }
    if (typeof window.satyaBookRecalc === "function") {
      window.satyaBookRecalc();
    }
  }

  function restoreWidgetToPaginatedPage() {
    if (!widgetEl || !widgetPageBand) return;
    if (widgetEl.parentElement !== widgetPageBand) {
      widgetPageBand.appendChild(widgetEl);
    }
    if (typeof window.satyaBookRecalc === "function") {
      window.satyaBookRecalc();
    }
  }

  function getView() {
    return document.body.dataset.bookView || "spread";
  }

  function renderActiveView() {
    const view = getView();
    const total = bookPages.length;
    const prevBtn = document.getElementById("btn-page-prev");
    const nextBtn = document.getElementById("btn-page-next");
    const pageInput = document.getElementById("book-page-input");
    const pageTotalEl = document.getElementById("book-page-total");

    if (view === "scroll") {
      restoreWidgetToSource();
      if (prevBtn) prevBtn.disabled = true;
      if (nextBtn) nextBtn.disabled = true;
      if (pageInput) pageInput.disabled = true;
      if (pageTotalEl) pageTotalEl.textContent = "Scroll";
      return;
    }

    restoreWidgetToPaginatedPage();
    if (total === 0) return;

    const isSpread = view === "spread" && total > 1;
    const leftIdx = isSpread ? currentPageIdx - (currentPageIdx % 2) : currentPageIdx;
    const rightIdx = isSpread && leftIdx + 1 < total ? leftIdx + 1 : -1;

    bookPages.forEach((el, idx) => {
      const visible = idx === leftIdx || idx === rightIdx;
      el.classList.toggle("is-hidden-page", !visible);
    });

    if (pageInput) {
      pageInput.disabled = false;
      pageInput.max = String(total);
      pageInput.value = String(leftIdx + 1);
    }
    if (pageTotalEl) {
      pageTotalEl.textContent =
        rightIdx >= 0 ? `–${rightIdx + 1} / ${total}` : `/ ${total}`;
    }
    if (prevBtn) prevBtn.disabled = leftIdx <= 0;
    if (nextBtn) {
      nextBtn.disabled = rightIdx >= 0 ? rightIdx >= total - 1 : leftIdx >= total - 1;
    }

    updateScale(rightIdx >= 0 ? 2 : 1);
  }

  function updateScale(visiblePageCount) {
    const view = getView();
    if (view === "scroll") return;
    const spread = document.querySelector(".book-spread");
    const viewport = document.querySelector(".book-spread-viewport");
    const workspace = document.querySelector("main.book-workspace");
    if (!spread || !viewport || !workspace) return;

    const availW = Math.max(320, workspace.clientWidth - 28);
    const targetW = visiblePageCount >= 2 ? 1632 : 816;
    const scale = Math.min(1, availW / targetW);
    spread.style.setProperty("--book-scale", scale.toFixed(4));
    viewport.style.height = Math.ceil(1056 * scale) + "px";
  }

  function stepPage(delta) {
    const view = getView();
    if (view === "scroll" || bookPages.length === 0) return;
    const step = view === "spread" ? 2 : 1;
    const aligned = view === "spread" ? currentPageIdx - (currentPageIdx % 2) : currentPageIdx;
    const next = Math.max(0, Math.min(bookPages.length - 1, aligned + delta * step));
    if (next !== currentPageIdx) {
      currentPageIdx = next;
      renderActiveView();
      window.scrollTo({ top: 0, behavior: "smooth" });
    }
  }

  function jumpToPageNumber(oneBased) {
    if (bookPages.length === 0) return;
    const idx = Math.max(0, Math.min(bookPages.length - 1, oneBased - 1));
    currentPageIdx = getView() === "spread" ? idx - (idx % 2) : idx;
    renderActiveView();
  }

  function jumpToHash(hash) {
    const id = (hash || "").replace(/^#/, "").trim();
    if (!id) return;
    if (headingPageMap.has(id)) {
      jumpToPageNumber(headingPageMap.get(id) + 1);
    }
  }

  function applyTheme(theme) {
    const chosen = VALID_THEMES.has(theme) ? theme : "sangha-robes";
    document.body.dataset.bookTheme = chosen;
    try {
      localStorage.setItem(THEME_KEY, chosen);
    } catch (_) {}
    document.querySelectorAll("[data-set-theme]").forEach((btn) => {
      btn.classList.toggle("is-active", btn.getAttribute("data-set-theme") === chosen);
    });
  }

  function applyView(view) {
    const chosen = VALID_VIEWS.has(view) ? view : "spread";
    document.body.dataset.bookView = chosen;
    try {
      localStorage.setItem(VIEW_KEY, chosen);
    } catch (_) {}
    document.querySelectorAll("[data-set-view]").forEach((btn) => {
      btn.classList.toggle("is-active", btn.getAttribute("data-set-view") === chosen);
    });
    renderActiveView();
  }

  function toggleToc(forceState) {
    const open =
      typeof forceState === "boolean"
        ? forceState
        : !document.body.classList.contains("toc-open");
    document.body.classList.toggle("toc-open", open);
    try {
      localStorage.setItem(TOC_KEY, open ? "1" : "0");
    } catch (_) {}
    const btn = document.getElementById("btn-toc-toggle");
    if (btn) btn.classList.toggle("is-active", open);
    window.setTimeout(() => {
      const view = getView();
      if (view !== "scroll") {
        const visibleCount = document.querySelectorAll(".book-page:not(.is-hidden-page)").length;
        updateScale(Math.max(1, visibleCount));
      }
    }, 20);
  }

  function bindUi() {
    document.querySelectorAll("[data-set-theme]").forEach((btn) => {
      btn.addEventListener("click", () => applyTheme(btn.getAttribute("data-set-theme")));
    });
    document.querySelectorAll("[data-set-view]").forEach((btn) => {
      btn.addEventListener("click", () => applyView(btn.getAttribute("data-set-view")));
    });

    const tocBtn = document.getElementById("btn-toc-toggle");
    if (tocBtn) tocBtn.addEventListener("click", () => toggleToc());

    const prevBtn = document.getElementById("btn-page-prev");
    if (prevBtn) prevBtn.addEventListener("click", () => stepPage(-1));

    const nextBtn = document.getElementById("btn-page-next");
    if (nextBtn) nextBtn.addEventListener("click", () => stepPage(1));

    const pageInput = document.getElementById("book-page-input");
    if (pageInput) {
      pageInput.addEventListener("change", () => {
        const n = parseInt(pageInput.value, 10);
        if (!Number.isNaN(n)) jumpToPageNumber(n);
      });
    }

    const printBtn = document.getElementById("btn-book-print");
    if (printBtn) {
      printBtn.addEventListener("click", () => window.print());
    }

    document.addEventListener("click", (ev) => {
      const anchor = ev.target && ev.target.closest ? ev.target.closest("a[href]") : null;
      if (!anchor) return;
      const href = anchor.getAttribute("href") || "";
      if (href.startsWith("#") && getView() !== "scroll") {
        const targetId = href.slice(1);
        if (headingPageMap.has(targetId)) {
          ev.preventDefault();
          jumpToHash(href);
        }
      }
    });

    window.addEventListener("keydown", (ev) => {
      const activeTag = document.activeElement ? document.activeElement.tagName : "";
      if (activeTag === "INPUT" || activeTag === "TEXTAREA" || activeTag === "SELECT") {
        return;
      }
      if (ev.key === "ArrowLeft") {
        stepPage(-1);
      } else if (ev.key === "ArrowRight") {
        stepPage(1);
      } else if (ev.key === "t" || ev.key === "T") {
        toggleToc();
      }
    });

    window.addEventListener("resize", () => {
      if (getView() !== "scroll") {
        const visibleCount = document.querySelectorAll(".book-page:not(.is-hidden-page)").length;
        updateScale(Math.max(1, visibleCount));
      }
    });
  }

  async function init() {
    const sourceEl = document.getElementById("book-source");
    if (!sourceEl) return;

    resolveAssetUrls(sourceEl);
    bindUi();

    let savedTheme = "sangha-robes";
    let savedView = window.innerWidth >= 1024 ? "spread" : "single";
    let savedToc = document.body.dataset.chapterId === "index" ? "1" : "0";
    try {
      savedTheme = localStorage.getItem(THEME_KEY) || savedTheme;
      savedView = localStorage.getItem(VIEW_KEY) || savedView;
      const tocPref = localStorage.getItem(TOC_KEY);
      if (tocPref === "1" || tocPref === "0") savedToc = tocPref;
    } catch (_) {}

    applyTheme(savedTheme);
    document.body.classList.toggle("toc-open", savedToc === "1");
    const tocBtn = document.getElementById("btn-toc-toggle");
    if (tocBtn) tocBtn.classList.toggle("is-active", savedToc === "1");

    await waitForImages(sourceEl, 2500);
    applyAutoImageLayouts(sourceEl);
    sourceEl.querySelectorAll(".rules-table-wrap").forEach(classifyTableWrap);
    paginateBook();
    applyView(savedView);

    if (window.location.hash) {
      jumpToHash(window.location.hash);
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
