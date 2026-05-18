use crate::assets::Assets;
use crate::diagnostics::PrepareError;
use crate::scene::{Scene, Vec3};

use super::{PreparedLights, identity_matrix4, shadows};

pub(in crate::render) fn collect_dynamic_light_from_world<F>(
    scene: &Scene,
    assets: Option<&Assets<F>>,
) -> Result<[f32; 16], PrepareError> {
    if assets.is_none()
        && let Some((node, _mesh, _transform)) = scene.mesh_nodes().next()
    {
        return Err(PrepareError::AssetsRequired { node });
    }
    let origin_shift = scene.origin_shift();
    let lights = PreparedLights::from_scene(scene, origin_shift);
    let Some(to_light_dir) = lights.primary_shadow_ray_direction() else {
        return Ok(identity_matrix4());
    };
    let light_direction = Vec3::new(-to_light_dir.x, -to_light_dir.y, -to_light_dir.z);
    let shadow_projection_points =
        shadows::collect_shadow_projection_points(scene, assets, origin_shift)?;
    Ok(shadows::directional_light_view_projection_from_points(
        light_direction,
        shadow_projection_points,
    ))
}
