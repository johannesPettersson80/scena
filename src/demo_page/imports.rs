use wasm_bindgen::prelude::*;

use crate::{
    Assets, FramingOptions, GridFloorOptions, OrbitControls, PerspectiveCamera, Scene, SceneAsset,
    Transform, Vec3,
};

use super::floor::ground_import_at_floor;
use super::{DemoApp, log_timing, now_ms};

#[wasm_bindgen]
pub async fn load_gltf_from_bytes(
    bytes: Box<[u8]>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    load_gltf_from_bytes_internal(bytes, viewport_width, viewport_height, false, None).await
}

#[wasm_bindgen]
pub async fn load_gltf_with_floor_from_bytes(
    bytes: Box<[u8]>,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    load_gltf_from_bytes_internal(bytes, viewport_width, viewport_height, true, None).await
}

#[wasm_bindgen]
pub async fn load_gltf_with_view_from_bytes(
    bytes: Box<[u8]>,
    viewport_width: u32,
    viewport_height: u32,
    add_floor: bool,
    orbit_yaw: f32,
    orbit_pitch: f32,
) -> Result<DemoApp, JsValue> {
    load_gltf_from_bytes_internal(
        bytes,
        viewport_width,
        viewport_height,
        add_floor,
        Some((orbit_yaw, orbit_pitch)),
    )
    .await
}

async fn load_gltf_from_bytes_internal(
    bytes: Box<[u8]>,
    viewport_width: u32,
    viewport_height: u32,
    add_floor: bool,
    view_angles: Option<(f32, f32)>,
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
    let import_bounds = if add_floor {
        ground_import_at_floor(&mut scene, &import)?
    } else {
        import.bounds_world(&scene)
    };
    if let (true, Some(bounds)) = (add_floor, import_bounds) {
        scene
            .add_grid_floor(
                &assets,
                GridFloorOptions::new()
                    .under_bounds(bounds)
                    .padding(0.42)
                    .line_spacing(0.24),
            )
            .map_err(|err| JsValue::from_str(&format!("add_grid_floor failed: {err:?}")))?;
    }
    let framing = import_bounds
        .map(|bounds| {
            let mut options = FramingOptions::new()
                .fill(0.72)
                .margin_px(24.0)
                .viewport(viewport_width.max(1), viewport_height.max(1));
            options = if let Some((yaw, pitch)) = view_angles {
                options.orbit(yaw, pitch)
            } else {
                options.isometric()
            };
            scene.frame_bounds(camera, bounds, options)
        })
        .transpose()
        .map_err(|err| JsValue::from_str(&format!("frame_bounds failed: {err:?}")))?;
    scene
        .add_studio_lighting()
        .map_err(|err| JsValue::from_str(&format!("add_studio_lighting failed: {err:?}")))?;
    log_timing("camera + frame_bounds", step_start);
    log_timing("load_gltf_from_bytes total", total_start);

    let controls = framing
        .map(OrbitControls::from_framing)
        .unwrap_or_else(|| OrbitControls::new(Vec3::ZERO, 2.0))
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

pub(super) async fn load_scene_asset_from_bytes(
    assets: &Assets,
    bytes: &[u8],
) -> Result<SceneAsset, JsValue> {
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
