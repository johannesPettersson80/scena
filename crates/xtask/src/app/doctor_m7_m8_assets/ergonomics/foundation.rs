use crate::app::prelude::*;

pub(crate) fn check_m7_foundation_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_files(
        root,
        findings,
        "ERGONOMICS-M7",
        &[
            "docs/guides/place-and-connect-objects.md",
            "docs/guides/units-axes-handedness.md",
            "docs/guides/authoring-gltf-anchors-connectors.md",
            "docs/guides/migrating-from-threejs.md",
            "docs/guides/troubleshooting-misplaced-assets.md",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/controls.rs",
        &[
            "with_damping",
            "focus",
            "apply_to_scene",
            "damping_factor",
            "TouchEvent",
            "pub const fn wheel",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/platform.rs",
        &["SurfaceViewport", "ViewportChanged", "device_pixel_ratio"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/picking.rs",
        &["pick_pointer", "pick_and_select", "InvalidViewport"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/visibility.rs",
        &[
            "set_camera_layer_mask",
            "camera_layer_mask",
            "visible_for_active_camera",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/import.rs",
        &[
            "ImportAnchorDebugMetadata",
            "YUpLeftHanded",
            "ZUpLeftHanded",
            "source_units",
            "source_coordinate_system",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/import/options.rs",
        &[
            "meters_per_unit",
            "convert_position",
            "convert_connector_transform",
            "has_negative_determinant",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/connectors.rs",
        &["source_coordinate_system", "connection_transform"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/connectors/error.rs",
        &["HandednessMismatch", "ConnectorHostNotPrepared"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/connectors/validation.rs",
        &[
            "validate_connector_handedness",
            "validate_connector_host_prepared",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/inspection.rs",
        &[
            "pub struct SceneInspectionReport",
            "pub struct SceneNodeInspection",
            "pub struct SceneDrawInspection",
            "pub fn inspect(&self)",
            "pub fn draw_list",
            "visible_drawable_count",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/assets.rs",
        &[
            "create_static_batch",
            "create_static_batch_with_report",
            "assets.material(texture)",
            "assets.geometry(material)",
            "assets.texture(material)",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene.rs",
        &["scene.mesh(geometry, texture)"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/static_batch.rs",
        &["pub fn static_batch", "transform_point"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/static_batch.rs",
        &[
            "pub struct StaticBatchReport",
            "pub fn static_batch_report",
            "requires_prepare_after_rebuild",
            "picking_debug_instances",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/helpers.rs",
        &[
            "pub fn bounding_box",
            "pub fn camera_frustum",
            "pub fn light_helper",
            "pub fn anchor_marker",
            "pub fn normal_lines",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/diagnostics/help.rs",
        &["add_default_camera", "anchors_named", "recover_context"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/render.rs",
        &[
            "pub fn diagnose_scene",
            "pub fn diagnose_scene_with_assets",
            "pub fn capability_report",
            "MissingActiveCamera",
            "InvisibleScene",
            "MissingLightingOrEnvironment",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/diagnostics/capabilities.rs",
        &[
            "pub struct CapabilityReport",
            "pub fn new(capabilities: Capabilities, adapter: Option<GpuAdapterReport>)",
            "pub const fn backend",
            "pub fn adapter",
            "pub fn diagnostics",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "README.md",
        &[
            "## Happy Path",
            "examples/camera_framing.rs",
            "examples/anchor_alignment.rs",
            "examples/connect_objects.rs",
            "examples/imported_anchor_connection.rs",
            "examples/industrial_connector_assembly.rs",
            "examples/coordinate_connector_repair.rs",
            "examples/coordinate_units.rs",
            "examples/static_batching.rs",
            "examples/layers_visibility.rs",
            "examples/beginner_diagnostics.rs",
        ],
    );
}
