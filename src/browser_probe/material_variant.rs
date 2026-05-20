//! Material-variant browser proof path for `<scena-viewer>`.

use serde_json::json;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use super::render_scene;
use crate::{Assets, Backend, PerspectiveCamera, Scene, Transform, Vec3};

const VARIANT_FIXTURE: &str = "/fixtures/gltf/material_variants_scene.gltf";

pub(super) async fn render_material_variant_probe(
    canvas: HtmlCanvasElement,
    backend: Backend,
    variant_name: &str,
) -> Result<String, JsValue> {
    let assets = Assets::new();
    let scene_asset = assets
        .load_scene(VARIANT_FIXTURE)
        .await
        .map_err(|error| JsValue::from_str(&format!("material variant load failed: {error:?}")))?;
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).map_err(|error| {
        JsValue::from_str(&format!("material variant instantiate failed: {error:?}"))
    })?;
    let available_variants = import.material_variants().to_vec();
    scene
        .set_active_variant(&import, Some(variant_name))
        .map_err(|error| {
            JsValue::from_str(&format!("material variant select failed: {error:?}"))
        })?;
    let active_variant = import.active_variant();
    let framed = import.bounds_world(&scene).is_some();
    let camera = if let Some(bounds) = import.bounds_world(&scene) {
        scene
            .add_perspective_camera_default_for(
                bounds,
                (canvas.width().max(1), canvas.height().max(1)),
            )
            .map_err(|error| {
                JsValue::from_str(&format!("material variant frame failed: {error:?}"))
            })?
    } else {
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::standard(),
                Transform::at(Vec3::new(0.0, 0.0, 2.0)),
            )
            .map_err(|error| {
                JsValue::from_str(&format!("material variant camera failed: {error:?}"))
            })?;
        scene.set_active_camera(camera).map_err(|error| {
            JsValue::from_str(&format!("material variant active camera failed: {error:?}"))
        })?;
        camera
    };
    render_scene(
        canvas,
        backend,
        "scena-viewer-material-variant-render",
        &assets,
        &mut scene,
        camera,
        json!({
            "proof_class": "scena-viewer-material-variant-render",
            "source": VARIANT_FIXTURE,
            "available_variants": available_variants,
            "selected_variant": variant_name,
            "active_variant": active_variant,
            "framed": framed,
        }),
        None,
    )
    .await
}
