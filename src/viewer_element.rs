//! Browser custom-element support for `<scena-viewer>`.
//!
//! The module keeps the browser adapter thin: attribute parsing is
//! platform-neutral and testable on native targets, while the actual custom
//! element registration is only exported for `wasm32` with the
//! `viewer-element` feature.

mod annotations;
mod inspector;
mod model;

pub use annotations::{ScenaViewerAnnotationAnchor, ScenaViewerAnnotationError};
pub use inspector::{ScenaViewerInspectorDiagnostic, ScenaViewerInspectorSnapshot};
pub use model::{
    ScenaViewerAccessibilityDefaults, ScenaViewerAttributes, ScenaViewerDropDecision,
    ScenaViewerDropKind, ScenaViewerDroppedFile, ScenaViewerGestureAction,
    ScenaViewerKeyboardAction, ScenaViewerProgress, ScenaViewerProgressPhase,
    ScenaViewerVariantOption, ScenaViewerVariantSelection,
};

pub const SCENA_VIEWER_TAG: &str = "scena-viewer";

#[cfg(all(target_arch = "wasm32", feature = "viewer-element"))]
mod wasm {
    use wasm_bindgen::prelude::*;

    use super::SCENA_VIEWER_TAG;

    #[wasm_bindgen(inline_js = r#"
export function defineScenaViewerElement(tagName) {
  if (!globalThis.customElements) {
    throw new Error("Custom Elements are not available in this browser");
  }
  if (globalThis.customElements.get(tagName)) {
    return false;
  }

  class ScenaViewerElement extends HTMLElement {
    static get observedAttributes() {
      return ["src", "environment", "tone-mapping", "camera-controls", "auto-rotate", "ar"];
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
      this._activePointers = new Map();
      this._lastPinchDistance = null;
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
      this.addEventListener("pointerdown", (event) => this._handlePointerDown(event));
      this.addEventListener("pointermove", (event) => this._handlePointerMove(event));
      this.addEventListener("pointerup", (event) => this._handlePointerEnd(event));
      this.addEventListener("pointercancel", (event) => this._handlePointerEnd(event));
      this.addEventListener("wheel", (event) => this._handleWheel(event), { passive: false });
    }

    connectedCallback() {
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

    attributeChangedCallback() {
      if (this.isConnected) {
        this._emit("scena-viewer-attributes");
      }
    }

    get canvas() {
      return this._canvas;
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
      let visible = 0;
      const elements = this._annotationElements();
      elements.forEach((element, index) => {
        const id = this._annotationId(element, index);
        const projection = byId.get(id);
        const x = Number(projection?.x ?? projection?.screenX);
        const y = Number(projection?.y ?? projection?.screenY);
        const isVisible = Boolean(projection) && projection.visible !== false && Number.isFinite(x) && Number.isFinite(y);
        if (isVisible) {
          visible += 1;
          element.style.setProperty("--scena-annotation-x", `${x}px`);
          element.style.setProperty("--scena-annotation-y", `${y}px`);
          element.removeAttribute("data-scena-hidden");
        } else {
          element.setAttribute("data-scena-hidden", "");
        }
      });
      this.dispatchEvent(new CustomEvent("scena-viewer-annotations-rendered", {
        bubbles: true,
        detail: { count: elements.length, visible }
      }));
    }

    setInspectorSnapshot(snapshot = {}) {
      const overlay = String(snapshot.overlay || "None");
      const diagnostics = Array.isArray(snapshot.diagnostics) ? snapshot.diagnostics : [];
      const stats = snapshot.stats || snapshot;
      const drawCalls = Number(stats.drawCalls ?? stats.draw_calls ?? 0);
      const triangles = Number(stats.triangles ?? 0);
      const width = Number(stats.targetWidth ?? stats.target_width ?? 0);
      const height = Number(stats.targetHeight ?? stats.target_height ?? 0);
      const errors = diagnostics.filter((diagnostic) => this._severity(diagnostic) === "error").length;
      const warnings = diagnostics.filter((diagnostic) => this._severity(diagnostic) === "warning").length;
      const status = String(snapshot.statusText || `${overlay} overlay; ${this._countLabel(errors, "error")}, ${this._countLabel(warnings, "warning")}; ${drawCalls} draws; ${triangles} triangles at ${width}x${height}`);
      this._inspector.hidden = false;
      this._inspector.dataset.overlay = overlay;
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
        detail: { overlay, errors, warnings, diagnostics: diagnostics.length, status }
      }));
    }

    setInspectorDiagnostics(diagnostics, overlay = "None", stats = {}) {
      this.setInspectorSnapshot({ overlay, diagnostics, stats });
    }

    clearInspectorSnapshot() {
      this._inspector.hidden = true;
      this._inspectorStatus.textContent = "";
      this._inspectorList.replaceChildren();
      delete this._inspector.dataset.overlay;
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
    }

    _handleWheel(event) {
      if (!this._booleanAttribute("camera-controls")) {
        return;
      }
      event.preventDefault();
      this._emitGesture("wheel-zoom", {
        pointerType: "wheel",
        pointers: 0,
        deltaY: Number(event.deltaY || 0)
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
  }

  globalThis.customElements.define(tagName, ScenaViewerElement);
  return true;
}
"#)]
    extern "C" {
        #[wasm_bindgen(catch, js_name = defineScenaViewerElement)]
        fn define_scena_viewer_element(tag_name: &str) -> Result<bool, JsValue>;
    }

    #[wasm_bindgen(js_name = defineScenaViewer)]
    pub fn define_scena_viewer() -> Result<bool, JsValue> {
        define_scena_viewer_element(SCENA_VIEWER_TAG)
    }
}

#[cfg(all(target_arch = "wasm32", feature = "viewer-element"))]
pub use wasm::define_scena_viewer;
