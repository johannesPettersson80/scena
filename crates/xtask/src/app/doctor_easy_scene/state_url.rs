use crate::app::prelude::*;

pub(super) fn check_state_via_url(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "STATE-VIA-URL",
        "Cargo.toml",
        &[
            "serde = { version = \"1\", features = [\"derive\"] }",
            "urlencoding = \"2\"",
        ],
    );
    require_contains(
        root,
        findings,
        "STATE-VIA-URL",
        "src/controls.rs",
        &[
            "mod url_state;",
            "CameraOrbitUrlState",
            "CameraOrbitUrlStateError",
        ],
    );
    require_contains(
        root,
        findings,
        "STATE-VIA-URL",
        "src/controls/url_state.rs",
        &[
            "pub struct CameraOrbitUrlState",
            "Serialize, Deserialize",
            "pub enum CameraOrbitUrlStateError",
            "from_url_query",
            "to_query_string",
            "camera-orbit",
            "camera-target",
            "urlencoding::encode",
            "urlencoding::decode",
        ],
    );
    require_contains(
        root,
        findings,
        "STATE-VIA-URL",
        "src/lib.rs",
        &["CameraOrbitUrlState", "CameraOrbitUrlStateError"],
    );
    require_contains(
        root,
        findings,
        "STATE-VIA-URL",
        "tests/round_d_viewer_url_state.rs",
        &[
            "camera_orbit_url_state_round_trips_orbit_controls",
            "camera_orbit_url_state_accepts_compact_checklist_query_shape",
            "camera_orbit_url_state_omits_asset_urls_and_secrets",
            "framing_outcome_exports_camera_orbit_url_state",
        ],
    );
    require_contains(
        root,
        findings,
        "STATE-VIA-URL",
        "docs/guides/easy-scene-setup.md",
        &[
            "controls.url_state().to_query_string()",
            "CameraOrbitUrlState::from_url_query",
            "controls.with_url_state(state)",
            "framing.url_state().to_query_string()",
        ],
    );
}
