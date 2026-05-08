use crate::assets::{Assets, TextureHandle};
use crate::diagnostics::PrepareError;
use crate::geometry::{
    GeometryDesc, GeometryTopology, Primitive, PrimitiveVertexAttributes, Vertex,
};
use crate::material::{MaterialDesc, MaterialKind};
use crate::scene::{NodeKey, Scene};

pub(super) use self::diagnostics::{
    collect_asset_camera_visibility_diagnostics, collect_camera_projection_diagnostics,
    collect_camera_visibility_diagnostics, collect_precision_diagnostics,
};
use self::environment::PreparedEnvironmentLighting;
pub(super) use self::environment::collect_environment_lighting;
use self::lighting::{MaterialShadingInput, PreparedLights, material_color};
pub(super) use self::lighting::{PreparedGpuLightUniform, collect_gpu_light_uniform};
use self::materials::{
    MaterialPass, base_color_texture_sample, emissive_texture_sample, material_pass,
    metallic_roughness_texture_sample, multiply_color, normal_texture_sample,
    occlusion_texture_sample, render_material_slot, validate_material_texture_handles,
};
pub(super) use self::resources::{
    PreparedMaterialSlot, collect_backend_material_slots, collect_logical_resource_stats,
};
use self::shadows::{collect_shadow_occluders, directional_shadow_factor};
pub(super) use self::stats::{
    PreparedDepthStats, PreparedLightingStats, collect_depth_prepass_stats,
    collect_environment_prepare_stats, collect_lighting_stats,
};
use self::tangents::{accumulate_vertex_tangents, authored_vertex_tangents};
use self::transforms::{
    compose_transform, transform_normal, transform_position, transform_primitive,
};
use self::types::{DeformationInputs, PrimitiveBakeParams, PrimitiveSinks, TransparentPrimitive};
use super::{RasterTarget, camera::CameraProjection};

mod diagnostics;
mod environment;
mod labels;
mod lighting;
mod materials;
mod resources;
mod shadows;
mod stats;
mod strokes;
mod tangents;
mod transforms;
mod types;

pub(super) fn collect_prepared_primitives<F>(
    target: RasterTarget,
    scene: &Scene,
    assets: Option<&Assets<F>>,
    camera_projection: Option<&CameraProjection>,
    backend_sampled_base_color_textures: &[TextureHandle],
    backend_material_slots: &[crate::assets::MaterialHandle],
    environment_lighting: PreparedEnvironmentLighting,
) -> Result<Vec<Primitive>, PrepareError> {
    if let Some(model_node) = scene.model_nodes().next() {
        return Err(PrepareError::UnsupportedModelNode { node: model_node });
    }

    let origin_shift = scene.origin_shift();
    let lights = PreparedLights::from_scene(scene, origin_shift);
    let shadow_occluders = collect_shadow_occluders(scene, assets, origin_shift)?;
    let mut primitives: Vec<Primitive> = scene
        .renderables()
        .flat_map(|(renderable, transform)| {
            renderable
                .primitives()
                .iter()
                .map(move |primitive| transform_primitive(primitive, transform, origin_shift))
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
                environment_lighting,
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
                    environment_lighting,
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

    Ok(primitives)
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
    sinks: PrimitiveSinks<'_>,
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
        let shadow_visibility_a =
            directional_shadow_factor(position_a, params.lights, params.shadow_occluders);
        let shadow_visibility_b =
            directional_shadow_factor(position_b, params.lights, params.shadow_occluders);
        let shadow_visibility_c =
            directional_shadow_factor(position_c, params.lights, params.shadow_occluders);
        let shaded_normal_a =
            normal_texture_sample(source.assets, source.material, uv_a, geometric_normal_a);
        let shaded_normal_b =
            normal_texture_sample(source.assets, source.material, uv_b, geometric_normal_b);
        let shaded_normal_c =
            normal_texture_sample(source.assets, source.material, uv_c, geometric_normal_c);
        let render_material_slot =
            render_material_slot(source.material_handle, params.backend_material_slots);
        let backend_shaded_material = render_material_slot != 0;
        let shade_vertex = |position, normal, uv, vertex_color, shadow_visibility| {
            if backend_shaded_material {
                vertex_color
            } else {
                multiply_color(
                    material_color(
                        source.material,
                        params.lights,
                        MaterialShadingInput {
                            position,
                            normal,
                            camera_position: params
                                .camera_projection
                                .map(CameraProjection::camera_position),
                            base_color_texture: base_color_texture_sample(
                                source.assets,
                                source.material,
                                uv,
                                params.backend_sampled_base_color_textures,
                            ),
                            metallic_roughness_texture: metallic_roughness_texture_sample(
                                source.assets,
                                source.material,
                                uv,
                            ),
                            occlusion_texture: occlusion_texture_sample(
                                source.assets,
                                source.material,
                                uv,
                            ),
                            emissive_texture: emissive_texture_sample(
                                source.assets,
                                source.material,
                                uv,
                            ),
                            environment: params.environment_lighting,
                            directional_shadow_factor: shadow_visibility,
                        },
                    ),
                    vertex_color,
                )
            }
        };
        let primitive = Primitive::triangle_with_attributes(
            [
                Vertex {
                    position: position_a,
                    color: shade_vertex(
                        position_a,
                        shaded_normal_a,
                        uv_a,
                        vertex_colors[triangle[0] as usize],
                        shadow_visibility_a,
                    ),
                },
                Vertex {
                    position: position_b,
                    color: shade_vertex(
                        position_b,
                        shaded_normal_b,
                        uv_b,
                        vertex_colors[triangle[1] as usize],
                        shadow_visibility_b,
                    ),
                },
                Vertex {
                    position: position_c,
                    color: shade_vertex(
                        position_c,
                        shaded_normal_c,
                        uv_c,
                        vertex_colors[triangle[2] as usize],
                        shadow_visibility_c,
                    ),
                },
            ],
            [
                PrimitiveVertexAttributes {
                    normal: geometric_normal_a,
                    tex_coord0: uv_a,
                    tangent: tangent_a.tangent,
                    tangent_handedness: tangent_a.handedness,
                    shadow_visibility: shadow_visibility_a,
                },
                PrimitiveVertexAttributes {
                    normal: geometric_normal_b,
                    tex_coord0: uv_b,
                    tangent: tangent_b.tangent,
                    tangent_handedness: tangent_b.handedness,
                    shadow_visibility: shadow_visibility_b,
                },
                PrimitiveVertexAttributes {
                    normal: geometric_normal_c,
                    tex_coord0: uv_c,
                    tangent: tangent_c.tangent,
                    tangent_handedness: tangent_c.handedness,
                    shadow_visibility: shadow_visibility_c,
                },
            ],
        )
        .with_render_material_slot(render_material_slot);
        match material_pass {
            MaterialPass::Opaque => sinks.primitives.push(primitive),
            MaterialPass::Blend => sinks.transparent_primitives.push(TransparentPrimitive {
                depth: average_sort_depth(&primitive, params.camera_projection),
                primitive,
            }),
            MaterialPass::Mask { cutoff } => {
                if primitive
                    .vertices()
                    .iter()
                    .any(|vertex| vertex.color.a >= cutoff)
                {
                    sinks.primitives.push(primitive);
                }
            }
        }
    }

    Ok(())
}

fn average_sort_depth(primitive: &Primitive, camera_projection: Option<&CameraProjection>) -> f32 {
    if let Some(camera_projection) = camera_projection {
        let vertices = primitive.vertices();
        let mut depth_sum = 0.0;
        let mut depth_count = 0;
        for vertex in vertices {
            if let Some(depth) = camera_projection.camera_depth(vertex.position) {
                depth_sum += depth;
                depth_count += 1;
            }
        }
        if depth_count > 0 {
            return depth_sum / depth_count as f32;
        }
    }

    let vertices = primitive.vertices();
    (vertices[0].position.z + vertices[1].position.z + vertices[2].position.z) / 3.0
}
