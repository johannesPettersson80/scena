use crate::diagnostics::LookupError;

use super::{NodeKey, Quat, Scene, Transform, Vec3};

impl Scene {
    pub fn set_transform(
        &mut self,
        node: NodeKey,
        transform: Transform,
    ) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.transform != transform {
            node.transform = transform;
            self.structure_revision = self.structure_revision.saturating_add(1);
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn world_transform(&self, node: NodeKey) -> Option<Transform> {
        let mut chain = Vec::new();
        let mut current = node;
        loop {
            let node = self.nodes.get(current)?;
            chain.push(node.transform);
            let Some(parent) = node.parent else {
                break;
            };
            current = parent;
        }
        Some(
            chain
                .into_iter()
                .rev()
                .fold(Transform::IDENTITY, compose_transform),
        )
    }
}

pub(super) fn compose_transform(parent: Transform, child: Transform) -> Transform {
    let scaled_child_translation = Vec3::new(
        child.translation.x * parent.scale.x,
        child.translation.y * parent.scale.y,
        child.translation.z * parent.scale.z,
    );
    Transform {
        translation: add_vec3(
            parent.translation,
            rotate_vec3(parent.rotation, scaled_child_translation),
        ),
        rotation: multiply_quat(parent.rotation, child.rotation),
        scale: Vec3::new(
            parent.scale.x * child.scale.x,
            parent.scale.y * child.scale.y,
            parent.scale.z * child.scale.z,
        ),
    }
}

pub(super) fn local_transform_from_world(parent: Transform, world: Transform) -> Option<Transform> {
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
    normalize_quat(Quat::from_xyzw(left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y, left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x, left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w, left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z))
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
    Quat::from_xyzw(value.x * inverse_length, value.y * inverse_length, value.z * inverse_length, value.w * inverse_length)
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn is_invertible_scale(scale: Vec3) -> bool {
    [scale.x, scale.y, scale.z]
        .into_iter()
        .all(|component| component.is_finite() && component.abs() > f32::EPSILON)
}
