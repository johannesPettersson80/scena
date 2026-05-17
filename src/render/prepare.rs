use crate::assets::{Assets, TextureHandle};
use crate::diagnostics::PrepareError;
use crate::geometry::{
    GeometryDesc, GeometryTopology, Primitive, PrimitiveVertexAttributes, Vertex,
};
use crate::material::{MaterialDesc, MaterialKind};
use crate::scene::{NodeKey, Scene, Vec3};

use self::cpu_bake::{
    CpuBakeCorner, baked_shadow_visibility, cpu_texture_subdivisions, push_material_pass_primitive,
    subdivided_cpu_corners,
};
pub(super) use self::diagnostics::{
    collect_asset_camera_visibility_diagnostics, collect_camera_projection_diagnostics,
    collect_camera_visibility_diagnostics, collect_precision_diagnostics,
};
pub(super) use self::environment::collect_environment_lighting;
pub(in crate::render) use self::environment::{
    EnvironmentLightingProfile, PreparedEnvironmentCubemap, PreparedEnvironmentLighting,
};
use self::lighting::{MaterialShadingInput, PreparedLights, material_color};
pub(super) use self::lighting::{PreparedGpuLightUniform, collect_gpu_light_uniform};
use self::materials::{
    base_color_texture_sample, emissive_texture_sample, material_pass,
    metallic_roughness_texture_sample, multiply_color, normal_texture_sample,
    occlusion_texture_sample, render_material_slot, validate_material_texture_handles,
};
pub(super) use self::resources::{
    PreparedLogicalResourceStats, PreparedMaterialSlot, collect_backend_material_slots,
    collect_logical_resource_stats, collect_material_texture_diagnostics,
};
use self::shadows::collect_shadow_occluders;
pub(super) use self::stats::{
    PreparedDepthStats, PreparedEnvironmentStats, PreparedLightingStats,
    collect_depth_prepass_stats, collect_environment_prepare_stats, collect_lighting_stats,
};
use self::tangents::{accumulate_vertex_tangents, authored_vertex_tangents};
use self::transforms::{
    compose_transform, prepared_primitive, transform_normal, transform_position,
};
use self::types::{DeformationInputs, PrimitiveBakeParams, PrimitiveSinks, TransparentPrimitive};
use super::{RasterTarget, camera::CameraProjection};

mod cpu_bake;
mod diagnostics;
mod environment;
mod environment_prefilter;
mod labels;
mod material_batch;
pub(in crate::render) use self::material_batch::compute_material_batch_plan;
mod lighting;
mod materials;
mod pbr_contract;
mod resources;
mod shadows;
mod stats;
mod strokes;
mod tangents;
pub(super) mod transforms;
mod types;

/// Collected primitives plus the directional-light view-projection matrix
/// derived from the shadow occluders used during preparation. The matrix is
/// the orthographic transform that maps world-space to light-clip-space and
/// is consumed by the GPU shadow caster pass + fragment-shader shadow
/// sampling. Phase 1B foundation for scena-wgpu-architect F3.
pub(super) struct PreparedScene {
    pub(super) primitives: Vec<Primitive>,
    pub(super) light_from_world: [f32; 16],
}

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

fn cpu_shadow_visibility_required(
    scene: &Scene,
    backend_material_slots: &[crate::assets::MaterialHandle],
) -> bool {
    for (_node, mesh, _transform) in scene.mesh_nodes() {
        if render_material_slot(mesh.material(), backend_material_slots) == 0 {
            return true;
        }
    }
    for (_node, instance_set, _transform) in scene.instance_set_nodes() {
        if render_material_slot(instance_set.material(), backend_material_slots) == 0 {
            return true;
        }
    }
    false
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

struct GeometryPrimitiveSource<'a, F> {
    node: NodeKey,
    material_handle: crate::assets::MaterialHandle,
    geometry: &'a GeometryDesc,
    material: &'a MaterialDesc,
    assets: &'a Assets<F>,
}

fn append_geometry_primitives<F>(
    source: GeometryPrimitiveSource<'_, F>,
    deformation: DeformationInputs<'_>,
    params: PrimitiveBakeParams<'_>,
    sinks: PrimitiveSinks<'_>,
) -> Result<(), PrepareError> {
    match source.geometry.topology() {
        GeometryTopology::Triangles => {
            append_triangle_primitives(source, deformation, params, sinks)
        }
        GeometryTopology::Lines => strokes::append_line_primitives(
            source.node,
            source.geometry,
            source.material,
            params.target,
            sinks.primitives,
        ),
    }
}

fn append_triangle_primitives<F>(
    source: GeometryPrimitiveSource<'_, F>,
    deformation: DeformationInputs<'_>,
    params: PrimitiveBakeParams<'_>,
    mut sinks: PrimitiveSinks<'_>,
) -> Result<(), PrepareError> {
    match source.material.kind() {
        MaterialKind::Unlit | MaterialKind::PbrMetallicRoughness => {}
        MaterialKind::Line => {
            return Err(PrepareError::UnsupportedMaterialKind {
                node: source.node,
                kind: source.material.kind(),
            });
        }
        MaterialKind::Wireframe => {
            return strokes::append_wireframe_primitives(
                source.node,
                source.geometry,
                source.material,
                params.target,
                sinks.primitives,
            );
        }
        MaterialKind::Edge => {
            return strokes::append_edge_primitives(
                source.node,
                source.geometry,
                source.material,
                params.target,
                sinks.primitives,
            );
        }
    }

    let material_pass = material_pass(source.node, source.material)?;
    let morphed_vertices = deformation
        .morph_weights
        .and_then(|weights| source.geometry.morphed_vertices(weights));
    let base_vertices = morphed_vertices
        .as_deref()
        .unwrap_or_else(|| source.geometry.vertices());
    let skinned_vertices = match deformation.skin_matrices {
        Some(matrices) => source
            .geometry
            .skinned_vertices(base_vertices, matrices)
            .map_err(|error| PrepareError::InvalidSkinGeometry {
                node: source.node,
                reason: format!("{error:?}"),
            })?,
        None if source.geometry.skin().is_some() => {
            return Err(PrepareError::InvalidSkinGeometry {
                node: source.node,
                reason: "skinned geometry is missing a scene skin binding".to_string(),
            });
        }
        None => None,
    };
    let vertices = skinned_vertices.as_deref().unwrap_or(base_vertices);
    let tex_coords0 = source.geometry.tex_coords0();
    let vertex_tangents =
        authored_vertex_tangents(source.geometry.tangents(), vertices, params.transform)
            .unwrap_or_else(|| {
                accumulate_vertex_tangents(
                    vertices,
                    source.geometry.indices(),
                    tex_coords0,
                    params.transform,
                    params.origin_shift,
                )
            });

    for triangle in source.geometry.indices().chunks_exact(3) {
        let position_a = transform_position(
            vertices[triangle[0] as usize].position,
            params.transform,
            params.origin_shift,
        );
        let position_b = transform_position(
            vertices[triangle[1] as usize].position,
            params.transform,
            params.origin_shift,
        );
        let position_c = transform_position(
            vertices[triangle[2] as usize].position,
            params.transform,
            params.origin_shift,
        );
        let geometric_normal_a =
            transform_normal(vertices[triangle[0] as usize].normal, params.transform);
        let geometric_normal_b =
            transform_normal(vertices[triangle[1] as usize].normal, params.transform);
        let geometric_normal_c =
            transform_normal(vertices[triangle[2] as usize].normal, params.transform);
        let vertex_colors = source.geometry.vertex_colors();
        let uv_a = tex_coords0[triangle[0] as usize];
        let uv_b = tex_coords0[triangle[1] as usize];
        let uv_c = tex_coords0[triangle[2] as usize];
        let tangent_a = vertex_tangents[triangle[0] as usize];
        let tangent_b = vertex_tangents[triangle[1] as usize];
        let tangent_c = vertex_tangents[triangle[2] as usize];
        let render_material_slot =
            render_material_slot(source.material_handle, params.backend_material_slots);
        let backend_shaded_material = render_material_slot != 0;
        let shadow_visibility_a = baked_shadow_visibility(
            position_a,
            params.lights,
            params.shadow_occluders,
            backend_shaded_material,
        );
        let shadow_visibility_b = baked_shadow_visibility(
            position_b,
            params.lights,
            params.shadow_occluders,
            backend_shaded_material,
        );
        let shadow_visibility_c = baked_shadow_visibility(
            position_c,
            params.lights,
            params.shadow_occluders,
            backend_shaded_material,
        );
        let shade_vertex = |corner: CpuBakeCorner| {
            if backend_shaded_material {
                corner.vertex_color
            } else {
                let normal = normal_texture_sample(
                    source.assets,
                    source.material,
                    corner.uv,
                    corner.geometric_normal,
                );
                multiply_color(
                    material_color(
                        source.material,
                        params.lights,
                        &MaterialShadingInput {
                            position: corner.position,
                            normal,
                            camera_position: params
                                .camera_projection
                                .map(CameraProjection::camera_position),
                            base_color_texture: base_color_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                                params.backend_sampled_base_color_textures,
                            ),
                            metallic_roughness_texture: metallic_roughness_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            occlusion_texture: occlusion_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            emissive_texture: emissive_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            environment: params.environment_lighting.clone(),
                            directional_shadow_factor: corner.shadow_visibility,
                        },
                    ),
                    corner.vertex_color,
                )
            }
        };
        let corners = [
            CpuBakeCorner {
                position: position_a,
                geometric_normal: geometric_normal_a,
                uv: uv_a,
                tangent: tangent_a.tangent,
                tangent_handedness: tangent_a.handedness,
                vertex_color: vertex_colors[triangle[0] as usize],
                shadow_visibility: shadow_visibility_a,
            },
            CpuBakeCorner {
                position: position_b,
                geometric_normal: geometric_normal_b,
                uv: uv_b,
                tangent: tangent_b.tangent,
                tangent_handedness: tangent_b.handedness,
                vertex_color: vertex_colors[triangle[1] as usize],
                shadow_visibility: shadow_visibility_b,
            },
            CpuBakeCorner {
                position: position_c,
                geometric_normal: geometric_normal_c,
                uv: uv_c,
                tangent: tangent_c.tangent,
                tangent_handedness: tangent_c.handedness,
                vertex_color: vertex_colors[triangle[2] as usize],
                shadow_visibility: shadow_visibility_c,
            },
        ];
        let subdivisions = cpu_texture_subdivisions(source.material, backend_shaded_material);
        for sub_triangle in subdivided_cpu_corners(corners, subdivisions) {
            let primitive = Primitive::triangle_with_attributes(
                sub_triangle.map(|corner| Vertex {
                    position: corner.position,
                    color: shade_vertex(corner),
                }),
                sub_triangle.map(|corner| PrimitiveVertexAttributes {
                    normal: corner.geometric_normal,
                    tex_coord0: corner.uv,
                    tangent: corner.tangent,
                    tangent_handedness: corner.tangent_handedness,
                    shadow_visibility: corner.shadow_visibility,
                }),
            )
            .with_render_material_slot(render_material_slot);
            push_material_pass_primitive(
                primitive,
                material_pass,
                &mut sinks,
                params.camera_projection,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_shaded_materials_skip_cpu_shadow_visibility_bake() {
        let scene = Scene::new();
        let lights = PreparedLights::from_scene(&scene, Vec3::ZERO);
        let position = Vec3::new(0.0, 0.0, 0.0);

        assert_eq!(baked_shadow_visibility(position, &lights, &[], true), 1.0);
        assert_eq!(baked_shadow_visibility(position, &lights, &[], false), 1.0);
    }
}
