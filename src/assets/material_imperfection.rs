#[cfg(any(feature = "scene-host", test))]
use crate::diagnostics::AssetError;
#[cfg(any(feature = "scene-host", test))]
use crate::material::MaterialDesc;

#[cfg(any(feature = "scene-host", test))]
use super::{Assets, TextureMemoryDesc, TextureMemoryId, TextureSlot, provenance};

#[cfg(any(feature = "scene-host", test))]
const IMPERFECTION_GENERATOR_VERSION: u8 = 1;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MaterialImperfectionProfileV1 {
    Dust,
    Smudge,
    FineScratches,
    OilFilm,
}

impl MaterialImperfectionProfileV1 {
    pub const NAMES: &'static [&'static str] = &["dust", "smudge", "fine_scratches", "oil_film"];

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "dust" => Some(Self::Dust),
            "smudge" => Some(Self::Smudge),
            "fine_scratches" => Some(Self::FineScratches),
            "oil_film" => Some(Self::OilFilm),
            _ => None,
        }
    }

    #[cfg(any(feature = "scene-host", test))]
    const fn code(self) -> u8 {
        match self {
            Self::Dust => 0,
            Self::Smudge => 1,
            Self::FineScratches => 2,
            Self::OilFilm => 3,
        }
    }

    #[cfg(any(feature = "scene-host", test))]
    pub(crate) const fn replaces_normal_texture(self) -> bool {
        !matches!(self, Self::OilFilm)
    }

    #[cfg(feature = "scene-host")]
    pub(crate) const fn replacement_texture_count(self) -> usize {
        if self.replaces_normal_texture() { 2 } else { 1 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialImperfectionDesc {
    profile: MaterialImperfectionProfileV1,
    strength: f32,
    physical_scale_m: f32,
    seed: u64,
}

impl MaterialImperfectionDesc {
    pub fn new(profile: MaterialImperfectionProfileV1) -> Self {
        Self {
            profile,
            strength: Self::default_strength(profile),
            physical_scale_m: 0.003,
            seed: 0,
        }
    }

    const fn default_strength(profile: MaterialImperfectionProfileV1) -> f32 {
        match profile {
            MaterialImperfectionProfileV1::Dust => 0.30,
            MaterialImperfectionProfileV1::Smudge => 0.40,
            MaterialImperfectionProfileV1::FineScratches => 0.30,
            MaterialImperfectionProfileV1::OilFilm => 0.65,
        }
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = if strength.is_finite() {
            strength.clamp(0.0, 1.0)
        } else {
            Self::default_strength(self.profile)
        };
        self
    }

    pub fn with_physical_scale_m(mut self, physical_scale_m: f32) -> Self {
        self.physical_scale_m = if physical_scale_m.is_finite() && physical_scale_m > 0.0 {
            physical_scale_m.clamp(1.0e-6, 10.0)
        } else {
            0.003
        };
        self
    }

    pub const fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
}

#[cfg(any(feature = "scene-host", test))]
impl<F> Assets<F> {
    pub(crate) fn composite_material_imperfection(
        &self,
        material: MaterialDesc,
        descriptor: MaterialImperfectionDesc,
    ) -> Result<MaterialDesc, AssetError> {
        let normal_handle = material.normal_texture().ok_or_else(|| AssetError::Parse {
            path: "material.imperfection".to_owned(),
            reason: "imperfection requires existing normal material data".to_owned(),
        })?;
        let orm_handle =
            material
                .metallic_roughness_texture()
                .ok_or_else(|| AssetError::Parse {
                    path: "material.imperfection".to_owned(),
                    reason: "imperfection requires existing roughness material data".to_owned(),
                })?;
        let normal_source = self.try_texture(normal_handle)?;
        let orm_source = self.try_texture(orm_handle)?;
        let (width, height, normal_pixels) = normal_source
            .decoded_rgba8()
            .ok_or_else(|| undecoded_texture_error(&normal_source))?;
        let (orm_width, orm_height, orm_pixels) = orm_source
            .decoded_rgba8()
            .ok_or_else(|| undecoded_texture_error(&orm_source))?;
        if (width, height) != (orm_width, orm_height) {
            return Err(AssetError::Parse {
                path: "material.imperfection".to_owned(),
                reason: "normal and roughness maps must have matching dimensions".to_owned(),
            });
        }

        let tile_size_m = material.photographic_surface_tile_size_m().unwrap_or(0.1);
        let signal = imperfection_signal(width, height, tile_size_m, descriptor);
        let orm = composite_roughness_map(orm_pixels, &signal, descriptor);
        let digest = imperfection_digest(&normal_source, &orm_source, tile_size_m, descriptor);
        let normal_texture = if descriptor.profile.replaces_normal_texture() {
            let normal =
                composite_normal_map(width, height, normal_pixels, &signal, descriptor.strength);
            self.create_texture_for_slot(
                TextureMemoryDesc::rgba8_for_slot(
                    TextureMemoryId::new(format!(
                        "scena/material-imperfection/v{IMPERFECTION_GENERATOR_VERSION}/{digest}/normal"
                    ))?,
                    width,
                    height,
                    normal,
                    TextureSlot::Normal,
                )
                .with_sampler(normal_source.sampler())
                .with_mip_policy(normal_source.mip_policy()),
                TextureSlot::Normal,
            )?
        } else {
            normal_handle
        };
        let orm_texture = self.create_texture_for_slot(
            TextureMemoryDesc::rgba8_for_slot(
                TextureMemoryId::new(format!(
                    "scena/material-imperfection/v{IMPERFECTION_GENERATOR_VERSION}/{digest}/orm"
                ))?,
                width,
                height,
                orm,
                TextureSlot::MetallicRoughness,
            )
            .with_sampler(orm_source.sampler())
            .with_mip_policy(orm_source.mip_policy()),
            TextureSlot::MetallicRoughness,
        )?;

        Ok(material
            .with_normal_texture(normal_texture)
            .with_metallic_roughness_texture(orm_texture)
            .with_occlusion_texture(orm_texture))
    }
}

#[cfg(any(feature = "scene-host", test))]
fn undecoded_texture_error(texture: &super::TextureDesc) -> AssetError {
    AssetError::Parse {
        path: texture.path().as_str().to_owned(),
        reason: "imperfection composition requires decoded RGBA8 material data".to_owned(),
    }
}

#[cfg(any(feature = "scene-host", test))]
fn imperfection_digest(
    normal: &super::TextureDesc,
    orm: &super::TextureDesc,
    tile_size_m: f32,
    descriptor: MaterialImperfectionDesc,
) -> String {
    let mut bytes = vec![IMPERFECTION_GENERATOR_VERSION, descriptor.profile.code()];
    bytes.extend_from_slice(&descriptor.strength.to_bits().to_le_bytes());
    bytes.extend_from_slice(&descriptor.physical_scale_m.to_bits().to_le_bytes());
    bytes.extend_from_slice(&descriptor.seed.to_le_bytes());
    bytes.extend_from_slice(&tile_size_m.to_bits().to_le_bytes());
    for texture in [normal, orm] {
        bytes.extend_from_slice(texture.path().as_str().as_bytes());
        if let Some(hash) = texture.provenance().source_sha256() {
            bytes.extend_from_slice(hash.as_bytes());
        }
    }
    provenance::sha256_hex(&bytes)
}

#[cfg(any(feature = "scene-host", test))]
fn imperfection_signal(
    width: u32,
    height: u32,
    tile_size_m: f32,
    descriptor: MaterialImperfectionDesc,
) -> Vec<f32> {
    let period = (tile_size_m / descriptor.physical_scale_m)
        .round()
        .clamp(2.0, (width.min(height) / 2).max(2) as f32) as u32;
    let mut signal = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height {
        for x in 0..width {
            let u = x as f32 / width as f32;
            let v = y as f32 / height as f32;
            let value = match descriptor.profile {
                MaterialImperfectionProfileV1::Dust => {
                    ((periodic_noise(u, v, period, period, descriptor.seed) - 0.70) / 0.30)
                        .clamp(0.0, 1.0)
                        .powi(2)
                }
                MaterialImperfectionProfileV1::Smudge => {
                    let broad = periodic_noise(
                        u,
                        v,
                        (period / 4).max(2),
                        (period / 8).max(2),
                        descriptor.seed ^ 0x6f18_2d95_44b7_a023,
                    );
                    smoothstep((broad - 0.32) / 0.45)
                }
                MaterialImperfectionProfileV1::FineScratches => {
                    let line = ((u * period as f32 + v * 0.18).fract() - 0.5).abs();
                    let gate = periodic_noise(
                        u,
                        v,
                        (period / 3).max(2),
                        2,
                        descriptor.seed ^ 0x9ab1_c7d3_214f_6805,
                    );
                    ((0.055 - line) / 0.055).clamp(0.0, 1.0) * smoothstep((gate - 0.55) / 0.35)
                }
                MaterialImperfectionProfileV1::OilFilm => periodic_noise(
                    u,
                    v,
                    (period / 6).max(2),
                    (period / 6).max(2),
                    descriptor.seed ^ 0x35de_019b_a426_77c1,
                ),
            };
            signal.push(value);
        }
    }
    signal
}

#[cfg(any(feature = "scene-host", test))]
fn composite_normal_map(
    width: u32,
    height: u32,
    source: &[u8],
    signal: &[f32],
    strength: f32,
) -> Vec<u8> {
    let mut output = source.to_vec();
    let gain = strength * 0.32;
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            let left = signal[(y * width + (x + width - 1) % width) as usize];
            let right = signal[(y * width + (x + 1) % width) as usize];
            let down = signal[(((y + height - 1) % height) * width + x) as usize];
            let up = signal[(((y + 1) % height) * width + x) as usize];
            let nx = source[index * 4] as f32 / 127.5 - 1.0 - (right - left) * gain;
            let ny = source[index * 4 + 1] as f32 / 127.5 - 1.0 - (up - down) * gain;
            let nz = (source[index * 4 + 2] as f32 / 127.5 - 1.0).max(0.05);
            let inverse_length = (nx * nx + ny * ny + nz * nz).sqrt().recip();
            output[index * 4] = encode_normal(nx * inverse_length);
            output[index * 4 + 1] = encode_normal(ny * inverse_length);
            output[index * 4 + 2] = encode_normal(nz * inverse_length);
        }
    }
    output
}

#[cfg(any(feature = "scene-host", test))]
fn composite_roughness_map(
    source: &[u8],
    signal: &[f32],
    descriptor: MaterialImperfectionDesc,
) -> Vec<u8> {
    let mut output = source.to_vec();
    for (index, value) in signal.iter().copied().enumerate() {
        let delta = match descriptor.profile {
            MaterialImperfectionProfileV1::Dust => value * 0.20,
            MaterialImperfectionProfileV1::Smudge => -value * 0.14,
            MaterialImperfectionProfileV1::FineScratches => value * 0.18,
            MaterialImperfectionProfileV1::OilFilm => (value - 0.5) * 0.16,
        } * descriptor.strength;
        let roughness = source[index * 4 + 1] as f32 / 255.0;
        output[index * 4 + 1] = ((roughness + delta).clamp(0.045, 0.96) * 255.0).round() as u8;
    }
    output
}

#[cfg(any(feature = "scene-host", test))]
fn periodic_noise(u: f32, v: f32, period_x: u32, period_y: u32, seed: u64) -> f32 {
    let x = u * period_x as f32;
    let y = v * period_y as f32;
    let x0 = x.floor() as u32 % period_x;
    let y0 = y.floor() as u32 % period_y;
    let x1 = (x0 + 1) % period_x;
    let y1 = (y0 + 1) % period_y;
    let tx = smoothstep(x.fract());
    let ty = smoothstep(y.fract());
    let bottom = lerp(lattice_noise(x0, y0, seed), lattice_noise(x1, y0, seed), tx);
    let top = lerp(lattice_noise(x0, y1, seed), lattice_noise(x1, y1, seed), tx);
    lerp(bottom, top, ty)
}

#[cfg(any(feature = "scene-host", test))]
fn lattice_noise(x: u32, y: u32, seed: u64) -> f32 {
    let mut value = seed
        ^ u64::from(x).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ u64::from(y).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    (value >> 40) as f32 / ((1_u32 << 24) - 1) as f32
}

#[cfg(any(feature = "scene-host", test))]
fn smoothstep(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

#[cfg(any(feature = "scene-host", test))]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(any(feature = "scene-host", test))]
fn encode_normal(value: f32) -> u8 {
    ((value.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8
}
