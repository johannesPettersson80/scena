use std::sync::Arc;

use crate::assets::{MaterialHandle, TextureHandle};
use crate::geometry::{Aabb, GeometryDesc, Primitive, PrimitiveVertexAttributes, SkinningMatrix};
use crate::material::{Color, MaterialDesc};
use crate::scene::{InstanceId, InstanceSetKey, NodeKey, ReflectionProbeKey, Transform, Vec3};

use super::super::physical_transmission::PreparedPhysicalTransmission as PhysicalTransmission;
use super::super::{PrepareWorkCounter, RasterTarget, camera::CameraProjection};
use super::environment::PreparedEnvironmentLighting;
use super::lighting::PreparedLights;
use super::materials::PreparedMaterialTextures;
use super::shadows::{ShadowOccluderSet, ShadowVisibilityCache};

mod geometry_storage;
pub(in crate::render) use geometry_storage::{
    PreparedDrawTransform, PreparedGeometryStorageMetrics, PreparedModelVertex,
    share_model_space_vertex_buffer,
};

mod labels;
pub(in crate::render) use labels::{PreparedLabelAtlas, PreparedLabelQuad};

pub(super) struct TransparentPrimitive {
    pub(super) depth: f32,
    pub(super) primitive: PreparedPrimitive,
}

pub(super) struct PrimitiveSinks<'out> {
    pub(super) primitives: &'out mut Vec<PreparedPrimitive>,
    pub(super) strokes: &'out mut Vec<PreparedStrokeSegment>,
    pub(super) transparent_primitives: &'out mut Vec<TransparentPrimitive>,
}

pub(super) struct GeometryPrimitiveSource<'a> {
    pub(super) node: NodeKey,
    /// G01: false for generated annotation overlays, which a section box must
    /// not delete.
    pub(super) clip_with_scene: bool,
    pub(super) instance: Option<InstanceId>,
    pub(super) material_handle: MaterialHandle,
    pub(super) geometry: &'a GeometryDesc,
    pub(super) material: &'a MaterialDesc,
    pub(super) textures: &'a PreparedMaterialTextures,
    pub(super) tint: Option<Color>,
}

pub(in crate::render) struct PreparedScene {
    pub(in crate::render) primitives: Vec<PreparedPrimitive>,
    pub(in crate::render) strokes: Vec<PreparedStrokeSegment>,
    pub(in crate::render) labels: PreparedLabelAtlas,
    pub(in crate::render) instances: Vec<PreparedInstanceSet>,
    pub(in crate::render) light_from_world: [f32; 16],
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedReflectionProbe {
    key: ReflectionProbeKey,
    slot: u32,
    bounds: Aabb,
    capture_position: Vec3,
    lighting: PreparedEnvironmentLighting,
}

impl PreparedReflectionProbe {
    pub(in crate::render) const fn new(
        key: ReflectionProbeKey,
        slot: u32,
        bounds: Aabb,
        capture_position: Vec3,
        lighting: PreparedEnvironmentLighting,
    ) -> Self {
        Self {
            key,
            slot,
            bounds,
            capture_position,
            lighting,
        }
    }

    pub(in crate::render) const fn key(&self) -> ReflectionProbeKey {
        self.key
    }

    pub(in crate::render) const fn slot(&self) -> u32 {
        self.slot
    }

    pub(in crate::render) const fn bounds(&self) -> Aabb {
        self.bounds
    }

    pub(in crate::render) const fn capture_position(&self) -> Vec3 {
        self.capture_position
    }

    pub(in crate::render) const fn lighting(&self) -> &PreparedEnvironmentLighting {
        &self.lighting
    }

    pub(in crate::render) fn with_origin_shift(mut self, origin_shift: Vec3) -> Self {
        self.bounds = Aabb::new(
            self.bounds.min - origin_shift,
            self.bounds.max - origin_shift,
        );
        self.capture_position -= origin_shift;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedPrimitive {
    primitive: Primitive,
    draw_transform: Arc<PreparedDrawTransform>,
    model_vertices: Option<Arc<[PreparedModelVertex]>>,
    model_vertex_offset: usize,
    source_node: Option<NodeKey>,
    source_instance: Option<InstanceId>,
    semantic_material: Option<MaterialHandle>,
    semantic_opaque: bool,
    semantic_overlay: bool,
    clip_with_scene: bool,
    semantic_alpha_cutoff: Option<f32>,
    original_vertex_offset: u32,
    tint: Color,
    gpu_triangle_path: bool,
    double_sided: bool,
    material_reflection: Option<PreparedMaterialReflection>,
    material_transmission: Option<PhysicalTransmission>,
    reflection_probe: Option<PreparedReflectionProbe>,
}

impl PreparedPrimitive {
    pub(in crate::render) fn new(
        primitive: Primitive,
        source_node: Option<NodeKey>,
        tint: Color,
    ) -> Self {
        Self::new_with_draw_transform(
            primitive,
            source_node,
            tint,
            PreparedDrawTransform::identity(),
        )
    }

    pub(in crate::render) fn new_with_draw_transform(
        primitive: Primitive,
        source_node: Option<NodeKey>,
        tint: Color,
        draw_transform: Arc<PreparedDrawTransform>,
    ) -> Self {
        Self {
            primitive,
            draw_transform,
            model_vertices: None,
            model_vertex_offset: 0,
            source_node,
            source_instance: None,
            semantic_material: None,
            semantic_opaque: true,
            semantic_overlay: false,
            clip_with_scene: true,
            semantic_alpha_cutoff: None,
            original_vertex_offset: 0,
            tint,
            gpu_triangle_path: true,
            double_sided: false,
            material_reflection: None,
            material_transmission: None,
            reflection_probe: None,
        }
    }

    #[cfg(test)]
    pub(in crate::render) fn with_draw_transform(
        mut self,
        draw_transform: Arc<PreparedDrawTransform>,
    ) -> Self {
        self.draw_transform = draw_transform;
        self
    }

    pub(in crate::render) const fn with_original_vertex_offset(
        mut self,
        original_vertex_offset: u32,
    ) -> Self {
        self.original_vertex_offset = original_vertex_offset;
        self
    }

    pub(in crate::render) fn without_depth_prepass(mut self) -> Self {
        self.primitive = self.primitive.without_depth_prepass();
        self
    }

    pub(in crate::render) const fn without_gpu_triangle_path(mut self) -> Self {
        self.gpu_triangle_path = false;
        self
    }

    pub(in crate::render) const fn with_double_sided(mut self, double_sided: bool) -> Self {
        self.double_sided = double_sided;
        self
    }

    pub(in crate::render) const fn with_source_instance(
        mut self,
        source_instance: Option<InstanceId>,
    ) -> Self {
        self.source_instance = source_instance;
        self
    }

    pub(in crate::render) const fn with_semantic_material(
        mut self,
        material: MaterialHandle,
        opaque: bool,
        alpha_cutoff: Option<f32>,
    ) -> Self {
        self.semantic_material = Some(material);
        self.semantic_opaque = opaque;
        self.semantic_overlay = false;
        self.semantic_alpha_cutoff = alpha_cutoff;
        self
    }

    pub(in crate::render) const fn without_semantic_attribution(mut self) -> Self {
        self.semantic_opaque = false;
        self.semantic_overlay = true;
        self
    }

    pub(in crate::render) const fn with_material_reflection(
        mut self,
        reflection: Option<PreparedMaterialReflection>,
    ) -> Self {
        self.material_reflection = reflection;
        self
    }

    pub(in crate::render) const fn with_material_transmission(
        mut self,
        transmission: Option<PhysicalTransmission>,
    ) -> Self {
        self.material_transmission = transmission;
        self
    }

    pub(in crate::render) fn with_reflection_probe(
        mut self,
        reflection_probe: Option<PreparedReflectionProbe>,
    ) -> Self {
        self.reflection_probe = reflection_probe;
        self
    }

    pub(in crate::render) const fn primitive(&self) -> &Primitive {
        &self.primitive
    }

    pub(in crate::render) const fn source_node(&self) -> Option<NodeKey> {
        self.source_node
    }

    pub(in crate::render) const fn source_instance(&self) -> Option<InstanceId> {
        self.source_instance
    }

    pub(in crate::render) const fn semantic_material(&self) -> Option<MaterialHandle> {
        self.semantic_material
    }

    pub(in crate::render) const fn semantic_opaque(&self) -> bool {
        self.semantic_opaque
    }

    /// Opts this primitive out of scene clipping (planes and section box).
    pub(in crate::render) const fn with_scene_clipping(mut self, clip_with_scene: bool) -> Self {
        self.clip_with_scene = clip_with_scene;
        self
    }

    pub(in crate::render) const fn clips_with_scene(&self) -> bool {
        self.clip_with_scene
    }

    pub(in crate::render) const fn semantic_overlay(&self) -> bool {
        self.semantic_overlay
    }

    #[cfg_attr(not(feature = "scene-host"), allow(dead_code))]
    pub(in crate::render) const fn semantic_alpha_cutoff(&self) -> Option<f32> {
        self.semantic_alpha_cutoff
    }

    pub(in crate::render) const fn original_vertex_offset(&self) -> u32 {
        self.original_vertex_offset
    }

    pub(in crate::render) const fn tint(&self) -> Color {
        self.tint
    }

    pub(in crate::render) fn set_tint(&mut self, tint: Color) {
        self.tint = tint;
    }

    pub(in crate::render) fn set_draw_transform(
        &mut self,
        draw_transform: Arc<PreparedDrawTransform>,
    ) {
        self.draw_transform = draw_transform;
    }

    pub(in crate::render) fn vertices(&self) -> &[crate::geometry::Vertex; 3] {
        self.primitive.vertices()
    }

    pub(in crate::render) fn vertex_attributes(&self) -> &[PrimitiveVertexAttributes; 3] {
        self.primitive.vertex_attributes()
    }

    pub(in crate::render) fn render_material_slot(&self) -> u32 {
        self.primitive.render_material_slot()
    }

    pub(in crate::render) const fn depth_prepass_eligible(&self) -> bool {
        self.primitive.depth_prepass_eligible()
    }

    pub(in crate::render) fn occlusion_culling_eligible(&self) -> bool {
        self.depth_prepass_eligible()
            && self.tint.a >= 1.0 - f32::EPSILON
            && self
                .primitive
                .vertices()
                .iter()
                .all(|vertex| vertex.color.a >= 1.0 - f32::EPSILON)
    }

    pub(in crate::render) const fn gpu_triangle_path(&self) -> bool {
        self.gpu_triangle_path
    }

    pub(in crate::render) const fn double_sided(&self) -> bool {
        self.double_sided
    }

    pub(in crate::render) const fn material_reflection(
        &self,
    ) -> Option<PreparedMaterialReflection> {
        self.material_reflection
    }

    pub(in crate::render) const fn material_transmission(&self) -> Option<PhysicalTransmission> {
        self.material_transmission
    }

    pub(in crate::render) const fn reflection_probe(&self) -> Option<&PreparedReflectionProbe> {
        self.reflection_probe.as_ref()
    }

    pub(in crate::render) fn world_from_model(&self) -> [f32; 16] {
        self.draw_transform.world_from_model
    }

    pub(in crate::render) fn normal_from_model(&self) -> [f32; 16] {
        self.draw_transform.normal_from_model
    }

    pub(in crate::render) fn world_to_model(&self) -> Option<[f32; 16]> {
        self.draw_transform.world_to_model
    }

    pub(in crate::render) fn model_from_normal(&self) -> Option<[f32; 16]> {
        self.draw_transform.model_from_normal
    }

    pub(in crate::render) fn model_vertices(&self) -> Option<&[PreparedModelVertex]> {
        let vertices = self.model_vertices.as_deref()?;
        vertices.get(self.model_vertex_offset..self.model_vertex_offset.saturating_add(3))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedMaterialReflection {
    metallic: f32,
    roughness: f32,
}

impl PreparedMaterialReflection {
    pub(in crate::render) fn new(metallic: f32, roughness: f32) -> Option<Self> {
        if !metallic.is_finite() || !roughness.is_finite() || metallic < 0.5 {
            return None;
        }
        Some(Self {
            metallic: metallic.clamp(0.0, 1.0),
            roughness: roughness.clamp(0.0, 1.0),
        })
    }

    pub(in crate::render) const fn metallic(self) -> f32 {
        self.metallic
    }

    pub(in crate::render) const fn roughness(self) -> f32 {
        self.roughness
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedInstanceSet {
    source_node: NodeKey,
    source_set: Option<InstanceSetKey>,
    primitives: Vec<PreparedPrimitive>,
    instances: Vec<PreparedInstanceRecord>,
}

impl PreparedInstanceSet {
    pub(in crate::render) fn new(
        source_node: NodeKey,
        source_set: InstanceSetKey,
        primitives: Vec<PreparedPrimitive>,
        instances: Vec<PreparedInstanceRecord>,
    ) -> Self {
        Self {
            source_node,
            source_set: Some(source_set),
            primitives,
            instances,
        }
    }

    pub(in crate::render) fn new_auto_batched(
        source_node: NodeKey,
        primitives: Vec<PreparedPrimitive>,
        instances: Vec<PreparedInstanceRecord>,
    ) -> Self {
        Self {
            source_node,
            source_set: None,
            primitives,
            instances,
        }
    }

    pub(in crate::render) const fn source_node(&self) -> NodeKey {
        self.source_node
    }

    pub(in crate::render) const fn source_set(&self) -> Option<InstanceSetKey> {
        self.source_set
    }

    pub(in crate::render) fn primitives(&self) -> &[PreparedPrimitive] {
        &self.primitives
    }

    pub(in crate::render) fn primitives_mut(&mut self) -> &mut [PreparedPrimitive] {
        &mut self.primitives
    }

    pub(in crate::render) fn instances(&self) -> &[PreparedInstanceRecord] {
        &self.instances
    }

    pub(in crate::render) fn set_instances(&mut self, instances: Vec<PreparedInstanceRecord>) {
        self.instances = instances;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedInstanceRecord {
    source_instance: Option<InstanceId>,
    world_from_model: [f32; 16],
    normal_from_model: [f32; 16],
    tint: Color,
}

impl PreparedInstanceRecord {
    pub(in crate::render) const fn new(
        source_instance: InstanceId,
        world_from_model: [f32; 16],
        normal_from_model: [f32; 16],
        tint: Color,
    ) -> Self {
        Self {
            source_instance: Some(source_instance),
            world_from_model,
            normal_from_model,
            tint,
        }
    }

    pub(in crate::render) const fn auto_batched(
        world_from_model: [f32; 16],
        normal_from_model: [f32; 16],
        tint: Color,
    ) -> Self {
        Self {
            source_instance: None,
            world_from_model,
            normal_from_model,
            tint,
        }
    }

    pub(in crate::render) const fn world_from_model(self) -> [f32; 16] {
        self.world_from_model
    }

    pub(in crate::render) const fn normal_from_model(self) -> [f32; 16] {
        self.normal_from_model
    }

    pub(in crate::render) const fn tint(self) -> Color {
        self.tint
    }

    pub(in crate::render) const fn source_instance(self) -> Option<InstanceId> {
        self.source_instance
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedStrokeSegment {
    source_node: Option<NodeKey>,
    clip_with_scene: bool,
    start: Vec3,
    end: Vec3,
    color: Color,
    width_px: f32,
    world_from_model: [f32; 16],
    tint: Color,
    original_segment_index: u32,
}

impl PreparedStrokeSegment {
    pub(in crate::render) const fn new(
        source_node: Option<NodeKey>,
        start: Vec3,
        end: Vec3,
        color: Color,
        width_px: f32,
        world_from_model: [f32; 16],
        tint: Color,
    ) -> Self {
        Self {
            source_node,
            clip_with_scene: true,
            start,
            end,
            color,
            width_px,
            world_from_model,
            tint,
            original_segment_index: 0,
        }
    }

    /// Opts this stroke out of scene clipping (planes and section box).
    pub(in crate::render) const fn with_scene_clipping(mut self, clip_with_scene: bool) -> Self {
        self.clip_with_scene = clip_with_scene;
        self
    }

    pub(in crate::render) const fn clips_with_scene(&self) -> bool {
        self.clip_with_scene
    }

    pub(in crate::render) const fn with_original_segment_index(
        mut self,
        original_segment_index: u32,
    ) -> Self {
        self.original_segment_index = original_segment_index;
        self
    }

    pub(in crate::render) const fn source_node(&self) -> Option<NodeKey> {
        self.source_node
    }

    pub(in crate::render) const fn start(&self) -> Vec3 {
        self.start
    }

    pub(in crate::render) const fn end(&self) -> Vec3 {
        self.end
    }

    pub(in crate::render) const fn color(&self) -> Color {
        self.color
    }

    pub(in crate::render) const fn width_px(&self) -> f32 {
        self.width_px
    }

    pub(in crate::render) const fn world_from_model(&self) -> [f32; 16] {
        self.world_from_model
    }

    pub(in crate::render) const fn tint(&self) -> Color {
        self.tint
    }

    pub(in crate::render) const fn original_segment_index(&self) -> u32 {
        self.original_segment_index
    }

    pub(in crate::render) fn set_tint(&mut self, tint: Color) {
        self.tint = tint;
    }

    pub(in crate::render) fn set_world_from_model(&mut self, world_from_model: [f32; 16]) {
        self.world_from_model = world_from_model;
    }
}

#[derive(Clone)]
pub(super) struct PrimitiveBakeParams<'lights> {
    pub(super) target: RasterTarget,
    pub(super) screen_space_scale: f32,
    pub(super) transform: Transform,
    pub(super) origin_shift: Vec3,
    pub(super) lights: &'lights PreparedLights,
    pub(super) shadow_occluders: &'lights ShadowOccluderSet,
    pub(super) shadow_visibility_cache: &'lights ShadowVisibilityCache,
    pub(super) baked_ambient_occlusion: Option<crate::BakedAmbientOcclusionConfig>,
    pub(super) camera_projection: Option<&'lights CameraProjection>,
    pub(super) backend_sampled_base_color_textures: &'lights [TextureHandle],
    pub(super) backend_material_slots: &'lights [MaterialHandle],
    /// Phase 1C step 1: holds the prepared environment cubemap behind an Arc
    /// so cloning the params per-primitive in the bake loop stays
    /// allocation-free.
    pub(super) environment_lighting: PreparedEnvironmentLighting,
    pub(super) reflection_probe: Option<PreparedReflectionProbe>,
    pub(super) work: Option<&'lights PrepareWorkCounter>,
}

#[derive(Clone, Copy, Default)]
pub(super) struct DeformationInputs<'scene> {
    pub(super) morph_weights: Option<&'scene [f32]>,
    pub(super) skin_matrices: Option<&'scene [SkinningMatrix]>,
}
