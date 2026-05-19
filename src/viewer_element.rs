//! Browser custom-element support for `<scena-viewer>`.
//!
//! The module keeps the browser adapter thin: attribute parsing is
//! platform-neutral and testable on native targets, while the actual custom
//! element registration is only exported for `wasm32` with the
//! `viewer-element` feature.

use crate::Tonemapper;
use crate::assets::AssetLoadProgress;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenaViewerProgressPhase {
    Idle,
    Loading,
    Fetching,
    Parsing,
    Caching,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaViewerProgress {
    phase: ScenaViewerProgressPhase,
    path: Option<String>,
    loaded_bytes: Option<usize>,
    external_buffer_index: Option<usize>,
    nodes: Option<usize>,
    meshes: Option<usize>,
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

impl Default for ScenaViewerProgress {
    fn default() -> Self {
        Self {
            phase: ScenaViewerProgressPhase::Idle,
            path: None,
            loaded_bytes: None,
            external_buffer_index: None,
            nodes: None,
            meshes: None,
        }
    }
}

impl ScenaViewerProgress {
    pub fn from_asset_event(event: &AssetLoadProgress) -> Self {
        match event {
            AssetLoadProgress::LoadStarted { path } => Self::for_path(
                ScenaViewerProgressPhase::Loading,
                Some(path.as_str().to_string()),
            ),
            AssetLoadProgress::CacheHit { path } => Self::for_path(
                ScenaViewerProgressPhase::Complete,
                Some(path.as_str().to_string()),
            ),
            AssetLoadProgress::AssetFetched { path, bytes } => {
                let mut progress = Self::for_path(
                    ScenaViewerProgressPhase::Fetching,
                    Some(path.as_str().to_string()),
                );
                progress.loaded_bytes = Some(*bytes);
                progress
            }
            AssetLoadProgress::ExternalBufferFetched { path, index, bytes } => {
                let mut progress = Self::for_path(
                    ScenaViewerProgressPhase::Fetching,
                    Some(path.as_str().to_string()),
                );
                progress.external_buffer_index = Some(*index);
                progress.loaded_bytes = Some(*bytes);
                progress
            }
            AssetLoadProgress::Parsed {
                path,
                nodes,
                meshes,
            } => {
                let mut progress = Self::for_path(
                    ScenaViewerProgressPhase::Parsing,
                    Some(path.as_str().to_string()),
                );
                progress.nodes = Some(*nodes);
                progress.meshes = Some(*meshes);
                progress
            }
            AssetLoadProgress::Cached { path } => Self::for_path(
                ScenaViewerProgressPhase::Complete,
                Some(path.as_str().to_string()),
            ),
        }
    }

    fn for_path(phase: ScenaViewerProgressPhase, path: Option<String>) -> Self {
        Self {
            phase,
            path,
            ..Self::default()
        }
    }

    pub const fn phase(&self) -> ScenaViewerProgressPhase {
        self.phase
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub const fn loaded_bytes(&self) -> Option<usize> {
        self.loaded_bytes
    }

    pub const fn external_buffer_index(&self) -> Option<usize> {
        self.external_buffer_index
    }

    pub const fn nodes(&self) -> Option<usize> {
        self.nodes
    }

    pub const fn meshes(&self) -> Option<usize> {
        self.meshes
    }

    pub const fn is_complete(&self) -> bool {
        matches!(self.phase, ScenaViewerProgressPhase::Complete)
    }

    pub fn aria_text(&self) -> String {
        let path = self.path.as_deref().unwrap_or("asset");
        match self.phase {
            ScenaViewerProgressPhase::Idle => "Ready".to_string(),
            ScenaViewerProgressPhase::Loading => format!("Loading {path}"),
            ScenaViewerProgressPhase::Fetching => {
                match (self.external_buffer_index, self.loaded_bytes) {
                    (Some(index), Some(bytes)) => {
                        format!("Fetched external buffer {index} with {bytes} bytes from {path}")
                    }
                    (_, Some(bytes)) => format!("Fetched {bytes} bytes from {path}"),
                    _ => format!("Fetching {path}"),
                }
            }
            ScenaViewerProgressPhase::Parsing => match (self.nodes, self.meshes) {
                (Some(nodes), Some(meshes)) => {
                    format!("Parsed {path} with {nodes} nodes and {meshes} meshes")
                }
                _ => format!("Parsing {path}"),
            },
            ScenaViewerProgressPhase::Caching => format!("Caching {path}"),
            ScenaViewerProgressPhase::Complete => format!("Loaded {path}"),
        }
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
      style.textContent = ":host{display:block;min-width:160px;min-height:120px;contain:content;position:relative}:host([hidden]){display:none}canvas{display:block;width:100%;height:100%;touch-action:none;background:transparent}[part=progress]{position:absolute;left:12px;right:12px;bottom:12px;display:grid;gap:6px;color:#f8fafc;font:12px/1.4 system-ui,sans-serif;text-shadow:0 1px 2px #0f172a}[part=progress][hidden]{display:none}[part=progress]::before{content:\"\";display:block;height:4px;border-radius:999px;background:rgba(15,23,42,.52)}[part=progress-bar]{height:4px;margin-top:-10px;border-radius:999px;background:#60a5fa;transform-origin:left center;transform:scaleX(0)}[part=progress-status]{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}";
      const canvas = document.createElement("canvas");
      canvas.part = "canvas";
      canvas.tabIndex = 0;
      canvas.setAttribute("aria-label", "scena 3D viewer canvas");
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
      root.append(style, canvas, progress);
      this._canvas = canvas;
      this._progress = progress;
      this._progressBar = bar;
      this._progressStatus = status;
      this.addEventListener("scena-viewer-progress", (event) => {
        this.setLoadProgress(event.detail || {});
      });
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
