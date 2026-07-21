use scena::{Primitive, Renderer, Scene, Transform};

#[test]
fn cpu_primitive_flags_are_scanned_once_not_once_per_row_band() {
    const PRIMITIVES: usize = 256;
    let mut scene = Scene::new();
    for _ in 0..PRIMITIVES {
        scene
            .add_renderable(
                scene.root(),
                vec![Primitive::unlit_triangle()],
                Transform::IDENTITY,
            )
            .expect("triangle inserts");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(640, 480).expect("CPU renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let metrics = renderer.last_render_work_metrics();
    assert!(metrics.cpu_parallel_workers > 1);
    assert_eq!(metrics.cpu_primitive_flag_scan_items, PRIMITIVES as u64);
}
