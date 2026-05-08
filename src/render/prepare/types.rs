use crate::assets::{MaterialHandle, TextureHandle};
use crate::geometry::{Primitive, SkinningMatrix};
use crate::scene::{Transform, Vec3};

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

#[derive(Clone, Copy)]
pub(super) struct PrimitiveBakeParams<'lights> {
    pub(super) target: RasterTarget,
    pub(super) transform: Transform,
    pub(super) origin_shift: Vec3,
    pub(super) lights: &'lights PreparedLights,
    pub(super) shadow_occluders: &'lights [ShadowOccluder],
    pub(super) camera_projection: Option<&'lights CameraProjection>,
    pub(super) backend_sampled_base_color_textures: &'lights [TextureHandle],
    pub(super) backend_material_slots: &'lights [MaterialHandle],
    pub(super) environment_lighting: PreparedEnvironmentLighting,
}

#[derive(Clone, Copy, Default)]
pub(super) struct DeformationInputs<'scene> {
    pub(super) morph_weights: Option<&'scene [f32]>,
    pub(super) skin_matrices: Option<&'scene [SkinningMatrix]>,
}
