use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::scene::{ClippingPlane, ClippingPlaneSet, PerspectiveCamera, Scene, Transform, Vec3};

use super::Renderer;

impl Renderer {
    fn set_depth_prepass_enabled_for_test(&mut self, enabled: bool) {
        self.depth_prepass_enabled_for_test = enabled;
    }
}

#[test]
fn depth_prepass_prevents_later_far_triangle_from_overdrawing_near_triangle() {
    let Ok(mut with_prepass) = Renderer::headless_gpu(64, 64) else {
        return;
    };
    let Ok(mut without_prepass) = Renderer::headless_gpu(64, 64) else {
        return;
    };
    without_prepass.set_depth_prepass_enabled_for_test(false);

    let mut prepass_scene = overlapping_triangles_scene();
    let prepass_camera = prepass_scene.active_camera().expect("camera is active");
    with_prepass
        .prepare(&mut prepass_scene)
        .expect("prepass scene prepares");
    with_prepass
        .render(&prepass_scene, prepass_camera)
        .expect("prepass scene renders");
    let prepass_stats = with_prepass.stats();
    let prepass_center = pixel_at(with_prepass.frame_rgba8(), 64, 32, 32);

    let mut no_prepass_scene = overlapping_triangles_scene();
    let no_prepass_camera = no_prepass_scene.active_camera().expect("camera is active");
    without_prepass
        .prepare(&mut no_prepass_scene)
        .expect("no-prepass scene prepares");
    without_prepass
        .render(&no_prepass_scene, no_prepass_camera)
        .expect("no-prepass scene renders");
    let no_prepass_stats = without_prepass.stats();
    let no_prepass_center = pixel_at(without_prepass.frame_rgba8(), 64, 32, 32);

    assert_eq!(prepass_stats.depth_prepass_passes, 1);
    assert_eq!(prepass_stats.depth_prepass_draws, 2);
    assert_eq!(no_prepass_stats.depth_prepass_passes, 0);
    assert_eq!(no_prepass_stats.depth_prepass_draws, 0);
    assert!(
        prepass_center[1] > prepass_center[0],
        "with depth prepass, near green triangle must win at the overlap center: {prepass_center:?}"
    );
    assert!(
        no_prepass_center[0] > no_prepass_center[1],
        "without depth prepass, the later far red triangle demonstrates the overdraw artifact this pass prevents: {no_prepass_center:?}"
    );
}

fn overlapping_triangles_scene() -> Scene {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");
    let keep_all_plane = scene.add_clipping_plane(ClippingPlane::new(Vec3::Z, 10.0));
    scene
        .set_clipping_planes(ClippingPlaneSet::new().with_plane(keep_all_plane))
        .expect("harmless clipping plane activates");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.5, -0.5, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.5, -0.5, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.5, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
            ])],
            Transform::default(),
        )
        .expect("near triangle inserts first");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.5, -0.5, -0.5),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.5, -0.5, -0.5),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.5, -0.5),
                    color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                },
            ])],
            Transform::default(),
        )
        .expect("far triangle inserts second");
    scene
}

fn pixel_at(frame: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let offset = (y * width + x) * 4;
    [
        frame[offset],
        frame[offset + 1],
        frame[offset + 2],
        frame[offset + 3],
    ]
}
