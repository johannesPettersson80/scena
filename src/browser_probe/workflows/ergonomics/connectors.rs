use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::{WorkflowScene, add_default_camera};
use crate::{
    Assets, Color, ConnectOptions, ConnectorFrame, GeometryDesc, MaterialDesc, Scene, Transform,
    Vec3,
};

pub(super) fn connector_connection_scene(connected: bool) -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let source_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.28, 0.2, 0.2));
    let target_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.28, 0.2, 0.2));
    let source_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(255, 70, 40)));
    let target_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(40, 120, 255)));
    let mut scene = Scene::new();
    let source = scene
        .mesh(source_geometry, source_material)
        .transform(Transform::at(Vec3::new(-0.65, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("source connector mesh failed: {error:?}")))?;
    let target = scene
        .mesh(target_geometry, target_material)
        .transform(Transform::at(Vec3::new(0.45, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("target connector mesh failed: {error:?}")))?;
    let mut connection_line = None;
    if connected {
        let preview = scene
            .connect(
                ConnectorFrame::new(source, Transform::at(Vec3::new(0.14, 0.0, 0.0)))
                    .named("source-face"),
                ConnectorFrame::new(target, Transform::at(Vec3::new(-0.14, 0.0, 0.0)))
                    .named("target-face"),
                ConnectOptions::default(),
            )
            .map_err(|error| JsValue::from_str(&format!("connector solve failed: {error:?}")))?;
        connection_line = Some(preview.connection_line());
    }
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "connected": connected,
            "connection_line": connection_line.map(|line| json!({
                "start": [line.start().x, line.start().y, line.start().z],
                "end": [line.end().x, line.end().y, line.end().z],
            })),
        }),
    })
}

pub(super) fn connector_magnet_preview_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let out_of_range = add_connector_magnet_pair(&assets, &mut scene, -0.35, -0.65)?;
    let snap_ready = add_connector_magnet_pair(&assets, &mut scene, 0.35, 0.18)?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "connector-magnet-preview",
            "magnet_sequence": [out_of_range, snap_ready],
        }),
    })
}

fn add_connector_magnet_pair(
    assets: &Assets,
    scene: &mut Scene,
    y: f32,
    source_x: f32,
) -> Result<serde_json::Value, JsValue> {
    let source_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.24, 0.18, 0.18));
    let target_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.24, 0.18, 0.18));
    let ghost_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.28, 0.22, 0.22));
    let source_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(250, 92, 64)));
    let target_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(62, 142, 255)));
    let source = scene
        .mesh(source_geometry, source_material)
        .transform(Transform::at(Vec3::new(source_x, y, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("magnet source mesh failed: {error:?}")))?;
    let target = scene
        .mesh(target_geometry, target_material)
        .transform(Transform::at(Vec3::new(0.45, y, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("magnet target mesh failed: {error:?}")))?;
    let preview = scene
        .preview_connector_magnet(
            ConnectorFrame::new(source, Transform::at(Vec3::new(0.14, 0.0, 0.0)))
                .named("source-face")
                .with_snap_tolerance(0.08),
            ConnectorFrame::new(target, Transform::at(Vec3::new(-0.14, 0.0, 0.0)))
                .named("target-face")
                .with_snap_tolerance(0.08),
            ConnectOptions::default(),
        )
        .map_err(|error| JsValue::from_str(&format!("magnet preview failed: {error:?}")))?;
    let accent = preview.visual_cue().accent_rgba();
    let accent_color = Color::from_linear_rgba(accent[0], accent[1], accent[2], accent[3]);
    let ghost_material = assets.create_material(MaterialDesc::unlit(accent_color));
    scene
        .mesh(ghost_geometry, ghost_material)
        .transform(preview.ghost_transform())
        .add()
        .map_err(|error| JsValue::from_str(&format!("magnet ghost mesh failed: {error:?}")))?;
    let line = preview.connection_line();
    let line_geometry = assets.create_geometry(GeometryDesc::line(line.start(), line.end()));
    let line_material = assets.create_material(MaterialDesc::line(accent_color, 2.0));
    scene
        .mesh(line_geometry, line_material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("magnet line mesh failed: {error:?}")))?;
    Ok(json!({
        "visual_cue": preview.visual_cue().css_class(),
        "snap_ready": preview.is_snap_ready(),
        "distance": preview.distance(),
        "tolerance": preview.tolerance(),
    }))
}
