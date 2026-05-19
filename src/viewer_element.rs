//! Browser custom-element support for `<scena-viewer>`.
//!
//! The module keeps the browser adapter thin: attribute parsing is
//! platform-neutral and testable on native targets, while the actual custom
//! element registration is only exported for `wasm32` with the
//! `viewer-element` feature.

use crate::Tonemapper;

pub const SCENA_VIEWER_TAG: &str = "scena-viewer";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerAttributes {
    src: Option<String>,
    environment: Option<String>,
    tonemapper: Tonemapper,
    camera_controls: bool,
    auto_rotate: bool,
    ar: bool,
}

impl Default for ScenaViewerAttributes {
    fn default() -> Self {
        Self {
            src: None,
            environment: None,
            tonemapper: Tonemapper::PbrNeutral,
            camera_controls: false,
            auto_rotate: false,
            ar: false,
        }
    }
}

impl ScenaViewerAttributes {
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut attributes = Self::default();
        for (name, value) in pairs {
            attributes.set_attribute(name.as_ref(), value.as_ref());
        }
        attributes
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        match name {
            "src" => self.src = non_empty_string(value),
            "environment" => self.environment = non_empty_string(value),
            "tone-mapping" | "tone_mapping" => self.tonemapper = parse_tonemapper(value),
            "camera-controls" | "camera_controls" => {
                self.camera_controls = parse_boolean_attribute(name, value);
            }
            "auto-rotate" | "auto_rotate" => {
                self.auto_rotate = parse_boolean_attribute(name, value);
            }
            "ar" => self.ar = parse_boolean_attribute(name, value),
            _ => {}
        }
    }

    pub fn src(&self) -> Option<&str> {
        self.src.as_deref()
    }

    pub fn environment(&self) -> Option<&str> {
        self.environment.as_deref()
    }

    pub const fn tonemapper(&self) -> Tonemapper {
        self.tonemapper
    }

    pub const fn camera_controls(&self) -> bool {
        self.camera_controls
    }

    pub const fn auto_rotate(&self) -> bool {
        self.auto_rotate
    }

    pub const fn ar(&self) -> bool {
        self.ar
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn parse_tonemapper(value: &str) -> Tonemapper {
    match value.trim().to_ascii_lowercase().as_str() {
        "aces" => Tonemapper::Aces,
        "standard" => Tonemapper::Standard,
        "neutral" | "pbr-neutral" | "pbr_neutral" => Tonemapper::PbrNeutral,
        _ => Tonemapper::PbrNeutral,
    }
}

fn parse_boolean_attribute(name: &str, value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.is_empty()
        || normalized == "true"
        || normalized == "1"
        || normalized == name
        || normalized == name.replace('_', "-")
}

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
      style.textContent = ":host{display:block;min-width:160px;min-height:120px;contain:content}:host([hidden]){display:none}canvas{display:block;width:100%;height:100%;touch-action:none;background:transparent}";
      const canvas = document.createElement("canvas");
      canvas.part = "canvas";
      canvas.tabIndex = 0;
      canvas.setAttribute("aria-label", "scena 3D viewer canvas");
      root.append(style, canvas);
      this._canvas = canvas;
    }

    connectedCallback() {
      if (!this.hasAttribute("role")) {
        this.setAttribute("role", "img");
      }
      if (!this.hasAttribute("aria-label")) {
        this.setAttribute("aria-label", "3D model viewer");
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
