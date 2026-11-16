//! The generated gate valve demo model loads with its stated finishes and
//! path traces with them.

use scena::{Assets, PathTraceOptions, Renderer, Scene};

const VALVE: &str = "demo/samples/gate-valve/gate_valve_dn100.glb";

#[test]
fn the_gate_valve_states_a_finish_for_every_part_and_traces_with_them() {
    let assets = Assets::new();
    let model = pollster::block_on(assets.load_scene(VALVE)).expect("the valve loads");
    let mut scene = Scene::new();
    let import = scene.instantiate(&model).expect("the valve instantiates");
    let bounds = import
        .bounds_world(&scene)
        .expect("the valve has visible geometry");

    // 229 mm face to face plus the raised faces, standing on its flange
    // rims, with the handwheel the widest thing across.
    let size = bounds.max - bounds.min;
    assert!((size.x - 0.271).abs() < 2.0e-3, "{size}");
    assert!((size.y - 0.7665).abs() < 2.0e-3, "{size}");
    assert!((size.z - 0.29).abs() < 2.0e-3, "{size}");
    assert!((bounds.min.y + 0.1145).abs() < 1.0e-4, "{}", bounds.min.y);

    scene.add_studio_lighting().expect("lights insert");
    let camera = scene
        .add_default_camera()
        .expect("a camera frames the valve");
    let renderer = Renderer::headless(48, 64).expect("headless renderer builds");
    let options = PathTraceOptions::preview().with_samples_per_pixel(2);
    let traced = renderer
        .path_trace(&scene, &assets, camera, &options)
        .expect("the valve path traces");
    let stats = traced.stats();
    assert_eq!(stats.triangles, 51_420);
    assert_eq!(stats.skipped_degenerate_triangles, 0);
    // Thirteen parts, and every one states a finish, which brings its edge
    // radius with it.
    assert_eq!((stats.finished_surfaces, stats.rounded_surfaces), (13, 13));
    let plain = renderer
        .path_trace(&scene, &assets, camera, &options.with_surface_detail(false))
        .expect("the plain valve path traces");
    assert_eq!(plain.stats().finished_surfaces, 0);
    assert_ne!(plain.rgba8(), traced.rgba8());
}
