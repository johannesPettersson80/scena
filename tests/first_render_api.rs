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
fn headless_gltf_viewer_defaults_make_pbr_assets_visible_and_explain_the_fallback() {
    let first = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/cad_terminal_block.gltf")
            .size(96, 96)
            .render(),
    )
    .expect("documented viewer defaults render a PBR glTF");

    assert_eq!(
        first.renderer().background_color(),
        scena::Background::Studio.color(),
        "the high-level viewer should not inherit the low-level black clear color"
    );
    let scene_diagnostics = first
        .renderer()
        .diagnose_scene_with_assets(first.scene(), first.assets());
    assert!(
        !scene_diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == scena::DiagnosticCode::MissingLightingOrEnvironment
        }),
        "the default viewer must install lighting when the asset authors none: {scene_diagnostics:#?}"
    );
    assert!(
        first.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == scena::DiagnosticCode::MissingLightingOrEnvironment
                && diagnostic.message().contains("applied")
                && diagnostic
                    .help()
                    .is_some_and(|help| help.contains("author"))
                && diagnostic.setting() == Some("viewer.lighting")
                && diagnostic.fallback_applied()
        }),
        "the high-level result must explain that its neutral fallback was applied: {:#?}",
        first.diagnostics()
    );
    let fallback = first
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.fallback_applied())
        .expect("fallback diagnostic is retained on the high-level outcome");
    let fallback_json = serde_json::to_value(fallback).expect("diagnostic serializes");
    assert_eq!(fallback_json["setting"], "viewer.lighting");
    assert_eq!(fallback_json["fallback_applied"], true);
    assert!(
        fallback_json["help"]
            .as_str()
            .is_some_and(|help| !help.is_empty())
    );

    let distinct_rgb = first
        .renderer()
        .frame_rgba8()
        .chunks_exact(4)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        distinct_rgb.len() > 8,
        "PBR geometry must be visible against the neutral background, distinct_colors={}",
        distinct_rgb.len()
    );

    let low_level = scena::Renderer::headless(8, 8).expect("low-level renderer builds");
    assert_eq!(
        low_level.background_color(),
        scena::Background::Black.color(),
        "low-level deterministic renderer construction must remain explicit"
    );
}

#[test]
fn headless_gltf_viewer_allows_an_explicit_diagnostic_lighting_opt_out() {
    let first = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/cad_terminal_block.gltf")
            .size(48, 48)
            .without_default_lighting()
            .with_background(scena::Background::Black)
            .render(),
    )
    .expect("explicit diagnostic setup still renders bytes");

    assert!(
        first.outcome().draw_calls > 0,
        "the no-light PBR scene is technically rendered"
    );
    // Pixel darkness is not the opt-out contract: CPU's explicit no-light PBR
    // diagnostic path preserves base color, while GPU lighting can be dark.
    // The structured warning below is the cross-backend evidence that no
    // fallback was silently applied.
    let missing = first
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.code() == scena::DiagnosticCode::MissingLightingOrEnvironment)
        .expect("explicit opt-out surfaces the existing missing-light diagnosis");
    assert!(!missing.fallback_applied());
    assert_eq!(missing.setting(), None);
}

#[cfg(feature = "inspection")]
#[test]
fn headless_gltf_viewer_reports_render_introspection() {
    let first = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf")
            .size(80, 48)
            .with_viewer_profile(scena::ViewerProfile::model_viewer())
            .render(),
    )
    .expect("builder renders the first glTF frame");

    let first_report = first
        .render_introspection(scena::RenderIntrospectionOptions::summary())
        .expect("first render exposes render introspection");
    assert_eq!(first_report.schema, scena::RENDER_INTROSPECTION_SCHEMA_V1);
    assert!(first_report.ok, "{first_report:#?}");
    assert!(
        first_report.visible_pixel_fraction > 0.0,
        "{first_report:#?}"
    );

    let mut viewer = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf")
            .size(80, 48)
            .with_viewer_profile(scena::ViewerProfile::model_viewer())
            .build(),
    )
    .expect("builder creates a reusable viewer");
    viewer
        .render_next_frame()
        .expect("viewer loop renders a frame");
    let viewer_report = viewer
        .render_introspection(scena::RenderIntrospectionOptions::summary())
        .expect("viewer loop exposes render introspection");
    assert_eq!(
        viewer_report.capabilities.backend,
        first_report.capabilities.backend
    );
    assert!(viewer_report.ok, "{viewer_report:#?}");
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
    assert!(
        first
            .diagnostics()
            .iter()
            .all(|diagnostic| !diagnostic.fallback_applied()),
        "an explicit environment must suppress the lighting fallback"
    );
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
fn headless_gltf_viewer_surfaces_asset_load_progress() {
    let mut observed = Vec::new();
    let viewer = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(24, 24)
            .build_with_progress(|event| observed.push(event)),
    )
    .expect("builder creates a prepared viewer with load progress");

    assert_eq!(viewer.load_progress_events(), observed.as_slice());
    assert!(observed.iter().any(|event| matches!(
        event,
        scena::AssetLoadProgress::LoadStarted { path }
            if path.as_str() == "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
    )));
    assert!(observed.iter().any(|event| matches!(
        event,
        scena::AssetLoadProgress::AssetFetched { path, bytes }
            if path.as_str() == "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
                && *bytes > 0
    )));
    assert!(observed.iter().any(|event| matches!(
        event,
        scena::AssetLoadProgress::Parsed { path, nodes, meshes }
            if path.as_str() == "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
                && *nodes > 0
                && *meshes > 0
    )));
    assert!(observed.iter().any(|event| matches!(
        event,
        scena::AssetLoadProgress::Cached { path }
            if path.as_str() == "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
    )));
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

#[test]
fn headless_gltf_viewer_switches_material_variants_and_reprepares() {
    let mut viewer = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/material_variants_scene.gltf")
            .size(48, 48)
            .build(),
    )
    .expect("viewer builds a variants fixture");

    assert_eq!(
        viewer.material_variants(),
        &["midnight".to_string(), "noon".to_string()],
    );
    assert_eq!(viewer.active_material_variant(), None);
    let default_material = variant_mesh_material(viewer.scene(), viewer.import());

    viewer
        .set_active_material_variant(Some("midnight"))
        .expect("viewer applies a known material variant");
    let midnight_material = variant_mesh_material(viewer.scene(), viewer.import());
    assert_ne!(
        default_material, midnight_material,
        "variant must swap the imported mesh material",
    );
    assert_eq!(
        viewer.active_material_variant(),
        Some("midnight".to_string())
    );
    assert!(
        viewer
            .render_next_frame()
            .expect("variant switch prepares before rendering")
            .draw_calls
            > 0,
    );

    viewer
        .set_active_material_variant(None)
        .expect("viewer clears material variant");
    assert_eq!(
        variant_mesh_material(viewer.scene(), viewer.import()),
        default_material,
        "clearing the variant restores the default material",
    );
    assert_eq!(viewer.active_material_variant(), None);
}

fn variant_mesh_material(
    scene: &scena::Scene,
    import: &scena::SceneImport,
) -> scena::MaterialHandle {
    for root in import.roots() {
        if let Some(handle) = walk_for_mesh(scene, *root) {
            return handle;
        }
    }
    panic!("scene has no mesh node under variant import");
}

fn walk_for_mesh(scene: &scena::Scene, node_key: scena::NodeKey) -> Option<scena::MaterialHandle> {
    let node = scene.node(node_key)?;
    if let scena::NodeKind::Mesh(mesh) = node.kind() {
        return Some(mesh.material());
    }
    for child in node.children() {
        if let Some(handle) = walk_for_mesh(scene, *child) {
            return Some(handle);
        }
    }
    None
}
