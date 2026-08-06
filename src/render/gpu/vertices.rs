use std::collections::HashMap;

use crate::geometry::{PrimitiveVertexAttributes, Vertex};
use crate::render::prepare::PreparedPrimitive;
use crate::render::prepare::transforms::{
    unbake_normal_to_model_space, unbake_position_to_model_space,
};
use crate::render::semantic_aov::GpuSemanticAttribution;

pub(super) const VERTEX_BYTE_LEN: usize = 18 * std::mem::size_of::<f32>();
pub(super) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 3 * std::mem::size_of::<f32>() as u64,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 7 * std::mem::size_of::<f32>() as u64,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 10 * std::mem::size_of::<f32>() as u64,
        shader_location: 3,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 12 * std::mem::size_of::<f32>() as u64,
        shader_location: 4,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 16 * std::mem::size_of::<f32>() as u64,
        shader_location: 5,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PrimitiveDrawBatch {
    pub(super) start_vertex: u32,
    pub(super) vertex_count: u32,
    pub(super) material_slot: u32,
    pub(super) draw_uniform_index: u32,
    pub(super) depth_prepass_eligible: bool,
    pub(super) double_sided: bool,
    pub(super) semantic_eligible: bool,
    pub(super) reflection_probe_slot: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DrawUniformValue {
    pub(super) world_from_model: [f32; 16],
    pub(super) normal_from_model: [f32; 16],
    pub(super) tint: crate::material::Color,
    pub(super) semantic_id: [f32; 4],
    pub(super) reflection_probe_bounds_min: [f32; 4],
    pub(super) reflection_probe_bounds_max: [f32; 4],
    pub(super) reflection_probe_capture: [f32; 4],
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct DrawUniformIndexMetrics {
    pub(super) lookup_probes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DrawUniformKey {
    world_from_model: [u32; 16],
    normal_from_model: [u32; 16],
    tint: [u32; 4],
    semantic_id: [u32; 4],
    reflection_probe_bounds_min: [u32; 4],
    reflection_probe_bounds_max: [u32; 4],
    reflection_probe_capture: [u32; 4],
}

impl From<DrawUniformValue> for DrawUniformKey {
    fn from(value: DrawUniformValue) -> Self {
        Self {
            world_from_model: value.world_from_model.map(f32::to_bits),
            normal_from_model: value.normal_from_model.map(f32::to_bits),
            tint: [
                value.tint.r.to_bits(),
                value.tint.g.to_bits(),
                value.tint.b.to_bits(),
                value.tint.a.to_bits(),
            ],
            semantic_id: value.semantic_id.map(f32::to_bits),
            reflection_probe_bounds_min: value.reflection_probe_bounds_min.map(f32::to_bits),
            reflection_probe_bounds_max: value.reflection_probe_bounds_max.map(f32::to_bits),
            reflection_probe_capture: value.reflection_probe_capture.map(f32::to_bits),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct DrawUniformInterner {
    values: Vec<DrawUniformValue>,
    indices: HashMap<DrawUniformKey, u32>,
    metrics: DrawUniformIndexMetrics,
}

impl DrawUniformInterner {
    pub(super) fn intern(&mut self, value: DrawUniformValue) -> u32 {
        self.metrics.lookup_probes = self.metrics.lookup_probes.saturating_add(1);
        let key = DrawUniformKey::from(value);
        if let Some(index) = self.indices.get(&key) {
            return *index;
        }
        let index = self.values.len() as u32;
        self.values.push(value);
        self.indices.insert(key, index);
        index
    }

    pub(super) fn finish(mut self) -> (Vec<DrawUniformValue>, DrawUniformIndexMetrics) {
        if self.values.is_empty() {
            self.intern(DrawUniformValue {
                world_from_model: identity_matrix4(),
                normal_from_model: identity_matrix4(),
                tint: crate::material::Color::WHITE,
                semantic_id: [0.0; 4],
                reflection_probe_bounds_min: [0.0; 4],
                reflection_probe_bounds_max: [0.0; 4],
                reflection_probe_capture: [0.0; 4],
            });
        }
        (self.values, self.metrics)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct GpuVertexEncodeMetrics {
    pub(super) matrix_inversions: u64,
}

/// Writes the prepared primitives as MODEL-SPACE vertex bytes for GPU upload.
/// CPU consumers (picking, culling, CPU rasterization, shadow occluders) read
/// the prepared primitives directly with world-baked vertices; the GPU
/// upload path recovers model-space by applying the inverse of the matrix
/// that produced the bake. The shader then applies the per-draw
/// `world_from_model` from the dynamic-offset draw uniform, yielding the
/// same world-space position as the CPU path. Phase 1A.2 closure for
/// scena-wgpu-architect F2.
#[cfg(test)]
pub(super) fn encode_vertices(primitives: &[PreparedPrimitive]) -> Vec<u8> {
    encode_vertices_profiled(primitives).0
}

#[cfg(test)]
pub(super) fn encode_vertices_profiled(
    primitives: &[PreparedPrimitive],
) -> (Vec<u8>, GpuVertexEncodeMetrics) {
    encode_vertices_iter(primitives.iter(), primitives.len())
}

pub(super) fn encode_vertices_iter<'a>(
    primitives: impl IntoIterator<Item = &'a PreparedPrimitive>,
    primitive_count: usize,
) -> (Vec<u8>, GpuVertexEncodeMetrics) {
    let mut bytes = Vec::with_capacity(primitive_count * 3 * VERTEX_BYTE_LEN);
    for prepared in primitives {
        if let Some(model_vertices) = prepared.model_vertices() {
            for model in model_vertices {
                encode_vertex(&mut bytes, model.vertex, model.attributes);
            }
            continue;
        }
        let primitive = prepared.primitive();
        let position_inverse = prepared.world_to_model();
        let normal_inverse = prepared.model_from_normal();
        for (vertex, attributes) in primitive
            .vertices()
            .iter()
            .zip(primitive.vertex_attributes().iter())
        {
            // Recover model-space position via the inverse-transform-of-bake.
            // `position_inverse` is None only if `world_from_model` is
            // singular (zero scale on an axis), in which case we fall back
            // to the world-baked vertex which the GPU will then double-
            // transform against the singular forward matrix — pixels would
            // be degenerate either way, so we avoid panicking and let the
            // upstream culling stage decide.
            let model_vertex = match position_inverse {
                Some(inv) => Vertex {
                    position: unbake_position_to_model_space(vertex.position, &inv),
                    color: vertex.color,
                },
                None => *vertex,
            };
            let model_attributes = match normal_inverse {
                Some(inv) => PrimitiveVertexAttributes {
                    normal: unbake_normal_to_model_space(attributes.normal, &inv),
                    tex_coord0: attributes.tex_coord0,
                    tangent: unbake_normal_to_model_space(attributes.tangent, &inv),
                    tangent_handedness: attributes.tangent_handedness,
                    shadow_visibility: attributes.shadow_visibility,
                    ambient_visibility: attributes.ambient_visibility,
                },
                None => *attributes,
            };
            encode_vertex(&mut bytes, model_vertex, model_attributes);
        }
    }
    (bytes, GpuVertexEncodeMetrics::default())
}

#[cfg(test)]
pub(super) fn encode_draw_batches(
    primitives: &[PreparedPrimitive],
) -> (Vec<PrimitiveDrawBatch>, Vec<DrawUniformValue>) {
    let (batches, uniforms, _) = encode_draw_batches_profiled(primitives);
    (batches, uniforms)
}

#[cfg(test)]
pub(super) fn encode_draw_batches_profiled(
    primitives: &[PreparedPrimitive],
) -> (
    Vec<PrimitiveDrawBatch>,
    Vec<DrawUniformValue>,
    DrawUniformIndexMetrics,
) {
    let mut interner = DrawUniformInterner::default();
    let batches = encode_draw_batches_indexed_with_semantics(primitives, &mut interner, None);
    let (uniforms, metrics) = interner.finish();
    (batches, uniforms, metrics)
}

pub(super) fn encode_draw_batches_indexed_with_semantics(
    primitives: &[PreparedPrimitive],
    interner: &mut DrawUniformInterner,
    attribution: Option<&GpuSemanticAttribution>,
) -> Vec<PrimitiveDrawBatch> {
    let mut batches: Vec<PrimitiveDrawBatch> = Vec::new();
    for primitive in primitives {
        let start_vertex = primitive.original_vertex_offset();
        let material_slot = primitive.render_material_slot();
        let depth_prepass_eligible = primitive.depth_prepass_eligible();
        let double_sided = primitive.double_sided();
        let semantic_eligible = primitive.semantic_opaque() && primitive.source_node().is_some();
        let reflection_probe_slot = primitive.reflection_probe().map(|probe| probe.slot());
        let draw_uniform_index =
            interner.intern(draw_uniform_value_for_semantics(primitive, attribution));
        if let Some(last) = batches.last_mut()
            && last.material_slot == material_slot
            && last.draw_uniform_index == draw_uniform_index
            && last.depth_prepass_eligible == depth_prepass_eligible
            && last.double_sided == double_sided
            && last.semantic_eligible == semantic_eligible
            && last.reflection_probe_slot == reflection_probe_slot
            && last.start_vertex.saturating_add(last.vertex_count) == start_vertex
        {
            last.vertex_count = last.vertex_count.saturating_add(3);
            continue;
        }
        batches.push(PrimitiveDrawBatch {
            start_vertex,
            vertex_count: 3,
            material_slot,
            draw_uniform_index,
            depth_prepass_eligible,
            double_sided,
            semantic_eligible,
            reflection_probe_slot,
        });
    }
    batches
}

pub(super) fn draw_uniform_value_for(primitive: &PreparedPrimitive) -> DrawUniformValue {
    draw_uniform_value_for_semantics(primitive, None)
}

pub(super) fn draw_uniform_value_for_semantics(
    primitive: &PreparedPrimitive,
    attribution: Option<&GpuSemanticAttribution>,
) -> DrawUniformValue {
    // F8 fallback: when world_from_model is singular (zero scale on an
    // axis), encode_vertices keeps the world-baked vertex unchanged. To
    // avoid the GPU shader re-multiplying that already-world-space vertex
    // against the singular forward matrix, upload identity in the draw
    // uniform for that primitive. Result: shader applies identity ×
    // world_baked = world_baked = correct (matches pre-1A.2 behavior for
    // degenerate primitives).
    let raw_world_from_model = primitive.world_from_model();
    let raw_normal_from_model = primitive.normal_from_model();
    let world_from_model = if primitive.world_to_model().is_some() {
        raw_world_from_model
    } else {
        identity_matrix4()
    };
    let normal_from_model = if primitive.model_from_normal().is_some() {
        raw_normal_from_model
    } else {
        identity_matrix4()
    };
    let (reflection_probe_bounds_min, reflection_probe_bounds_max, reflection_probe_capture) =
        primitive
            .reflection_probe()
            .map_or(([0.0; 4], [0.0; 4], [0.0; 4]), |probe| {
                let bounds = probe.bounds();
                let capture = probe.capture_position();
                (
                    [bounds.min.x, bounds.min.y, bounds.min.z, 1.0],
                    [bounds.max.x, bounds.max.y, bounds.max.z, 0.0],
                    [capture.x, capture.y, capture.z, 0.0],
                )
            });
    DrawUniformValue {
        world_from_model,
        normal_from_model,
        tint: primitive.tint(),
        semantic_id: attribution
            .and_then(|attribution| {
                primitive.source_node().map(|node| {
                    palette_rgba_f32(attribution.palette_index(
                        node,
                        primitive.source_instance(),
                        primitive.semantic_material(),
                    ))
                })
            })
            .unwrap_or([0.0; 4]),
        reflection_probe_bounds_min,
        reflection_probe_bounds_max,
        reflection_probe_capture,
    }
}

pub(super) fn palette_rgba_f32(index: u32) -> [f32; 4] {
    if index == 0 {
        [0.0; 4]
    } else {
        [
            (index & 0xff) as f32 / 255.0,
            ((index >> 8) & 0xff) as f32 / 255.0,
            ((index >> 16) & 0xff) as f32 / 255.0,
            1.0,
        ]
    }
}

const fn identity_matrix4() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn encode_vertex(bytes: &mut Vec<u8>, vertex: Vertex, attributes: PrimitiveVertexAttributes) {
    for value in [
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        vertex.color.r,
        vertex.color.g,
        vertex.color.b,
        vertex.color.a,
        attributes.normal.x,
        attributes.normal.y,
        attributes.normal.z,
        attributes.tex_coord0[0],
        attributes.tex_coord0[1],
        attributes.tangent.x,
        attributes.tangent.y,
        attributes.tangent.z,
        attributes.tangent_handedness,
        attributes.shadow_visibility.clamp(0.0, 1.0),
        attributes.ambient_visibility.clamp(0.0, 1.0),
    ] {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Aabb, Primitive, PrimitiveVertexAttributes, Vertex};
    use crate::material::Color;
    use crate::render::prepare::{
        PreparedDrawTransform, PreparedEnvironmentLighting, PreparedReflectionProbe,
    };
    use crate::scene::{NodeKey, ReflectionProbeKey, Vec3};

    #[test]
    fn gpu_vertex_stream_carries_normals_and_texcoord0() {
        assert_eq!(VERTEX_BYTE_LEN, 18 * std::mem::size_of::<f32>());
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 2
                    && attribute.format == wgpu::VertexFormat::Float32x3),
            "normal attribute must be passed to GPU shaders"
        );
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 3
                    && attribute.format == wgpu::VertexFormat::Float32x2),
            "TEXCOORD_0 must be passed to GPU shaders"
        );
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 4
                    && attribute.format == wgpu::VertexFormat::Float32x4),
            "tangent attribute must include handedness for tangent-space normal maps"
        );
        assert!(
            VERTEX_ATTRIBUTES
                .iter()
                .any(|attribute| attribute.shader_location == 5
                    && attribute.format == wgpu::VertexFormat::Float32x2),
            "prepared direct and ambient visibility must share one GPU attribute"
        );

        let primitive = Primitive::triangle_with_attributes(
            [
                Vertex {
                    position: Vec3::new(1.0, 2.0, 3.0),
                    color: Color::from_linear_rgba(0.1, 0.2, 0.3, 0.4),
                },
                Vertex {
                    position: Vec3::new(4.0, 5.0, 6.0),
                    color: Color::from_linear_rgba(0.5, 0.6, 0.7, 0.8),
                },
                Vertex {
                    position: Vec3::new(7.0, 8.0, 9.0),
                    color: Color::from_linear_rgba(0.9, 1.0, 0.1, 0.2),
                },
            ],
            [
                PrimitiveVertexAttributes {
                    normal: Vec3::new(0.0, 1.0, 0.0),
                    tex_coord0: [0.25, 0.75],
                    tangent: Vec3::new(1.0, 0.0, 0.0),
                    tangent_handedness: -1.0,
                    shadow_visibility: 0.25,
                    ambient_visibility: 0.40,
                },
                PrimitiveVertexAttributes::default(),
                PrimitiveVertexAttributes::default(),
            ],
        );

        let bytes = encode_vertices(&[prepared(primitive, 0)]);
        let first_vertex = bytes[..VERTEX_BYTE_LEN]
            .chunks_exact(4)
            .map(|chunk| f32::from_ne_bytes(chunk.try_into().expect("f32 bytes")))
            .collect::<Vec<_>>();
        assert_eq!(
            first_vertex,
            vec![
                1.0, 2.0, 3.0, 0.1, 0.2, 0.3, 0.4, 0.0, 1.0, 0.0, 0.25, 0.75, 1.0, 0.0, 0.0, -1.0,
                0.25, 0.40
            ]
        );
    }

    #[test]
    fn gpu_draw_batches_preserve_prepared_material_slots() {
        let first = prepared(Primitive::unlit_triangle().with_render_material_slot(1), 0);
        let second = prepared(Primitive::unlit_triangle().with_render_material_slot(1), 3);
        let third = prepared(Primitive::unlit_triangle().with_render_material_slot(2), 6);

        let (batches, draw_uniforms) = encode_draw_batches(&[first, second, third]);

        assert_eq!(
            batches,
            vec![
                PrimitiveDrawBatch {
                    start_vertex: 0,
                    vertex_count: 6,
                    material_slot: 1,
                    draw_uniform_index: 0,
                    depth_prepass_eligible: true,
                    double_sided: false,
                    semantic_eligible: true,
                    reflection_probe_slot: None,
                },
                PrimitiveDrawBatch {
                    start_vertex: 6,
                    vertex_count: 3,
                    material_slot: 2,
                    draw_uniform_index: 0,
                    depth_prepass_eligible: true,
                    double_sided: false,
                    semantic_eligible: true,
                    reflection_probe_slot: None,
                },
            ],
            "GPU draw encoding must preserve prepared per-material slots instead of drawing \
             every primitive with one global material bind group"
        );
        assert_eq!(
            draw_uniforms.len(),
            1,
            "primitives sharing identity world_from_model collapse to a single draw-uniform slot",
        );
    }

    #[test]
    fn gpu_draw_batches_split_when_world_from_model_differs() {
        let first = prepared(Primitive::unlit_triangle().with_render_material_slot(1), 0);
        let translated = prepared_with_transform(
            Primitive::unlit_triangle().with_render_material_slot(1),
            3,
            [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 5.0, 0.0, 0.0, 1.0,
            ],
            identity_matrix4(),
        );

        let (batches, draw_uniforms) = encode_draw_batches(&[first, translated]);

        assert_eq!(
            batches.len(),
            2,
            "primitives with distinct world_from_model must split into separate draw batches"
        );
        assert_eq!(
            batches[0].draw_uniform_index, 0,
            "the first batch maps to the first draw-uniform slot"
        );
        assert_eq!(
            batches[1].draw_uniform_index, 1,
            "the second batch indexes the new draw-uniform slot for the translated primitive"
        );
        assert_eq!(
            draw_uniforms.len(),
            2,
            "each unique world_from_model must produce its own draw-uniform slot"
        );
        assert_eq!(
            draw_uniforms[1].world_from_model[12], 5.0,
            "the second draw-uniform slot must record the translated world transform exactly, \
             not the per-vertex baked positions"
        );
    }

    #[test]
    fn gpu_draw_encoding_carries_probe_slot_and_box_projection() {
        let bounds = Aabb::new(Vec3::new(-2.0, -1.0, -3.0), Vec3::new(2.0, 3.0, 1.0));
        let capture_position = Vec3::new(0.25, 0.5, -0.75);
        let primitive = prepared(Primitive::unlit_triangle(), 0).with_reflection_probe(Some(
            PreparedReflectionProbe::new(
                ReflectionProbeKey::default(),
                2,
                bounds,
                capture_position,
                PreparedEnvironmentLighting::default(),
            ),
        ));

        let (batches, uniforms) = encode_draw_batches(&[primitive]);

        assert_eq!(batches[0].reflection_probe_slot, Some(2));
        assert_eq!(
            uniforms[0].reflection_probe_bounds_min,
            [bounds.min.x, bounds.min.y, bounds.min.z, 1.0],
        );
        assert_eq!(
            uniforms[0].reflection_probe_bounds_max,
            [bounds.max.x, bounds.max.y, bounds.max.z, 0.0],
        );
        assert_eq!(
            uniforms[0].reflection_probe_capture,
            [
                capture_position.x,
                capture_position.y,
                capture_position.z,
                0.0
            ],
        );
    }

    #[test]
    fn gpu_draw_batches_split_when_opaque_tint_differs() {
        let first = PreparedPrimitive::new(
            Primitive::unlit_triangle().with_render_material_slot(1),
            None,
            Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0),
        )
        .with_original_vertex_offset(0);
        let second = PreparedPrimitive::new(
            Primitive::unlit_triangle().with_render_material_slot(1),
            None,
            Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0),
        )
        .with_original_vertex_offset(3);

        let (batches, draw_uniforms) = encode_draw_batches(&[first, second]);

        assert_eq!(
            batches.len(),
            2,
            "same-transform/same-material primitives with different opaque tints must not share a draw batch"
        );
        assert_eq!(
            draw_uniforms.len(),
            2,
            "opaque tint is part of draw-uniform identity"
        );
        assert_eq!(
            draw_uniforms[0].tint,
            Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0)
        );
        assert_eq!(
            draw_uniforms[1].tint,
            Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0)
        );
    }

    #[test]
    fn gpu_draw_batches_split_when_depth_prepass_eligibility_differs() {
        let opaque = prepared(Primitive::unlit_triangle().with_render_material_slot(1), 0);
        let helper_stroke = prepared(
            Primitive::unlit_triangle()
                .with_render_material_slot(1)
                .without_depth_prepass(),
            3,
        );

        let (batches, draw_uniforms) = encode_draw_batches(&[opaque, helper_stroke]);

        assert_eq!(
            batches,
            vec![
                PrimitiveDrawBatch {
                    start_vertex: 0,
                    vertex_count: 3,
                    material_slot: 1,
                    draw_uniform_index: 0,
                    depth_prepass_eligible: true,
                    double_sided: false,
                    semantic_eligible: true,
                    reflection_probe_slot: None,
                },
                PrimitiveDrawBatch {
                    start_vertex: 3,
                    vertex_count: 3,
                    material_slot: 1,
                    draw_uniform_index: 0,
                    depth_prepass_eligible: false,
                    double_sided: false,
                    semantic_eligible: true,
                    reflection_probe_slot: None,
                },
            ],
            "eligible triangles and ineligible helper strokes must not merge into one draw batch; the depth pass needs to skip the helper stroke while the color pass still draws it",
        );
        assert_eq!(
            draw_uniforms.len(),
            1,
            "depth eligibility should not force another draw uniform when the transform is shared",
        );
    }

    #[test]
    fn gpu_draw_batches_split_when_material_sidedness_differs() {
        let single_sided = prepared(Primitive::unlit_triangle().with_render_material_slot(1), 0);
        let double_sided = prepared(Primitive::unlit_triangle().with_render_material_slot(1), 3)
            .with_double_sided(true);

        let (batches, draw_uniforms) = encode_draw_batches(&[single_sided, double_sided]);

        assert_eq!(
            batches,
            vec![
                PrimitiveDrawBatch {
                    start_vertex: 0,
                    vertex_count: 3,
                    material_slot: 1,
                    draw_uniform_index: 0,
                    depth_prepass_eligible: true,
                    double_sided: false,
                    semantic_eligible: true,
                    reflection_probe_slot: None,
                },
                PrimitiveDrawBatch {
                    start_vertex: 3,
                    vertex_count: 3,
                    material_slot: 1,
                    draw_uniform_index: 0,
                    depth_prepass_eligible: true,
                    double_sided: true,
                    semantic_eligible: true,
                    reflection_probe_slot: None,
                },
            ],
            "single- and double-sided triangles need separate GPU draw batches so the encoder can use the culling pipeline for only the single-sided draws",
        );
        assert_eq!(draw_uniforms.len(), 1);
    }

    #[test]
    fn pf10_unique_draw_uniform_indexing_is_near_linear_and_bitwise_stable() {
        const COUNT: usize = 4_096;
        let primitives = (0..COUNT)
            .map(|index| {
                let mut world = identity_matrix4();
                world[12] = f32::from_bits(index as u32);
                prepared_with_transform(
                    Primitive::unlit_triangle(),
                    (index * 3) as u32,
                    world,
                    identity_matrix4(),
                )
            })
            .collect::<Vec<_>>();

        let (batches, uniforms, metrics) = encode_draw_batches_profiled(&primitives);
        assert_eq!(batches.len(), COUNT);
        assert_eq!(uniforms.len(), COUNT);
        assert!(
            metrics.lookup_probes <= COUNT as u64 * 2,
            "stable bitwise indexing must scale near-linearly, got {} probes for {COUNT} uniforms",
            metrics.lookup_probes
        );
        assert_eq!(uniforms[1].world_from_model[12].to_bits(), 1);
    }

    #[test]
    fn pf10_gpu_vertex_encoding_reuses_prepared_matrix_inverses() {
        let prepared = prepared_with_transform(
            Primitive::unlit_triangle(),
            0,
            [
                2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 5.0, 6.0, 7.0, 1.0,
            ],
            [
                0.5,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0 / 3.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.25,
                0.0,
                0.0,
                0.0,
                0.0,
                1.0,
            ],
        );

        let (_bytes, metrics) = encode_vertices_profiled(&[prepared]);
        assert_eq!(
            metrics.matrix_inversions, 0,
            "GPU encoding must consume inverses carried by prepared primitives"
        );
    }

    fn prepared(primitive: Primitive, original_vertex_offset: u32) -> PreparedPrimitive {
        PreparedPrimitive::new(primitive, Some(NodeKey::default()), Color::WHITE)
            .with_original_vertex_offset(original_vertex_offset)
    }

    fn prepared_with_transform(
        primitive: Primitive,
        original_vertex_offset: u32,
        world_from_model: [f32; 16],
        normal_from_model: [f32; 16],
    ) -> PreparedPrimitive {
        prepared(primitive, original_vertex_offset).with_draw_transform(
            PreparedDrawTransform::shared(world_from_model, normal_from_model),
        )
    }
}
