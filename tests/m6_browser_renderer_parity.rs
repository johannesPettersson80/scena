#![cfg(target_arch = "wasm32")]

use scena::{
    Assets, Backend, BuildError, Color, PerspectiveCamera, PlatformSurface, PrepareError,
    Primitive, RenderError, Renderer, RetainPolicy, Scene, SurfaceEvent, Transform, Vec3, Vertex,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlCanvasElement;

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(feature = "browser-probe")]
#[wasm_bindgen_test(async)]
async fn m6_cpu_webgl2_parity_requires_cpu_and_gpu_renderer_owned_frames() {
    let canvas = browser_canvas(64, 64);
    let report = scena::browser_probe::m6_render_webgl2_probe(canvas)
        .await
        .expect("CPU/WebGL2 parity probe runs");
    let report: serde_json::Value =
        serde_json::from_str(&report).expect("CPU/WebGL2 parity report is JSON");
    let parity = report
        .get("parity")
        .expect("browser probe includes CPU/WebGL2 parity evidence");

    assert_eq!(
        parity.get("schema").and_then(serde_json::Value::as_str),
        Some("scena.m6.cpu_webgl2_parity.v1")
    );
    assert_eq!(
        parity.get("status").and_then(serde_json::Value::as_str),
        Some("passed"),
        "CPU/WebGL2 parity report failed: codes={:?}, metrics={:#}, full_parity={:#}",
        parity.get("failure_codes"),
        parity.get("metrics").unwrap_or(&serde_json::Value::Null),
        parity,
    );
    let cpu_frame = parity.get("cpu_frame").expect("parity has a CPU frame");
    let gpu_frame = parity.get("gpu_frame").expect("parity has a GPU frame");
    assert_eq!(
        cpu_frame.get("source").and_then(serde_json::Value::as_str),
        Some("renderer-owned-cpu-frame")
    );
    assert_eq!(
        gpu_frame.get("source").and_then(serde_json::Value::as_str),
        Some("renderer-owned-gpu-copy")
    );
    assert_eq!(cpu_frame.get("width"), gpu_frame.get("width"));
    assert_eq!(cpu_frame.get("height"), gpu_frame.get("height"));
    assert!(
        cpu_frame
            .get("rgba8_base64")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|rgba| !rgba.is_empty())
            && gpu_frame
                .get("rgba8_base64")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|rgba| !rgba.is_empty()),
        "parity must carry both renderer-owned RGBA8 frame inputs"
    );

    let normalization = parity
        .get("normalization")
        .expect("parity declares frame normalization");
    assert_eq!(
        normalization
            .get("row_origin")
            .and_then(serde_json::Value::as_str),
        Some("top-left")
    );
    assert_eq!(
        normalization
            .get("transfer")
            .and_then(serde_json::Value::as_str),
        Some("srgb8")
    );
    assert_eq!(
        normalization
            .get("alpha")
            .and_then(serde_json::Value::as_str),
        Some("straight-opaque")
    );
    assert_eq!(
        normalization
            .get("dimensions")
            .and_then(serde_json::Value::as_str),
        Some("exact")
    );

    let metrics = parity.get("metrics").expect("parity records metrics");
    assert!(
        metrics
            .get("rmse")
            .and_then(serde_json::Value::as_f64)
            .is_some_and(|value| value <= 0.08)
    );
    assert!(
        metrics
            .get("ssim")
            .and_then(serde_json::Value::as_f64)
            .is_some_and(|value| value >= 0.93)
    );
    assert!(
        metrics
            .get("p95_channel_delta")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|value| value <= 24)
    );
    assert!(
        metrics
            .get("mean_channel_delta")
            .and_then(serde_json::Value::as_f64)
            .is_some_and(|value| value <= 6.0)
    );
    assert!(
        metrics
            .get("foreground_iou")
            .and_then(serde_json::Value::as_f64)
            .is_some_and(|value| value >= 0.90)
    );

    let mutation = parity
        .get("known_bad_mutation")
        .expect("parity records a deliberately perturbed GPU fixture output");
    assert_eq!(
        mutation.get("kind").and_then(serde_json::Value::as_str),
        Some("gpu-center-channel-perturbation")
    );
    assert_eq!(
        mutation
            .get("rejected")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    assert!(
        mutation
            .get("failure_codes")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|codes| !codes.is_empty()),
        "the perturbed GPU frame must fail the same parity evaluator"
    );
}

#[wasm_bindgen_test]
fn wasm_recipe_validation_rejects_non_ascii_hex_without_runtime_abort() {
    let report = scena::validate_scene_recipe_json(
        r#"{"schema":"scena.scene_recipe.v1","colors":{"bad":"€abc"}}"#,
    );
    assert!(!report.ok);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_color" && diagnostic.path == "$.colors.bad"
    }));
    let json = serde_json::to_string(&report).expect("WASM validation report serializes");
    assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
}

#[wasm_bindgen_test(async)]
async fn m6_webgl2_attached_canvas_is_not_hard_disabled() {
    let canvas = browser_canvas(32, 32);
    let surface = PlatformSurface::browser_webgl2_canvas_element(canvas.clone(), 32, 32);
    let mut renderer = match Renderer::from_surface_async(surface).await {
        Ok(renderer) => renderer,
        Err(BuildError::CreateSurface { backend })
        | Err(BuildError::NoAdapter { backend })
        | Err(BuildError::RequestDevice { backend })
        | Err(BuildError::SurfaceUnsupported { backend }) => {
            assert_eq!(backend, Backend::WebGl2);
            panic!("attached WebGL2 browser canvas could not build on the proof lane");
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

    renderer.prepare(&mut scene).expect("WebGL2 scene prepares");
    let outcome = match renderer.render(&scene, camera) {
        Ok(outcome) => outcome,
        Err(error) => panic!("WebGL2 scene renders: {error:?}"),
    };

    assert_eq!(outcome.draw_calls, 1);
    assert_eq!(renderer.stats().gpu_submissions, 1);
    assert!(
        nonblack_pixel_count(
            &browser_canvas_rgba8(&canvas).expect("WebGL2 canvas readback exists")
        ) > 0,
        "WebGL2 proof must include rendered pixels, not only draw counters"
    );
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
            panic!("attached WebGL2 browser canvas could not build on the proof lane");
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
        .expect("WebGL2 lifecycle scene prepares");
    renderer
        .render(&scene, camera)
        .expect("WebGL2 lifecycle scene renders");
    assert!(
        nonblack_pixel_count(
            &browser_canvas_rgba8(&canvas).expect("WebGL2 lifecycle canvas readback exists"),
        ) > 0,
        "WebGL2 lifecycle proof must start from visible rendered pixels"
    );

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
        .handle_surface_event(SurfaceEvent::Lost)
        .expect("surface-loss event is accepted");
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::SurfaceLost { recoverable: true })
        ),
        "surface loss must surface as a structured render error",
    );

    renderer
        .handle_surface_event(SurfaceEvent::DeviceLost { recoverable: true })
        .expect("device-loss event is accepted");
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::GpuDeviceLost { recoverable: true })
        ),
        "terminal device loss must take precedence over surface loss",
    );
    assert!(matches!(
        renderer.recover_context(&assets, &mut scene),
        Err(PrepareError::GpuDeviceRebuildRequired {
            recoverable: true,
            ..
        })
    ));
    assert!(matches!(
        renderer.prepare_with_assets(&mut scene, &assets),
        Err(PrepareError::GpuDeviceRebuildRequired {
            recoverable: true,
            ..
        })
    ));
    assert!(
        matches!(
            renderer.render(&scene, camera),
            Err(RenderError::GpuDeviceLost { recoverable: true })
        ),
        "failed recovery must leave device loss latched",
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

fn browser_canvas_rgba8(canvas: &HtmlCanvasElement) -> Option<Vec<u8>> {
    let width = canvas.width();
    let height = canvas.height();
    let len = width.checked_mul(height)?.checked_mul(4)?;
    if len == 0 {
        return None;
    }

    let get_context = js_sys::Reflect::get(
        canvas.as_ref(),
        &wasm_bindgen::JsValue::from_str("getContext"),
    )
    .expect("canvas exposes getContext")
    .dyn_into::<js_sys::Function>()
    .expect("getContext is callable");
    let options = js_sys::Object::new();
    js_sys::Reflect::set(
        &options,
        &wasm_bindgen::JsValue::from_str("preserveDrawingBuffer"),
        &wasm_bindgen::JsValue::TRUE,
    )
    .expect("preserveDrawingBuffer option can be set");
    let context = get_context
        .call2(
            canvas.as_ref(),
            &wasm_bindgen::JsValue::from_str("webgl2"),
            options.as_ref(),
        )
        .expect("webgl2 context lookup succeeds");
    if context.is_null() || context.is_undefined() {
        return None;
    }

    let rgba = js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("RGBA"))
        .expect("WebGL2 RGBA enum exists");
    let unsigned_byte =
        js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("UNSIGNED_BYTE"))
            .expect("WebGL2 UNSIGNED_BYTE enum exists");
    let read_pixels =
        js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("readPixels"))
            .expect("WebGL2 readPixels exists")
            .dyn_into::<js_sys::Function>()
            .expect("readPixels is callable");
    let bytes = js_sys::Uint8Array::new_with_length(len);
    let args = js_sys::Array::new();
    args.push(&wasm_bindgen::JsValue::from_f64(0.0));
    args.push(&wasm_bindgen::JsValue::from_f64(0.0));
    args.push(&wasm_bindgen::JsValue::from_f64(f64::from(width)));
    args.push(&wasm_bindgen::JsValue::from_f64(f64::from(height)));
    args.push(&rgba);
    args.push(&unsigned_byte);
    args.push(bytes.as_ref());
    read_pixels
        .apply(&context, &args)
        .expect("WebGL2 readPixels succeeds");

    let mut rgba8 = vec![0; len as usize];
    bytes.copy_to(rgba8.as_mut_slice());
    Some(rgba8)
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
                    position: Vec3::new(-0.6, -0.5, -2.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.6, -0.5, -2.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(0.0, 0.6, -2.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("triangle inserts under root");
    (scene, camera)
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}
