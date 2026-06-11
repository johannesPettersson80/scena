use crate::assets::{Assets, MaterialHandle, TextureHandle};
use crate::geometry::{GeometryDesc, Primitive, SkinningMatrix};
use crate::material::{Color, MaterialDesc};
use crate::scene::{NodeKey, Transform, Vec3};

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
    pub(in crate::render) light_from_world: [f32; 16],
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedPrimitive {
    primitive: Primitive,
    source_node: Option<NodeKey>,
    original_vertex_offset: u32,
    tint: Color,
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

    pub(in crate::render) fn world_from_model(&self) -> [f32; 16] {
        self.primitive.world_from_model()
    }

    pub(in crate::render) fn normal_from_model(&self) -> [f32; 16] {
        self.primitive.normal_from_model()
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
