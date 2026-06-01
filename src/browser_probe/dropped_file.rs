//! Dropped-file browser proof path for `<scena-viewer>`.

use serde_json::json;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::render_scene;
use crate::{
    AssetError, AssetFetcher, AssetPath, Assets, Backend, PerspectiveCamera, Scene, SceneAsset,
    Transform, Vec3, auto_frame_metadata,
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
        // Keep the browser proof anchored to Scene::project_world_point and
        // the viewer-level-auto-framing proof class even though the reusable
        // metadata builder now lives in crate::capture.
        let auto_frame = serde_json::to_value(
            auto_frame_metadata(&scene, camera, bounds, viewport.0, viewport.1).map_err(
                |error| JsValue::from_str(&format!("auto-frame projection failed: {error}")),
            )?,
        )
        .map_err(|error| {
            JsValue::from_str(&format!(
                "auto-frame metadata serialization failed: {error}"
            ))
        })?;
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
