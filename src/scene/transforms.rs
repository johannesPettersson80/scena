use crate::diagnostics::LookupError;

use super::{Mat4, NodeKey, Quat, Scene, Transform, Vec3, WorldTransform};

impl Scene {
    pub fn set_transform(
        &mut self,
        node: NodeKey,
        transform: Transform,
    ) -> Result<(), LookupError> {
        let transform = validate_transform(transform)?;
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.transform != transform {
            node.transform = transform;
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn set_transforms(
        &mut self,
        transforms: &[(NodeKey, Transform)],
    ) -> Result<(), LookupError> {
        for (node, transform) in transforms {
            if !self.nodes.contains_key(*node) {
                return Err(LookupError::NodeNotFound(*node));
            }
            validate_transform(*transform)?;
        }

        let mut changed = false;
        for (node, transform) in transforms {
            let node = self
                .nodes
                .get_mut(*node)
                .expect("batch transform preflight verified node existence");
            if node.transform != *transform {
                node.transform = *transform;
                changed = true;
            }
        }

        if changed {
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }

    /// Returns the node's world transform as translation, rotation, and scale.
    ///
    /// This is the TRS decomposition of [`Self::world_matrix`]. It is exact
    /// whenever the hierarchy cannot shear: every ancestor has uniform scale,
    /// or no node below a non-uniformly scaled ancestor is rotated. Those
    /// hierarchies return the composed transform unchanged.
    ///
    /// A rotated node under a non-uniformly scaled ancestor is sheared, which
    /// translation/rotation/scale cannot represent. For such a node the
    /// translation is still exact, the rotation is the nearest orthonormal
    /// basis that keeps the node's Z axis, and the scale is the length of each
    /// basis axis (negative on X when the hierarchy mirrors). Use
    /// [`Self::world_matrix`] when placing geometry or converting points.
    pub fn world_transform(&self, node: NodeKey) -> Option<Transform> {
        self.resolved_world(node).map(WorldTransform::trs)
    }

    /// Returns the exact world transform of a node as a column-major matrix.
    ///
    /// The matrix is the product of the local transforms from the scene root
    /// down to `node`, so it carries the shear a non-uniformly scaled or
    /// mirrored ancestor gives a rotated descendant. Both render backends,
    /// asset-aware picking, bounds, and framing place the node with this
    /// matrix: `world_matrix(node)?.transform_point3(local)` is where a point
    /// in the node's local space is drawn.
    ///
    /// Returns `None` when `node` is not in this scene.
    pub fn world_matrix(&self, node: NodeKey) -> Option<Mat4> {
        self.resolved_world(node).map(WorldTransform::to_mat4)
    }

    /// The resolved world placement of a node: exact matrix plus TRS view.
    pub(crate) fn resolved_world(&self, node: NodeKey) -> Option<WorldTransform> {
        self.resolved_node_state(node)
            .map(|resolved| resolved.world)
    }
}

pub(super) fn validate_transform(transform: Transform) -> Result<Transform, LookupError> {
    if transform.translation.is_finite()
        && transform.rotation.is_finite()
        && transform.scale.is_finite()
    {
        Ok(transform)
    } else {
        Err(LookupError::InvalidTransform {
            reason: "translation, rotation, and scale must contain only finite values",
        })
    }
}

/// Composes two TRS frames that are known not to shear, such as the rigid
/// connector frames the connection solver mates. Node hierarchies compose
/// through [`WorldTransform::child`] instead.
pub(super) fn compose_transform(parent: Transform, child: Transform) -> Transform {
    Transform::compose(parent, child)
}

/// The local transform that gives a node the world transform `world` under
/// `parent`, or `None` when the parent placement cannot be inverted.
///
/// Under a sheared parent the exact local transform is `parent^-1 * world`,
/// which is itself a matrix; its TRS decomposition is returned and is exact
/// whenever that product has no shear.
pub(super) fn local_transform_from_world(
    parent: WorldTransform,
    world: Transform,
) -> Option<Transform> {
    if let Some(parent) = parent.exact_trs() {
        return local_transform_from_world_trs(parent, world);
    }
    let local_from_world = parent.inverse()?;
    let local = local_from_world.matrix() * WorldTransform::from(world).matrix();
    Some(WorldTransform::from_matrix(local).trs())
}

/// The local position that puts a node origin at `world_origin` under
/// `parent`. Exact for every parent, sheared or not.
pub(super) fn local_point_from_world(parent: WorldTransform, world_origin: Vec3) -> Option<Vec3> {
    parent
        .inverse()
        .map(|local_from_world| local_from_world.transform_point(world_origin))
}

pub(super) fn local_transform_from_world_trs(
    parent: Transform,
    world: Transform,
) -> Option<Transform> {
    if !is_invertible_scale(parent.scale) {
        return None;
    }
    let inverse_parent_rotation = inverse_unit_quat(parent.rotation);
    let local_translation = rotate_vec3(
        inverse_parent_rotation,
        subtract_vec3(world.translation, parent.translation),
    );
    Some(Transform {
        translation: Vec3::new(
            local_translation.x / parent.scale.x,
            local_translation.y / parent.scale.y,
            local_translation.z / parent.scale.z,
        ),
        rotation: multiply_quat(inverse_parent_rotation, world.rotation),
        scale: Vec3::new(
            world.scale.x / parent.scale.x,
            world.scale.y / parent.scale.y,
            world.scale.z / parent.scale.z,
        ),
    })
}

pub(super) fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}

pub(super) fn multiply_quat(left: Quat, right: Quat) -> Quat {
    normalize_quat(Quat::from_xyzw(
        left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    ))
}

fn inverse_unit_quat(rotation: Quat) -> Quat {
    let normalized = normalize_quat(rotation);
    Quat::from_xyzw(-normalized.x, -normalized.y, -normalized.z, normalized.w)
}

fn normalize_quat(value: Quat) -> Quat {
    let length_squared =
        value.x * value.x + value.y * value.y + value.z * value.z + value.w * value.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return Quat::IDENTITY;
    }
    let inverse_length = length_squared.sqrt().recip();
    Quat::from_xyzw(
        value.x * inverse_length,
        value.y * inverse_length,
        value.z * inverse_length,
        value.w * inverse_length,
    )
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn is_invertible_scale(scale: Vec3) -> bool {
    [scale.x, scale.y, scale.z]
        .into_iter()
        .all(|component| component.is_finite() && component.abs() > f32::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_transform_marks_transform_dirty_without_structure_change() {
        let mut scene = Scene::new();
        let structure_before = scene.structure_revision;
        let transform_before = scene.transform_revision;

        scene
            .set_transform(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
            .expect("root transform updates");

        assert_eq!(
            scene.structure_revision, structure_before,
            "moving an existing node must not invalidate static scene structure"
        );
        assert!(
            scene.transform_revision > transform_before,
            "moving an existing node must mark transform-dependent prepared data dirty"
        );
    }
}
