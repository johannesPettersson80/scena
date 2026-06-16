#![cfg(feature = "inspection")]

use scena::{
    Aabb, Assets, Backend, CAPTURE_SCHEMA_V1, ClippingPlane, ClippingPlaneSet, Color, GeometryDesc,
    MaterialDesc, RENDER_INTROSPECTION_SCHEMA_V1, RenderIntrospectionOptions,
    RenderIntrospectionReportV1, Renderer, RendererStats, Scene, SceneInspectionReportV1,
    Transform, Vec3, capture_rgba8_from_pixels,
};

#[test]
fn render_introspection_classifies_agent_failure_frames() {
    let (assets, scene, renderer) = rendered_box_scene(64, 64);
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();

    let empty = introspect_supplied_pixels(
        &scene,
        &renderer,
        &inspection,
        vec![0; 64 * 64 * 4],
        RendererStats::default(),
    );
    assert!(!empty.ok);
    assert_reason(&empty, "empty_frame");
    assert_fix(&empty, "frame_bounds");

    let all_culled_stats = RendererStats {
        culled_objects: 1,
        ..Default::default()
    };
    let all_culled = introspect_supplied_pixels(
        &scene,
        &renderer,
        &inspection,
        vec![0; 64 * 64 * 4],
        all_culled_stats,
    );
    assert!(!all_culled.ok);
    assert_reason(&all_culled, "all_culled");
    assert_fix(&all_culled, "frame_bounds");

    let tiny = introspect_supplied_pixels(
        &scene,
        &renderer,
        &inspection,
        one_pixel_frame(64, 64, 32, 32),
        RendererStats::default(),
    );
    assert!(
        tiny.ok,
        "warning-only framing reports must not fail the agent loop: {tiny:#?}"
    );
    assert_reason(&tiny, "tiny_in_frame");

    let cropped = introspect_supplied_pixels(
        &scene,
        &renderer,
        &inspection,
        cropped_frame(64, 64),
        RendererStats::default(),
    );
    assert!(
        cropped.ok,
        "warning-only framing reports must not fail the agent loop: {cropped:#?}"
    );
    assert_reason(&cropped, "cropped");

    let near_edge = introspect_supplied_pixels(
        &scene,
        &renderer,
        &inspection,
        near_edge_frame(64, 64),
        RendererStats::default(),
    );
    assert!(
        near_edge.ok,
        "edge warning reports must not fail the agent loop: {near_edge:#?}"
    );
    assert_reason(&near_edge, "cropped");
}

#[test]
fn render_introspection_classifies_camera_transform_material_and_clipping_failures() {
    let behind = rendered_box_scene_at(64, 64, Vec3::new(0.0, 0.0, 4.0), Color::WHITE, false);
    let behind_report = introspect_rendered_scene(&behind);
    assert!(!behind_report.ok, "{behind_report:#?}");
    assert_reason(&behind_report, "behind_camera");
    assert_fix(&behind_report, "frame_bounds");

    let outside = rendered_box_scene_at(64, 64, Vec3::new(100.0, 0.0, 0.0), Color::WHITE, false);
    let outside_report = introspect_rendered_scene(&outside);
    assert!(!outside_report.ok, "{outside_report:#?}");
    assert_reason(&outside_report, "outside_frustum");
    assert_fix(&outside_report, "frame_bounds");

    let alpha_zero = rendered_box_scene_at(64, 64, Vec3::ZERO, Color::TRANSPARENT, false);
    let alpha_zero_report = introspect_rendered_scene(&alpha_zero);
    assert!(!alpha_zero_report.ok, "{alpha_zero_report:#?}");
    assert_reason(&alpha_zero_report, "alpha_zero");
    assert!(
        alpha_zero_report
            .fixes
            .iter()
            .all(|fix| fix.action != "set_material_alpha"),
        "material alpha requires a host material edit and must not advertise a patchless render-introspection fix"
    );

    let nan_transform =
        rendered_box_scene_at(64, 64, Vec3::new(f32::NAN, 0.0, 0.0), Color::WHITE, false);
    let nan_transform_report = introspect_rendered_scene(&nan_transform);
    assert!(!nan_transform_report.ok, "{nan_transform_report:#?}");
    assert_reason(&nan_transform_report, "nan_transform");
    assert_fix(&nan_transform_report, "set_transform");

    let clipped = rendered_box_scene_at(64, 64, Vec3::ZERO, Color::WHITE, true);
    let clipped_report = introspect_supplied_pixels(
        &clipped.1,
        &clipped.2,
        &clipped.1.inspect_with_assets(&clipped.0).to_schema_report(),
        vec![0; 64 * 64 * 4],
        RendererStats::default(),
    );
    assert!(!clipped_report.ok, "{clipped_report:#?}");
    assert_reason(&clipped_report, "clipped_by_active_clipping_plane");
    assert_fix(&clipped_report, "clear_clipping_planes");
}

#[test]
fn render_introspection_uses_configured_background_for_content_detection() {
    let background = Color::from_srgb_u8(32, 48, 64);
    let background_rgba8 = [32, 48, 64, 255];
    let (assets, scene, renderer) = rendered_box_scene_with_background(64, 64, background);
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let capture = capture_rgba8_from_pixels(
        &scene,
        &renderer,
        Default::default(),
        64,
        64,
        solid_frame(64, 64, background_rgba8),
    )
    .expect("supplied background-only frame captures against rendered scene state");

    let report =
        renderer.introspect_capture(&capture, &inspection, RenderIntrospectionOptions::default());

    assert!(!report.ok, "{report:#?}");
    assert_reason(&report, "empty_frame");
    assert_eq!(report.visible_pixel_fraction, 0.0);
    assert_eq!(report.content_bbox_css_px, None);
    assert_eq!(report.content_bbox_fraction, None);
}

#[test]
fn render_introspection_reports_valid_centered_content_and_is_deterministic() {
    let (assets, scene, renderer) = rendered_box_scene(64, 64);
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let capture = renderer
        .capture_rgba8(&scene, Default::default())
        .expect("rendered scene captures");

    let first =
        renderer.introspect_capture(&capture, &inspection, RenderIntrospectionOptions::default());
    let second =
        renderer.introspect_capture(&capture, &inspection, RenderIntrospectionOptions::default());

    assert!(first.ok, "{first:#?}");
    assert_eq!(first.schema, RENDER_INTROSPECTION_SCHEMA_V1);
    assert!(first.visible_pixel_fraction > 0.01);
    assert!(first.content_bbox_css_px.is_some());
    assert!(first.content_bbox_fraction.is_some());
    assert_eq!(first.framing.active_camera, inspection.active_camera);
    assert!(!first.framing.cropped);
    assert!(!first.framing.tiny_in_frame);
    assert_eq!(first.artifacts.capture.schema, CAPTURE_SCHEMA_V1);
    assert_eq!(first.capabilities.backend, Backend::Headless);
    assert_eq!(
        first, second,
        "same frame must produce byte-stable JSON data"
    );
    assert_eq!(
        serde_json::to_string(&first).expect("report serializes"),
        serde_json::to_string(&second).expect("report serializes"),
        "same frame must produce byte-stable JSON"
    );
}

#[test]
fn render_introspection_golden_fixture_matches_live_schema() {
    let fixture: RenderIntrospectionReportV1 = serde_json::from_str(include_str!(
        "assets/stable-contracts/render_introspection.v1.json"
    ))
    .expect("fixture deserializes");

    assert_eq!(fixture.schema, RENDER_INTROSPECTION_SCHEMA_V1);
    assert!(fixture.ok);
    assert!(fixture.content_bbox_css_px.is_some());
    assert_eq!(fixture.artifacts.capture.schema, CAPTURE_SCHEMA_V1);
}

fn introspect_supplied_pixels(
    scene: &Scene,
    renderer: &Renderer,
    inspection: &SceneInspectionReportV1,
    rgba8: Vec<u8>,
    stats: RendererStats,
) -> RenderIntrospectionReportV1 {
    let capture = capture_rgba8_from_pixels(scene, renderer, Default::default(), 64, 64, rgba8)
        .expect("supplied frame captures against rendered scene state");
    RenderIntrospectionReportV1::from_capture(
        &capture,
        inspection,
        stats,
        RenderIntrospectionOptions::default(),
    )
}

fn rendered_box_scene(width: u32, height: u32) -> (Assets, Scene, Renderer) {
    rendered_box_scene_with_background(width, height, Color::BLACK)
}

fn rendered_box_scene_with_background(
    width: u32,
    height: u32,
    background: Color,
) -> (Assets, Scene, Renderer) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("default camera inserts");
    scene
        .mesh(geometry, material)
        .add()
        .expect("box mesh inserts");
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_background_color(background);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");
    (assets, scene, renderer)
}

fn rendered_box_scene_at(
    width: u32,
    height: u32,
    translation: Vec3,
    color: Color,
    clipping_plane: bool,
) -> (Assets, Scene, Renderer) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(color));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("default camera inserts");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(translation))
        .add()
        .expect("box mesh inserts");
    if clipping_plane {
        let plane = scene.add_clipping_plane(ClippingPlane::new(Vec3::X, -10.0));
        scene
            .set_clipping_planes(ClippingPlaneSet::new().with_plane(plane))
            .expect("clipping plane activates");
    }
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    let _ = renderer.render_active(&scene);
    (assets, scene, renderer)
}

fn introspect_rendered_scene(
    (assets, scene, renderer): &(Assets, Scene, Renderer),
) -> RenderIntrospectionReportV1 {
    let inspection = scene.inspect_with_assets(assets).to_schema_report();
    let capture = renderer
        .capture_rgba8(scene, Default::default())
        .expect("rendered scene captures");
    renderer.introspect_capture(&capture, &inspection, RenderIntrospectionOptions::default())
}

fn solid_frame(width: usize, height: usize, rgba8: [u8; 4]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(width * height * 4);
    for _ in 0..width * height {
        frame.extend_from_slice(&rgba8);
    }
    frame
}

fn one_pixel_frame(width: usize, height: usize, x: usize, y: usize) -> Vec<u8> {
    let mut rgba8 = vec![0; width * height * 4];
    let offset = (y * width + x) * 4;
    rgba8[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
    rgba8
}

fn cropped_frame(width: usize, height: usize) -> Vec<u8> {
    let mut rgba8 = vec![0; width * height * 4];
    for y in 8..56 {
        for x in 0..16 {
            let offset = (y * width + x) * 4;
            rgba8[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    rgba8
}

fn near_edge_frame(width: usize, height: usize) -> Vec<u8> {
    let mut rgba8 = vec![0; width * height * 4];
    for y in 8..56 {
        for x in 1..17 {
            let offset = (y * width + x) * 4;
            rgba8[offset..offset + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    rgba8
}

fn assert_reason(report: &RenderIntrospectionReportV1, code: &str) {
    assert!(
        report.reasons.iter().any(|reason| reason.code == code),
        "expected reason {code} in {:#?}",
        report.reasons
    );
}

fn assert_fix(report: &RenderIntrospectionReportV1, action: &str) {
    let fix = report
        .fixes
        .iter()
        .find(|fix| fix.action == action)
        .unwrap_or_else(|| panic!("expected fix {action} in {:#?}", report.fixes));
    if action == "frame_bounds" {
        assert!(
            fix.patch.is_some(),
            "frame_bounds fixes must carry a visual_patch camera payload: {fix:#?}"
        );
    }
}

#[allow(dead_code)]
fn _keep_aabb_in_public_test_surface(_: Aabb, _: Transform, _: Vec3) {}
