use crate::app::prelude::*;

pub(super) fn check_scena_viewer_element(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "SCENA-VIEWER-ELEMENT",
        "Cargo.toml",
        &["viewer-element = []"],
    );
    require_contains(
        root,
        findings,
        "SCENA-VIEWER-ELEMENT",
        "src/lib.rs",
        &[
            "pub mod viewer_element;",
            "SCENA_VIEWER_TAG",
            "ScenaViewerAttributes",
            "ScenaViewerProgress",
            "ScenaViewerProgressPhase",
            "define_scena_viewer",
        ],
    );
    require_contains(
        root,
        findings,
        "SCENA-VIEWER-ELEMENT",
        "src/viewer_element.rs",
        &[
            "pub const SCENA_VIEWER_TAG",
            "pub struct ScenaViewerAttributes",
            "from_pairs",
            "pub enum ScenaViewerProgressPhase",
            "pub struct ScenaViewerProgress",
            "from_asset_event",
            "aria_text",
            "defineScenaViewerElement",
            "customElements.define",
            "attachShadow",
            "observedAttributes",
            "scena-viewer-ready",
            "setLoadProgress",
            "progress.part = \"progress\"",
            "progressbar",
            "scena-viewer-progress",
            "scena-viewer-progress-rendered",
        ],
    );
    require_contains(
        root,
        findings,
        "SCENA-VIEWER-ELEMENT",
        "tests/scena_viewer_element.rs",
        &[
            "scena_viewer_attributes_parse_model_viewer_style_surface",
            "scena_viewer_attributes_default_to_safe_drop_in_viewer_values",
            "scena_viewer_progress_maps_asset_events_to_accessible_details",
            "camera-controls",
            "tone-mapping",
        ],
    );
    require_contains(
        root,
        findings,
        "SCENA-VIEWER-ELEMENT",
        "docs/browser.md",
        &[
            "<scena-viewer",
            "defineScenaViewer",
            "viewer-element",
            "shadow DOM",
            "canvas",
            "progressbar",
            "scena-viewer-progress",
        ],
    );
    require_contains(
        root,
        findings,
        "SCENA-VIEWER-ELEMENT",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "custom-element\nfoundation **[shipped]**",
            "src/viewer_element.rs",
            "SCENA-VIEWER-ELEMENT",
            "ScenaViewerProgress",
            "scena-viewer-progress-rendered",
            "Full\n  asset loading/rendering parity remains open under bet 1.1",
        ],
    );
}
