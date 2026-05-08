#![cfg(not(target_arch = "wasm32"))]

use scena::{
    Backend, BuildError, CameraKey, Color, NotPreparedReason, PerspectiveCamera, PlatformSurface,
    Primitive, RenderError, Renderer, Scene, SurfaceEvent, Transform, Vec3, Vertex,
};

#[cfg(not(target_arch = "wasm32"))]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle,
};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct UnavailableWindow;

#[cfg(not(target_arch = "wasm32"))]
impl HasDisplayHandle for UnavailableWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Err(HandleError::Unavailable)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl HasWindowHandle for UnavailableWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Err(HandleError::Unavailable)
    }
}

fn scene_with_triangle() -> (Scene, CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts under root");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("triangle inserts under root");
    (scene, camera)
}

fn scene_with_primitive(primitive: Primitive) -> (Scene, CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts under root");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(scene.root(), vec![primitive], Transform::default())
        .expect("primitive inserts under root");
    (scene, camera)
}

fn white_triangle() -> Primitive {
    Primitive::triangle([
        Vertex {
            position: Vec3::new(-0.6, -0.5, 0.0),
            color: Color::from_linear_rgb(1.0, 1.0, 1.0),
        },
        Vertex {
            position: Vec3::new(0.6, -0.5, 0.0),
            color: Color::from_linear_rgb(1.0, 1.0, 1.0),
        },
        Vertex {
            position: Vec3::new(0.0, 0.6, 0.0),
            color: Color::from_linear_rgb(1.0, 1.0, 1.0),
        },
    ])
}

fn has_tonemapped_white_scene_pixel(frame: &[u8]) -> bool {
    frame.chunks_exact(4).any(|pixel| {
        let [r, g, b, a] = [pixel[0], pixel[1], pixel[2], pixel[3]];
        let max_delta = r.abs_diff(g).max(r.abs_diff(b)).max(g.abs_diff(b));
        (190..=220).contains(&r)
            && (190..=220).contains(&g)
            && (190..=220).contains(&b)
            && max_delta <= 3
            && a == 255
    })
}

#[test]
fn tonemapped_white_scene_pixel_check_tracks_visual_contract() {
    assert!(has_tonemapped_white_scene_pixel(&[206, 206, 206, 255]));
    assert!(!has_tonemapped_white_scene_pixel(&[255, 255, 255, 255]));
    assert!(!has_tonemapped_white_scene_pixel(&[206, 80, 40, 255]));
    assert!(!has_tonemapped_white_scene_pixel(&[206, 206, 206, 0]));
}

#[test]
fn surface_descriptors_and_attached_surfaces_are_explicit() {
    let descriptor = PlatformSurface::native_window(80, 60);
    assert!(!descriptor.is_attached());

    #[cfg(not(target_arch = "wasm32"))]
    {
        let attached = PlatformSurface::native_window_handle(UnavailableWindow, 80, 60);
        assert!(attached.is_attached());
        assert_eq!(attached.kind(), scena::SurfaceKind::NativeWindow);
        assert_eq!(attached.size().width, 80);
        assert_eq!(attached.size().height, 60);
    }
}

#[test]
fn descriptor_surface_async_initialization_is_structured() {
    let renderer = pollster::block_on(Renderer::from_surface_async(
        PlatformSurface::browser_webgpu_canvas(80, 60),
    ))
    .expect("descriptor surface renderer builds");

    assert_eq!(renderer.capabilities().backend, Backend::SurfaceDescriptor);
    assert!(!renderer.capabilities().gpu_device);
    assert!(!renderer.capabilities().surface_attached);
}

#[test]
fn public_vocabulary_can_be_constructed() {
    let scene = Scene::new();
    let renderer = Renderer::headless(32, 32).expect("headless renderer builds");

    assert!(scene.node(scene.root()).is_some());
    assert_eq!(renderer.capabilities().backend, Backend::Headless);
}

#[test]
fn invalid_headless_size_is_structured_build_error() {
    assert!(matches!(
        Renderer::headless(0, 32),
        Err(BuildError::InvalidTargetSize {
            width: 0,
            height: 32
        })
    ));
}

#[test]
fn headless_gpu_renderer_initialization_is_structured() {
    match Renderer::headless_gpu(32, 32) {
        Ok(renderer) => {
            assert_eq!(renderer.capabilities().backend, Backend::HeadlessGpu);
            assert!(renderer.capabilities().gpu_device);
            assert!(renderer.has_gpu_device());
            assert_eq!(renderer.stats().target_width, 32);
            assert_eq!(renderer.stats().target_height, 32);
        }
        Err(BuildError::NoAdapter { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(BuildError::RequestDevice { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(error) => panic!("unexpected headless GPU initialization result: {error:?}"),
    }
}

#[test]
fn headless_gpu_triangle_render_uses_gpu_submission_when_available() {
    match Renderer::headless_gpu(64, 64) {
        Ok(mut renderer) => {
            let (mut scene, camera) = scene_with_triangle();
            renderer.prepare(&mut scene).expect("prepare succeeds");

            assert_eq!(renderer.stats().gpu_submissions, 0);
            let outcome = renderer.render(&scene, camera).expect("render succeeds");

            assert_eq!(outcome.draw_calls, 1);
            assert_eq!(renderer.stats().gpu_submissions, 1);
            assert!(
                renderer
                    .frame_rgba8()
                    .chunks_exact(4)
                    .any(|pixel| pixel[0..3] != [0, 0, 0])
            );
        }
        Err(BuildError::NoAdapter { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(BuildError::RequestDevice { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(error) => panic!("unexpected headless GPU render setup result: {error:?}"),
    }
}

#[test]
fn headless_gpu_triangle_render_uses_scene_vertex_data_when_available() {
    match Renderer::headless_gpu(64, 64) {
        Ok(mut renderer) => {
            let (mut scene, camera) = scene_with_primitive(white_triangle());
            renderer.prepare(&mut scene).expect("prepare succeeds");
            renderer.render(&scene, camera).expect("render succeeds");

            assert!(
                has_tonemapped_white_scene_pixel(renderer.frame_rgba8()),
                "GPU frame should contain a neutral ACES/sRGB white scene triangle, not a \
                 hard-coded colored shader triangle"
            );
        }
        Err(BuildError::NoAdapter { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(BuildError::RequestDevice { backend }) => {
            assert_eq!(backend, Backend::HeadlessGpu);
        }
        Err(error) => panic!("unexpected headless GPU render setup result: {error:?}"),
    }
}

#[test]
fn native_surface_descriptor_reports_descriptor_capability() {
    let renderer = Renderer::from_surface(PlatformSurface::native_window(80, 60))
        .expect("native surface renderer builds");

    assert_eq!(renderer.capabilities().backend, Backend::SurfaceDescriptor);
    assert!(!renderer.capabilities().gpu_device);
    assert!(!renderer.capabilities().surface_attached);
    assert_eq!(renderer.stats().target_width, 80);
    assert_eq!(renderer.stats().target_height, 60);
}

#[test]
fn browser_canvas_descriptor_reports_descriptor_capability() {
    let renderer = Renderer::from_surface(PlatformSurface::browser_canvas(80, 60))
        .expect("browser canvas renderer builds");

    assert_eq!(renderer.capabilities().backend, Backend::SurfaceDescriptor);
    assert!(!renderer.capabilities().gpu_device);
    assert!(!renderer.capabilities().surface_attached);
    assert_eq!(renderer.stats().target_width, 80);
    assert_eq!(renderer.stats().target_height, 60);
}

#[test]
fn browser_webgl2_canvas_descriptor_reports_descriptor_capability() {
    let renderer = Renderer::from_surface(PlatformSurface::browser_webgl2_canvas(80, 60))
        .expect("browser webgl2 canvas renderer builds");

    assert_eq!(renderer.capabilities().backend, Backend::SurfaceDescriptor);
    assert!(!renderer.capabilities().gpu_device);
    assert!(!renderer.capabilities().surface_attached);
    assert_eq!(renderer.stats().target_width, 80);
    assert_eq!(renderer.stats().target_height, 60);
}

#[test]
fn invalid_surface_size_is_structured_build_error() {
    assert!(matches!(
        Renderer::from_surface(PlatformSurface::native_window(0, 60)),
        Err(BuildError::InvalidTargetSize {
            width: 0,
            height: 60
        })
    ));
}

#[test]
fn resize_surface_requires_prepare_before_next_render() {
    let (mut scene, camera) = scene_with_triangle();
    let mut renderer = Renderer::from_surface(PlatformSurface::native_window(80, 60))
        .expect("native surface renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("first render succeeds");

    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 160,
            height: 90,
        })
        .expect("resize is accepted");

    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::TargetChanged { .. }
        })
    ));

    renderer.prepare(&mut scene).expect("re-prepare succeeds");
    let outcome = renderer
        .render(&scene, camera)
        .expect("render after resize succeeds");

    assert_eq!(outcome.width, 160);
    assert_eq!(outcome.height, 90);
    assert_eq!(renderer.frame_rgba8().len(), 160 * 90 * 4);
}

#[test]
fn render_before_prepare_returns_not_prepared() {
    let (scene, camera) = scene_with_triangle();
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");

    assert_eq!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::NeverPrepared
        })
    );
}

#[test]
fn render_active_before_prepare_returns_not_prepared() {
    let (scene, _) = scene_with_triangle();
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");

    assert_eq!(
        renderer.render_active(&scene),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::NeverPrepared
        })
    );
}

#[test]
fn render_active_requires_active_camera() {
    let mut scene = Scene::new();
    scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts under root");
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");

    assert_eq!(
        renderer.render_active(&scene),
        Err(RenderError::NoActiveCamera)
    );
}

#[test]
fn structural_change_after_prepare_requires_prepare_again() {
    let (mut scene, camera) = scene_with_triangle();
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("second triangle inserts");

    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::SceneChanged { .. }
        })
    ));
}

#[test]
fn rendering_a_different_scene_after_prepare_is_not_prepared() {
    let (mut prepared_scene, _) = scene_with_triangle();
    let (different_scene, different_camera) = scene_with_triangle();
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");
    renderer
        .prepare(&mut prepared_scene)
        .expect("prepare succeeds");

    assert_eq!(
        renderer.render(&different_scene, different_camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::DifferentScene
        })
    );
}

#[test]
fn headless_triangle_render_is_deterministic_and_nonblack() {
    let (mut scene, camera) = scene_with_triangle();
    let mut renderer = Renderer::headless(64, 64).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("prepare succeeds");

    let first = renderer.render(&scene, camera).expect("render succeeds");
    let first_frame = renderer.frame_rgba8().to_vec();
    let second = renderer
        .render_active(&scene)
        .expect("active render succeeds");
    let second_frame = renderer.frame_rgba8().to_vec();

    assert_eq!(first, second);
    assert_eq!(first_frame, second_frame);
    assert!(
        first_frame
            .chunks_exact(4)
            .any(|pixel| pixel[0..3] != [0, 0, 0])
    );
    assert_eq!(renderer.stats().frames_rendered, 2);
}

#[test]
fn surface_triangle_render_is_deterministic_and_nonblack() {
    for surface in [
        PlatformSurface::native_window(64, 64),
        PlatformSurface::browser_webgpu_canvas(64, 64),
        PlatformSurface::browser_webgl2_canvas(64, 64),
    ] {
        let (mut scene, camera) = scene_with_triangle();
        let mut renderer = Renderer::from_surface(surface).expect("surface renderer builds");
        renderer.prepare(&mut scene).expect("prepare succeeds");

        let first = renderer.render(&scene, camera).expect("render succeeds");
        let first_frame = renderer.frame_rgba8().to_vec();
        let second = renderer
            .render_active(&scene)
            .expect("active render succeeds");
        let second_frame = renderer.frame_rgba8().to_vec();

        assert_eq!(first, second);
        assert_eq!(first_frame, second_frame);
        assert!(
            first_frame
                .chunks_exact(4)
                .any(|pixel| pixel[0..3] != [0, 0, 0])
        );
    }
}
