//! Drag-and-drop demo page WASM exports.
//!
//! Gated by the `demo-page` feature. Four `wasm_bindgen` entry points:
//! [`load_gltf_from_bytes`], [`attach_to_canvas`], [`forward_pointer_event`],
//! and [`tick`]. The opaque [`DemoApp`] handle is held by the JS side and
//! threaded back into each call.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{
    AntiAliasing, Assets, AutoExposureConfig, Background, CameraKey, Color, FramingOptions,
    GridFloorOptions, NodeKey, OrbitControls, PerspectiveCamera, PlatformSurface, PointerEvent,
    PostBloomConfig, Renderer, Scene, SurfaceEvent, Transform, Vec3,
};

mod bounds;
mod connectors;
mod floor;
mod imports;
mod material_presets;
mod replay;

use bounds::{combined_import_bounds, union_optional_bounds};
use connectors::{ConnectorMarker, connector_marker, project_connector_marker, set_object_value};
use floor::{ground_import_roots, ground_offset_to_floor, translate_transform};
use imports::load_scene_asset_from_bytes;
use replay::{lerp_transform, smoothstep};

const CONNECTOR_REPLAY_SECONDS: f64 = 1.8;
const DEMO_HDR_ENVIRONMENT: &str = "samples/environment/white_studio_03_1k.hdr";
const CONNECTOR_SOLVE_SEED_OFFSET: Vec3 = Vec3::new(-0.62, 0.11, 0.0);
const CONNECTOR_REPLAY_SEPARATION_X: f32 = 0.48;
const DEMO_BACKGROUND: Color = Color::TRANSPARENT;

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
    shaft_marker: ConnectorMarker,
    hub_marker: ConnectorMarker,
    elapsed_seconds: f64,
    active: bool,
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
    let shaft_marker = connector_marker(&drive, "shaft")?;
    let hub_marker = connector_marker(&load, "hub")?;
    let solve_seed = Transform::at(CONNECTOR_SOLVE_SEED_OFFSET);
    scene
        .set_transform(drive_root, solve_seed)
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
    let ungrounded_end_bounds = combined_import_bounds(&scene, &drive, &load);
    let ground_offset = ground_offset_to_floor(ungrounded_end_bounds);
    ground_import_roots(&mut scene, &load, ground_offset)?;
    let end = translate_transform(end, ground_offset);
    let start = horizontal_replay_start_from_end(end);
    scene.set_transform(drive_root, start).map_err(|err| {
        JsValue::from_str(&format!(
            "set grounded connector before pose failed: {err:?}"
        ))
    })?;
    let drive_replay_bounds = scene
        .bounds_for_transforms(drive_root, &[start, end], &assets)
        .map_err(|err| JsValue::from_str(&format!("connector replay bounds failed: {err:?}")))?;
    let connector_bounds =
        union_optional_bounds(Some(drive_replay_bounds), load.bounds_world(&scene))
            .ok_or_else(|| JsValue::from_str("connector scene has no renderable bounds"))?;
    scene
        .add_grid_floor(
            &assets,
            GridFloorOptions::new()
                .under_bounds(connector_bounds)
                .padding(0.46)
                .line_spacing(0.24),
        )
        .map_err(|err| JsValue::from_str(&format!("add_grid_floor failed: {err:?}")))?;

    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .map_err(|err| JsValue::from_str(&format!("add_perspective_camera failed: {err:?}")))?;
    scene
        .set_active_camera(camera)
        .map_err(|err| JsValue::from_str(&format!("set_active_camera failed: {err:?}")))?;
    let is_mobile_viewport = viewport_width < 640;
    let connector_fill = if is_mobile_viewport { 0.82 } else { 0.72 };
    let connector_margin = if is_mobile_viewport { 12.0 } else { 24.0 };
    let framing = scene
        .frame_bounds(
            camera,
            connector_bounds,
            FramingOptions::new()
                .azimuth_elevation(-27.5, 17.8)
                .fill(connector_fill)
                .margin_px(connector_margin)
                .viewport(viewport_width.max(1), viewport_height.max(1)),
        )
        .map_err(|err| JsValue::from_str(&format!("connector frame_bounds failed: {err:?}")))?;
    let controls = OrbitControls::from_framing(framing).cinematic();
    controls.apply_to_scene(&mut scene, camera).map_err(|err| {
        JsValue::from_str(&format!("apply initial connector camera failed: {err:?}"))
    })?;
    scene
        .add_studio_lighting()
        .map_err(|err| JsValue::from_str(&format!("add_studio_lighting failed: {err:?}")))?;
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
            shaft_marker,
            hub_marker,
            elapsed_seconds: 0.0,
            active: false,
        }),
    })
}

fn horizontal_replay_start_from_end(end: Transform) -> Transform {
    let mut start = end;
    start.translation.x -= CONNECTOR_REPLAY_SEPARATION_X;
    start
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
    renderer.set_background_color(DEMO_BACKGROUND);
    renderer.set_exposure_ev(-0.35);
    renderer.set_auto_exposure(AutoExposureConfig::product_studio());
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
pub fn connector_marker_positions(
    app: &DemoApp,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<JsValue, JsValue> {
    let replay = app.connector_replay.as_ref().ok_or_else(|| {
        JsValue::from_str("connector marker positions are only available for connector scenes")
    })?;
    let shaft = project_connector_marker(
        &app.scene,
        app.camera,
        replay.shaft_marker,
        viewport_width,
        viewport_height,
    )?;
    let hub = project_connector_marker(
        &app.scene,
        app.camera,
        replay.hub_marker,
        viewport_width,
        viewport_height,
    )?;
    let object = js_sys::Object::new();
    set_object_value(&object, "shaft", shaft)?;
    set_object_value(&object, "hub", hub)?;
    Ok(object.into())
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
    log_renderer_auto_exposure(renderer);
    log_timing("Renderer::render", step_start);
    log_timing("tick total", total_start);
    Ok(())
}

fn log_renderer_auto_exposure(renderer: &Renderer) {
    if !demo_timing_enabled() {
        return;
    }
    if let Some(result) = renderer.last_auto_exposure() {
        web_sys::console::log_1(
            &format!(
                "[scena-demo] renderer auto_exposure: luminance={:.4} target={:.4} ev={:.2} samples={} clamped={}",
                result.measured_luminance(),
                result.target_luminance(),
                result.exposure_ev(),
                result.sample_count(),
                result.clamped()
            )
            .into(),
        );
    }
}

/// Switch the renderer background to a named [`Background`] scheme.
///
/// Accepted scheme keys (case-insensitive):
/// `studio`, `dark_studio`, `neutral_gray`, `white`, `black`, `sky`,
/// `transparent`.
#[wasm_bindgen]
pub fn set_background_scheme(app: &mut DemoApp, scheme: &str) -> Result<(), JsValue> {
    let background = match scheme.to_ascii_lowercase().as_str() {
        "studio" => Background::Studio,
        "dark_studio" | "darkstudio" | "dark-studio" => Background::DarkStudio,
        "neutral_gray" | "neutralgray" | "neutral-gray" => Background::NeutralGray,
        "white" => Background::White,
        "black" => Background::Black,
        "sky" => Background::Sky,
        "transparent" => Background::Transparent,
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown background scheme: {other}"
            )));
        }
    };
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_background_scheme")
    })?;
    renderer.set_background(background);
    Ok(())
}

/// Switch the renderer's auto-exposure to a named scenario.
///
/// Accepted preset keys: `product_studio`, `indoor`, `outdoor`, `mixed`.
#[wasm_bindgen]
pub fn set_auto_exposure_preset(app: &mut DemoApp, preset: &str) -> Result<(), JsValue> {
    let config = match preset.to_ascii_lowercase().as_str() {
        "product_studio" | "productstudio" | "product-studio" => {
            AutoExposureConfig::product_studio()
        }
        "indoor" => AutoExposureConfig::indoor(),
        "outdoor" => AutoExposureConfig::outdoor(),
        "mixed" | "default" => AutoExposureConfig::mixed(),
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown auto-exposure preset: {other}"
            )));
        }
    };
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_auto_exposure_preset")
    })?;
    renderer.set_auto_exposure(config);
    Ok(())
}

/// Switch the renderer's anti-aliasing mode.
///
/// Accepted mode keys: `fxaa` (the default), `none`.
#[wasm_bindgen]
pub fn set_anti_aliasing_mode(app: &mut DemoApp, mode: &str) -> Result<(), JsValue> {
    let setting = match mode.to_ascii_lowercase().as_str() {
        "fxaa" | "on" | "true" => AntiAliasing::Fxaa,
        "none" | "off" | "false" => AntiAliasing::None,
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown anti-aliasing mode: {other}"
            )));
        }
    };
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_anti_aliasing_mode")
    })?;
    renderer.set_anti_aliasing(setting);
    Ok(())
}

/// Toggle the subtle post bloom pass on or off.
#[wasm_bindgen]
pub fn set_bloom_enabled(app: &mut DemoApp, enabled: bool) -> Result<(), JsValue> {
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_bloom_enabled")
    })?;
    renderer.set_bloom(if enabled {
        Some(PostBloomConfig::subtle())
    } else {
        None
    });
    Ok(())
}

/// Encode the latest rendered frame as PNG bytes for download or upload.
///
/// Requires that `tick` has been called at least once after `attach_to_canvas`
/// so the renderer holds a populated readback buffer.
#[wasm_bindgen]
pub fn capture_png_bytes(app: &DemoApp) -> Result<Box<[u8]>, JsValue> {
    use std::io::Cursor;
    let renderer = app.renderer.as_ref().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before capture_png_bytes")
    })?;
    let stats = renderer.stats();
    let width = stats.target_width;
    let height = stats.target_height;
    let frame = renderer.frame_rgba8();
    let expected = width as usize * height as usize * 4;
    if frame.len() != expected {
        return Err(JsValue::from_str(&format!(
            "renderer frame buffer is {} bytes; expected {} RGBA8 bytes",
            frame.len(),
            expected
        )));
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|err| JsValue::from_str(&format!("png header failed: {err:?}")))?;
        writer
            .write_image_data(frame)
            .map_err(|err| JsValue::from_str(&format!("png body failed: {err:?}")))?;
    }
    Ok(bytes.into_boxed_slice())
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
