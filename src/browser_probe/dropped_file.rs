//! Dropped-file browser proof path for `<scena-viewer>`.

use serde_json::json;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::render_scene;
use crate::{
    Aabb, AssetError, AssetFetcher, AssetPath, Assets, Backend, CameraKey, PerspectiveCamera,
    Scene, SceneAsset, Transform, Vec3,
};

pub(super) async fn render_dropped_file_probe(
    canvas: HtmlCanvasElement,
    backend: Backend,
    bytes: &[u8],
    file_name: &str,
) -> Result<String, JsValue> {
    let dropped_path = AssetPath::from(format!("memory:dropped/{file_name}"));
    let assets = Assets::with_fetcher(DroppedFileFetcher {
        path: dropped_path.clone(),
        bytes: bytes.to_vec(),
    });
    let scene_asset = load_scene_asset_from_bytes(&assets, dropped_path.clone()).await?;
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).map_err(|error| {
        JsValue::from_str(&format!("dropped file instantiate failed: {error:?}"))
    })?;
    let roots = import.roots().len();
    let bounds = import.bounds_world(&scene);
    let viewport = (canvas.width().max(1), canvas.height().max(1));
    let (camera, auto_frame) = if let Some(bounds) = bounds {
        let camera = scene
            .add_perspective_camera_default_for(bounds, viewport)
            .map_err(|error| JsValue::from_str(&format!("dropped file frame failed: {error:?}")))?;
        let auto_frame = auto_frame_metadata(&scene, camera, bounds, viewport.0, viewport.1)?;
        (camera, auto_frame)
    } else {
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::standard(),
                Transform::at(Vec3::new(0.0, 0.0, 2.0)),
            )
            .map_err(|error| {
                JsValue::from_str(&format!("dropped file camera failed: {error:?}"))
            })?;
        scene.set_active_camera(camera).map_err(|error| {
            JsValue::from_str(&format!("dropped file active camera failed: {error:?}"))
        })?;
        (
            camera,
            json!({
                "status": "unavailable",
                "reason": "dropped scene had no world bounds",
            }),
        )
    };
    scene
        .add_studio_lighting()
        .map_err(|error| JsValue::from_str(&format!("dropped file lighting failed: {error:?}")))?;
    render_scene(
        canvas,
        backend,
        "scena-viewer-drop-render",
        &assets,
        &mut scene,
        camera,
        json!({
            "proof_class": "scena-viewer-drop-render",
            "file_name": file_name,
            "dropped_bytes": bytes.len(),
            "roots": roots,
            "framed": bounds.is_some(),
            "auto_frame": auto_frame,
        }),
        None,
    )
    .await
}

fn auto_frame_metadata(
    scene: &Scene,
    camera: CameraKey,
    bounds: Aabb,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<serde_json::Value, JsValue> {
    let corners = [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ];
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for corner in corners {
        let point = scene
            .project_world_point(camera, corner, viewport_width, viewport_height)
            .map_err(|error| {
                JsValue::from_str(&format!("auto-frame projection failed: {error:?}"))
            })?
            .ok_or_else(|| {
                JsValue::from_str("auto-frame projected a bounds corner outside the camera")
            })?;
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    let viewport_width_f = viewport_width as f32;
    let viewport_height_f = viewport_height as f32;
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let center_error_x = (center_x - viewport_width_f * 0.5).abs();
    let center_error_y = (center_y - viewport_height_f * 0.5).abs();
    let fill_fraction = (width / viewport_width_f).max(height / viewport_height_f);
    let inside_viewport = min_x >= -0.5
        && min_y >= -0.5
        && max_x <= viewport_width_f + 0.5
        && max_y <= viewport_height_f + 0.5;
    let centered =
        center_error_x <= viewport_width_f * 0.05 && center_error_y <= viewport_height_f * 0.05;
    let passed = inside_viewport && centered && fill_fraction > 0.2 && fill_fraction <= 0.75;

    Ok(json!({
        "status": if passed { "passed" } else { "failed" },
        "proof_class": "viewer-level-auto-framing",
        "viewport": {
            "width": viewport_width,
            "height": viewport_height,
        },
        "projected_rect": {
            "min_x": min_x,
            "min_y": min_y,
            "max_x": max_x,
            "max_y": max_y,
            "width": width,
            "height": height,
            "center_x": center_x,
            "center_y": center_y,
        },
        "center_error_px": {
            "x": center_error_x,
            "y": center_error_y,
        },
        "fill_fraction": fill_fraction,
        "inside_viewport": inside_viewport,
        "centered": centered,
    }))
}

async fn load_scene_asset_from_bytes(
    assets: &Assets<DroppedFileFetcher>,
    path: AssetPath,
) -> Result<SceneAsset, JsValue> {
    assets
        .load_scene(path)
        .await
        .map_err(|error| JsValue::from_str(&format!("dropped file load_scene failed: {error:?}")))
}

#[derive(Debug, Clone)]
struct DroppedFileFetcher {
    path: AssetPath,
    bytes: Vec<u8>,
}

impl AssetFetcher for DroppedFileFetcher {
    type Future<'a> = std::future::Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        if path.as_str() == self.path.as_str() {
            std::future::ready(Ok(self.bytes.clone()))
        } else {
            std::future::ready(Err(AssetError::NotFound {
                path: path.as_str().to_string(),
            }))
        }
    }
}
