#![cfg(not(target_arch = "wasm32"))]

use scena::{Primitive, Renderer, Scene, Transform};

#[test]
fn routine_gpu_prepare_polls_without_blocking() {
    let mut renderer = Renderer::headless_gpu(16, 16)
        .expect("P03 focused proof requires the remote builder GPU adapter");
    let mut scene = Scene::new();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::IDENTITY,
        )
        .expect("P03 triangle inserts");
    scene.add_default_camera().expect("P03 camera inserts");

    let metrics = renderer
        .prepare_profiled(&mut scene)
        .expect("profiled GPU prepare succeeds");

    assert_eq!(metrics.gpu_nonblocking_polls, 1);
    assert_eq!(metrics.gpu_blocking_polls, 0);
}
