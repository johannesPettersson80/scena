use crate::app::prelude::*;

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_prose_only_guide_snippets() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-prose-only-guide");
    write_easy_scene_fixture(
        &fixture_root,
        "Scene::new() scene.add_studio_lighting() scene.add_grid_floor( scene.frame_bounds(\n```rust\nlet scene = Scene::new();\n```",
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-EASY-SCENE-SETUP"),
        "doctor must reject guide snippets when required calls are only prose substrings: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_demo_orbit_literal_residue() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-orbit-literals");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor FramingOptions::new().orbit(-0.48, 0.31)",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-CAMERA-VIEWS-NAMED"),
        "doctor must reject inline demo orbit literal residue instead of only dead names: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_reordered_open_diagnostics() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-reordered-open");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details class="diagnostics" open id="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-DIAGNOSTICS"),
        "doctor must reject open diagnostics regardless of attribute order: {findings:?}",
    );
}

const VALID_GUIDE: &str = "frame_bounds add_studio_lighting add_grid_floor set_auto_exposure scene.mate project_world_point Camera views azimuth_elevation three_quarter_front_right\n```rust\nlet mut scene = Scene::new();\nscene.add_studio_lighting()?;\nscene.add_grid_floor(&assets, GridFloorOptions::new())?;\nscene.frame_bounds(camera, bounds, FramingOptions::new().azimuth_elevation(-27.5, 17.8))?;\n```";

fn write_easy_scene_fixture(
    fixture_root: &Path,
    guide: &str,
    demo_rs: &str,
    diagnostics_html: &str,
) {
    let _ = fs::remove_dir_all(fixture_root);
    for dir in [
        "demo",
        "docs/guides",
        "docs/release-notes",
        "src",
        "src/demo_page",
        "src/scene",
        "tests",
    ] {
        fs::create_dir_all(fixture_root.join(dir)).expect("fixture dir");
    }
    fs::write(fixture_root.join("docs/guides/easy-scene-setup.md"), guide).expect("guide fixture");
    fs::write(
        fixture_root.join("docs/guides/migrating-from-threejs.md"),
        "new THREE.Box3 controls.target.copy OrbitControls::from_framing spherical.theta spherical.phi azimuth_elevation",
    )
    .expect("migration fixture");
    fs::write(
        fixture_root.join("docs/release-notes/v1.3.0.md"),
        "Status: ready OrbitControls::from_framing Aabb::union ScreenRect ProjectedPoint GridFloorHandles LookupError::InvalidBounds LookupError::UnsupportedCameraType FramingOptions::azimuth_elevation FramingOptions::front FramingOptions::back FramingOptions::left FramingOptions::right FramingOptions::top FramingOptions::bottom FramingOptions::three_quarter_front_left FramingOptions::three_quarter_front_right FramingOptions::three_quarter_back_left FramingOptions::three_quarter_back_right",
    )
    .expect("release notes fixture");
    fs::write(
        fixture_root.join("docs/README.md"),
        "Easy scene setup guides/easy-scene-setup.md",
    )
    .expect("docs readme fixture");
    fs::write(
        fixture_root.join("README.md"),
        "## Easy Scene Setup docs/guides/easy-scene-setup.md docs/release-notes/v1.3.0.md",
    )
    .expect("readme fixture");
    fs::write(fixture_root.join("src/demo_page.rs"), demo_rs).expect("demo fixture");
    fs::write(
        fixture_root.join("src/demo_page/connectors.rs"),
        "project_world_point",
    )
    .expect("connector projection fixture");
    fs::write(
        fixture_root.join("src/diagnostics.rs"),
        "InvalidBounds InvalidFramingOption UnsupportedCameraType A viewport width or height was zero Bounds were empty A named framing option failed validation does not support the camera type",
    )
    .expect("diagnostics fixture");
    fs::write(
        fixture_root.join("src/scene/framing.rs"),
        "pre-existing aspect # Examples # Errors LookupError::UnsupportedCameraType LookupError::InvalidFramingOption",
    )
    .expect("framing fixture");
    fs::write(fixture_root.join("src/scene/lights.rs"), "studio docs").expect("lights fixture");
    fs::write(
        fixture_root.join("tests/examples_visual_proof.rs"),
        "frame_bounds_rendered_output_proves_fill_center_and_unclipped_object frame-bounds-rendered-output computed_distance projected_rect nonblack_pixel_rect",
    )
    .expect("visual proof fixture");
    fs::write(fixture_root.join("src/lib.rs"), "").expect("lib fixture");
    fs::write(fixture_root.join("src/geometry.rs"), "").expect("geometry fixture");
    fs::write(fixture_root.join("demo/index.html"), diagnostics_html).expect("demo html fixture");
    fs::write(
        fixture_root.join("demo/main.js"),
        "setStatus('demo', 'rendered');",
    )
    .expect("demo js fixture");
}
