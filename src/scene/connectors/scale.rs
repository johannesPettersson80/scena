use super::super::transforms::rotate_vec3;
use super::{Transform, Vec3};

pub(super) fn preserve_source_scale(
    mut desired_source_world: Transform,
    source_scale: Vec3,
    source_connector: Transform,
    target_connector_translation: Vec3,
) -> Transform {
    desired_source_world.scale = source_scale;
    let scaled_connector_translation = Vec3::new(
        source_connector.translation.x * source_scale.x,
        source_connector.translation.y * source_scale.y,
        source_connector.translation.z * source_scale.z,
    );
    let rotated_connector_translation =
        rotate_vec3(desired_source_world.rotation, scaled_connector_translation);
    desired_source_world.translation =
        subtract_vec3(target_connector_translation, rotated_connector_translation);
    desired_source_world
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}
