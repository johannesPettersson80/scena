use crate::diagnostics::PrepareError;
use crate::geometry::{GeometryTopology, Primitive, PrimitiveVertexAttributes, Vertex};
use crate::material::{MaterialDesc, MaterialKind};
use crate::render::camera::CameraProjection;

use super::cpu_bake::{
    CpuBakeCorner, baked_area_shadow_visibility, baked_shadow_visibility, cpu_texture_subdivisions,
    push_material_pass_primitive, subdivided_cpu_corners,
};
use super::lighting::{MaterialShadingInput, material_color};
use super::materials::{
    anisotropy_texture_sample, base_color_texture_sample, clearcoat_normal_texture_sample,
    clearcoat_roughness_texture_sample, clearcoat_texture_sample, emissive_texture_sample,
    iridescence_texture_sample, iridescence_thickness_texture_sample, material_pass,
    metallic_roughness_texture_sample, multiply_color, normal_texture_sample,
    occlusion_texture_sample, render_material_slot, sheen_color_texture_sample,
    sheen_roughness_texture_sample, thickness_texture_sample, transmission_texture_sample,
};
use super::strokes;
use super::tangents::{accumulate_vertex_tangents, authored_vertex_tangents};
use super::transforms::{
    normal_from_model_matrix, transform_normal, transform_position, world_from_model_matrix,
};
use super::types::{
    DeformationInputs, GeometryPrimitiveSource, PreparedMaterialReflection, PreparedPrimitive,
    PrimitiveBakeParams, PrimitiveSinks,
};

pub(super) fn append_geometry_primitives<F>(
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
            strokes::StrokeBakeInputs {
                tint: source.tint,
                params,
                sinks,
            },
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
                strokes::StrokeBakeInputs {
                    tint: source.tint,
                    params,
                    sinks,
                },
            );
        }
        MaterialKind::Edge => {
            return strokes::append_edge_primitives(
                source.node,
                source.geometry,
                source.material,
                strokes::StrokeBakeInputs {
                    tint: source.tint,
                    params,
                    sinks,
                },
            );
        }
    }

    let material_pass = match (material_pass(source.node, source.material)?, source.tint) {
        (_, Some(tint)) if tint.a < 1.0 => super::materials::MaterialPass::Blend,
        (pass, _) => pass,
    };
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
    let world_from_model = world_from_model_matrix(params.transform, params.origin_shift);
    let normal_from_model = normal_from_model_matrix(params.transform);

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
        let directional_shadow_visibility_a = baked_shadow_visibility(
            position_a,
            params.lights,
            params.shadow_occluders,
            backend_shaded_material,
        );
        let directional_shadow_visibility_b = baked_shadow_visibility(
            position_b,
            params.lights,
            params.shadow_occluders,
            backend_shaded_material,
        );
        let directional_shadow_visibility_c = baked_shadow_visibility(
            position_c,
            params.lights,
            params.shadow_occluders,
            backend_shaded_material,
        );
        let area_shadow_visibility_a =
            baked_area_shadow_visibility(position_a, params.lights, params.shadow_occluders);
        let area_shadow_visibility_b =
            baked_area_shadow_visibility(position_b, params.lights, params.shadow_occluders);
        let area_shadow_visibility_c =
            baked_area_shadow_visibility(position_c, params.lights, params.shadow_occluders);
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
                let clearcoat_normal = clearcoat_normal_texture_sample(
                    source.assets,
                    source.material,
                    corner.uv,
                    normal,
                );
                multiply_color(
                    material_color(
                        source.material,
                        params.lights,
                        &MaterialShadingInput {
                            position: corner.position,
                            normal,
                            tangent: corner.tangent,
                            tangent_handedness: corner.tangent_handedness,
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
                            clearcoat_texture: clearcoat_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            clearcoat_roughness_texture: clearcoat_roughness_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            clearcoat_normal,
                            sheen_color_texture: sheen_color_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            sheen_roughness_texture: sheen_roughness_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            anisotropy_texture: anisotropy_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            iridescence_texture: iridescence_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            iridescence_thickness_texture: iridescence_thickness_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            transmission_texture: transmission_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            thickness_texture: thickness_texture_sample(
                                source.assets,
                                source.material,
                                corner.uv,
                            ),
                            environment: params.environment_lighting.clone(),
                            directional_shadow_factor: corner.directional_shadow_visibility,
                            area_shadow_factor: corner.area_shadow_visibility,
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
                vertex_color: tinted_vertex_color(
                    vertex_colors[triangle[0] as usize],
                    structural_vertex_tint(source.tint),
                ),
                directional_shadow_visibility: directional_shadow_visibility_a,
                area_shadow_visibility: area_shadow_visibility_a,
            },
            CpuBakeCorner {
                position: position_b,
                geometric_normal: geometric_normal_b,
                uv: uv_b,
                tangent: tangent_b.tangent,
                tangent_handedness: tangent_b.handedness,
                vertex_color: tinted_vertex_color(
                    vertex_colors[triangle[1] as usize],
                    structural_vertex_tint(source.tint),
                ),
                directional_shadow_visibility: directional_shadow_visibility_b,
                area_shadow_visibility: area_shadow_visibility_b,
            },
            CpuBakeCorner {
                position: position_c,
                geometric_normal: geometric_normal_c,
                uv: uv_c,
                tangent: tangent_c.tangent,
                tangent_handedness: tangent_c.handedness,
                vertex_color: tinted_vertex_color(
                    vertex_colors[triangle[2] as usize],
                    structural_vertex_tint(source.tint),
                ),
                directional_shadow_visibility: directional_shadow_visibility_c,
                area_shadow_visibility: area_shadow_visibility_c,
            },
        ];
        let subdivisions = cpu_texture_subdivisions(source.material, backend_shaded_material);
        let material_reflection = material_reflection(source.material);
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
                    shadow_visibility: corner.area_shadow_visibility,
                }),
            )
            .with_render_material_slot(render_material_slot);
            let primitive = primitive.with_world_from_model(world_from_model, normal_from_model);
            let primitive = PreparedPrimitive::new(
                primitive,
                Some(source.node),
                draw_uniform_tint(source.tint),
            )
            .with_double_sided(source.material.double_sided())
            .with_material_reflection(material_reflection);
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

fn material_reflection(material: &MaterialDesc) -> Option<PreparedMaterialReflection> {
    if material.kind() != MaterialKind::PbrMetallicRoughness {
        return None;
    }
    PreparedMaterialReflection::new(material.metallic_factor(), material.roughness_factor())
}

fn structural_vertex_tint(tint: Option<crate::material::Color>) -> Option<crate::material::Color> {
    tint.filter(|tint| tint.a < 1.0)
}

pub(in crate::render) fn draw_uniform_tint(
    tint: Option<crate::material::Color>,
) -> crate::material::Color {
    tint.filter(|tint| tint.a >= 1.0)
        .unwrap_or(crate::material::Color::WHITE)
}

fn tinted_vertex_color(
    color: crate::material::Color,
    tint: Option<crate::material::Color>,
) -> crate::material::Color {
    tint.map_or(color, |tint| multiply_color(color, tint))
}
