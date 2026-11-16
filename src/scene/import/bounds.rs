use crate::geometry::Aabb;
use crate::scene::view_math::union_aabb;

pub(super) use crate::scene::view_math::transform_aabb;

pub(super) fn union_optional(current: Option<Aabb>, next: Aabb) -> Aabb {
    match current {
        Some(current) => union_aabb(current, next),
        None => next,
    }
}
