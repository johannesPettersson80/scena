//! Browser custom-element support for `<scena-viewer>`.
//!
//! The module keeps the browser adapter thin: attribute parsing is
//! platform-neutral and testable on native targets, while the actual custom
//! element registration is only exported for `wasm32` with the
//! `viewer-element` feature.

mod model;

pub use model::{
    ScenaViewerAccessibilityDefaults, ScenaViewerAttributes, ScenaViewerDropDecision,
    ScenaViewerDropKind, ScenaViewerDroppedFile, ScenaViewerKeyboardAction, ScenaViewerProgress,
    ScenaViewerProgressPhase, ScenaViewerVariantOption, ScenaViewerVariantSelection,
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
      style.textContent = ":host{display:block;min-width:160px;min-height:120px;contain:content;position:relative}:host([hidden]){display:none}canvas{display:block;width:100%;height:100%;touch-action:none;background:transparent}[part=variant-picker]{position:absolute;right:12px;top:12px;max-width:min(220px,calc(100% - 24px));font:13px/1.3 system-ui,sans-serif}[part=variant-picker][hidden]{display:none}[part=progress]{position:absolute;left:12px;right:12px;bottom:12px;display:grid;gap:6px;color:#f8fafc;font:12px/1.4 system-ui,sans-serif;text-shadow:0 1px 2px #0f172a}[part=progress][hidden]{display:none}[part=progress]::before{content:\"\";display:block;height:4px;border-radius:999px;background:rgba(15,23,42,.52)}[part=progress-bar]{height:4px;margin-top:-10px;border-radius:999px;background:#60a5fa;transform-origin:left center;transform:scaleX(0)}[part=progress-status]{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}";
      const canvas = document.createElement("canvas");
      canvas.part = "canvas";
      canvas.tabIndex = 0;
      canvas.setAttribute("aria-label", "scena 3D viewer canvas");
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
      root.append(style, canvas, variantPicker, progress);
      this._canvas = canvas;
      this._variantPicker = variantPicker;
      this._progress = progress;
      this._progressBar = bar;
      this._progressStatus = status;
      variantPicker.addEventListener("change", () => {
        const name = variantPicker.value || null;
        this.dispatchEvent(new CustomEvent("scena-viewer-variant-change", {
          bubbles: true,
          detail: { name }
        }));
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
