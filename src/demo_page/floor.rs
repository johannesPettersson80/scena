use wasm_bindgen::prelude::*;

use crate::{Aabb, Scene, Transform, Vec3};

const DEMO_FLOOR_Y: f32 = 0.0;

pub(super) fn ground_import_at_floor(
    scene: &mut Scene,
    import: &crate::SceneImport,
) -> Result<Option<Aabb>, JsValue> {
    let bounds = import.bounds_world(scene);
    let offset = ground_offset_to_floor(bounds);
    ground_import_roots(scene, import, offset)?;
    Ok(translate_bounds(bounds, offset))
}

pub(super) fn ground_import_roots(
    scene: &mut Scene,
    import: &crate::SceneImport,
    offset: Vec3,
) -> Result<(), JsValue> {
    if offset.length_squared() <= f32::EPSILON {
        return Ok(());
    }
    for root in import.roots() {
        let transform = scene
            .world_transform(*root)
            .ok_or_else(|| JsValue::from_str("import root transform missing"))?;
        scene
            .set_transform(*root, translate_transform(transform, offset))
            .map_err(|err| JsValue::from_str(&format!("ground import root failed: {err:?}")))?;
    }
    Ok(())
}

pub(super) fn ground_offset_to_floor(bounds: Option<Aabb>) -> Vec3 {
    bounds.map_or(Vec3::ZERO, |bounds| {
        Vec3::new(0.0, DEMO_FLOOR_Y - bounds.min.y, 0.0)
    })
}

pub(super) fn translate_bounds(bounds: Option<Aabb>, offset: Vec3) -> Option<Aabb> {
    bounds.map(|bounds| Aabb::new(bounds.min + offset, bounds.max + offset))
}

pub(super) fn translate_transform(transform: Transform, offset: Vec3) -> Transform {
    transform.with_translation(transform.translation + offset)
}
