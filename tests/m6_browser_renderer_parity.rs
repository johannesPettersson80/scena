#![cfg(target_arch = "wasm32")]

use scena::{
    Assets, Backend, BuildError, Color, PerspectiveCamera, PlatformSurface, PrepareError,
    Primitive, RenderError, Renderer, RetainPolicy, Scene, SurfaceEvent, Transform, Vec3, Vertex,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlCanvasElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn m6_webgl2_attached_canvas_is_not_hard_disabled() {
    let canvas = browser_canvas(32, 32);
    let surface = PlatformSurface::browser_webgl2_canvas_element(canvas, 32, 32);
    let mut renderer = match Renderer::from_surface_async(surface).await {
        Ok(renderer) => renderer,
        Err(BuildError::CreateSurface { backend })
        | Err(BuildError::NoAdapter { backend })
        | Err(BuildError::RequestDevice { backend })
        | Err(BuildError::SurfaceUnsupported { backend }) => {
            assert_eq!(backend, Backend::WebGl2);
            return;
        }
        Err(BuildError::UnsupportedBackend { backend }) => {
            panic!("attached WebGL2 browser canvas is still hard-disabled for {backend:?}");
        }
        Err(error) => panic!("unexpected attached WebGL2 build error: {error:?}"),
    };
    let (mut scene, camera) = scene_with_white_triangle();

    assert_eq!(renderer.capabilities().backend, Backend::WebGl2);
    assert!(renderer.capabilities().gpu_device);
    assert!(renderer.capabilities().surface_attached);

    renderer
        .prepare(&mut scene)
        .expect("attached WebGL2 renderer prepares scene resources");
    let outcome = renderer
        .render(&scene, camera)
        .expect("attached WebGL2 renderer submits through Renderer::render");

    assert_eq!(outcome.draw_calls, 1);
    assert_eq!(renderer.stats().gpu_submissions, 1);
    assert!(
        !matches!(
            Renderer::from_surface_async(PlatformSurface::browser_webgl2_canvas(32, 32)).await,
            Err(BuildError::UnsupportedBackend {
                backend: Backend::WebGl2
            })
        ),
        "descriptor-only browser surfaces must not be confused with unsupported attached canvases",
    );
}

#[wasm_bindgen_test(async)]
async fn m6_webgl2_surface_lifecycle_requires_prepare_and_retained_assets() {
    let canvas = browser_canvas(32, 32);
    let surface = PlatformSurface::browser_webgl2_canvas_element(canvas.clone(), 32, 32);
    let mut renderer = match Renderer::from_surface_async(surface).await {
        Ok(renderer) => renderer,
        Err(BuildError::CreateSurface { backend })
        | Err(BuildError::NoAdapter { backend })
        | Err(BuildError::RequestDevice { backend })
        | Err(BuildError::SurfaceUnsupported { backend }) => {
            assert_eq!(backend, Backend::WebGl2);
            return;
        }
        Err(BuildError::UnsupportedBackend { backend }) => {
            panic!("attached WebGL2 browser canvas is still hard-disabled for {backend:?}");
        }
        Err(error) => panic!("unexpected attached WebGL2 build error: {error:?}"),
    };
    let assets = Assets::new();
    let (mut scene, camera) = scene_with_white_triangle();

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial browser prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial browser render succeeds");

    canvas.set_width(48);
    canvas.set_height(48);
    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 48,
            height: 48,
        })
        .expect("resize event is accepted");
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::NotPrepared {
                reason: scena::NotPreparedReason::TargetChanged { .. }
            })
        ),
        "resize must invalidate prepare instead of hiding GPU work inside render",
    );
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("explicit prepare after resize succeeds");
    renderer
        .render(&scene, camera)
        .expect("render after resize prepare succeeds");

    renderer
        .handle_surface_event(SurfaceEvent::ContextLost { recoverable: true })
        .expect("context-loss event is accepted");
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::ContextLost { recoverable: true })
        ),
        "context loss must surface as a structured render error",
    );
    let mut unretained_assets = Assets::new();
    unretained_assets.set_retain_policy(RetainPolicy::Never);
    assert!(
        matches!(
            renderer.recover_context(&unretained_assets, &mut scene),
            Err(PrepareError::BackendCapabilityMismatch {
                feature: "context recovery",
                ..
            })
        ),
        "browser context recovery must reject Assets with RetainPolicy::Never",
    );
    renderer
        .handle_surface_event(SurfaceEvent::ContextRestored)
        .expect("context-restored event is accepted");
    renderer
        .recover_context(&assets, &mut scene)
        .expect("retained assets allow context recovery");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("explicit prepare after context recovery succeeds");
    renderer
        .render(&scene, camera)
        .expect("render after context recovery succeeds");

    renderer
        .handle_surface_event(SurfaceEvent::DeviceLost { recoverable: true })
        .expect("device-loss event is accepted");
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::GpuDeviceLost { recoverable: true })
        ),
        "device loss must surface as a structured render error",
    );
    renderer
        .recover_context(&assets, &mut scene)
        .expect("retained assets allow device recovery");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("explicit prepare after device recovery succeeds");
    renderer
        .render(&scene, camera)
        .expect("render after device recovery succeeds");

    renderer
        .handle_surface_event(SurfaceEvent::Lost)
        .expect("surface-loss event is accepted");
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::SurfaceLost { recoverable: true })
        ),
        "surface loss must surface as a structured render error",
    );
}

fn browser_canvas(width: u32, height: u32) -> HtmlCanvasElement {
    let window = web_sys::window().expect("browser window exists");
    let document = window.document().expect("browser document exists");
    let canvas = document
        .create_element("canvas")
        .expect("canvas element can be created")
        .dyn_into::<HtmlCanvasElement>()
        .expect("element is a canvas");
    canvas.set_width(width);
    canvas.set_height(height);
    document
        .body()
        .expect("document has body")
        .append_child(&canvas)
        .expect("canvas appends to document");
    canvas
}

fn scene_with_white_triangle() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts under root");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.6, -0.5, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.6, -0.5, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.0, 0.6, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("triangle inserts under root");
    (scene, camera)
}
