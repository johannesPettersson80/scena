//! Browser custom-element support for `<scena-viewer>`.
//!
//! The module keeps the browser adapter thin: attribute parsing is
//! platform-neutral and testable on native targets, while the actual custom
//! element registration is only exported for `wasm32` with the
//! `viewer-element` feature.

mod annotation_layout;
mod annotations;
mod inspector;
mod model;

pub use annotation_layout::{
    ScenaViewerAnnotationLayoutEntry, ScenaViewerAnnotationLayoutInput,
    ScenaViewerAnnotationLayoutOptions, ScenaViewerAnnotationLayoutReport,
    layout_scena_viewer_annotations,
};
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

    #[wasm_bindgen(module = "/src/viewer_element/element.js")]
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
