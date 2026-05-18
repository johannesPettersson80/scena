use crate::app::prelude::*;

pub(crate) fn check_m7_example_contracts(root: &Path, findings: &mut Vec<Finding>) {
    if let Ok(entries) = fs::read_dir(root.join("examples")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                let rel = Path::new("examples").join(entry.file_name());
                forbid_contains_path(
                    root,
                    findings,
                    "ERGONOMICS-M7",
                    &rel,
                    &["Primitive::unlit_triangle()", "add_renderable("],
                );
            }
        }
    }
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/first_visible_render.rs",
        // `Scene::with_default_camera()` is the smaller-of-two-default-camera
        // paths shipped in commit f3105a3 (api-ergonomics-F1 closure) and
        // wraps `add_default_camera` internally; either ergonomic counts.
        &["with_default_camera", "render_active"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/industrial_connector_assembly.rs",
        &[
            "ConnectorFrame::from_import_connector",
            "ConnectionAlignment::ForwardToBack",
            "lock_node_for_connections",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/coordinate_connector_repair.rs",
        &[
            "ConnectionError::HandednessMismatch",
            "SourceCoordinateSystem::YUpLeftHanded",
            "SourceCoordinateSystem::ZUpRightHanded",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/controls.rs",
        &["apply_to_scene", "ensure_camera_depth_reaches"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls.rs",
        &[
            "OrbitControls",
            "with_damping",
            "apply_to_scene",
            "TouchEvent",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls_native_adapter.rs",
        &[
            "NativeMouseButton",
            "native_press",
            "PointerEvent::released",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls_browser_adapter.rs",
        &["browser_pointer_drag", "browser_wheel", "browser_pinch"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/camera_framing.rs",
        &["frame_bounds(", "FramingOptions", "from_framing", "render("],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/anchor_alignment.rs",
        &[
            "snap_anchor",
            "anchor(\"inspection\")",
            "anchor_debug_metadata",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/connect_objects.rs",
        &["add_connector", "connect_by_key", "ConnectorFrame::new"],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/connect_objects.rs",
        &["Mat4", "from_matrix"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/imported_anchor_connection.rs",
        &[
            "ConnectorFrame::from_import_anchor",
            "connect_by_key",
            "anchor(\"inspection\")",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/imported_anchor_connection.rs",
        &["Mat4", "from_matrix"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/coordinate_units.rs",
        &[
            "SourceUnits::Millimeters",
            "SourceCoordinateSystem::ZUpRightHanded",
            "meters_per_unit",
            "convert_position",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/static_batching.rs",
        &[
            "create_static_batch_with_report",
            "requires_prepare_after_rebuild",
            "picking_debug_instances",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/layers_visibility.rs",
        &[
            "set_layer_mask",
            "set_camera_layer_mask",
            "set_render_group",
            "set_helper_on_top",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/beginner_diagnostics.rs",
        &[
            "diagnose_scene",
            "DiagnosticSeverity::Error",
            "add_default_camera",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/diagnostics/diagnostic.rs",
        &[
            "pub fn code(&self) -> DiagnosticCode",
            "pub fn severity(&self) -> DiagnosticSeverity",
            "pub fn message(&self) -> &str",
            "pub fn help(&self) -> Option<&str>",
            "pub fn suggested_fix(&self) -> Option<&str>",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/glb_model_viewer.rs",
        &["first_render_gltf_headless", "first.import().roots()"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/viewer.rs",
        &[
            "pub struct FirstRender",
            "pub struct HeadlessGltfViewer",
            "pub struct HeadlessGltfViewerBuilder",
            "pub fn headless_gltf_viewer",
            "pub const fn size",
            "pub const fn with_default_light",
            "pub const fn with_default_environment",
            "pub const fn with_render_mode",
            "pub const fn on_change",
            "pub async fn build",
            "pub async fn render",
            "pub fn render_next_frame",
            "pub fn prepare",
            "pub async fn first_render_gltf_headless",
            "assets.load_scene",
            "assets.default_environment",
            "scene.instantiate",
            "scene.frame_import",
            "renderer.set_environment",
            "renderer.prepare_with_assets",
            "renderer.render_active",
            "renderer.diagnostics().to_vec()",
        ],
    );
}
