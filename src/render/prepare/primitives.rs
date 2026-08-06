use std::sync::Arc;

use crate::diagnostics::{Backend, PrepareError};
use crate::geometry::{GeometryVertex, Primitive, PrimitiveVertexAttributes, Vertex};
use crate::material::{MaterialDesc, MaterialKind};
use crate::render::camera::CameraProjection;
use crate::render::physical_transmission::{
    PreparedPhysicalTransmission, PreparedPhysicalTransmissionInput,
};
use crate::scene::Vec3;

use super::cpu_bake::{
    CpuBakeCorner, area_shadow_subdivisions_for_scale, baked_ambient_visibility_profiled,
    baked_area_shadow_visibility_profiled, baked_shadow_visibility_profiled,
    bounded_gpu_subdivisions, cpu_texture_subdivisions, push_material_pass_primitive,
    subdivided_cpu_corners,
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
use super::tangents::{
    TangentFrame, accumulate_vertex_tangents, authored_vertex_tangents, generate_model_tangents,
    transform_model_tangents,
};
use super::transforms::{
    normal_from_model_matrix, transform_normal, transform_position, world_from_model_matrix,
};
use super::types::{
    DeformationInputs, GeometryPrimitiveSource, PreparedDrawTransform, PreparedMaterialReflection,
    PreparedPrimitive, PrimitiveBakeParams, PrimitiveSinks,
};

mod dispatch;
mod material_helpers;
pub(in crate::render) use dispatch::append_geometry_primitives;
pub(in crate::render) use material_helpers::draw_uniform_tint;
use material_helpers::{
    average_texture_sample, brighter_color, camera_facing_double_sided_normal,
    cpu_texture_sample_slot_count, material_reflection, material_transmission,
    photographic_uv_scale, scale_uv, structural_vertex_tint, tinted_vertex_color,
    triangle_screen_edge_pixels, triangle_uv_span,
};

#[derive(Debug)]
struct VisibilityDebugStats {
    samples: u64,
    area_sum: f64,
    area_min: f32,
    area_occluded: u64,
    ambient_sum: f64,
    ambient_min: f32,
    ambient_occluded: u64,
}

impl Default for VisibilityDebugStats {
    fn default() -> Self {
        Self {
            samples: 0,
            area_sum: 0.0,
            area_min: 1.0,
            area_occluded: 0,
            ambient_sum: 0.0,
            ambient_min: 1.0,
            ambient_occluded: 0,
        }
    }
}

impl VisibilityDebugStats {
    fn record(&mut self, area: f32, ambient: f32) {
        self.samples = self.samples.saturating_add(1);
        self.area_sum += f64::from(area);
        self.area_min = self.area_min.min(area);
        self.area_occluded = self.area_occluded.saturating_add(u64::from(area < 0.999));
        self.ambient_sum += f64::from(ambient);
        self.ambient_min = self.ambient_min.min(ambient);
        self.ambient_occluded = self
            .ambient_occluded
            .saturating_add(u64::from(ambient < 0.999));
    }

    fn log(self, node: crate::NodeKey) {
        if self.samples == 0 {
            return;
        }
        let samples = self.samples as f64;
        eprintln!(
            "[visibility] node={node:?} samples={} area_min={:.4} area_mean={:.4} area_occluded_fraction={:.4} ambient_min={:.4} ambient_mean={:.4} ambient_occluded_fraction={:.4}",
            self.samples,
            self.area_min,
            self.area_sum / samples,
            self.area_occluded as f64 / samples,
            self.ambient_min,
            self.ambient_sum / samples,
            self.ambient_occluded as f64 / samples,
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn visibility_debug_enabled() -> bool {
    std::env::var_os("SCENA_DEBUG_LOG_VISIBILITY").is_some()
}

#[cfg(target_arch = "wasm32")]
const fn visibility_debug_enabled() -> bool {
    false
}

fn append_triangle_primitives(
    source: GeometryPrimitiveSource<'_>,
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
                    clip_with_scene: source.clip_with_scene,
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
                    clip_with_scene: source.clip_with_scene,
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
    let deformed_vertices = source
        .geometry
        .deformed_vertices(deformation.morph_weights, deformation.skin_matrices)
        .map_err(|error| PrepareError::InvalidSkinGeometry {
            node: source.node,
            reason: format!("{error:?}"),
        })?;
    if matches!(&deformed_vertices, std::borrow::Cow::Owned(_))
        && let Some(work) = params.work
    {
        work.record_deformed_vertex_bytes(
            (deformed_vertices.len() as u64)
                .saturating_mul(std::mem::size_of::<GeometryVertex>() as u64),
        );
    }
    let vertices = deformed_vertices.as_ref();
    let tex_coords0 = source.geometry.authored_tex_coords0();
    let morphed_tangents = deformation
        .morph_weights
        .and_then(|weights| source.geometry.morphed_tangents(weights));
    let vertex_tangents = authored_vertex_tangents(
        morphed_tangents
            .as_deref()
            .or_else(|| source.geometry.tangents()),
        vertices,
        params.transform,
    )
    .unwrap_or_else(|| {
        if matches!(&deformed_vertices, std::borrow::Cow::Borrowed(_)) {
            let (model_tangents, cache_hit) = source.geometry.cached_generated_tangents(|| {
                generate_model_tangents(vertices, source.geometry.indices(), tex_coords0)
            });
            if let Some(work) = params.work {
                work.record_generated_tangent_cache(cache_hit);
                if !cache_hit {
                    let vertex_count = vertices.len() as u64;
                    work.record_generated_tangents(
                        source.geometry.indices().len() / 3,
                        vertices.len(),
                        vertex_count
                            .saturating_mul(std::mem::size_of::<Vec3>() as u64)
                            .saturating_mul(2),
                        vertex_count.saturating_mul(std::mem::size_of::<[f32; 4]>() as u64),
                    );
                }
                work.record_tangent_output_bytes(
                    (vertices.len() as u64)
                        .saturating_mul(std::mem::size_of::<TangentFrame>() as u64),
                );
            }
            return transform_model_tangents(&model_tangents, vertices, params.transform);
        }
        if let Some(work) = params.work {
            let vertex_count = vertices.len() as u64;
            work.record_generated_tangents(
                source.geometry.indices().len() / 3,
                vertices.len(),
                vertex_count
                    .saturating_mul(std::mem::size_of::<Vec3>() as u64)
                    .saturating_mul(2),
                vertex_count.saturating_mul(std::mem::size_of::<TangentFrame>() as u64),
            );
        }
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
    let draw_transform = PreparedDrawTransform::shared(world_from_model, normal_from_model);
    let render_material_slot =
        render_material_slot(source.material_handle, params.backend_material_slots);
    let backend_shaded_material = render_material_slot != 0;
    let camera_position = params
        .camera_projection
        .map(CameraProjection::camera_position);
    let transmissive = source.material.kind() == MaterialKind::PbrMetallicRoughness
        && source.material.transmission_factor() > 0.001;
    let textured_thickness = transmissive && source.material.thickness_factor() > 0.0;
    let texture_samples_per_shaded_vertex = cpu_texture_sample_slot_count(source.material);
    let material_reflection = material_reflection(source.material);
    let source_triangle_count = (source.geometry.indices().len() / 3).max(1) as u32;
    let gpu_backend = matches!(
        params.target.backend,
        Backend::HeadlessGpu | Backend::NativeSurface | Backend::WebGpu | Backend::WebGl2
    );
    let mut subdivision_scratch = Vec::new();
    let photographic_uv_scale = photographic_uv_scale(&source, params.transform.scale);
    let mut visibility_debug = visibility_debug_enabled().then(VisibilityDebugStats::default);

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
        let uv_a = scale_uv(
            source.geometry.tex_coord0_or_default(triangle[0] as usize),
            photographic_uv_scale,
        );
        let uv_b = scale_uv(
            source.geometry.tex_coord0_or_default(triangle[1] as usize),
            photographic_uv_scale,
        );
        let uv_c = scale_uv(
            source.geometry.tex_coord0_or_default(triangle[2] as usize),
            photographic_uv_scale,
        );
        let tangent_a = vertex_tangents[triangle[0] as usize];
        let tangent_b = vertex_tangents[triangle[1] as usize];
        let tangent_c = vertex_tangents[triangle[2] as usize];
        let directional_shadow_visibility_a = baked_shadow_visibility_profiled(
            position_a,
            params.lights,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            backend_shaded_material,
            params.work,
        );
        let directional_shadow_visibility_b = baked_shadow_visibility_profiled(
            position_b,
            params.lights,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            backend_shaded_material,
            params.work,
        );
        let directional_shadow_visibility_c = baked_shadow_visibility_profiled(
            position_c,
            params.lights,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            backend_shaded_material,
            params.work,
        );
        let area_shadow_visibility_a = baked_area_shadow_visibility_profiled(
            position_a,
            params.lights,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            params.work,
        );
        let area_shadow_visibility_b = baked_area_shadow_visibility_profiled(
            position_b,
            params.lights,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            params.work,
        );
        let area_shadow_visibility_c = baked_area_shadow_visibility_profiled(
            position_c,
            params.lights,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            params.work,
        );
        let ambient_visibility_a = baked_ambient_visibility_profiled(
            position_a,
            geometric_normal_a,
            params.baked_ambient_occlusion,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            params.work,
        );
        let ambient_visibility_b = baked_ambient_visibility_profiled(
            position_b,
            geometric_normal_b,
            params.baked_ambient_occlusion,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            params.work,
        );
        let ambient_visibility_c = baked_ambient_visibility_profiled(
            position_c,
            geometric_normal_c,
            params.baked_ambient_occlusion,
            params.shadow_occluders,
            params.shadow_visibility_cache,
            params.work,
        );
        let shade_vertex = |corner: CpuBakeCorner| {
            if backend_shaded_material {
                corner.vertex_color
            } else {
                if let Some(work) = params.work {
                    work.record_cpu_bake_shaded_vertex(texture_samples_per_shaded_vertex);
                }
                let geometric_normal = camera_facing_double_sided_normal(
                    corner.geometric_normal,
                    source.material.double_sided(),
                    corner.position,
                    camera_position,
                );
                let normal = normal_texture_sample(
                    source.textures,
                    source.material,
                    corner.uv,
                    geometric_normal,
                    corner.tangent,
                    corner.tangent_handedness,
                );
                let clearcoat_normal = clearcoat_normal_texture_sample(
                    source.textures,
                    source.material,
                    corner.uv,
                    normal,
                    geometric_normal,
                    corner.tangent,
                    corner.tangent_handedness,
                );
                let base_color_texture = base_color_texture_sample(
                    source.textures,
                    source.material,
                    corner.uv,
                    params.backend_sampled_base_color_textures,
                );
                let metallic_roughness_texture =
                    metallic_roughness_texture_sample(source.textures, source.material, corner.uv);
                let occlusion_texture =
                    occlusion_texture_sample(source.textures, source.material, corner.uv);
                let emissive_texture =
                    emissive_texture_sample(source.textures, source.material, corner.uv);
                let clearcoat_texture =
                    clearcoat_texture_sample(source.textures, source.material, corner.uv);
                let clearcoat_roughness_texture =
                    clearcoat_roughness_texture_sample(source.textures, source.material, corner.uv);
                let sheen_color_texture =
                    sheen_color_texture_sample(source.textures, source.material, corner.uv);
                let sheen_roughness_texture =
                    sheen_roughness_texture_sample(source.textures, source.material, corner.uv);
                let anisotropy_texture =
                    anisotropy_texture_sample(source.textures, source.material, corner.uv);
                let iridescence_texture =
                    iridescence_texture_sample(source.textures, source.material, corner.uv);
                let iridescence_thickness_texture = iridescence_thickness_texture_sample(
                    source.textures,
                    source.material,
                    corner.uv,
                );
                let transmission_texture = if transmissive {
                    transmission_texture_sample(source.textures, source.material, corner.uv)
                } else {
                    1.0
                };
                let thickness_texture = if textured_thickness {
                    thickness_texture_sample(source.textures, source.material, corner.uv)
                } else {
                    1.0
                };
                let shade_with_normal = |normal, clearcoat_normal| {
                    material_color(
                        source.material,
                        params.lights,
                        &MaterialShadingInput {
                            position: corner.position,
                            normal,
                            tangent: corner.tangent,
                            tangent_handedness: corner.tangent_handedness,
                            camera_position,
                            base_color_texture,
                            metallic_roughness_texture,
                            occlusion_texture,
                            emissive_texture,
                            clearcoat_texture,
                            clearcoat_roughness_texture,
                            clearcoat_normal,
                            sheen_color_texture,
                            sheen_roughness_texture,
                            anisotropy_texture,
                            iridescence_texture,
                            iridescence_thickness_texture,
                            transmission_texture,
                            thickness_texture,
                            environment: params
                                .reflection_probe
                                .as_ref()
                                .map(|probe| probe.lighting().clone())
                                .unwrap_or_else(|| params.environment_lighting.clone()),
                            directional_shadow_factor: corner.directional_shadow_visibility,
                            area_shadow_factor: corner.area_shadow_visibility,
                            ambient_visibility: corner.ambient_visibility,
                        },
                    )
                };
                let lit = shade_with_normal(normal, clearcoat_normal);
                let lit = if source.material.double_sided()
                    && matches!(source.material.kind(), MaterialKind::PbrMetallicRoughness)
                {
                    brighter_color(lit, shade_with_normal(-normal, -clearcoat_normal))
                } else {
                    lit
                };
                multiply_color(lit, corner.vertex_color)
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
                    source
                        .geometry
                        .vertex_color_or_default(triangle[0] as usize),
                    structural_vertex_tint(source.tint),
                ),
                directional_shadow_visibility: directional_shadow_visibility_a,
                area_shadow_visibility: area_shadow_visibility_a,
                ambient_visibility: ambient_visibility_a,
            },
            CpuBakeCorner {
                position: position_b,
                geometric_normal: geometric_normal_b,
                uv: uv_b,
                tangent: tangent_b.tangent,
                tangent_handedness: tangent_b.handedness,
                vertex_color: tinted_vertex_color(
                    source
                        .geometry
                        .vertex_color_or_default(triangle[1] as usize),
                    structural_vertex_tint(source.tint),
                ),
                directional_shadow_visibility: directional_shadow_visibility_b,
                area_shadow_visibility: area_shadow_visibility_b,
                ambient_visibility: ambient_visibility_b,
            },
            CpuBakeCorner {
                position: position_c,
                geometric_normal: geometric_normal_c,
                uv: uv_c,
                tangent: tangent_c.tangent,
                tangent_handedness: tangent_c.handedness,
                vertex_color: tinted_vertex_color(
                    source
                        .geometry
                        .vertex_color_or_default(triangle[2] as usize),
                    structural_vertex_tint(source.tint),
                ),
                directional_shadow_visibility: directional_shadow_visibility_c,
                area_shadow_visibility: area_shadow_visibility_c,
                ambient_visibility: ambient_visibility_c,
            },
        ];
        let screen_edge_pixels =
            triangle_screen_edge_pixels(corners, params.camera_projection, params.target);
        let requested_subdivisions = cpu_texture_subdivisions(
            source.material,
            backend_shaded_material,
            screen_edge_pixels,
            triangle_uv_span(corners),
            source.textures.max_decoded_dimension() as f32,
        )
        .max(area_shadow_subdivisions_for_scale(
            params.lights.has_area_lights() || params.baked_ambient_occlusion.is_some(),
            screen_edge_pixels,
            params.screen_space_scale,
        ));
        let subdivisions =
            bounded_gpu_subdivisions(requested_subdivisions, source_triangle_count, gpu_backend);
        let sub_triangles = subdivided_cpu_corners(
            corners,
            subdivisions,
            backend_shaded_material,
            &mut subdivision_scratch,
        );
        if let Some(work) = params.work {
            work.record_cpu_bake_triangles(
                sub_triangles.len(),
                (sub_triangles.len() as u64)
                    .saturating_mul(std::mem::size_of::<[CpuBakeCorner; 3]>() as u64),
            );
        }
        for mut sub_triangle in sub_triangles {
            if subdivisions > 1 && params.lights.has_area_lights() {
                for corner in &mut sub_triangle {
                    corner.area_shadow_visibility = baked_area_shadow_visibility_profiled(
                        corner.position,
                        params.lights,
                        params.shadow_occluders,
                        params.shadow_visibility_cache,
                        params.work,
                    );
                }
            }
            if subdivisions > 1 && params.baked_ambient_occlusion.is_some() {
                for corner in &mut sub_triangle {
                    corner.ambient_visibility = baked_ambient_visibility_profiled(
                        corner.position,
                        corner.geometric_normal,
                        params.baked_ambient_occlusion,
                        params.shadow_occluders,
                        params.shadow_visibility_cache,
                        params.work,
                    );
                }
            }
            if let Some(debug) = &mut visibility_debug {
                for corner in sub_triangle {
                    debug.record(corner.area_shadow_visibility, corner.ambient_visibility);
                }
            }
            if let Some(work) = params.work {
                let averaged_texture_samples = 3u64.saturating_mul(
                    u64::from(transmissive && source.material.transmission_texture().is_some())
                        .saturating_add(u64::from(
                            textured_thickness && source.material.thickness_texture().is_some(),
                        )),
                );
                work.record_texture_samples(averaged_texture_samples);
            }
            let material_transmission = material_transmission(
                source.material,
                if transmissive {
                    average_texture_sample(&sub_triangle, |uv| {
                        transmission_texture_sample(source.textures, source.material, uv)
                    })
                } else {
                    1.0
                },
                if textured_thickness {
                    average_texture_sample(&sub_triangle, |uv| {
                        thickness_texture_sample(source.textures, source.material, uv)
                    })
                } else {
                    1.0
                },
            );
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
                    ambient_visibility: corner.ambient_visibility,
                }),
            )
            .with_render_material_slot(render_material_slot);
            let primitive = material_helpers::semantic_attribution(
                PreparedPrimitive::new_with_draw_transform(
                    primitive,
                    Some(source.node),
                    draw_uniform_tint(source.tint),
                    Arc::clone(&draw_transform),
                ),
                source.instance,
                source.material_handle,
                material_pass,
            )
            .with_double_sided(source.material.double_sided())
            .with_material_reflection(material_reflection)
            .with_material_transmission(material_transmission)
            .with_reflection_probe(params.reflection_probe.clone());
            push_material_pass_primitive(
                primitive,
                material_pass,
                &mut sinks,
                params.camera_projection,
            );
        }
    }

    if let Some(debug) = visibility_debug {
        debug.log(source.node);
    }
    Ok(())
}
