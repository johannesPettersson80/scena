use crate::assets::{Assets, GeometryHandle, TextureHandle};
use crate::diagnostics::PrepareError;
use crate::geometry::Aabb;
use crate::scene::{Scene, Vec3};

use self::auto_instance::append_auto_instanced_mesh_groups;
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
pub(super) use self::lighting::{
    PreparedGpuLightUniform, TiledLightAssignment, collect_gpu_light_uniform,
    collect_gpu_tiled_light_assignment, gpu_tiled_light_assignment_required,
};
use self::materials::validate_material_texture_handles;
use self::primitives::append_geometry_primitives;
pub(in crate::render) use self::primitives::draw_uniform_tint;
#[cfg(test)]
pub(in crate::render) use self::resources::PreparedMaterialTexture;
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

mod auto_instance;
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
mod particles;
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
pub(in crate::render) use types::{
    PreparedInstanceRecord, PreparedInstanceSet, PreparedLabelAtlas, PreparedLabelQuad,
    PreparedMaterialReflection, PreparedPrimitive, PreparedStrokeSegment,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn collect_prepared_primitives<F>(
    target: RasterTarget,
    screen_space_scale: f32,
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
    let needs_cpu_shadow_visibility =
        cpu_shadow_visibility_required(scene, backend_material_slots) || lights.has_area_lights();
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
    let mut primitives: Vec<PreparedPrimitive> = scene
        .renderable_nodes()
        .flat_map(|(node, renderable, transform)| {
            renderable.primitives().iter().map(move |primitive| {
                PreparedPrimitive::new(
                    prepared_primitive(primitive, transform, origin_shift),
                    Some(node),
                    draw_uniform_tint(scene.node_tint(node).unwrap_or(None)),
                )
            })
        })
        .collect();
    let mut transparent_primitives = Vec::new();
    let labels = labels::prepare_label_atlas(target, scene, camera_projection, origin_shift);
    particles::append_particle_primitives(scene, camera_projection, origin_shift, &mut primitives);
    let mut strokes = Vec::new();
    let mut instances = Vec::new();
    let gpu_instance_path = matches!(
        target.backend,
        crate::diagnostics::Backend::HeadlessGpu
            | crate::diagnostics::Backend::WebGpu
            | crate::diagnostics::Backend::WebGl2
    );
    let auto_instanced_nodes = if gpu_instance_path {
        append_auto_instanced_mesh_groups(
            target,
            screen_space_scale,
            scene,
            assets,
            origin_shift,
            &lights,
            &shadow_occluders,
            camera_projection,
            backend_sampled_base_color_textures,
            backend_material_slots,
            environment_lighting.clone(),
            &mut instances,
            &mut strokes,
        )?
    } else {
        Vec::new()
    };

    for (node, mesh, transform) in scene.mesh_nodes() {
        if auto_instanced_nodes.contains(&node) {
            continue;
        }
        let Some(assets) = assets else {
            return Err(PrepareError::AssetsRequired { node });
        };
        let base_geometry =
            assets
                .geometry(mesh.geometry())
                .ok_or(PrepareError::GeometryNotFound {
                    node,
                    geometry: mesh.geometry(),
                })?;
        let geometry_handle = select_mesh_lod_geometry(
            scene,
            node,
            mesh.geometry(),
            base_geometry.bounds(),
            transform,
            camera_projection,
        );
        let geometry = assets
            .geometry(geometry_handle)
            .ok_or(PrepareError::GeometryNotFound {
                node,
                geometry: geometry_handle,
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
                screen_space_scale,
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
                strokes: &mut strokes,
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

        let can_use_gpu_instance_path = gpu_instance_path
            && !matches!(
                material.kind(),
                crate::material::MaterialKind::Line
                    | crate::material::MaterialKind::Wireframe
                    | crate::material::MaterialKind::Edge
            );
        if can_use_gpu_instance_path {
            let mut retained = Vec::new();
            let mut instance_strokes = Vec::new();
            let mut transparent = Vec::new();
            append_geometry_primitives(
                GeometryPrimitiveSource {
                    node,
                    material_handle: instance_set.material(),
                    geometry: &geometry,
                    material: &material,
                    assets,
                    tint: None,
                },
                DeformationInputs::default(),
                PrimitiveBakeParams {
                    target,
                    screen_space_scale,
                    transform: node_transform,
                    origin_shift,
                    lights: &lights,
                    shadow_occluders: &shadow_occluders,
                    camera_projection,
                    backend_sampled_base_color_textures,
                    backend_material_slots,
                    environment_lighting: environment_lighting.clone(),
                },
                PrimitiveSinks {
                    primitives: &mut retained,
                    strokes: &mut instance_strokes,
                    transparent_primitives: &mut transparent,
                },
            )?;
            retained.extend(
                transparent
                    .into_iter()
                    .map(|transparent| transparent.primitive),
            );
            strokes.extend(instance_strokes);
            if !retained.is_empty() {
                let Some(source_set) = instance_set_key_for_node(scene, node) else {
                    continue;
                };
                instances.push(PreparedInstanceSet::new(
                    node,
                    source_set,
                    retained,
                    collect_prepared_instance_records(scene, node, instance_set),
                ));
            }
        } else {
            for instance in instance_set
                .instances()
                .filter(|instance| instance.visible())
            {
                append_geometry_primitives(
                    GeometryPrimitiveSource {
                        node,
                        material_handle: instance_set.material(),
                        geometry: &geometry,
                        material: &material,
                        assets,
                        tint: multiply_tint(scene.node_tint(node).unwrap_or(None), instance.tint()),
                    },
                    DeformationInputs::default(),
                    PrimitiveBakeParams {
                        target,
                        screen_space_scale,
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
                        strokes: &mut strokes,
                        transparent_primitives: &mut transparent_primitives,
                    },
                )?;
            }
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
        strokes,
        labels,
        instances,
        light_from_world,
    })
}

fn select_mesh_lod_geometry(
    scene: &Scene,
    node: crate::scene::NodeKey,
    base_geometry: GeometryHandle,
    local_bounds: Aabb,
    transform: crate::scene::Transform,
    camera_projection: Option<&CameraProjection>,
) -> GeometryHandle {
    let Some(levels) = scene.mesh_lods(node) else {
        return base_geometry;
    };
    let Some(fraction) =
        projected_bounds_screen_fraction(local_bounds, transform, camera_projection)
    else {
        return base_geometry;
    };
    levels
        .iter()
        .find(|level| fraction <= level.max_screen_fraction())
        .map_or(base_geometry, |level| level.geometry())
}

fn projected_bounds_screen_fraction(
    local_bounds: Aabb,
    transform: crate::scene::Transform,
    camera_projection: Option<&CameraProjection>,
) -> Option<f32> {
    let projection = camera_projection?;
    let bounds = crate::scene::view_math::transform_aabb(local_bounds, transform);
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut projected_any = false;
    for corner in aabb_corners(bounds) {
        let Some(projected) = projection.project(corner) else {
            continue;
        };
        min_x = min_x.min(projected.ndc_x);
        min_y = min_y.min(projected.ndc_y);
        max_x = max_x.max(projected.ndc_x);
        max_y = max_y.max(projected.ndc_y);
        projected_any = true;
    }
    if !projected_any {
        return None;
    }
    let width_fraction = ((max_x - min_x).abs() * 0.5).clamp(0.0, 1.0);
    let height_fraction = ((max_y - min_y).abs() * 0.5).clamp(0.0, 1.0);
    Some(width_fraction.max(height_fraction))
}

fn aabb_corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
}

fn collect_prepared_instance_records(
    scene: &Scene,
    node: crate::scene::NodeKey,
    instance_set: &crate::scene::InstanceSet,
) -> Vec<PreparedInstanceRecord> {
    let node_tint = scene.node_tint(node).unwrap_or(None);
    instance_set
        .instances()
        .filter(|instance| instance.visible())
        .map(|instance| {
            PreparedInstanceRecord::new(
                instance.id(),
                transforms::world_from_model_matrix(instance.transform(), Vec3::ZERO),
                transforms::normal_from_model_matrix(instance.transform()),
                draw_uniform_tint(multiply_tint(node_tint, instance.tint())),
            )
        })
        .collect()
}

fn instance_set_key_for_node(
    scene: &Scene,
    node: crate::scene::NodeKey,
) -> Option<crate::scene::InstanceSetKey> {
    let crate::scene::NodeKind::InstanceSet(instance_set) = *scene.node(node)?.kind() else {
        return None;
    };
    Some(instance_set)
}

fn multiply_tint(
    node_tint: Option<crate::material::Color>,
    instance_tint: Option<crate::material::Color>,
) -> Option<crate::material::Color> {
    match (node_tint, instance_tint) {
        (Some(left), Some(right)) => Some(crate::material::Color::from_linear_rgba(
            left.r * right.r,
            left.g * right.g,
            left.b * right.b,
            left.a * right.a,
        )),
        (Some(tint), None) | (None, Some(tint)) => Some(tint),
        (None, None) => None,
    }
}
