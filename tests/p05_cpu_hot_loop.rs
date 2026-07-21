use scena::{Primitive, Renderer, Scene, Transform};

#[test]
fn cpu_output_encoding_is_bounded_by_pixels_not_overdraw() {
    const WIDTH: u32 = 48;
    const HEIGHT: u32 = 48;
    let (mut baseline_scene, baseline_camera) = overlapping_scene(1);
    let (mut overdraw_scene, overdraw_camera) = overlapping_scene(64);
    let mut baseline = Renderer::headless(WIDTH, HEIGHT).expect("baseline CPU renderer builds");
    let mut overdraw = Renderer::headless(WIDTH, HEIGHT).expect("overdraw CPU renderer builds");

    baseline
        .prepare(&mut baseline_scene)
        .expect("baseline scene prepares");
    baseline
        .render(&baseline_scene, baseline_camera)
        .expect("baseline scene renders");
    overdraw
        .prepare(&mut overdraw_scene)
        .expect("overdraw scene prepares");
    overdraw
        .render(&overdraw_scene, overdraw_camera)
        .expect("overdraw scene renders");

    assert_eq!(baseline.frame_rgba8(), overdraw.frame_rgba8());
    let baseline_encoded = baseline
        .last_render_work_metrics()
        .cpu_output_pixels_encoded;
    let overdraw_encoded = overdraw
        .last_render_work_metrics()
        .cpu_output_pixels_encoded;
    assert_eq!(baseline_encoded, overdraw_encoded);
    assert!(overdraw_encoded <= u64::from(WIDTH) * u64::from(HEIGHT));
}

fn overlapping_scene(count: usize) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    for _ in 0..count {
        scene
            .add_renderable(
                scene.root(),
                vec![Primitive::unlit_triangle()],
                Transform::IDENTITY,
            )
            .expect("overdraw triangle inserts");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    (scene, camera)
}
