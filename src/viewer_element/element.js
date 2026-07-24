export function normalizeScenaViewerWheelDelta(deltaY, deltaMode = 0) {
  const value = Number(deltaY);
  if (!Number.isFinite(value)) {
    return 0;
  }
  const unitsPerStep = deltaMode === 1 ? 3 : deltaMode === 2 ? 1 : 100;
  return Math.max(-4, Math.min(4, value / unitsPerStep));
}

export function defineScenaViewerElement(tagName) {
  if (!globalThis.customElements) {
    throw new Error("Custom Elements are not available in this browser");
  }
  if (globalThis.customElements.get(tagName)) {
    return false;
  }

  class ScenaViewerElement extends HTMLElement {
    static get observedAttributes() {
      return ["src", "environment", "lighting", "background", "tone-mapping", "camera-controls", "auto-rotate", "ar"];
    }

    constructor() {
      super();
      const root = this.attachShadow({ mode: "open" });
      const style = document.createElement("style");
      style.textContent = ":host{display:block;min-width:160px;min-height:120px;contain:content;position:relative}:host([hidden]){display:none}canvas{display:block;width:100%;height:100%;touch-action:none;background:transparent}[part=annotations]{position:absolute;inset:0;overflow:hidden;pointer-events:none}::slotted([slot=annotation]){position:absolute;left:0;top:0;transform:translate(var(--scena-annotation-x,-9999px),var(--scena-annotation-y,-9999px));pointer-events:auto}::slotted([slot=annotation][data-scena-hidden]){display:none}[part=variant-picker]{position:absolute;right:12px;top:12px;max-width:min(220px,calc(100% - 24px));font:13px/1.3 system-ui,sans-serif}[part=variant-picker][hidden]{display:none}[part=progress]{position:absolute;left:12px;right:12px;bottom:12px;display:grid;gap:6px;color:#f8fafc;font:12px/1.4 system-ui,sans-serif;text-shadow:0 1px 2px #0f172a}[part=progress][hidden]{display:none}[part=progress]::before{content:\"\";display:block;height:4px;border-radius:999px;background:rgba(15,23,42,.52)}[part=progress-bar]{height:4px;margin-top:-10px;border-radius:999px;background:#60a5fa;transform-origin:left center;transform:scaleX(0)}[part=progress-status]{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}[part=inspector]{position:absolute;left:12px;top:12px;max-width:min(320px,calc(100% - 24px));max-height:calc(100% - 24px);overflow:auto;padding:10px;border:1px solid rgba(148,163,184,.55);background:rgba(15,23,42,.86);color:#e2e8f0;font:12px/1.4 system-ui,sans-serif}[part=inspector][hidden]{display:none}[part=inspector-status]{display:block;font-weight:600;margin-bottom:6px}[part=inspector-list]{margin:0;padding-left:16px}";
      const canvas = document.createElement("canvas");
      canvas.part = "canvas";
      canvas.tabIndex = 0;
      canvas.setAttribute("aria-label", "scena 3D viewer canvas");
      const annotationLayer = document.createElement("div");
      annotationLayer.part = "annotations";
      const annotationSlot = document.createElement("slot");
      annotationSlot.name = "annotation";
      annotationLayer.append(annotationSlot);
      const variantPicker = document.createElement("select");
      variantPicker.part = "variant-picker";
      variantPicker.hidden = true;
      variantPicker.setAttribute("aria-label", "Material variant");
      const progress = document.createElement("div");
      progress.part = "progress";
      progress.hidden = true;
      progress.setAttribute("role", "progressbar");
      progress.setAttribute("aria-live", "polite");
      const bar = document.createElement("div");
      bar.part = "progress-bar";
      const status = document.createElement("span");
      status.part = "progress-status";
      progress.append(bar, status);
      const inspector = document.createElement("section");
      inspector.part = "inspector";
      inspector.hidden = true;
      inspector.setAttribute("aria-live", "polite");
      const inspectorStatus = document.createElement("strong");
      inspectorStatus.part = "inspector-status";
      const inspectorList = document.createElement("ul");
      inspectorList.part = "inspector-list";
      inspector.append(inspectorStatus, inspectorList);
      root.append(style, canvas, annotationLayer, variantPicker, progress, inspector);
      this._canvas = canvas;
      this._annotationSlot = annotationSlot;
      this._annotationDataAttributes = ["data-position", "data-normal", "data-surface"];
      this._variantPicker = variantPicker;
      this._progress = progress;
      this._progressBar = bar;
      this._progressStatus = status;
      this._inspector = inspector;
      this._inspectorStatus = inspectorStatus;
      this._inspectorList = inspectorList;
      this._host = null;
      this._activePointers = new Map();
      this._lastPinchDistance = null;
      this._controlListenersAttached = false;
      this._boundPointerDown = (event) => this._handlePointerDown(event);
      this._boundPointerMove = (event) => this._handlePointerMove(event);
      this._boundPointerEnd = (event) => this._handlePointerEnd(event);
      this._boundLostPointerCapture = (event) => this._handleLostPointerCapture(event);
      this._boundWheel = (event) => this._handleWheel(event);
      variantPicker.addEventListener("change", () => {
        const name = variantPicker.value || null;
        this.dispatchEvent(new CustomEvent("scena-viewer-variant-change", {
          bubbles: true,
          detail: { name }
        }));
      });
      annotationSlot.addEventListener("slotchange", () => {
        this.requestAnnotationProjections();
      });
      this.addEventListener("scena-viewer-progress", (event) => {
        this.setLoadProgress(event.detail || {});
      });
      this.addEventListener("dragenter", (event) => this._handleDragOver(event));
      this.addEventListener("dragover", (event) => this._handleDragOver(event));
      this.addEventListener("dragleave", () => {
        delete this.dataset.drag;
      });
      this.addEventListener("drop", (event) => this._handleDrop(event));
      this.addEventListener("keydown", (event) => this._handleKeydown(event));
    }

    connectedCallback() {
      this._attachControlListeners();
      if (!this.hasAttribute("role")) {
        this.setAttribute("role", "img");
      }
      if (!this.hasAttribute("aria-label")) {
        this.setAttribute("aria-label", "3D model viewer");
      }
      if (!this.hasAttribute("tabindex")) {
        this.tabIndex = 0;
      }
      if (!this.hasAttribute("aria-roledescription")) {
        this.setAttribute("aria-roledescription", "interactive 3D model");
      }
      this._emit("scena-viewer-ready");
    }

    disconnectedCallback() {
      this._detachControlListeners();
      this._releasePointerState();
    }

    attributeChangedCallback() {
      if (this.isConnected) {
        this._emit("scena-viewer-attributes");
      }
    }

    get canvas() {
      return this._canvas;
    }

    bindHost(host) {
      this._host = host || null;
      this.dispatchEvent(new CustomEvent("scena-viewer-host-bound", {
        bubbles: true,
        detail: { bound: this._host !== null }
      }));
      return this;
    }

    clearHost() {
      this._host = null;
      this.dispatchEvent(new CustomEvent("scena-viewer-host-bound", {
        bubbles: true,
        detail: { bound: false }
      }));
    }

    get host() {
      return this._host;
    }

    applyPatch(patch) {
      const host = this._requireHost("applyPatch");
      const patchJson = typeof patch === "string" ? patch : JSON.stringify(patch || {});
      const result = this._parseMaybeJson(host.applyPatch(patchJson));
      this._dispatchHostEvents();
      this.dispatchEvent(new CustomEvent("scena-viewer-patch-applied", {
        bubbles: true,
        detail: result
      }));
      return result;
    }

    applyVisualPatch(patch) {
      return this.applyPatch(patch);
    }

    capturePng() {
      const host = this._requireHost("capturePng");
      const capture = host.capturePng();
      const descriptor = this._parseMaybeJson(capture?.descriptorJson || "{}");
      const pngBytes = this._byteLength(capture?.png);
      this._dispatchHostEvents();
      this.dispatchEvent(new CustomEvent("scena-viewer-capture-ready", {
        bubbles: true,
        detail: {
          descriptor,
          descriptorJson: capture?.descriptorJson || null,
          bytes: pngBytes,
          png: capture?.png || null
        }
      }));
      return capture;
    }

    capturePNG() {
      return this.capturePng();
    }

    downloadPng(filename = "scena-viewer.png", options = {}) {
      const capture = this.capturePng();
      const bytes = this._byteLength(capture?.png);
      const detail = {
        filename: String(filename || "scena-viewer.png"),
        bytes,
        descriptorJson: capture?.descriptorJson || null
      };
      if (options?.click !== false) {
        const blob = new Blob([capture?.png || new Uint8Array()], { type: "image/png" });
        const url = URL.createObjectURL(blob);
        const anchor = document.createElement("a");
        anchor.href = url;
        anchor.download = detail.filename;
        anchor.style.display = "none";
        document.body.append(anchor);
        anchor.click();
        anchor.remove();
        URL.revokeObjectURL(url);
      }
      this.dispatchEvent(new CustomEvent("scena-viewer-capture-download", {
        bubbles: true,
        detail
      }));
      return detail;
    }

    pickAt(x, y) {
      const host = this._requireHost("pick");
      const handle = this._optionalHandle(host.pick(Number(x), Number(y)));
      const batch = this._dispatchHostEvents();
      return { handle, events: batch.events };
    }

    hoverAt(x, y) {
      const host = this._requireHost("hover");
      const handle = this._optionalHandle(host.hover(Number(x), Number(y)));
      const batch = this._dispatchHostEvents();
      return { handle, events: batch.events };
    }

    selectAt(x, y) {
      const host = this._requireHost("select");
      const handle = this._optionalHandle(host.select(Number(x), Number(y)));
      const batch = this._dispatchHostEvents();
      return { handle, events: batch.events };
    }

    drainHostEvents() {
      return this._dispatchHostEvents();
    }

    frameAll() {
      const host = this._requireHost("frameAll");
      const result = host.frameAll();
      this._dispatchHostEvents();
      return result;
    }

    frameNode(node, preset = null) {
      const host = this._requireHost(preset ? "frameNodeWithPreset" : "frameNode");
      const result = preset ? host.frameNodeWithPreset(Number(node), String(preset)) : host.frameNode(Number(node));
      this._dispatchHostEvents();
      return result;
    }

    setCamera(camera) {
      const host = this._requireHost("setCameraJson");
      const cameraJson = typeof camera === "string" ? camera : JSON.stringify(camera || {});
      const result = host.setCameraJson(cameraJson);
      this._dispatchHostEvents();
      return result;
    }

    applyLightingPreset(preset = null, options = {}) {
      const normalized = String(preset || this.getAttribute("lighting") || "studio").replace(/_/g, "-");
      if (normalized !== "studio" && normalized !== "product-studio") {
        throw new Error(`Unsupported scena-viewer lighting preset ${normalized}`);
      }
      const host = this._requireHost("applyProductStudioVisuals");
      const background = String(options.background || this.getAttribute("background") || this.getAttribute("environment") || "studio");
      const result = host.applyProductStudioVisuals(background);
      this._dispatchHostEvents();
      this.dispatchEvent(new CustomEvent("scena-viewer-lighting-applied", {
        bubbles: true,
        detail: { preset: normalized, background }
      }));
      return result;
    }

    setLoadProgress(detail) {
      const phase = String(detail.phase || "loading");
      const complete = phase === "complete" || detail.complete === true;
      const text = String(detail.ariaText || detail.label || (complete ? "Loaded" : "Loading"));
      const value = Number(detail.value ?? detail.ratio ?? detail.percent);
      this._progress.hidden = complete && detail.keepVisible !== true;
      this._progress.dataset.phase = phase;
      this._progress.setAttribute("aria-label", text);
      this._progressStatus.textContent = text;
      if (Number.isFinite(value)) {
        const clamped = Math.max(0, Math.min(1, value > 1 ? value / 100 : value));
        this._progress.setAttribute("aria-valuemin", "0");
        this._progress.setAttribute("aria-valuemax", "100");
        this._progress.setAttribute("aria-valuenow", String(Math.round(clamped * 100)));
        this._progressBar.style.transform = `scaleX(${clamped})`;
      } else {
        this._progress.removeAttribute("aria-valuenow");
        this._progressBar.style.transform = complete ? "scaleX(1)" : "scaleX(.35)";
      }
      this.dispatchEvent(new CustomEvent("scena-viewer-progress-rendered", {
        bubbles: true,
        detail: { phase, text, complete }
      }));
    }

    setMaterialVariants(variants, activeName = null) {
      const normalized = Array.from(variants || [])
        .map((variant) => {
          if (typeof variant === "string") {
            return { name: variant, label: variant };
          }
          return {
            name: String(variant?.name || ""),
            label: String(variant?.label || variant?.name || "")
          };
        })
        .filter((variant) => variant.name.length > 0);
      this._variantPicker.replaceChildren();
      const defaultOption = document.createElement("option");
      defaultOption.value = "";
      defaultOption.textContent = "Default material";
      this._variantPicker.append(defaultOption);
      for (const variant of normalized) {
        const option = document.createElement("option");
        option.value = variant.name;
        option.textContent = variant.label;
        this._variantPicker.append(option);
      }
      const names = normalized.map((variant) => variant.name);
      this._variantPicker.hidden = normalized.length === 0;
      this._variantPicker.value = names.includes(activeName) ? activeName : "";
      this.dispatchEvent(new CustomEvent("scena-viewer-variants-ready", {
        bubbles: true,
        detail: { names, activeName: this._variantPicker.value || null }
      }));
    }

    annotationAnchors() {
      return this._annotationElements()
        .map((element, index) => {
          const position = this._parseVector(element.dataset.position);
          if (!position) {
            return null;
          }
          return {
            id: this._annotationId(element, index),
            position,
            normal: this._parseVector(element.dataset.normal),
            surface: element.dataset.surface || null
          };
        })
        .filter(Boolean);
    }

    requestAnnotationProjections() {
      this.dispatchEvent(new CustomEvent("scena-viewer-annotations-request", {
        bubbles: true,
        detail: { anchors: this.annotationAnchors() }
      }));
    }

    setAnnotationProjections(projections = []) {
      const byId = new Map(Array.from(projections || []).map((projection) => [
        String(projection?.id || ""),
        projection
      ]));
      const elements = this._annotationElements();
      const hostRect = this.getBoundingClientRect();
      const viewportWidth = Math.max(1, Number(this.clientWidth || hostRect.width || 1));
      const viewportHeight = Math.max(1, Number(this.clientHeight || hostRect.height || 1));
      const layout = elements.map((element, index) => {
        const id = this._annotationId(element, index);
        const projection = byId.get(id);
        const x = Number(projection?.x ?? projection?.screenX);
        const y = Number(projection?.y ?? projection?.screenY);
        const rect = element.getBoundingClientRect();
        const width = Math.max(1, Number(element.offsetWidth || rect.width || element.dataset.width || 1));
        const height = Math.max(1, Number(element.offsetHeight || rect.height || element.dataset.height || 1));
        const priority = Number(element.dataset.priority ?? projection?.priority ?? 0);
        const hasPoint = Boolean(projection) && Number.isFinite(x) && Number.isFinite(y);
        const behindCamera = projection?.behind_camera === true || projection?.behindCamera === true;
        const occluded = projection?.occluded === true || element.dataset.occluded === "true";
        const originalX = hasPoint ? x : 0;
        const originalY = hasPoint ? y : 0;
        let hiddenReason = null;
        if (!hasPoint || projection.visible === false) {
          hiddenReason = "hidden";
        } else if (behindCamera) {
          hiddenReason = "behind_camera";
        } else if (occluded) {
          hiddenReason = "occluded";
        }
        return {
          id,
          element,
          original_x: originalX,
          original_y: originalY,
          x: Math.max(0, Math.min(viewportWidth - width, originalX)),
          y: Math.max(0, Math.min(viewportHeight - height, originalY)),
          width,
          height,
          priority: Number.isFinite(priority) ? priority : 0,
          visible: hiddenReason === null,
          hidden_reason: hiddenReason
        };
      });
      const accepted = [];
      for (const entry of [...layout].sort((left, right) => {
        if (left.priority !== right.priority) {
          return right.priority - left.priority;
        }
        return left.id.localeCompare(right.id);
      })) {
        if (!entry.visible) {
          continue;
        }
        if (accepted.some((placed) => this._annotationBoxesOverlap(entry, placed))) {
          entry.visible = false;
          entry.hidden_reason = "overlap";
        } else {
          accepted.push(entry);
        }
      }
      let visible = 0;
      for (const entry of layout) {
        if (entry.visible) {
          visible += 1;
          entry.element.style.setProperty("--scena-annotation-x", `${entry.x}px`);
          entry.element.style.setProperty("--scena-annotation-y", `${entry.y}px`);
          entry.element.removeAttribute("data-scena-hidden");
        } else {
          entry.element.setAttribute("data-scena-hidden", "");
        }
      }
      const layoutReport = {
        coordinate_space: "css_pixels",
        viewport_width: viewportWidth,
        viewport_height: viewportHeight,
        entries: layout.map((entry) => ({
          id: entry.id,
          original_x: entry.original_x,
          original_y: entry.original_y,
          x: entry.x,
          y: entry.y,
          width: entry.width,
          height: entry.height,
          priority: entry.priority,
          visible: entry.visible,
          hidden_reason: entry.hidden_reason
        }))
      };
      this.dispatchEvent(new CustomEvent("scena-viewer-annotations-rendered", {
        bubbles: true,
        detail: { count: elements.length, visible, layout_report: layoutReport }
      }));
    }

    setInspectorSnapshot(snapshot = {}) {
      const diagnostics = Array.isArray(snapshot.diagnostics) ? snapshot.diagnostics : [];
      const stats = snapshot.stats || snapshot;
      const drawCalls = Number(stats.drawCalls ?? stats.draw_calls ?? 0);
      const triangles = Number(stats.triangles ?? 0);
      const width = Number(stats.targetWidth ?? stats.target_width ?? 0);
      const height = Number(stats.targetHeight ?? stats.target_height ?? 0);
      const errors = diagnostics.filter((diagnostic) => this._severity(diagnostic) === "error").length;
      const warnings = diagnostics.filter((diagnostic) => this._severity(diagnostic) === "warning").length;
      const status = String(snapshot.statusText || `${this._countLabel(errors, "error")}, ${this._countLabel(warnings, "warning")}; ${drawCalls} draws; ${triangles} triangles at ${width}x${height}`);
      this._inspector.hidden = false;
      this._inspectorStatus.textContent = status;
      this._inspectorList.replaceChildren();
      for (const diagnostic of diagnostics) {
        const item = document.createElement("li");
        const code = String(diagnostic.code || "Diagnostic");
        const message = String(diagnostic.message || "");
        item.dataset.severity = this._severity(diagnostic);
        item.textContent = message ? `${code}: ${message}` : code;
        this._inspectorList.append(item);
      }
      this.dispatchEvent(new CustomEvent("scena-viewer-inspector-rendered", {
        bubbles: true,
        detail: { errors, warnings, diagnostics: diagnostics.length, status }
      }));
    }

    setInspectorDiagnostics(diagnostics, stats = {}) {
      this.setInspectorSnapshot({ diagnostics, stats });
    }

    clearInspectorSnapshot() {
      this._inspector.hidden = true;
      this._inspectorStatus.textContent = "";
      this._inspectorList.replaceChildren();
    }

    _handleDragOver(event) {
      event.preventDefault();
      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "copy";
      }
      this.dataset.drag = "over";
    }

    _handleDrop(event) {
      event.preventDefault();
      delete this.dataset.drag;
      const files = Array.from(event.dataTransfer?.files || []);
      const accepted = [];
      const rejected = [];
      for (const file of files) {
        if (this._isSupportedAssetFile(file.name)) {
          accepted.push(file);
        } else {
          rejected.push(file.name || "unnamed file");
        }
      }
      const acceptedNames = accepted.map((file) => file.name);
      if (accepted.length > 0) {
        this.dispatchEvent(new CustomEvent("scena-viewer-file-drop", {
          bubbles: true,
          detail: { files: accepted, names: acceptedNames, rejectedNames: rejected }
        }));
      }
      if (accepted.length === 0 || rejected.length > 0) {
        const message = accepted.length === 0
          ? "Drop a .glb or .gltf file"
          : `Rejected ${rejected.join(", ")}`;
        this.dispatchEvent(new CustomEvent("scena-viewer-drop-error", {
          bubbles: true,
          detail: { names: rejected, acceptedNames, message }
        }));
      }
    }

    _isSupportedAssetFile(name) {
      return /\.(glb|gltf)$/i.test(String(name || ""));
    }

    _annotationBoxesOverlap(left, right) {
      return left.x < right.x + right.width &&
        left.x + left.width > right.x &&
        left.y < right.y + right.height &&
        left.y + left.height > right.y;
    }

    _annotationElements() {
      return this._annotationSlot
        ? this._annotationSlot.assignedElements({ flatten: true }).filter((element) => element.hasAttribute(this._annotationDataAttributes[0]))
        : [];
    }

    _annotationId(element, index) {
      return element.dataset.annotationId || element.id || `annotation-${index}`;
    }

    _parseVector(value) {
      if (!value) {
        return null;
      }
      const parts = String(value).replace(/,/g, " ").trim().split(/\s+/).filter(Boolean).map(Number);
      return parts.length === 3 && parts.every(Number.isFinite) ? parts : null;
    }

    _severity(diagnostic) {
      return String(diagnostic?.severity || "").toLowerCase();
    }

    _countLabel(count, singular) {
      return count === 1 ? `1 ${singular}` : `${count} ${singular}s`;
    }

    _handleKeydown(event) {
      const action = this._keyboardAction(event.key);
      if (!action) {
        return;
      }
      event.preventDefault();
      this.dispatchEvent(new CustomEvent("scena-viewer-key-control", {
        bubbles: true,
        detail: { action, key: event.key }
      }));
    }

    _keyboardAction(key) {
      switch (key) {
        case "ArrowLeft": return "orbit-left";
        case "ArrowRight": return "orbit-right";
        case "ArrowUp": return "orbit-up";
        case "ArrowDown": return "orbit-down";
        case "+":
        case "=": return "zoom-in";
        case "-":
        case "_": return "zoom-out";
        case "Escape":
        case "Home": return "reset-view";
        default: return null;
      }
    }

    _handlePointerDown(event) {
      if (!this._booleanAttribute("camera-controls")) {
        return;
      }
      event.preventDefault();
      this._activePointers.set(event.pointerId, {
        x: Number(event.clientX || 0),
        y: Number(event.clientY || 0),
        pointerType: event.pointerType || "mouse"
      });
      try {
        this.setPointerCapture(event.pointerId);
      } catch (_error) {
        // Synthetic events do not own an active browser pointer. Real pointer
        // events still use capture; synthetic contract probes remain usable.
      }
      if (this._activePointers.size >= 2) {
        this._lastPinchDistance = this._pinchDistance();
      }
    }

    _handlePointerMove(event) {
      if (!this._booleanAttribute("camera-controls") || !this._activePointers.has(event.pointerId)) {
        return;
      }
      event.preventDefault();
      const previous = this._activePointers.get(event.pointerId);
      const next = {
        x: Number(event.clientX || 0),
        y: Number(event.clientY || 0),
        pointerType: event.pointerType || previous.pointerType || "mouse"
      };
      this._activePointers.set(event.pointerId, next);
      if (this._activePointers.size >= 2) {
        const distance = this._pinchDistance();
        const deltaDistance = this._lastPinchDistance == null ? 0 : distance - this._lastPinchDistance;
        this._lastPinchDistance = distance;
        this._emitGesture("pinch-zoom", {
          pointerType: next.pointerType,
          pointers: this._activePointers.size,
          deltaDistance
        });
        return;
      }
      this._emitGesture("orbit", {
        pointerType: next.pointerType,
        pointers: 1,
        deltaX: next.x - previous.x,
        deltaY: next.y - previous.y
      });
    }

    _handlePointerEnd(event) {
      this._activePointers.delete(event.pointerId);
      if (this._activePointers.size < 2) {
        this._lastPinchDistance = null;
      }
      try {
        if (this.hasPointerCapture(event.pointerId)) {
          this.releasePointerCapture(event.pointerId);
        }
      } catch (_error) {
        // The browser may already have released capture after cancellation.
      }
    }

    _handleLostPointerCapture(event) {
      this._activePointers.delete(event.pointerId);
      if (this._activePointers.size < 2) {
        this._lastPinchDistance = null;
      }
    }

    _attachControlListeners() {
      if (this._controlListenersAttached) {
        return;
      }
      this.addEventListener("pointerdown", this._boundPointerDown);
      this.addEventListener("pointermove", this._boundPointerMove);
      this.addEventListener("pointerup", this._boundPointerEnd);
      this.addEventListener("pointercancel", this._boundPointerEnd);
      this.addEventListener("lostpointercapture", this._boundLostPointerCapture);
      this.addEventListener("wheel", this._boundWheel, { passive: false });
      this._controlListenersAttached = true;
    }

    _detachControlListeners() {
      if (!this._controlListenersAttached) {
        return;
      }
      this.removeEventListener("pointerdown", this._boundPointerDown);
      this.removeEventListener("pointermove", this._boundPointerMove);
      this.removeEventListener("pointerup", this._boundPointerEnd);
      this.removeEventListener("pointercancel", this._boundPointerEnd);
      this.removeEventListener("lostpointercapture", this._boundLostPointerCapture);
      this.removeEventListener("wheel", this._boundWheel);
      this._controlListenersAttached = false;
    }

    _releasePointerState() {
      for (const pointerId of this._activePointers.keys()) {
        try {
          if (this.hasPointerCapture(pointerId)) {
            this.releasePointerCapture(pointerId);
          }
        } catch (_error) {
          // A detached element may have lost capture before cleanup runs.
        }
      }
      this._activePointers.clear();
      this._lastPinchDistance = null;
    }

    _handleWheel(event) {
      if (!this._booleanAttribute("camera-controls")) {
        return;
      }
      const deltaY = normalizeScenaViewerWheelDelta(event.deltaY, event.deltaMode);
      if (deltaY === 0) {
        return;
      }
      event.preventDefault();
      this._emitGesture("wheel-zoom", {
        pointerType: "wheel",
        pointers: 0,
        deltaY,
        rawDeltaY: Number(event.deltaY || 0),
        deltaMode: Number(event.deltaMode || 0)
      });
    }

    _pinchDistance() {
      const pointers = Array.from(this._activePointers.values());
      if (pointers.length < 2) {
        return 0;
      }
      const dx = pointers[0].x - pointers[1].x;
      const dy = pointers[0].y - pointers[1].y;
      return Math.hypot(dx, dy);
    }

    _emitGesture(action, detail = {}) {
      this.dispatchEvent(new CustomEvent("scena-viewer-gesture-control", {
        bubbles: true,
        detail: { action, ...detail }
      }));
    }

    _booleanAttribute(name) {
      if (!this.hasAttribute(name)) {
        return false;
      }
      const value = this.getAttribute(name);
      return value === "" || value === name || value === "true" || value === "1";
    }

    _detail() {
      return {
        src: this.getAttribute("src") || "",
        environment: this.getAttribute("environment") || "",
        lighting: this.getAttribute("lighting") || "",
        background: this.getAttribute("background") || "",
        toneMapping: this.getAttribute("tone-mapping") || "neutral",
        cameraControls: this._booleanAttribute("camera-controls"),
        autoRotate: this._booleanAttribute("auto-rotate"),
        ar: this._booleanAttribute("ar")
      };
    }

    _emit(name) {
      this.dispatchEvent(new CustomEvent(name, {
        bubbles: true,
        detail: this._detail()
      }));
    }

    _requireHost(methodName) {
      if (!this._host || typeof this._host[methodName] !== "function") {
        throw new Error(`scena-viewer requires a bound SceneHost with ${methodName}()`);
      }
      return this._host;
    }

    _dispatchHostEvents() {
      if (!this._host || typeof this._host.drainEventsJson !== "function") {
        return { schema: "scena.host_event.v1", events: [] };
      }
      const batch = this._parseMaybeJson(this._host.drainEventsJson()) || { schema: "scena.host_event.v1", events: [] };
      const events = Array.isArray(batch.events) ? batch.events : [];
      for (const event of events) {
        this.dispatchEvent(new CustomEvent("scena-viewer-host-event", {
          bubbles: true,
          detail: event
        }));
        const kind = String(event?.kind || "").replace(/_/g, "-");
        if (kind.length > 0) {
          this.dispatchEvent(new CustomEvent(`scena-viewer-${kind}`, {
            bubbles: true,
            detail: event
          }));
        }
      }
      return { schema: batch.schema || "scena.host_event.v1", events };
    }

    _parseMaybeJson(value) {
      if (typeof value !== "string") {
        return value;
      }
      return JSON.parse(value);
    }

    _optionalHandle(value) {
      return value === undefined ? null : value;
    }

    _byteLength(value) {
      if (!value) {
        return 0;
      }
      return Number(value.byteLength ?? value.length ?? 0);
    }
  }

  globalThis.customElements.define(tagName, ScenaViewerElement);
  return true;
}
