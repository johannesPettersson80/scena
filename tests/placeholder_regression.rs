#[cfg(feature = "inspection")]
use scena::{AlphaMode, RenderIntrospectionOptions};
use scena::{
    AntiAliasing, Assets, Color, GeometryDesc, GeometryTopology, GeometryVertex, MaterialDesc,
    PerspectiveCamera, Quality, Renderer, RendererOptions, Scene, Transform, Vec3,
};

#[cfg(feature = "inspection")]
#[test]
fn agent_contract_fields_differ_across_distinct_scenes() {
    let opaque =
        introspection_for_material(MaterialDesc::unlit(Color::WHITE), Vec3::new(0.0, 0.0, -2.0));
    let transparent = introspection_for_material(
        MaterialDesc::unlit(Color::TRANSPARENT).with_alpha_mode(AlphaMode::Blend),
        Vec3::new(0.0, 0.0, -2.0),
    );
    let hidden = introspection_for_material(
        MaterialDesc::unlit(Color::WHITE),
        Vec3::new(f32::NAN, 0.0, -2.0),
    );

    assert_ne!(
        opaque.nodes_summary.transparent, transparent.nodes_summary.transparent,
        "transparent summary must be computed from scene materials, not a constant"
    );
    assert!(
        hidden
            .reasons
            .iter()
            .any(|reason| reason.code == "nan_transform" && !reason.affected_handles.is_empty()),
        "node-targeted failures must carry affected stable handles"
    );
    assert!(
        hidden
            .fixes
            .iter()
            .any(|fix| fix.action == "set_transform" && fix.patch.is_some()),
        "repairable render failures must carry an apply-ready patch"
    );
}

#[test]
fn public_quality_knob_changes_renderer_state() {
    let low = Renderer::headless_with_options(
        16,
        16,
        RendererOptions::default().with_quality(Quality::Low),
    )
    .expect("low-quality renderer builds");
    let high = Renderer::headless_with_options(
        16,
        16,
        RendererOptions::default().with_quality(Quality::High),
    )
    .expect("high-quality renderer builds");

    assert_eq!(low.quality(), Quality::Low);
    assert_eq!(high.quality(), Quality::High);
    assert_eq!(low.anti_aliasing(), AntiAliasing::None);
    assert_eq!(high.anti_aliasing(), AntiAliasing::Fxaa);
}

#[test]
fn public_double_sided_material_knob_changes_pixels() {
    let single_sided = render_backface_cpu(false);
    let double_sided = render_backface_cpu(true);

    assert_eq!(
        nonblack_pixel_count(&single_sided),
        0,
        "single-sided back-facing mesh should be culled on the CPU path"
    );
    assert!(
        nonblack_pixel_count(&double_sided) > 0,
        "double-sided back-facing mesh should render visible pixels on the CPU path"
    );

    let single_sided_gpu = render_backface_gpu(false);
    let double_sided_gpu = render_backface_gpu(true);
    assert_eq!(
        nonblack_pixel_count(&single_sided_gpu),
        0,
        "single-sided back-facing mesh should be culled on the HeadlessGpu path"
    );
    assert!(
        nonblack_pixel_count(&double_sided_gpu) > 0,
        "double-sided back-facing mesh should render visible pixels on the HeadlessGpu path"
    );
}

#[cfg(feature = "inspection")]
fn introspection_for_material(
    material: MaterialDesc,
    translation: Vec3,
) -> scena::RenderIntrospectionReportV1 {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(translation))
        .add()
        .expect("mesh inserts");
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    let _ = renderer.render_active(&scene);
    let inspection = scene.inspect_with_assets(&assets).to_schema_report();
    let capture = renderer
        .capture_rgba8(&scene, Default::default())
        .expect("capture succeeds");
    renderer.introspect_capture(&capture, &inspection, RenderIntrospectionOptions::default())
}

fn render_backface_cpu(double_sided: bool) -> Vec<u8> {
    let (assets, mut scene) = backface_scene(double_sided);
    let mut renderer = Renderer::headless(32, 32).expect("CPU renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("CPU scene prepares");
    renderer.render_active(&scene).expect("CPU scene renders");
    renderer.frame_rgba8().to_vec()
}

fn render_backface_gpu(double_sided: bool) -> Vec<u8> {
    let (assets, mut scene) = backface_scene(double_sided);
    let mut renderer = Renderer::headless_gpu(32, 32).expect("HeadlessGpu renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("HeadlessGpu scene prepares");
    renderer
        .render_active(&scene)
        .expect("HeadlessGpu scene renders");
    renderer.frame_rgba8().to_vec()
}

fn backface_scene(double_sided: bool) -> (Assets, Scene) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                GeometryVertex {
                    position: Vec3::new(-0.6, -0.5, -2.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                GeometryVertex {
                    position: Vec3::new(0.0, 0.6, -2.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                GeometryVertex {
                    position: Vec3::new(0.6, -0.5, -2.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
            ],
            vec![0, 1, 2],
        )
        .expect("triangle geometry is valid"),
    );
    let material =
        assets.create_material(MaterialDesc::unlit(Color::WHITE).with_double_sided(double_sided));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    scene.mesh(geometry, material).add().expect("mesh inserts");
    (assets, scene)
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}
