//! Drag-and-drop demo page WASM exports.
//!
//! Gated by the `demo-page` feature. Four `wasm_bindgen` entry points:
//! [`load_gltf_from_bytes`], [`attach_to_canvas`], [`forward_pointer_event`],
//! and [`tick`]. The opaque [`DemoApp`] handle is held by the JS side and
//! threaded back into each call.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{
    Assets, CameraKey, Color, OrbitControls, PerspectiveCamera, PlatformSurface, PointerEvent,
    Renderer, Scene, SurfaceEvent, Transform, Vec3,
};

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn log_timing(label: &str, start_ms: f64) -> f64 {
    let elapsed_ms = now_ms() - start_ms;
    web_sys::console::log_1(&format!("[scena-demo] {label}: {elapsed_ms:.1}ms").into());
    now_ms()
}

fn demo_environment_enabled() -> bool {
    let search = web_sys::window()
        .and_then(|window| js_sys::Reflect::get(&window, &JsValue::from_str("location")).ok())
        .and_then(|location| js_sys::Reflect::get(&location, &JsValue::from_str("search")).ok())
        .and_then(|search| search.as_string());
    search.is_none_or(|search| !search.contains("env=0") && !search.contains("environment=0"))
}

#[wasm_bindgen]
pub struct DemoApp {
    assets: Assets,
    scene: Scene,
    camera: CameraKey,
    controls: OrbitControls,
    renderer: Option<Renderer>,
}

#[wasm_bindgen]
pub async fn load_gltf_from_bytes(
    bytes: Box<[u8]>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    let total_start = now_ms();
    let mut step_start = total_start;
    let array = js_sys::Uint8Array::from(bytes.as_ref());
    let parts = js_sys::Array::of1(&array.into());
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts.into())
        .map_err(|err| JsValue::from_str(&format!("Blob construction failed: {err:?}")))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|err| JsValue::from_str(&format!("createObjectURL failed: {err:?}")))?;
    step_start = log_timing("blob URL creation", step_start);

    let assets = Assets::new();
    let scene_asset = assets
        .load_scene(url.as_str())
        .await
        .map_err(|err| JsValue::from_str(&format!("load_scene failed: {err:?}")))?;
    step_start = log_timing("Assets::load_scene", step_start);
    let _ = web_sys::Url::revoke_object_url(&url);

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .map_err(|err| JsValue::from_str(&format!("instantiate failed: {err:?}")))?;
    step_start = log_timing("Scene::instantiate", step_start);
    let aspect = if viewport_width > 0 && viewport_height > 0 {
        viewport_width as f32 / viewport_height as f32
    } else {
        1.0
    };
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(aspect),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .map_err(|err| JsValue::from_str(&format!("add_perspective_camera failed: {err:?}")))?;
    scene
        .set_active_camera(camera)
        .map_err(|err| JsValue::from_str(&format!("set_active_camera failed: {err:?}")))?;
    scene
        .frame_import(camera, &import)
        .map_err(|err| JsValue::from_str(&format!("frame_import failed: {err:?}")))?;
    log_timing("camera + frame_import", step_start);
    log_timing("load_gltf_from_bytes total", total_start);

    let controls = orbit_controls_from_framed_import(&scene, &import, camera)
        .unwrap_or_else(|| OrbitControls::new(Vec3::ZERO, 2.0))
        .with_angles(-0.62, 0.28)
        .with_damping(0.12);

    Ok(DemoApp {
        assets,
        scene,
        camera,
        controls,
        renderer: None,
    })
}

fn orbit_controls_from_framed_import(
    scene: &Scene,
    import: &crate::SceneImport,
    camera: CameraKey,
) -> Option<OrbitControls> {
    let target = import.bounds_world(scene)?.center();
    let camera_node = scene.camera_node(camera)?;
    let camera_position = scene.world_transform(camera_node)?.translation;
    let distance = camera_position.distance(target);
    Some(OrbitControls::new(target, distance))
}

#[wasm_bindgen]
pub async fn attach_to_canvas(app: &mut DemoApp, canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    let total_start = now_ms();
    let width = canvas.width();
    let height = canvas.height();
    let surface = PlatformSurface::browser_webgl2_canvas_element(canvas, width, height);
    let mut renderer = Renderer::from_surface_async(surface)
        .await
        .map_err(|err| JsValue::from_str(&format!("renderer creation failed: {err:?}")))?;
    let step_start = log_timing("Renderer::from_surface_async(WebGL2)", total_start);
    if demo_environment_enabled() {
        renderer.set_environment(app.assets.default_environment());
    }
    renderer.set_background_color(Color::from_linear_rgb(0.014, 0.017, 0.024));
    renderer.set_exposure_ev(0.5);
    app.renderer = Some(renderer);
    log_timing("attach_to_canvas setup", step_start);
    log_timing("attach_to_canvas total", total_start);
    Ok(())
}

#[wasm_bindgen]
pub fn forward_pointer_event(
    app: &mut DemoApp,
    kind: &str,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
) {
    let event = match kind {
        "down" => PointerEvent::primary_pressed(x, y),
        "up" => PointerEvent::released(x, y),
        "move" => PointerEvent::moved(x, y, delta_x, delta_y),
        "wheel" => PointerEvent::wheel(x, y, delta_y),
        _ => return,
    };
    let _ = app.controls.handle_pointer(event);
}

#[wasm_bindgen]
pub fn resize(app: &mut DemoApp, width: u32, height: u32) -> Result<(), JsValue> {
    let Some(renderer) = app.renderer.as_mut() else {
        return Ok(());
    };
    renderer
        .handle_surface_event(SurfaceEvent::Resize { width, height })
        .map_err(|err| JsValue::from_str(&format!("resize failed: {err:?}")))?;
    Ok(())
}

#[wasm_bindgen]
pub fn tick(app: &mut DemoApp, _dt_seconds: f64) -> Result<(), JsValue> {
    let total_start = now_ms();
    let renderer = app
        .renderer
        .as_mut()
        .ok_or_else(|| JsValue::from_str("attach_to_canvas must be called before tick"))?;
    app.controls
        .apply_to_scene(&mut app.scene, app.camera)
        .map_err(|err| JsValue::from_str(&format!("apply_to_scene failed: {err:?}")))?;
    let step_start = log_timing("OrbitControls::apply_to_scene", total_start);
    renderer
        .prepare_with_assets(&mut app.scene, &app.assets)
        .map_err(|err| JsValue::from_str(&format!("prepare failed: {err:?}")))?;
    let step_start = log_timing("Renderer::prepare_with_assets", step_start);
    renderer
        .render(&app.scene, app.camera)
        .map_err(|err| JsValue::from_str(&format!("render failed: {err:?}")))?;
    log_timing("Renderer::render", step_start);
    log_timing("tick total", total_start);
    Ok(())
}
