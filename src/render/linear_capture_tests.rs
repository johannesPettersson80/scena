use crate::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene, Transform, Vec3,
};

#[test]
fn scene_linear_capture_preserves_hdr_before_tonemapping() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.4, 1.4, 0.1));
    let material = assets.create_material(
        MaterialDesc::unlit(Color::BLACK)
            .with_emissive(Color::from_linear_rgb(1.0, 0.25, 0.1))
            .with_emissive_strength(8.0),
    );
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .mesh(geometry, material)
        .add()
        .expect("emissive mesh inserts");

    let Ok(mut renderer) = Renderer::headless_gpu(64, 64) else {
        return;
    };
    renderer.set_scene_linear_capture_enabled(true);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("linear capture scene prepares");
    renderer
        .render_active(&scene)
        .expect("linear capture scene renders");

    let capture = renderer
        .scene_linear_capture()
        .expect("scene-linear radiance reads back");
    assert_eq!((capture.width(), capture.height()), (64, 64));
    let center = capture.rgba32f()[(32 * 64 + 32) as usize];
    assert!(
        center[0] > 2.0 && center[0] > center[1] * 2.0,
        "pre-tonemap capture must preserve HDR emissive radiance, got {center:?}",
    );
    assert_eq!(renderer.frame_rgba8().len(), 64 * 64 * 4);
}
