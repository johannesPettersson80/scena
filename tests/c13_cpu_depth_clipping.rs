#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Color, CursorPosition, DepthRange, HitTarget, OrderIndependentTransparencyConfig,
    PerspectiveCamera, Primitive, Renderer, Scene, Transform, Vec3, Vertex, Viewport,
};

fn render_triangle(vertices: [Vertex; 3]) -> (Vec<u8>, u64) {
    render_triangle_with_oit(vertices, false)
}

fn render_triangle_with_oit(vertices: [Vertex; 3], oit: bool) -> (Vec<u8>, u64) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_depth_range(DepthRange::new(0.5, 5.0)),
            Transform::default(),
        )
        .expect("depth-clipping camera inserts");
    scene
        .set_active_camera(camera)
        .expect("depth-clipping camera becomes active");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle(vertices)],
            Transform::default(),
        )
        .expect("depth-clipping triangle inserts");
    let mut renderer = Renderer::headless(96, 96).expect("CPU renderer builds");
    renderer.set_background_color(Color::BLACK);
    if oit {
        renderer.set_order_independent_transparency(Some(
            OrderIndependentTransparencyConfig::weighted_blended(),
        ));
    }
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render_active(&scene).expect("scene renders");
    (
        renderer.frame_rgba8().to_vec(),
        renderer.stats().order_independent_transparency_passes,
    )
}

fn visible_pixel_count(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8)
        .count()
}

#[test]
fn cpu_triangle_crossing_near_plane_is_clipped_instead_of_dropped() {
    let white = Color::WHITE;
    let (frame, _) = render_triangle([
        Vertex {
            position: Vec3::new(-0.4, -0.4, -1.0),
            color: white,
        },
        Vertex {
            position: Vec3::new(0.4, -0.4, -1.0),
            color: white,
        },
        Vertex {
            position: Vec3::new(0.0, 0.4, -0.25),
            color: white,
        },
    ]);

    assert!(
        visible_pixel_count(&frame) > 200,
        "the in-frustum polygon must survive clipping when one source vertex is before near"
    );
}

#[test]
fn cpu_triangle_crossing_far_plane_is_clipped_instead_of_dropped() {
    let (frame, _) = render_triangle([
        vertex(-0.45, -0.4, -4.0, Color::RED),
        vertex(0.45, -0.4, -4.0, Color::GREEN),
        vertex(0.0, 0.45, -6.0, Color::BLUE),
    ]);
    assert!(
        visible_pixel_count(&frame) > 20,
        "the in-frustum polygon must survive clipping when one source vertex is beyond far"
    );
}

#[test]
fn cpu_triangle_spanning_near_and_far_planes_keeps_the_depth_slab() {
    let (frame, _) = render_triangle([
        vertex(-0.45, -0.35, -0.25, Color::RED),
        vertex(0.45, -0.35, -7.0, Color::GREEN),
        vertex(0.0, 0.5, -2.0, Color::BLUE),
    ]);
    assert!(
        visible_pixel_count(&frame) > 100,
        "a triangle crossing both depth planes must retain its in-slab polygon"
    );
}

#[test]
fn cpu_depth_clipping_accepts_vertices_exactly_on_each_plane() {
    for (label, depth) in [("near", -0.5), ("far", -5.0)] {
        let (frame, _) = render_triangle([
            vertex(-0.35, -0.3, depth, Color::WHITE),
            vertex(0.35, -0.3, depth, Color::WHITE),
            vertex(0.0, 0.35, -1.0, Color::WHITE),
        ]);
        assert!(
            visible_pixel_count(&frame) > 20,
            "vertices exactly on the {label} plane are inside the depth slab"
        );
    }
}

#[test]
fn cpu_depth_clipping_rejects_empty_and_degenerate_results_without_artifacts() {
    for vertices in [
        [
            vertex(-0.4, -0.4, -0.2, Color::WHITE),
            vertex(0.4, -0.4, -0.2, Color::WHITE),
            vertex(0.0, 0.4, -0.2, Color::WHITE),
        ],
        [
            vertex(-0.4, 0.0, -1.0, Color::WHITE),
            vertex(0.0, 0.0, -1.0, Color::WHITE),
            vertex(0.4, 0.0, -1.0, Color::WHITE),
        ],
    ] {
        let (frame, _) = render_triangle(vertices);
        assert_eq!(visible_pixel_count(&frame), 0);
        assert!(frame.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }
}

#[test]
fn cpu_large_triangle_spanning_the_camera_clips_to_finite_screen_bounds() {
    let (frame, _) = render_triangle([
        vertex(-100_000.0, -100_000.0, -1.0, Color::WHITE),
        vertex(100_000.0, -100_000.0, -1.0, Color::WHITE),
        vertex(0.0, 1_000_000.0, 1.0, Color::WHITE),
    ]);
    let visible = visible_pixel_count(&frame);
    assert!(
        visible > 4_000,
        "large clipped polygon must cover the viewport; covered={visible}"
    );
    assert!(visible <= 96 * 96);
}

#[test]
fn cpu_oit_triangle_crossing_near_plane_is_clipped_and_resolved() {
    let translucent = Color::from_linear_rgba(0.9, 0.2, 0.1, 0.45);
    let (frame, passes) = render_triangle_with_oit(
        [
            vertex(-0.4, -0.4, -1.0, translucent),
            vertex(0.4, -0.4, -1.0, translucent),
            vertex(0.0, 0.4, -0.25, translucent),
        ],
        true,
    );
    assert_eq!(passes, 1, "fixture must exercise the OIT path");
    assert!(visible_pixel_count(&frame) > 200);
}

#[test]
fn near_crossing_triangle_keeps_scene_picking_identity() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_depth_range(DepthRange::new(0.5, 5.0)),
            Transform::default(),
        )
        .expect("depth-clipping camera inserts");
    let triangle = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                vertex(-0.4, -0.4, -1.0, Color::RED),
                vertex(0.4, -0.4, -1.0, Color::GREEN),
                vertex(0.0, 0.4, -0.25, Color::BLUE),
            ])],
            Transform::default(),
        )
        .expect("near-crossing triangle inserts");

    let hit = scene
        .pick(
            camera,
            CursorPosition::physical(48.0, 48.0),
            Viewport::new(96, 96, 1.0).expect("viewport is valid"),
        )
        .expect("near-crossing pick succeeds")
        .expect("center ray hits the visible clipped polygon");

    assert!(matches!(hit.target(), HitTarget::Node(node) if node == triangle));
}

fn vertex(x: f32, y: f32, z: f32, color: Color) -> Vertex {
    Vertex {
        position: Vec3::new(x, y, z),
        color,
    }
}
