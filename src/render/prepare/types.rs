use crate::assets::{Assets, MaterialHandle, TextureHandle};
use crate::geometry::{GeometryDesc, Primitive, SkinningMatrix};
use crate::material::{Color, MaterialDesc};
use crate::scene::{InstanceId, InstanceSetKey, NodeKey, Transform, Vec3};

use super::super::{RasterTarget, camera::CameraProjection};
use super::environment::PreparedEnvironmentLighting;
use super::lighting::PreparedLights;
use super::shadows::ShadowOccluder;

pub(super) struct TransparentPrimitive {
    pub(super) depth: f32,
    pub(super) primitive: PreparedPrimitive,
}

pub(super) struct PrimitiveSinks<'out> {
    pub(super) primitives: &'out mut Vec<PreparedPrimitive>,
    pub(super) strokes: &'out mut Vec<PreparedStrokeSegment>,
    pub(super) transparent_primitives: &'out mut Vec<TransparentPrimitive>,
}

pub(super) struct GeometryPrimitiveSource<'a, F> {
    pub(super) node: NodeKey,
    pub(super) material_handle: MaterialHandle,
    pub(super) geometry: &'a GeometryDesc,
    pub(super) material: &'a MaterialDesc,
    pub(super) assets: &'a Assets<F>,
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
pub(in crate::render) struct PreparedPrimitive {
    primitive: Primitive,
    source_node: Option<NodeKey>,
    original_vertex_offset: u32,
    tint: Color,
    gpu_triangle_path: bool,
    double_sided: bool,
}

impl PreparedPrimitive {
    pub(in crate::render) const fn new(
        primitive: Primitive,
        source_node: Option<NodeKey>,
        tint: Color,
    ) -> Self {
        Self {
            primitive,
            source_node,
            original_vertex_offset: 0,
            tint,
            gpu_triangle_path: true,
            double_sided: false,
        }
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

    pub(in crate::render) const fn primitive(&self) -> &Primitive {
        &self.primitive
    }

    pub(in crate::render) const fn source_node(&self) -> Option<NodeKey> {
        self.source_node
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

    pub(in crate::render) fn vertices(&self) -> &[crate::geometry::Vertex; 3] {
        self.primitive.vertices()
    }

    pub(in crate::render) fn render_material_slot(&self) -> u32 {
        self.primitive.render_material_slot()
    }

    pub(in crate::render) const fn depth_prepass_eligible(&self) -> bool {
        self.primitive.depth_prepass_eligible()
    }

    pub(in crate::render) const fn gpu_triangle_path(&self) -> bool {
        self.gpu_triangle_path
    }

    pub(in crate::render) const fn double_sided(&self) -> bool {
        self.double_sided
    }

    pub(in crate::render) fn world_from_model(&self) -> [f32; 16] {
        self.primitive.world_from_model()
    }

    pub(in crate::render) fn normal_from_model(&self) -> [f32; 16] {
        self.primitive.normal_from_model()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedInstanceSet {
    source_node: NodeKey,
    source_set: InstanceSetKey,
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
            source_set,
            primitives,
            instances,
        }
    }

    pub(in crate::render) const fn source_node(&self) -> NodeKey {
        self.source_node
    }

    pub(in crate::render) const fn source_set(&self) -> InstanceSetKey {
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
    source_instance: InstanceId,
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
            source_instance,
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

    #[cfg(test)]
    pub(in crate::render) const fn source_instance(self) -> InstanceId {
        self.source_instance
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedStrokeSegment {
    source_node: Option<NodeKey>,
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
            start,
            end,
            color,
            width_px,
            world_from_model,
            tint,
            original_segment_index: 0,
        }
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
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedLabelAtlas {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
    quads: Vec<PreparedLabelQuad>,
}

impl PreparedLabelAtlas {
    pub(in crate::render) fn new(
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
        quads: Vec<PreparedLabelQuad>,
    ) -> Self {
        Self {
            width,
            height,
            rgba8,
            quads,
        }
    }

    pub(in crate::render) const fn width(&self) -> u32 {
        self.width
    }

    pub(in crate::render) const fn height(&self) -> u32 {
        self.height
    }

    pub(in crate::render) fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }

    pub(in crate::render) fn quads(&self) -> &[PreparedLabelQuad] {
        &self.quads
    }

    pub(in crate::render) fn quads_mut(&mut self) -> &mut [PreparedLabelQuad] {
        &mut self.quads
    }

    pub(in crate::render) fn set_quads(&mut self, quads: Vec<PreparedLabelQuad>) {
        self.quads = quads;
    }

    pub(in crate::render) fn is_empty(&self) -> bool {
        self.quads.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedLabelQuad {
    source_node: Option<NodeKey>,
    anchor: Vec3,
    right: Vec3,
    up: Vec3,
    world_units_per_px: f32,
    rect_px: [f32; 4],
    uv_rect: [f32; 4],
    color: Color,
    tint: Color,
    solid_coverage: bool,
    original_quad_index: u32,
}

impl PreparedLabelQuad {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::render) const fn new(
        source_node: Option<NodeKey>,
        anchor: Vec3,
        right: Vec3,
        up: Vec3,
        world_units_per_px: f32,
        rect_px: [f32; 4],
        uv_rect: [f32; 4],
        color: Color,
        tint: Color,
    ) -> Self {
        Self {
            source_node,
            anchor,
            right,
            up,
            world_units_per_px,
            rect_px,
            uv_rect,
            color,
            tint,
            solid_coverage: false,
            original_quad_index: 0,
        }
    }

    pub(in crate::render) const fn with_solid_coverage(mut self) -> Self {
        self.solid_coverage = true;
        self
    }

    pub(in crate::render) const fn with_original_quad_index(
        mut self,
        original_quad_index: u32,
    ) -> Self {
        self.original_quad_index = original_quad_index;
        self
    }

    pub(in crate::render) const fn source_node(&self) -> Option<NodeKey> {
        self.source_node
    }

    pub(in crate::render) const fn anchor(&self) -> Vec3 {
        self.anchor
    }

    pub(in crate::render) const fn right(&self) -> Vec3 {
        self.right
    }

    pub(in crate::render) const fn up(&self) -> Vec3 {
        self.up
    }

    pub(in crate::render) const fn world_units_per_px(&self) -> f32 {
        self.world_units_per_px
    }

    pub(in crate::render) const fn rect_px(&self) -> [f32; 4] {
        self.rect_px
    }

    pub(in crate::render) const fn uv_rect(&self) -> [f32; 4] {
        self.uv_rect
    }

    pub(in crate::render) const fn solid_coverage(&self) -> bool {
        self.solid_coverage
    }

    pub(in crate::render) fn set_tint(&mut self, tint: Color) {
        self.tint = tint;
    }

    pub(in crate::render) fn final_color(&self) -> Color {
        Color::from_linear_rgba(
            self.color.r * self.tint.r,
            self.color.g * self.tint.g,
            self.color.b * self.tint.b,
            self.color.a * self.tint.a,
        )
    }
}

#[derive(Clone)]
pub(super) struct PrimitiveBakeParams<'lights> {
    pub(super) target: RasterTarget,
    pub(super) transform: Transform,
    pub(super) origin_shift: Vec3,
    pub(super) lights: &'lights PreparedLights,
    pub(super) shadow_occluders: &'lights [ShadowOccluder],
    pub(super) camera_projection: Option<&'lights CameraProjection>,
    pub(super) backend_sampled_base_color_textures: &'lights [TextureHandle],
    pub(super) backend_material_slots: &'lights [MaterialHandle],
    /// Phase 1C step 1: holds the prepared environment cubemap behind an Arc
    /// so cloning the params per-primitive in the bake loop stays
    /// allocation-free.
    pub(super) environment_lighting: PreparedEnvironmentLighting,
}

#[derive(Clone, Copy, Default)]
pub(super) struct DeformationInputs<'scene> {
    pub(super) morph_weights: Option<&'scene [f32]>,
    pub(super) skin_matrices: Option<&'scene [SkinningMatrix]>,
}
