use crate::app::prelude::*;

pub(crate) fn check_c17_visible_bounds_framing_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C17-VISIBLE-BOUNDS-FRAMING";

    require_contains(
        root,
        findings,
        RULE,
        "src/scene/view.rs",
        &[
            "pub fn frame_import_with_options(",
            "pub fn frame_all_with_options(",
            "pub fn frame_all_with_assets_and_options<F>(",
            "self.frame_import_with_options(camera, import, options)",
            "self.frame_all_with_options(camera, options)",
            "pub fn move_origin_to(",
            "#[deprecated(note = \"use move_origin_to or center_visible_bounds_on\")]",
            "pub fn center_visible_bounds_on<F>(",
            "visible_asset_backed_node_subtree_bounds_world(node, assets, false)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene/view_bounds.rs",
        &[
            "pub(super) fn scene_bounds_world",
            "pub(super) fn visible_asset_backed_node_subtree_bounds_world",
            "scena:inspection:helper",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene/framing.rs",
        &[
            "include_helpers: bool",
            "include_helpers: false",
            "pub const fn include_helpers(mut self, include: bool)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/viewer/load_progress.rs",
        &[
            "FramingOptions::new()",
            ".three_quarter_front_right()",
            ".viewport(self.width, self.height)",
            "let surface_size = self.surface.size();",
            ".viewport(surface_size.width, surface_size.height)",
            "build_orbit_controls(self.orbit_controls, &scene, &import, camera, framing)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/viewer/interaction.rs",
        &[
            "framing: Option<FramingOutcome>",
            "OrbitControls::from_framing(framing)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene_host/camera.rs",
        &[
            "frame_all_with_assets_and_options(",
            ".tighten_depth_range(true)",
            ".viewport(width, height)",
            "OrbitControls::from_framing(framing)",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c17_visible_bounds_framing.rs",
        &[
            "visible_bounds_centering_moves_content_center_not_node_origin",
            "framing_options_exclude_hidden_and_inspection_helpers_unless_requested",
            "aggregate_framing_contains_multiple_imports",
            "target_viewport_aspect_drives_projection_and_cpu_pixels",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/m7_interactive_viewer.rs",
        &["interactive_gltf_viewer_with_orbit_controls_attaches_controller_seeded_from_framing"],
    );
    for (path, needles) in [
        (
            "docs/api.md",
            &[
                "Scene::frame_all_with_assets_and_options",
                "Scene::center_visible_bounds_on",
                "actual output dimensions",
            ][..],
        ),
        (
            "docs/rendering.md",
            &[
                "include_helpers(false)",
                "Scene::move_origin_to",
                "camera's current aspect",
            ][..],
        ),
        (
            "docs/guides/migrating-from-threejs.md",
            &["Box3::setFromObject", "center_visible_bounds_on"][..],
        ),
        (
            "docs/specs/public-api.md",
            &[
                "Scene::frame_import_with_options",
                "FramingOptions::include_helpers",
            ][..],
        ),
        (
            "README.md",
            &["Framing builders use the real output size"][..],
        ),
        (
            "examples/camera_framing.rs",
            &["center_visible_bounds_on", "three_quarter_front_right"][..],
        ),
        (
            "tests/m5_release.rs",
            &[
                "Scene::frame_all_with_assets_and_options",
                "Scene::center_visible_bounds_on",
                "FramingOptions::include_helpers",
            ][..],
        ),
        (
            "CHANGELOG.md",
            &["Frame visible aggregate/import bounds"][..],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &["aggregate framing helpers centered all known bounds"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }
}
