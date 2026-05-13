#![cfg(not(target_arch = "wasm32"))]

#[test]
fn first_render_gltf_headless_loads_frames_prepares_and_renders() {
    let first = pollster::block_on(scena::first_render_gltf_headless(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        64,
        64,
    ))
    .expect("first render helper loads and renders glTF");

    assert_eq!(first.outcome().width, 64);
    assert_eq!(first.outcome().height, 64);
    assert!(first.outcome().draw_calls > 0);
    assert!(!first.import().roots().is_empty());
    assert!(first.scene().active_camera().is_some());
    assert!(
        first
            .renderer()
            .screenshot_rgba8()
            .rgba8()
            .chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
        "first render helper produces visible pixels"
    );
}

#[test]
fn headless_gltf_viewer_builder_loads_frames_lights_and_renders() {
    let first = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(80, 48)
            .with_default_light()
            .render(),
    )
    .expect("builder renders the first glTF frame");

    assert_eq!(first.outcome().width, 80);
    assert_eq!(first.outcome().height, 48);
    assert!(first.outcome().draw_calls > 0);
    assert!(first.scene().active_camera().is_some());
    assert!(
        first
            .renderer()
            .screenshot_rgba8()
            .rgba8()
            .chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
        "viewer builder should produce visible pixels without user-authored matrix math"
    );
}

#[test]
fn headless_gltf_viewer_builder_can_attach_environment_and_report_diagnostics() {
    let first = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(80, 48)
            .with_default_environment()
            .render(),
    )
    .expect("builder renders with a default environment");

    assert_eq!(
        first.renderer().environment(),
        Some(first.assets().default_environment())
    );
    assert_eq!(first.renderer().stats().environments, 1);
    assert_eq!(first.diagnostics(), first.renderer().diagnostics());
}

#[test]
fn headless_gltf_viewer_builder_can_build_on_change_render_loop() {
    let mut viewer = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(80, 48)
            .with_default_light()
            .on_change()
            .build(),
    )
    .expect("builder creates a prepared viewer loop");

    assert_eq!(viewer.renderer().render_mode(), scena::RenderMode::OnChange);
    assert!(viewer.scene().active_camera().is_some());

    let first = viewer
        .render_next_frame()
        .expect("first viewer-loop frame renders");
    assert!(!first.skipped);
    assert!(first.draw_calls > 0);

    let idle = viewer
        .render_next_frame()
        .expect("unchanged viewer-loop frame skips");
    assert!(idle.skipped);
    assert_eq!(idle.draw_calls, 0);
    assert_eq!(viewer.renderer().stats().skipped_frames, 1);

    assert!(
        viewer
            .renderer()
            .screenshot_rgba8()
            .rgba8()
            .chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
        "viewer loop should keep the first rendered frame visible across idle skips"
    );
}

#[test]
fn headless_gltf_viewer_builder_with_environment_loads_explicit_path() {
    // Phase 5B step 1: `with_environment(path)` accepts an explicit asset
    // path and overrides the default-environment toggle. Loading the
    // bundled neutral-studio fixture should attach an environment handle
    // distinct from `Assets::default_environment` only when the path is
    // different from the bundled one — for the same path it returns the
    // cached default handle, which is still a positive signal that the
    // override path took.
    let first = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(48, 32)
            .with_environment("tests/assets/environment/neutral-studio.fixture.txt")
            .render(),
    )
    .expect("builder accepts explicit environment paths");

    assert!(
        first.renderer().environment().is_some(),
        "with_environment must attach an environment handle to the renderer"
    );
    assert_eq!(first.renderer().stats().environments, 1);
}

#[test]
fn headless_gltf_viewer_with_environment_overrides_default_environment_call_order() {
    // Phase 5B step 1: setting `with_default_environment` then
    // `with_environment(path)` must end up using the explicit path —
    // confirming the override semantics documented on the builder.
    let first = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(32, 32)
            .with_default_environment()
            .with_environment("tests/assets/environment/neutral-studio.fixture.txt")
            .render(),
    )
    .expect("builder accepts explicit environment after default toggle");

    assert!(first.renderer().environment().is_some());
}

#[test]
fn headless_gltf_viewer_snapshot_rgba8_and_capabilities_accessors_match_renderer() {
    // Phase 5B step 1: `snapshot_rgba8` and `capabilities` are convenience
    // accessors that forward to the renderer; their results must match
    // `renderer.frame_rgba8()` / `renderer.capabilities()` so callers can
    // skip the indirection in screenshot + capability-gate code paths.
    let viewer = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(16, 16)
            .with_default_light()
            .build(),
    )
    .expect("viewer builder produces a prepared viewer");

    let mut viewer = viewer;
    viewer.render_next_frame().expect("first frame renders");

    assert_eq!(viewer.snapshot_rgba8(), viewer.renderer().frame_rgba8());
    assert_eq!(viewer.capabilities(), viewer.renderer().capabilities());
}
