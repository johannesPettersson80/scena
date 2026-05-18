use wasm_bindgen::prelude::*;

use crate::{CameraKey, ConnectorFrame, NodeKey, Scene, Transform, Vec3};

#[derive(Debug, Clone, Copy)]
pub(super) struct ConnectorMarker {
    pub(super) node: NodeKey,
    pub(super) local_transform: Transform,
}

pub(super) fn connector_marker(
    import: &crate::SceneImport,
    name: &str,
) -> Result<ConnectorMarker, JsValue> {
    let connector = import
        .connector(name)
        .map_err(|err| JsValue::from_str(&format!("connector {name:?} missing: {err:?}")))?;
    let frame = ConnectorFrame::from_import_connector(connector);
    Ok(ConnectorMarker {
        node: frame.node(),
        local_transform: frame.local_transform(),
    })
}

pub(super) fn project_connector_marker(
    scene: &Scene,
    camera: CameraKey,
    marker: ConnectorMarker,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<JsValue, JsValue> {
    let world = marker_world_transform(scene, marker)
        .ok_or_else(|| JsValue::from_str("connector marker node transform missing"))?;
    let projected = scene
        .project_world_point(
            camera,
            world.translation,
            viewport_width.max(1),
            viewport_height.max(1),
        )
        .map_err(|err| JsValue::from_str(&format!("project_world_point failed: {err:?}")))?;
    let object = js_sys::Object::new();
    match projected {
        Some(projected) => {
            set_object_bool(&object, "visible", true)?;
            set_object_number(&object, "x", projected.x as f64)?;
            set_object_number(&object, "y", projected.y as f64)?;
            set_object_number(&object, "depth", projected.depth as f64)?;
        }
        None => {
            set_object_bool(&object, "visible", false)?;
        }
    }
    Ok(object.into())
}

fn marker_world_transform(scene: &Scene, marker: ConnectorMarker) -> Option<Transform> {
    let node_world = scene.world_transform(marker.node)?;
    Some(compose_marker_transform(node_world, marker.local_transform))
}

fn compose_marker_transform(parent: Transform, child: Transform) -> Transform {
    let scaled_child_translation = Vec3::new(
        child.translation.x * parent.scale.x,
        child.translation.y * parent.scale.y,
        child.translation.z * parent.scale.z,
    );
    Transform {
        translation: parent.translation + parent.rotation * scaled_child_translation,
        rotation: normalize_rotation(parent.rotation * child.rotation),
        scale: Vec3::new(
            parent.scale.x * child.scale.x,
            parent.scale.y * child.scale.y,
            parent.scale.z * child.scale.z,
        ),
    }
}

fn normalize_rotation(rotation: crate::Quat) -> crate::Quat {
    let length_sq = rotation.length_squared();
    if length_sq <= f32::EPSILON || !length_sq.is_finite() {
        return crate::Quat::IDENTITY;
    }
    rotation.normalize()
}

pub(super) fn set_object_value(
    object: &js_sys::Object,
    key: &str,
    value: JsValue,
) -> Result<(), JsValue> {
    js_sys::Reflect::set(object, &JsValue::from_str(key), &value).map(|_| ())
}

fn set_object_number(object: &js_sys::Object, key: &str, value: f64) -> Result<(), JsValue> {
    set_object_value(object, key, JsValue::from_f64(value))
}

fn set_object_bool(object: &js_sys::Object, key: &str, value: bool) -> Result<(), JsValue> {
    set_object_value(object, key, JsValue::from_bool(value))
}
