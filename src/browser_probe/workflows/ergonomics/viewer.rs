use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::super::{WorkflowScene, add_default_camera};
use crate::{
    Assets, Color, ConnectOptions, ConnectorFrame, CursorPosition, GeometryDesc, MaterialDesc,
    Scene, TextureColorSpace, Transform, Vec3, Viewport,
};

const WORKFLOW_NAME: &str = "textured-connector-viewer";

pub(super) async fn textured_connector_viewer_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let red_png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let base = assets
        .load_texture(red_png, TextureColorSpace::Srgb)
        .await
        .map_err(|error| JsValue::from_str(&format!("assembly texture failed: {error:?}")))?;
    let decoded_texture = assets
        .texture(base)
        .is_some_and(|texture| texture.has_decoded_pixels());
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.35, 0.28, 0.24));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(base));
    let mut scene = Scene::new();
    let source = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(-0.55, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("assembly source failed: {error:?}")))?;
    let target = scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(0.55, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("assembly target failed: {error:?}")))?;
    let connection = scene
        .connect(
            ConnectorFrame::new(source, Transform::at(Vec3::new(0.175, 0.0, 0.0)))
                .named("source-face"),
            ConnectorFrame::new(target, Transform::at(Vec3::new(-0.175, 0.0, 0.0)))
                .named("target-face"),
            ConnectOptions::default(),
        )
        .map_err(|error| JsValue::from_str(&format!("assembly connect failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    scene
        .frame_all_with_assets(camera, &assets)
        .map_err(|error| JsValue::from_str(&format!("assembly frame failed: {error:?}")))?;
    let viewport =
        Viewport::new(96, 96, 1.0).ok_or_else(|| JsValue::from_str("invalid assembly viewport"))?;
    let picked = scene
        .pick_and_select_with_assets(
            camera,
            CursorPosition::physical(48.0, 48.0),
            viewport,
            &assets,
        )
        .map_err(|error| JsValue::from_str(&format!("assembly pick failed: {error:?}")))?
        .is_some();
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "workflow": WORKFLOW_NAME,
            "decoded_base_color_texture": decoded_texture,
            "connected": true,
            "framed": true,
            "picked": picked,
            "selected": true,
            "connection_line": {
                "start": [
                    connection.connection_line().start().x,
                    connection.connection_line().start().y,
                    connection.connection_line().start().z,
                ],
                "end": [
                    connection.connection_line().end().x,
                    connection.connection_line().end().y,
                    connection.connection_line().end().z,
                ],
            },
        }),
    })
}
