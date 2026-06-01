use crate::assets::{Assets, TextureHandle};
use crate::diagnostics::PrepareError;
use crate::geometry::Primitive;
use crate::scene::{Scene, Vec3};

pub(super) use self::diagnostics::{
    collect_asset_camera_visibility_diagnostics, collect_camera_projection_diagnostics,
    collect_camera_visibility_diagnostics, collect_precision_diagnostics,
};
pub(super) use self::dynamic::collect_dynamic_light_from_world;
pub(super) use self::environment::collect_environment_lighting;
#[doc(hidden)]
pub use self::environment::precompute_environment_sidecar;
pub(in crate::render) use self::environment::{
    EnvironmentLightingProfile, PreparedEnvironmentCubemap, PreparedEnvironmentLighting,
};
use self::lighting::PreparedLights;
pub(super) use self::lighting::{PreparedGpuLightUniform, collect_gpu_light_uniform};
use self::materials::validate_material_texture_handles;
use self::primitives::append_geometry_primitives;
pub(super) use self::resources::{
    PreparedLogicalResourceStats, PreparedMaterialSlot, collect_backend_material_slots,
    collect_logical_resource_stats, collect_material_texture_diagnostics,
};
use self::shadows::{collect_shadow_occluders, cpu_shadow_visibility_required};
pub(super) use self::stats::{
    PreparedDepthStats, PreparedEnvironmentStats, PreparedLightingStats,
    collect_depth_prepass_stats, collect_environment_prepare_stats, collect_lighting_stats,
};
use self::transforms::{compose_transform, identity_matrix4, prepared_primitive};
use self::types::{
    DeformationInputs, GeometryPrimitiveSource, PreparedScene, PrimitiveBakeParams, PrimitiveSinks,
    TransparentPrimitive,
};
use super::{RasterTarget, camera::CameraProjection};

mod cpu_bake;
mod diagnostics;
mod dynamic;
mod environment;
mod environment_prefilter;
mod labels;
mod material_batch;
pub(in crate::render) use self::material_batch::compute_material_batch_plan;
mod lighting;
mod materials;
mod pbr_contract;
mod primitives;
mod resources;
mod shadows;
mod stats;
mod strokes;
mod tangents;
#[cfg(test)]
mod tests;
pub(super) mod transforms;
mod types;

pub(super) fn collect_prepared_primitives<F>(
    target: RasterTarget,
    scene: &Scene,
    assets: Option<&Assets<F>>,
    camera_projection: Option<&CameraProjection>,
    backend_sampled_base_color_textures: &[TextureHandle],
    backend_material_slots: &[crate::assets::MaterialHandle],
    environment_lighting: PreparedEnvironmentLighting,
) -> Result<PreparedScene, PrepareError> {
    if let Some(model_node) = scene.model_nodes().next() {
        return Err(PrepareError::UnsupportedModelNode { node: model_node });
    }

    let origin_shift = scene.origin_shift();
    let lights = PreparedLights::from_scene(scene, origin_shift);
    let needs_cpu_shadow_visibility = cpu_shadow_visibility_required(scene, backend_material_slots);
    let shadow_occluders = if needs_cpu_shadow_visibility {
        collect_shadow_occluders(scene, assets, origin_shift)?
    } else {
        Vec::new()
    };
    let shadow_projection_points = if needs_cpu_shadow_visibility {
        None
    } else {
        Some(shadows::collect_shadow_projection_points(
            scene,
            assets,
            origin_shift,
        )?)
    };
    let mut primitives: Vec<Primitive> = scene
        .renderables()
        .flat_map(|(renderable, transform)| {
            renderable
                .primitives()
                .iter()
                .map(move |primitive| prepared_primitive(primitive, transform, origin_shift))
        })
        .collect();
    labels::append_label_primitives(scene, origin_shift, &mut primitives);
    let mut transparent_primitives = Vec::new();

    for (node, mesh, transform) in scene.mesh_nodes() {
        let Some(assets) = assets else {
            return Err(PrepareError::AssetsRequired { node });
        };
        let geometry = assets
            .geometry(mesh.geometry())
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: mesh.geometry(),
            })?;
        let material = assets
            .material(mesh.material())
            .ok_or(PrepareError::MaterialNotFound {
                node,
                material: mesh.material(),
            })?;
        validate_material_texture_handles(node, mesh.material(), &material, assets)?;
        append_geometry_primitives(
            GeometryPrimitiveSource {
                node,
                material_handle: mesh.material(),
                geometry: &geometry,
                material: &material,
                assets,
                tint: scene.node_tint(node).unwrap_or(None),
            },
            DeformationInputs {
                morph_weights: scene.morph_weights(node),
                skin_matrices: scene.skin_matrices(node).as_deref(),
            },
            PrimitiveBakeParams {
                target,
                transform,
                origin_shift,
                lights: &lights,
                shadow_occluders: &shadow_occluders,
                camera_projection,
                backend_sampled_base_color_textures,
                backend_material_slots,
                environment_lighting: environment_lighting.clone(),
            },
            PrimitiveSinks {
                primitives: &mut primitives,
                transparent_primitives: &mut transparent_primitives,
            },
        )?;
    }

    for (node, instance_set, node_transform) in scene.instance_set_nodes() {
        let Some(assets) = assets else {
            return Err(PrepareError::AssetsRequired { node });
        };
        let geometry =
            assets
                .geometry(instance_set.geometry())
                .ok_or(PrepareError::GeometryNotFound {
                    node,
                    geometry: instance_set.geometry(),
                })?;
        let material =
            assets
                .material(instance_set.material())
                .ok_or(PrepareError::MaterialNotFound {
                    node,
                    material: instance_set.material(),
                })?;
        validate_material_texture_handles(node, instance_set.material(), &material, assets)?;

        for instance in instance_set.instances() {
            append_geometry_primitives(
                GeometryPrimitiveSource {
                    node,
                    material_handle: instance_set.material(),
                    geometry: &geometry,
                    material: &material,
                    assets,
                    tint: scene.node_tint(node).unwrap_or(None),
                },
                DeformationInputs::default(),
                PrimitiveBakeParams {
                    target,
                    transform: compose_transform(node_transform, instance.transform()),
                    origin_shift,
                    lights: &lights,
                    shadow_occluders: &shadow_occluders,
                    camera_projection,
                    backend_sampled_base_color_textures,
                    backend_material_slots,
                    environment_lighting: environment_lighting.clone(),
                },
                PrimitiveSinks {
                    primitives: &mut primitives,
                    transparent_primitives: &mut transparent_primitives,
                },
            )?;
        }
    }

    // Descending depth: larger local-space z is treated as farther for the M1 foundation.
    transparent_primitives
        .sort_by(|left: &TransparentPrimitive, right| right.depth.total_cmp(&left.depth));
    primitives.extend(
        transparent_primitives
            .into_iter()
            .map(|transparent| transparent.primitive),
    );

    let light_from_world = lights
        .primary_shadow_ray_direction()
        .map(|to_light_dir| {
            // primary_shadow_ray_direction returns the vector pointing toward
            // the light; the shadow projection wants the direction the light
            // travels (forward), so negate.
            let light_direction = Vec3::new(-to_light_dir.x, -to_light_dir.y, -to_light_dir.z);
            match shadow_projection_points.as_ref() {
                Some(points) => shadows::directional_light_view_projection_from_points(
                    light_direction,
                    points.iter().copied(),
                ),
                None => {
                    shadows::directional_light_view_projection(light_direction, &shadow_occluders)
                }
            }
        })
        .unwrap_or_else(identity_matrix4);

    Ok(PreparedScene {
        primitives,
        light_from_world,
    })
}
