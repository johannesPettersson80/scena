//! Materials and textures as the path tracer evaluates them.

use glam::Vec3;

use crate::assets::{Assets, TextureDesc, TextureHandle, TextureWrap};
use crate::material::{
    AlphaMode, MaterialDesc, MaterialKind, SurfaceFinish, TextureTransform, srgb_u8_to_linear,
};

/// Decoded RGBA8 texture with bilinear lookup.
pub(super) struct TraceTexture {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
    srgb: bool,
    wrap: [TextureWrap; 2],
}

impl TraceTexture {
    fn from_desc(desc: &TextureDesc, srgb: bool) -> Option<Self> {
        let (width, height, rgba) = desc.decoded_rgba8()?;
        (width > 0 && height > 0).then(|| Self {
            width: width as usize,
            height: height as usize,
            rgba: rgba.to_vec(),
            srgb,
            wrap: [desc.sampler().wrap_s(), desc.sampler().wrap_t()],
        })
    }

    fn texel(&self, x: i64, y: i64) -> [f32; 4] {
        let wrap = |value: i64, size: usize, mode: TextureWrap| -> usize {
            let size = size as i64;
            (match mode {
                TextureWrap::Repeat => value.rem_euclid(size),
                TextureWrap::ClampToEdge => value.clamp(0, size - 1),
                TextureWrap::MirroredRepeat => {
                    let period = value.rem_euclid(2 * size);
                    if period < size {
                        period
                    } else {
                        2 * size - 1 - period
                    }
                }
            }) as usize
        };
        let offset = (wrap(y, self.height, self.wrap[1]) * self.width
            + wrap(x, self.width, self.wrap[0]))
            * 4;
        let p = &self.rgba[offset..offset + 4];
        let channel = |value: u8| {
            if self.srgb {
                srgb_u8_to_linear(value)
            } else {
                f32::from(value) / 255.0
            }
        };
        [
            channel(p[0]),
            channel(p[1]),
            channel(p[2]),
            f32::from(p[3]) / 255.0,
        ]
    }

    /// Bilinear sample; glTF UVs put `(0, 0)` at the top-left texel.
    pub(super) fn sample(&self, uv: [f32; 2]) -> [f32; 4] {
        let x = uv[0] * self.width as f32 - 0.5;
        let y = uv[1] * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let (a, b) = (self.texel(x0, y0), self.texel(x0 + 1, y0));
        let (c, d) = (self.texel(x0, y0 + 1), self.texel(x0 + 1, y0 + 1));
        std::array::from_fn(|i| {
            (a[i] * (1.0 - fx) + b[i] * fx) * (1.0 - fy) + (c[i] * (1.0 - fx) + d[i] * fx) * fy
        })
    }
}

/// A texture slot: which texture and how its UVs are transformed.
#[derive(Clone, Copy)]
pub(super) struct TextureSlot {
    texture: u32,
    transform: Option<TextureTransform>,
}

impl TextureSlot {
    fn uv(self, uv: [f32; 2]) -> [f32; 2] {
        let Some(transform) = self.transform else {
            return uv;
        };
        // KHR_texture_transform: scale, then rotate, then offset.
        let (scale, offset) = (transform.scale(), transform.offset());
        let (sin, cos) = transform.rotation_radians().sin_cos();
        let (x, y) = (uv[0] * scale[0], uv[1] * scale[1]);
        [
            cos * x + sin * y + offset[0],
            -sin * x + cos * y + offset[1],
        ]
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Coverage {
    Opaque,
    Mask(f32),
    Blend,
}

pub(super) struct TraceMaterial {
    pub(super) unlit: bool,
    pub(super) double_sided: bool,
    pub(super) coverage: Coverage,
    base_color: Vec3,
    base_alpha: f32,
    base_color_texture: Option<TextureSlot>,
    metallic: f32,
    roughness: f32,
    metallic_roughness_texture: Option<TextureSlot>,
    normal_texture: Option<TextureSlot>,
    normal_scale: f32,
    emissive: Vec3,
    emissive_texture: Option<TextureSlot>,
    transmission: f32,
    ior: f32,
    thin_walled: bool,
    attenuation_color: Vec3,
    attenuation_distance: f32,
    clearcoat: f32,
    clearcoat_roughness: f32,
    sheen_color: Vec3,
    sheen_roughness: f32,
    pub(super) finish: Option<SurfaceFinish>,
    /// Physical size of one tile of generated textures, which sets how
    /// often they repeat across a mesh.
    pub(super) tile_size_m: Option<f32>,
}

/// Collects decoded textures, sharing one copy per handle and colour space.
#[derive(Default)]
pub(super) struct TextureStore {
    pub(super) textures: Vec<TraceTexture>,
    seen: Vec<(TextureHandle, bool, u32)>,
}

impl TextureStore {
    fn slot<F>(
        &mut self,
        assets: &Assets<F>,
        handle: Option<TextureHandle>,
        transform: Option<TextureTransform>,
        srgb: bool,
    ) -> Option<TextureSlot> {
        let handle = handle?;
        if let Some((_, _, texture)) = self
            .seen
            .iter()
            .find(|(h, s, _)| *h == handle && *s == srgb)
        {
            return Some(TextureSlot {
                texture: *texture,
                transform,
            });
        }
        let texture = TraceTexture::from_desc(assets.texture_snapshot(handle)?.as_ref(), srgb)?;
        let index = self.textures.len() as u32;
        self.textures.push(texture);
        self.seen.push((handle, srgb, index));
        Some(TextureSlot {
            texture: index,
            transform,
        })
    }
}

fn rgb(color: crate::material::Color) -> Vec3 {
    Vec3::new(color.r, color.g, color.b)
}

impl TraceMaterial {
    pub(super) fn from_desc<F>(
        desc: &MaterialDesc,
        assets: &Assets<F>,
        store: &mut TextureStore,
    ) -> Self {
        let base = desc.base_color();
        Self {
            unlit: !matches!(desc.kind(), MaterialKind::PbrMetallicRoughness),
            double_sided: desc.double_sided(),
            coverage: match desc.alpha_mode() {
                AlphaMode::Opaque => Coverage::Opaque,
                AlphaMode::Mask { cutoff } => Coverage::Mask(cutoff),
                AlphaMode::Blend => Coverage::Blend,
            },
            base_color: rgb(base),
            base_alpha: base.a,
            base_color_texture: store.slot(
                assets,
                desc.base_color_texture(),
                desc.base_color_texture_transform(),
                true,
            ),
            metallic: desc.metallic_factor(),
            roughness: desc.roughness_factor(),
            metallic_roughness_texture: store.slot(
                assets,
                desc.metallic_roughness_texture(),
                desc.metallic_roughness_texture_transform(),
                false,
            ),
            normal_texture: store.slot(
                assets,
                desc.normal_texture(),
                desc.normal_texture_transform(),
                false,
            ),
            normal_scale: desc.normal_scale(),
            emissive: rgb(desc.emissive()) * desc.emissive_strength(),
            emissive_texture: store.slot(
                assets,
                desc.emissive_texture(),
                desc.emissive_texture_transform(),
                true,
            ),
            transmission: desc.transmission_factor().clamp(0.0, 1.0),
            ior: desc.ior().max(1.0),
            thin_walled: desc.thickness_factor() <= 0.0,
            attenuation_color: rgb(desc.attenuation_color()),
            attenuation_distance: desc.attenuation_distance(),
            clearcoat: desc.clearcoat_factor().clamp(0.0, 1.0),
            clearcoat_roughness: desc.clearcoat_roughness_factor(),
            sheen_color: rgb(desc.sheen_color_factor()),
            sheen_roughness: desc.sheen_roughness_factor(),
            finish: desc.surface_finish(),
            tile_size_m: desc.photographic_surface_tile_size_m(),
        }
    }

    /// The material of raw vertex-coloured triangles: unlit and two-sided,
    /// showing the interpolated vertex colour.
    pub(super) fn vertex_colored() -> Self {
        Self {
            unlit: true,
            ..Self::matte(Vec3::ONE)
        }
    }

    /// Whether light passes into the surface, so its back faces matter.
    pub(super) fn transmits(&self) -> bool {
        self.transmission > 0.0
    }

    pub(super) fn ior(&self) -> f32 {
        self.ior
    }

    /// Share of light a shadow ray carries through the surface.
    pub(super) fn shadow_transmission(&self) -> f32 {
        self.transmission * (1.0 - self.metallic.clamp(0.0, 1.0))
    }

    /// Colour light takes on crossing the surface once. A solid is crossed
    /// twice, entering and leaving, so each crossing takes the square root.
    pub(super) fn transmission_tint(&self, textures: &[TraceTexture], uv: [f32; 2]) -> Vec3 {
        let mut base = self.base_color;
        if let Some(texel) = self.sample(self.base_color_texture, textures, uv) {
            base *= Vec3::new(texel[0], texel[1], texel[2]);
        }
        if self.thin_walled {
            base
        } else {
            Vec3::new(base.x.sqrt(), base.y.sqrt(), base.z.sqrt())
        }
    }

    /// Whether shading needs a tangent frame.
    pub(super) fn uses_tangents(&self) -> bool {
        self.normal_texture.is_some()
    }

    /// Brightest emission the surface can show, ignoring its texture.
    pub(super) fn emissive_luminance(&self) -> f32 {
        if self.unlit {
            0.0
        } else {
            super::sampling::luminance(self.emissive)
        }
    }

    /// A plain matte surface.
    pub(super) fn matte(base_color: Vec3) -> Self {
        Self {
            unlit: false,
            double_sided: true,
            coverage: Coverage::Opaque,
            base_color,
            base_alpha: 1.0,
            base_color_texture: None,
            metallic: 0.0,
            roughness: 0.9,
            metallic_roughness_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            emissive: Vec3::ZERO,
            emissive_texture: None,
            transmission: 0.0,
            ior: 1.5,
            thin_walled: true,
            attenuation_color: Vec3::ONE,
            attenuation_distance: f32::INFINITY,
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            sheen_color: Vec3::ZERO,
            sheen_roughness: 0.5,
            finish: None,
            tile_size_m: None,
        }
    }

    fn sample(
        &self,
        slot: Option<TextureSlot>,
        textures: &[TraceTexture],
        uv: [f32; 2],
    ) -> Option<[f32; 4]> {
        let slot = slot?;
        Some(textures[slot.texture as usize].sample(slot.uv(uv)))
    }

    /// Opacity of the surface at `uv`: 1 for opaque materials, 0 or 1 for a
    /// cut-out, fractional for blended ones.
    pub(super) fn opacity(
        &self,
        textures: &[TraceTexture],
        uv: [f32; 2],
        vertex_alpha: f32,
    ) -> f32 {
        if self.coverage == Coverage::Opaque {
            return 1.0;
        }
        let texture_alpha = self
            .sample(self.base_color_texture, textures, uv)
            .map_or(1.0, |texel| texel[3]);
        let alpha = self.base_alpha * texture_alpha * vertex_alpha;
        match self.coverage {
            Coverage::Mask(cutoff) => f32::from(alpha >= cutoff),
            _ => alpha.clamp(0.0, 1.0),
        }
    }

    /// Light a volume of this material absorbs over `distance`.
    pub(super) fn transmittance(&self, distance: f32) -> Vec3 {
        if self.thin_walled
            || !self.attenuation_distance.is_finite()
            || self.attenuation_distance <= 0.0
        {
            return Vec3::ONE;
        }
        let exponent = distance / self.attenuation_distance;
        Vec3::new(
            self.attenuation_color.x.max(1.0e-4).powf(exponent),
            self.attenuation_color.y.max(1.0e-4).powf(exponent),
            self.attenuation_color.z.max(1.0e-4).powf(exponent),
        )
    }
}

/// Everything the integrator needs to know about one surface point.
pub(super) struct Surface {
    pub(super) base: Vec3,
    pub(super) metallic: f32,
    /// GGX width, roughness squared.
    pub(super) alpha: f32,
    pub(super) f0: Vec3,
    pub(super) clearcoat: f32,
    pub(super) clearcoat_alpha: f32,
    pub(super) sheen: Vec3,
    pub(super) sheen_alpha: f32,
    pub(super) transmission: f32,
    pub(super) ior: f32,
    pub(super) thin_walled: bool,
    pub(super) emission: Vec3,
    /// Tangent-space normal from the normal map, if any.
    pub(super) normal_map: Option<Vec3>,
    pub(super) unlit: bool,
}

/// Lowest roughness traced as a rough lobe; below it highlights alias badly
/// and the sampling densities lose precision.
pub(super) const MIN_ROUGHNESS: f32 = 0.03;

impl TraceMaterial {
    pub(super) fn surface(
        &self,
        textures: &[TraceTexture],
        uv: [f32; 2],
        vertex_color: Vec3,
    ) -> Surface {
        let mut base = self.base_color * vertex_color;
        if let Some(texel) = self.sample(self.base_color_texture, textures, uv) {
            base *= Vec3::new(texel[0], texel[1], texel[2]);
        }
        let (mut metallic, mut roughness) = (self.metallic, self.roughness);
        if let Some(texel) = self.sample(self.metallic_roughness_texture, textures, uv) {
            roughness *= texel[1];
            metallic *= texel[2];
        }
        let metallic = metallic.clamp(0.0, 1.0);
        let roughness = roughness.clamp(MIN_ROUGHNESS, 1.0);
        let mut emission = self.emissive;
        if let Some(texel) = self.sample(self.emissive_texture, textures, uv) {
            emission *= Vec3::new(texel[0], texel[1], texel[2]);
        }
        let dielectric_f0 = ((self.ior - 1.0) / (self.ior + 1.0)).powi(2);
        Surface {
            base,
            metallic,
            alpha: roughness * roughness,
            f0: Vec3::splat(dielectric_f0).lerp(base, metallic),
            clearcoat: self.clearcoat,
            clearcoat_alpha: self.clearcoat_roughness.clamp(MIN_ROUGHNESS, 1.0).powi(2),
            sheen: self.sheen_color,
            sheen_alpha: self.sheen_roughness.clamp(0.07, 1.0).powi(2),
            transmission: self.transmission * (1.0 - metallic),
            ior: self.ior,
            thin_walled: self.thin_walled,
            emission,
            normal_map: self.sample(self.normal_texture, textures, uv).map(|texel| {
                Vec3::new(
                    (texel[0] * 2.0 - 1.0) * self.normal_scale,
                    (texel[1] * 2.0 - 1.0) * self.normal_scale,
                    texel[2] * 2.0 - 1.0,
                )
            }),
            unlit: self.unlit,
        }
    }
}
