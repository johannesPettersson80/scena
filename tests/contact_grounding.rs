#![cfg(all(feature = "scene-host", feature = "inspection"))]

use std::fs;

use scena::{
    AssetPath, RenderIntrospectionOptions, RenderIntrospectionReportV1, SceneHostCore,
    SceneHostGroundingPathV1, SceneInspectionReportV1, Transform, Vec3,
};

#[test]
fn product_grounding_preset_renders_visible_receiver_and_reports_non_physical_shadow_scope() {
    let (grounded, grounded_report, grounded_stats) =
        render_product_grounding_scene().expect("grounded product scene renders");

    assert!(
        grounded.ok,
        "grounded render should be visible: {grounded:#?}"
    );
    let bbox = grounded
        .content_bbox_css_px
        .expect("grounded render must report non-background content bounds");
    assert!(
        grounded.visible_pixel_fraction > 0.01,
        "grounding preset should produce visible product and receiver pixels; fraction={}",
        grounded.visible_pixel_fraction
    );
    assert!(
        bbox.max_y > 104.0 * 0.6,
        "grounded receiver should reach the lower frame, bbox={bbox:?}"
    );
    assert!(
        grounded_stats.ambient_occlusion_passes > 0,
        "grounding preset must run the configured SSAO pass when the backend supports it"
    );
    assert!(
        grounded_report
            .active_paths
            .contains(&SceneHostGroundingPathV1::FloorReceiver)
    );
    assert!(
        grounded_report
            .active_paths
            .contains(&SceneHostGroundingPathV1::ScreenSpaceAmbientOcclusion)
    );
    assert!(
        !grounded_report
            .active_paths
            .contains(&SceneHostGroundingPathV1::DirectionalShadowReceiver),
        "4.1 must not promote the directional shadow receiver before 4.2 proof closure"
    );
    assert!(!grounded_report.physical_shadow_claimed);
}

fn render_product_grounding_scene() -> Result<
    (
        RenderIntrospectionReportV1,
        scena::SceneHostGroundingReportV1,
        scena::RendererStats,
    ),
    Box<dyn std::error::Error>,
> {
    let mut host = SceneHostCore::headless(128, 104)?;
    let target = host.add_empty(
        None,
        Transform::at(Vec3::new(0.0, 0.2, 0.0)),
        Some("contact-grounding-target"),
    )?;
    let import = pollster::block_on(host.instantiate_url_under(
        target,
        AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    ))?;
    let roots = host.import_roots(import)?;
    assert!(
        !roots.is_empty(),
        "fixture import must create visible roots"
    );

    let report = host.apply_product_grounding_preset(target, "studio_neutral")?;
    host.frame_node_product_view(target)?;
    let mut camera = host.camera_state();
    camera.distance *= 0.25;
    host.set_camera(camera)?;
    host.prepare()?;
    host.render()?;
    let capture = host.capture()?;
    fs::create_dir_all("target/gate-artifacts/contact-grounding")?;
    capture.write_png("target/gate-artifacts/contact-grounding/headless-product-grounding.png")?;
    let inspection: SceneInspectionReportV1 = serde_json::from_str(&host.inspect_json()?)?;
    let introspection = host.renderer().introspect_capture(
        &capture,
        &inspection,
        RenderIntrospectionOptions::default(),
    );
    Ok((introspection, report, host.renderer().stats()))
}
