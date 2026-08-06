mod generator;
mod profile;

use generator::generate_surface_maps;
use profile::SurfaceProfile;

use crate::diagnostics::AssetError;
use crate::material::{Color, MaterialDesc, TextureTransform};

use super::{
    AssetMaterialSource, Assets, MaterialHandle, TextureFilter, TextureHandle, TextureMemoryDesc,
    TextureMemoryId, TextureMipPolicy, TextureSamplerDesc, TextureSlot, TextureWrap,
};

pub(super) type GeneratedFinishStore =
    std::collections::BTreeMap<PhotographicSurfaceKey, PhotographicSurfaceAssets>;

const SURFACE_GENERATOR_VERSION: u8 = 1;
const MIN_RESOLUTION: u32 = 16;
const MAX_RESOLUTION: u32 = 1_024;

/// A manufactured surface model synthesized by scena into ordinary PBR maps.
///
/// These are rendering descriptions, not manufacturing or CAD operations.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PhotographicSurfaceKind {
    PolishedMetal,
    SatinMetal,
    BrushedMetal,
    MachinedMetal,
    CastMetal,
    PaintedMetal,
    PowderCoatedMetal,
    MoldedPlastic,
    ClearcoatPlastic,
    Rubber,
    Fabric,
}

impl PhotographicSurfaceKind {
    pub const NAMES: &'static [&'static str] = &[
        "polished_metal",
        "satin_metal",
        "brushed_metal",
        "machined_metal",
        "cast_metal",
        "painted_metal",
        "powder_coated_metal",
        "molded_plastic",
        "clearcoat_plastic",
        "rubber",
        "fabric",
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PolishedMetal => "polished_metal",
            Self::SatinMetal => "satin_metal",
            Self::BrushedMetal => "brushed_metal",
            Self::MachinedMetal => "machined_metal",
            Self::CastMetal => "cast_metal",
            Self::PaintedMetal => "painted_metal",
            Self::PowderCoatedMetal => "powder_coated_metal",
            Self::MoldedPlastic => "molded_plastic",
            Self::ClearcoatPlastic => "clearcoat_plastic",
            Self::Rubber => "rubber",
            Self::Fabric => "fabric",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "polished_metal" => Some(Self::PolishedMetal),
            "satin_metal" => Some(Self::SatinMetal),
            "brushed_metal" => Some(Self::BrushedMetal),
            "machined_metal" => Some(Self::MachinedMetal),
            "cast_metal" => Some(Self::CastMetal),
            "painted_metal" => Some(Self::PaintedMetal),
            "powder_coated_metal" => Some(Self::PowderCoatedMetal),
            "molded_plastic" => Some(Self::MoldedPlastic),
            "clearcoat_plastic" => Some(Self::ClearcoatPlastic),
            "rubber" => Some(Self::Rubber),
            "fabric" => Some(Self::Fabric),
            _ => None,
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::PolishedMetal => 0,
            Self::SatinMetal => 10,
            Self::BrushedMetal => 1,
            Self::MachinedMetal => 2,
            Self::CastMetal => 3,
            Self::PaintedMetal => 4,
            Self::PowderCoatedMetal => 5,
            Self::MoldedPlastic => 6,
            Self::ClearcoatPlastic => 7,
            Self::Rubber => 8,
            Self::Fabric => 9,
        }
    }
}

/// Physical and artistic controls for a deterministic scena-generated surface.
///
/// The texture tile spans `tile_size_m` in both directions. Until a
/// world-projected mapping is selected by the scene, imported UVs should map a
/// 0..1 tile to that physical span.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhotographicSurfaceDesc {
    kind: PhotographicSurfaceKind,
    base_color: Color,
    tile_size_m: f32,
    feature_scale_m: f32,
    metallic: Option<f32>,
    roughness: Option<f32>,
    variation: f32,
    wear: f32,
    seed: u64,
    resolution: u32,
}

impl PhotographicSurfaceDesc {
    pub fn new(kind: PhotographicSurfaceKind, base_color: Color) -> Self {
        let profile = SurfaceProfile::for_kind(kind);
        Self {
            kind,
            base_color: sanitize_color(base_color),
            tile_size_m: 0.1,
            feature_scale_m: profile.default_feature_scale_m,
            metallic: None,
            roughness: None,
            variation: 0.5,
            wear: 0.0,
            seed: 0,
            resolution: 256,
        }
    }

    pub const fn kind(self) -> PhotographicSurfaceKind {
        self.kind
    }

    pub const fn base_color(self) -> Color {
        self.base_color
    }

    pub const fn tile_size_m(self) -> f32 {
        self.tile_size_m
    }

    pub const fn feature_scale_m(self) -> f32 {
        self.feature_scale_m
    }

    pub const fn metallic(self) -> Option<f32> {
        self.metallic
    }

    pub const fn roughness(self) -> Option<f32> {
        self.roughness
    }

    pub const fn variation(self) -> f32 {
        self.variation
    }

    pub const fn wear(self) -> f32 {
        self.wear
    }

    pub const fn seed(self) -> u64 {
        self.seed
    }

    pub const fn resolution(self) -> u32 {
        self.resolution
    }

    pub fn with_base_color(mut self, base_color: Color) -> Self {
        self.base_color = sanitize_color(base_color);
        self
    }

    pub fn with_tile_size_m(mut self, tile_size_m: f32) -> Self {
        self.tile_size_m = sanitize_positive(tile_size_m, 0.1).clamp(0.001, 10.0);
        self.feature_scale_m = self.feature_scale_m.min(self.tile_size_m);
        self
    }

    pub fn with_feature_scale_m(mut self, feature_scale_m: f32) -> Self {
        self.feature_scale_m = sanitize_positive(feature_scale_m, self.feature_scale_m)
            .clamp(1.0e-6, self.tile_size_m);
        self
    }

    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = Some(sanitize_unit(metallic, 0.0));
        self
    }

    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = Some(sanitize_unit(roughness, 1.0));
        self
    }

    pub fn with_variation(mut self, variation: f32) -> Self {
        self.variation = sanitize_unit(variation, 0.5);
        self
    }

    pub fn with_wear(mut self, wear: f32) -> Self {
        self.wear = sanitize_unit(wear, 0.0);
        self
    }

    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = sanitize_resolution(resolution);
        self
    }

    fn sanitized(mut self) -> Self {
        self.base_color = sanitize_color(self.base_color);
        self.tile_size_m = sanitize_positive(self.tile_size_m, 0.1).clamp(0.001, 10.0);
        self.feature_scale_m = sanitize_positive(
            self.feature_scale_m,
            SurfaceProfile::for_kind(self.kind).default_feature_scale_m,
        )
        .clamp(1.0e-6, self.tile_size_m);
        self.metallic = self.metallic.map(|value| sanitize_unit(value, 0.0));
        self.roughness = self.roughness.map(|value| sanitize_unit(value, 1.0));
        self.variation = sanitize_unit(self.variation, 0.5);
        self.wear = sanitize_unit(self.wear, 0.0);
        self.resolution = sanitize_resolution(self.resolution);
        self
    }
}

/// Handles produced for one synthesized surface.
///
/// The occlusion texture is the same packed linear texture as
/// `metallic_roughness_texture`; its red, green, and blue channels hold
/// occlusion, roughness, and metallic respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhotographicSurfaceAssets {
    material: MaterialHandle,
    base_color_texture: TextureHandle,
    normal_texture: TextureHandle,
    metallic_roughness_texture: TextureHandle,
}

impl PhotographicSurfaceAssets {
    pub const fn material(self) -> MaterialHandle {
        self.material
    }

    pub const fn base_color_texture(self) -> TextureHandle {
        self.base_color_texture
    }

    pub const fn normal_texture(self) -> TextureHandle {
        self.normal_texture
    }

    pub const fn metallic_roughness_texture(self) -> TextureHandle {
        self.metallic_roughness_texture
    }

    pub const fn occlusion_texture(self) -> TextureHandle {
        self.metallic_roughness_texture
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PhotographicSurfaceKey {
    kind: u8,
    color_bits: [u32; 4],
    tile_size_bits: u32,
    feature_scale_bits: u32,
    metallic_bits: Option<u32>,
    roughness_bits: Option<u32>,
    variation_bits: u32,
    wear_bits: u32,
    seed: u64,
    resolution: u32,
}

impl PhotographicSurfaceKey {
    fn new(descriptor: PhotographicSurfaceDesc) -> Self {
        Self {
            kind: descriptor.kind.code(),
            color_bits: [
                descriptor.base_color.r.to_bits(),
                descriptor.base_color.g.to_bits(),
                descriptor.base_color.b.to_bits(),
                descriptor.base_color.a.to_bits(),
            ],
            tile_size_bits: descriptor.tile_size_m.to_bits(),
            feature_scale_bits: descriptor.feature_scale_m.to_bits(),
            metallic_bits: descriptor.metallic.map(f32::to_bits),
            roughness_bits: descriptor.roughness.map(f32::to_bits),
            variation_bits: descriptor.variation.to_bits(),
            wear_bits: descriptor.wear.to_bits(),
            seed: descriptor.seed,
            resolution: descriptor.resolution,
        }
    }

    fn digest(self) -> String {
        let mut bytes = Vec::with_capacity(49);
        bytes.push(SURFACE_GENERATOR_VERSION);
        bytes.push(self.kind);
        for bits in self.color_bits {
            bytes.extend_from_slice(&bits.to_le_bytes());
        }
        bytes.extend_from_slice(&self.tile_size_bits.to_le_bytes());
        bytes.extend_from_slice(&self.feature_scale_bits.to_le_bytes());
        push_optional_bits(&mut bytes, self.metallic_bits);
        push_optional_bits(&mut bytes, self.roughness_bits);
        bytes.extend_from_slice(&self.variation_bits.to_le_bytes());
        bytes.extend_from_slice(&self.wear_bits.to_le_bytes());
        bytes.extend_from_slice(&self.seed.to_le_bytes());
        bytes.extend_from_slice(&self.resolution.to_le_bytes());
        super::provenance::sha256_hex(&bytes)
    }
}

impl<F> Assets<F> {
    /// Synthesizes or reuses a physically scaled PBR surface before prepare.
    ///
    /// The returned handles are ordinary scena material and texture handles,
    /// so CPU, native GPU, WebGPU, and WebGL2 paths consume the same maps.
    pub fn create_photographic_surface(
        &self,
        descriptor: PhotographicSurfaceDesc,
    ) -> Result<PhotographicSurfaceAssets, AssetError> {
        let descriptor = descriptor.sanitized();
        let key = PhotographicSurfaceKey::new(descriptor);
        if let Some(cached) = self
            .storage()
            .photographic_surface_lookup
            .get(&key)
            .copied()
        {
            return Ok(cached);
        }

        let generated = generate_surface_maps(descriptor);
        let digest = key.digest();
        let sampler = TextureSamplerDesc::new(
            Some(TextureFilter::Linear),
            Some(TextureFilter::LinearMipmapLinear),
            TextureWrap::Repeat,
            TextureWrap::Repeat,
        );
        let texture = |slot_name: &str,
                       slot: TextureSlot,
                       pixels: Vec<u8>|
         -> Result<TextureHandle, AssetError> {
            let identity = TextureMemoryId::new(format!(
                "scena/photographic-surface/v{SURFACE_GENERATOR_VERSION}/{digest}/{slot_name}"
            ))?;
            self.create_texture_for_slot(
                TextureMemoryDesc::rgba8_for_slot(
                    identity,
                    descriptor.resolution,
                    descriptor.resolution,
                    pixels,
                    slot,
                )
                .with_sampler(sampler)
                .with_mip_policy(TextureMipPolicy::Generate),
                slot,
            )
        };
        let base_color_texture =
            texture("base-color", TextureSlot::BaseColor, generated.base_color)?;
        let normal_texture = texture("normal", TextureSlot::Normal, generated.normal)?;
        let metallic_roughness_texture = texture(
            "orm",
            TextureSlot::MetallicRoughness,
            generated.occlusion_roughness_metallic,
        )?;

        let profile = SurfaceProfile::for_kind(descriptor.kind);
        let transform = TextureTransform::new([0.0, 0.0], 0.0, [1.0, 1.0]);
        let mut material = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 1.0, 1.0)
            .with_base_color_texture(base_color_texture)
            .with_base_color_texture_transform(transform)
            .with_normal_texture(normal_texture)
            .with_normal_texture_transform(transform)
            .with_metallic_roughness_texture(metallic_roughness_texture)
            .with_metallic_roughness_texture_transform(transform)
            .with_occlusion_texture(metallic_roughness_texture)
            .with_occlusion_texture_transform(transform)
            .with_normal_scale(profile.normal_scale)
            .with_occlusion_strength(profile.occlusion_strength)
            .with_photographic_surface_tile_size_m(descriptor.tile_size_m);
        if profile.clearcoat_factor > 0.0 {
            material = material
                .with_clearcoat_factor(profile.clearcoat_factor)
                .with_clearcoat_roughness_factor(profile.clearcoat_roughness);
        }

        let mut storage = self.storage();
        if let Some(cached) = storage.photographic_surface_lookup.get(&key).copied() {
            return Ok(cached);
        }
        let material_handle = storage.materials.insert(std::sync::Arc::new(material));
        storage
            .material_sources
            .insert(material_handle, AssetMaterialSource::user_created());
        storage.user_created_materials.insert(material_handle);
        let assets = PhotographicSurfaceAssets {
            material: material_handle,
            base_color_texture,
            normal_texture,
            metallic_roughness_texture,
        };
        storage.photographic_surface_lookup.insert(key, assets);
        Ok(assets)
    }
}

fn push_optional_bits(output: &mut Vec<u8>, bits: Option<u32>) {
    output.push(u8::from(bits.is_some()));
    output.extend_from_slice(&bits.unwrap_or_default().to_le_bytes());
}

fn sanitize_color(color: Color) -> Color {
    Color::from_linear_rgba(
        sanitize_unit(color.r, 0.5),
        sanitize_unit(color.g, 0.5),
        sanitize_unit(color.b, 0.5),
        sanitize_unit(color.a, 1.0),
    )
}

fn sanitize_unit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

fn sanitize_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn sanitize_resolution(resolution: u32) -> u32 {
    resolution
        .max(MIN_RESOLUTION)
        .next_power_of_two()
        .clamp(MIN_RESOLUTION, MAX_RESOLUTION)
}
