//! Drag-and-drop demo page WASM exports.
//!
//! Gated by the `demo-page` feature. Four `wasm_bindgen` entry points:
//! [`load_gltf_from_bytes`], [`attach_to_canvas`], [`forward_pointer_event`],
//! and [`tick`]. The opaque [`DemoApp`] handle is held by the JS side and
//! threaded back into each call.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{
    Aabb, Assets, CameraKey, Color, DirectionalLight, NodeKey, OrbitControls, PerspectiveCamera,
    PlatformSurface, PointerEvent, Renderer, Scene, SceneAsset, SurfaceEvent, Transform, Vec3,
};

const DEMO_HDR_ENVIRONMENT: &str = "samples/environment/studio_small_03_1k.hdr";
const CONNECTOR_REPLAY_SECONDS: f64 = 1.8;
const CONNECTOR_START_OFFSET: Vec3 = Vec3::new(-1.08, 0.11, 0.0);
const CONNECTOR_KEY_LIGHT_LUX: f32 = 300.0;

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn log_timing(label: &str, start_ms: f64) -> f64 {
    let elapsed_ms = now_ms() - start_ms;
    if demo_timing_enabled() {
        web_sys::console::log_1(&format!("[scena-demo] {label}: {elapsed_ms:.1}ms").into());
    }
    now_ms()
}

fn demo_timing_enabled() -> bool {
    let search = web_sys::window()
        .and_then(|window| js_sys::Reflect::get(&window, &JsValue::from_str("location")).ok())
        .and_then(|location| js_sys::Reflect::get(&location, &JsValue::from_str("search")).ok())
        .and_then(|search| search.as_string());
    search.is_some_and(|search| {
        query_flag_enabled(&search, "perf") || query_flag_enabled(&search, "timing")
    })
}

fn demo_environment_enabled() -> bool {
    let search = web_sys::window()
        .and_then(|window| js_sys::Reflect::get(&window, &JsValue::from_str("location")).ok())
        .and_then(|location| js_sys::Reflect::get(&location, &JsValue::from_str("search")).ok())
        .and_then(|search| search.as_string());
    search.is_none_or(|search| !search.contains("env=0") && !search.contains("environment=0"))
}

fn query_flag_enabled(search: &str, name: &str) -> bool {
    let needle = format!("{name}=");
    search
        .trim_start_matches('?')
        .split('&')
        .filter(|part| !part.is_empty())
        .any(|part| {
            let Some(rest) = part.strip_prefix(&needle) else {
                return part == name;
            };
            rest != "0" && rest != "false"
        })
}

#[wasm_bindgen]
pub struct DemoApp {
    assets: Assets,
    scene: Scene,
    camera: CameraKey,
    controls: OrbitControls,
    renderer: Option<Renderer>,
    connector_replay: Option<ConnectorReplay>,
}

struct ConnectorReplay {
    drive_root: NodeKey,
    start: Transform,
    end: Transform,
    elapsed_seconds: f64,
    active: bool,
}

#[wasm_bindgen]
pub async fn load_gltf_from_bytes(
    bytes: Box<[u8]>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    let total_start = now_ms();
    let mut step_start = total_start;
    let assets = Assets::new();
    let scene_asset = load_scene_asset_from_bytes(&assets, bytes.as_ref()).await?;
    step_start = log_timing("Assets::load_scene", step_start);

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
        .with_angles(-0.46, 0.34)
        .with_damping(0.12);

    Ok(DemoApp {
        assets,
        scene,
        camera,
        controls,
        renderer: None,
        connector_replay: None,
    })
}

#[wasm_bindgen]
pub async fn load_connector_snap_from_bytes(
    drive_bytes: Box<[u8]>,
    load_bytes: Box<[u8]>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    let total_start = now_ms();
    let assets = Assets::new();
    let drive_part = load_scene_asset_from_bytes(&assets, drive_bytes.as_ref()).await?;
    let load_part = load_scene_asset_from_bytes(&assets, load_bytes.as_ref()).await?;
    let mut scene = Scene::new();
    let load = scene
        .instantiate(&load_part)
        .map_err(|err| JsValue::from_str(&format!("instantiate load failed: {err:?}")))?;
    let drive = scene
        .instantiate(&drive_part)
        .map_err(|err| JsValue::from_str(&format!("instantiate drive failed: {err:?}")))?;
    let drive_root = *drive
        .roots()
        .first()
        .ok_or_else(|| JsValue::from_str("drive import has no root node"))?;
    let start = Transform::at(CONNECTOR_START_OFFSET);
    scene
        .set_transform(drive_root, start)
        .map_err(|err| JsValue::from_str(&format!("set replay start failed: {err:?}")))?;
    scene
        .mate(&drive, "shaft", &load, "hub")
        .map_err(|err| JsValue::from_str(&format!("connector mate failed: {err:?}")))?;
    let end = scene
        .world_transform(drive_root)
        .ok_or_else(|| JsValue::from_str("drive root transform missing after mate"))?;
    scene.set_transform(drive_root, end).map_err(|err| {
        JsValue::from_str(&format!("set assembled connector pose failed: {err:?}"))
    })?;
    let connector_bounds = combined_import_bounds(&scene, &drive, &load);

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
    add_connector_key_light(&mut scene)?;
    let (target, distance) = connector_bounds
        .map(|bounds| {
            (
                bounds.center(),
                (bounds.bounding_sphere_radius() * 3.1).clamp(2.0, 3.6),
            )
        })
        .unwrap_or((Vec3::new(-0.24, 0.06, 0.0), 2.15));
    let controls = OrbitControls::new(target, distance)
        .with_angles(-0.48, 0.31)
        .with_damping(0.12);
    controls.apply_to_scene(&mut scene, camera).map_err(|err| {
        JsValue::from_str(&format!("apply initial connector camera failed: {err:?}"))
    })?;
    log_timing("load_connector_snap_from_bytes total", total_start);

    Ok(DemoApp {
        assets,
        scene,
        camera,
        controls,
        renderer: None,
        connector_replay: Some(ConnectorReplay {
            drive_root,
            start,
            end,
            elapsed_seconds: 0.0,
            active: false,
        }),
    })
}

async fn load_scene_asset_from_bytes(assets: &Assets, bytes: &[u8]) -> Result<SceneAsset, JsValue> {
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&array.into());
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts.into())
        .map_err(|err| JsValue::from_str(&format!("Blob construction failed: {err:?}")))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|err| JsValue::from_str(&format!("createObjectURL failed: {err:?}")))?;
    let scene_asset = assets
        .load_scene(url.as_str())
        .await
        .map_err(|err| JsValue::from_str(&format!("load_scene failed: {err:?}")));
    let _ = web_sys::Url::revoke_object_url(&url);
    scene_asset
}

fn add_connector_key_light(scene: &mut Scene) -> Result<(), JsValue> {
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::WHITE)
                .with_illuminance_lux(CONNECTOR_KEY_LIGHT_LUX)
                .with_shadows(true),
        )
        .transform(Transform::default().rotate_x_deg(-34.0).rotate_y_deg(26.0))
        .add()
        .map(|_| ())
        .map_err(|err| JsValue::from_str(&format!("add connector key light failed: {err:?}")))
}

fn combined_import_bounds(
    scene: &Scene,
    left: &crate::SceneImport,
    right: &crate::SceneImport,
) -> Option<Aabb> {
    match (left.bounds_world(scene), right.bounds_world(scene)) {
        (Some(left), Some(right)) => Some(union_aabb(left, right)),
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}

fn union_aabb(left: Aabb, right: Aabb) -> Aabb {
    Aabb::new(
        Vec3::new(
            left.min.x.min(right.min.x),
            left.min.y.min(right.min.y),
            left.min.z.min(right.min.z),
        ),
        Vec3::new(
            left.max.x.max(right.max.x),
            left.max.y.max(right.max.y),
            left.max.z.max(right.max.z),
        ),
    )
}

fn orbit_controls_from_framed_import(
    scene: &Scene,
    import: &crate::SceneImport,
    camera: CameraKey,
) -> Option<OrbitControls> {
    let target = import.bounds_world(scene)?.center();
    let camera_node = scene.camera_node(camera)?;
    let camera_position = scene.world_transform(camera_node)?.translation;
    let distance = camera_position.distance(target) * 0.82;
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
        let environment = app
            .assets
            .load_environment(DEMO_HDR_ENVIRONMENT)
            .await
            .map_err(|err| JsValue::from_str(&format!("load_environment failed: {err:?}")))?;
        renderer.set_environment(environment);
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
pub fn replay_connector_snap(app: &mut DemoApp) -> Result<(), JsValue> {
    if let Some(replay) = app.connector_replay.as_mut() {
        replay.elapsed_seconds = 0.0;
        replay.active = true;
        app.scene
            .set_transform(replay.drive_root, replay.start)
            .map_err(|err| JsValue::from_str(&format!("replay reset failed: {err:?}")))?;
    }
    Ok(())
}

#[wasm_bindgen]
pub fn connector_replay_active(app: &DemoApp) -> bool {
    app.connector_replay
        .as_ref()
        .is_some_and(|replay| replay.active)
}

#[wasm_bindgen]
pub fn tick(app: &mut DemoApp, dt_seconds: f64) -> Result<(), JsValue> {
    let total_start = now_ms();
    app.apply_connector_replay(dt_seconds)?;
    app.controls
        .apply_to_scene(&mut app.scene, app.camera)
        .map_err(|err| JsValue::from_str(&format!("apply_to_scene failed: {err:?}")))?;
    let step_start = log_timing("OrbitControls::apply_to_scene", total_start);
    let renderer = app
        .renderer
        .as_mut()
        .ok_or_else(|| JsValue::from_str("attach_to_canvas must be called before tick"))?;
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

impl DemoApp {
    fn apply_connector_replay(&mut self, dt_seconds: f64) -> Result<(), JsValue> {
        let Some(replay) = self.connector_replay.as_mut() else {
            return Ok(());
        };
        if !replay.active {
            return Ok(());
        }
        replay.elapsed_seconds += dt_seconds.max(0.0);
        let raw = (replay.elapsed_seconds / CONNECTOR_REPLAY_SECONDS).clamp(0.0, 1.0) as f32;
        let amount = smoothstep(raw);
        self.scene
            .set_transform(
                replay.drive_root,
                lerp_transform(replay.start, replay.end, amount),
            )
            .map_err(|err| {
                JsValue::from_str(&format!("connector replay transform failed: {err:?}"))
            })?;
        if raw >= 1.0 {
            replay.active = false;
            self.scene
                .set_transform(replay.drive_root, replay.end)
                .map_err(|err| {
                    JsValue::from_str(&format!("connector replay finish failed: {err:?}"))
                })?;
        }
        Ok(())
    }
}

fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn lerp_transform(start: Transform, end: Transform, amount: f32) -> Transform {
    Transform {
        translation: start.translation.lerp(end.translation, amount),
        rotation: start.rotation.slerp(end.rotation, amount),
        scale: start.scale.lerp(end.scale, amount),
    }
}
