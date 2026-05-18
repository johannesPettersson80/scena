use crate::{Aabb, Scene, SceneImport};

pub(super) fn combined_import_bounds(
    scene: &Scene,
    left: &SceneImport,
    right: &SceneImport,
) -> Option<Aabb> {
    match (left.bounds_world(scene), right.bounds_world(scene)) {
        (Some(left), Some(right)) => Some(left.union(right)),
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}

pub(super) fn union_optional_bounds(left: Option<Aabb>, right: Option<Aabb>) -> Option<Aabb> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.union(right)),
        (Some(bounds), None) | (None, Some(bounds)) => Some(bounds),
        (None, None) => None,
    }
}
