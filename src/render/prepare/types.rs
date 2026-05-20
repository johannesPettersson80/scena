use crate::assets::{Assets, MaterialHandle, TextureHandle};
use crate::geometry::{GeometryDesc, Primitive, SkinningMatrix};
use crate::material::MaterialDesc;
use crate::scene::{NodeKey, Transform, Vec3};

use super::super::{RasterTarget, camera::CameraProjection};
use super::environment::PreparedEnvironmentLighting;
use super::lighting::PreparedLights;
use super::shadows::ShadowOccluder;

pub(super) struct TransparentPrimitive {
    pub(super) depth: f32,
    pub(super) primitive: Primitive,
}

pub(super) struct PrimitiveSinks<'out> {
    pub(super) primitives: &'out mut Vec<Primitive>,
    pub(super) transparent_primitives: &'out mut Vec<TransparentPrimitive>,
}

pub(super) struct GeometryPrimitiveSource<'a, F> {
    pub(super) node: NodeKey,
    pub(super) material_handle: MaterialHandle,
    pub(super) geometry: &'a GeometryDesc,
    pub(super) material: &'a MaterialDesc,
    pub(super) assets: &'a Assets<F>,
}

pub(in crate::render) struct PreparedScene {
    pub(in crate::render) primitives: Vec<Primitive>,
    pub(in crate::render) light_from_world: [f32; 16],
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
