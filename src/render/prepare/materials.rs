use std::collections::BTreeMap;
use std::sync::Arc;

use crate::assets::{Assets, MaterialHandle, TextureDesc, TextureHandle};
use crate::diagnostics::PrepareError;
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind, TextureTransform};
use crate::scene::{NodeKey, Vec3};

#[derive(Clone, Copy)]
pub(super) enum MaterialPass {
    Opaque,
    Blend,
    Mask { cutoff: f32 },
}

pub(super) trait TextureSampleSource {
    fn sample_texture(&self, handle: TextureHandle, uv: [f32; 2]) -> Option<Color>;
}

impl<F> TextureSampleSource for Assets<F> {
    fn sample_texture(&self, handle: TextureHandle, uv: [f32; 2]) -> Option<Color> {
        Assets::sample_texture(self, handle, uv)
    }
}

pub(super) struct PreparedMaterialTextures {
    textures: BTreeMap<TextureHandle, Arc<TextureDesc>>,
}

impl PreparedMaterialTextures {
    pub(super) fn new<F>(assets: &Assets<F>, material: &MaterialDesc) -> Self {
        let handles = material_texture_slots(material).map(|(_, handle)| handle);
        Self {
            textures: assets.texture_snapshots(handles),
        }
    }

    pub(super) fn max_decoded_dimension(&self) -> u32 {
        self.textures
            .values()
            .filter_map(|texture| texture.decoded_dimensions())
            .map(|(width, height)| width.max(height))
            .max()
            .unwrap_or(0)
    }
}

impl TextureSampleSource for PreparedMaterialTextures {
    fn sample_texture(&self, handle: TextureHandle, uv: [f32; 2]) -> Option<Color> {
        self.textures
            .get(&handle)
            .and_then(|texture| texture.sample_bilinear(uv))
    }
}

pub(super) fn material_pass(
    node: NodeKey,
    material: &MaterialDesc,
) -> Result<MaterialPass, PrepareError> {
    match material.kind() {
        MaterialKind::Unlit | MaterialKind::PbrMetallicRoughness => {}
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge => {
            return Err(PrepareError::UnsupportedMaterialKind {
                node,
                kind: material.kind(),
            });
        }
    }

    if material_requires_scene_color_transmission(material) {
        return Ok(MaterialPass::Blend);
    }

    match material.alpha_mode() {
        AlphaMode::Opaque => Ok(MaterialPass::Opaque),
        AlphaMode::Blend => Ok(MaterialPass::Blend),
        AlphaMode::Mask { cutoff } => Ok(MaterialPass::Mask { cutoff }),
    }
}

fn material_requires_scene_color_transmission(material: &MaterialDesc) -> bool {
    matches!(material.kind(), MaterialKind::PbrMetallicRoughness)
        && material.transmission_factor() > 0.0
}

pub(super) fn validate_material_texture_handles(
    node: NodeKey,
    material_handle: MaterialHandle,
    material: &MaterialDesc,
    assets: &Assets<impl Sized>,
) -> Result<(), PrepareError> {
    for (slot, texture) in material_texture_slots(material) {
        if assets.texture(texture).is_none() {
            return Err(PrepareError::TextureNotFound {
                node,
                material: material_handle,
                texture,
                slot,
            });
        }
    }
    Ok(())
}

pub(super) fn base_color_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
    backend_sampled_base_color_textures: &[TextureHandle],
) -> Color {
    let Some(texture) = material.base_color_texture() else {
        return Color::WHITE;
    };
    if backend_sampled_base_color_textures.contains(&texture) {
        return Color::WHITE;
    }
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.base_color_texture_transform()),
        )
        .unwrap_or(Color::WHITE)
}

pub(super) fn emissive_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> Color {
    let Some(texture) = material.emissive_texture() else {
        return Color::WHITE;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.emissive_texture_transform()),
        )
        .unwrap_or(Color::WHITE)
}

pub(super) fn normal_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
    geometric_normal: Vec3,
    tangent: Vec3,
    tangent_handedness: f32,
) -> Vec3 {
    let Some(texture) = material.normal_texture() else {
        return geometric_normal;
    };
    let Some(sample) = assets.sample_texture(
        texture,
        transform_texture_uv(uv, material.normal_texture_transform()),
    ) else {
        return geometric_normal;
    };
    tangent_space_normal_from_sample(sample, material.normal_scale())
        .map(|normal| tangent_space_to_world(normal, geometric_normal, tangent, tangent_handedness))
        .unwrap_or(geometric_normal)
}

pub(super) fn clearcoat_normal_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
    fallback: Vec3,
    geometric_normal: Vec3,
    tangent: Vec3,
    tangent_handedness: f32,
) -> Vec3 {
    let Some(texture) = material.clearcoat_normal_texture() else {
        return fallback;
    };
    let Some(sample) = assets.sample_texture(
        texture,
        transform_texture_uv(uv, material.clearcoat_normal_texture_transform()),
    ) else {
        return fallback;
    };
    tangent_space_normal_from_sample(sample, material.clearcoat_normal_scale())
        .map(|normal| tangent_space_to_world(normal, geometric_normal, tangent, tangent_handedness))
        .unwrap_or(fallback)
}

fn tangent_space_normal_from_sample(sample: Color, scale: f32) -> Option<Vec3> {
    let scale = if scale.is_finite() {
        scale.max(0.0)
    } else {
        1.0
    };
    let normal = Vec3::new(
        sample.r.mul_add(2.0, -1.0) * scale,
        sample.g.mul_add(2.0, -1.0) * scale,
        sample.b.mul_add(2.0, -1.0),
    );
    let length = (normal.x * normal.x + normal.y * normal.y + normal.z * normal.z).sqrt();
    (length > f32::EPSILON && length.is_finite())
        .then(|| Vec3::new(normal.x / length, normal.y / length, normal.z / length))
}

fn tangent_space_to_world(
    tangent_space: Vec3,
    geometric_normal: Vec3,
    tangent: Vec3,
    tangent_handedness: f32,
) -> Vec3 {
    let handedness = if tangent_handedness < 0.0 { -1.0 } else { 1.0 };
    let bitangent = Vec3::new(
        geometric_normal.y * tangent.z - geometric_normal.z * tangent.y,
        geometric_normal.z * tangent.x - geometric_normal.x * tangent.z,
        geometric_normal.x * tangent.y - geometric_normal.y * tangent.x,
    );
    normalize_or(
        Vec3::new(
            tangent.x * tangent_space.x
                + bitangent.x * handedness * tangent_space.y
                + geometric_normal.x * tangent_space.z,
            tangent.y * tangent_space.x
                + bitangent.y * handedness * tangent_space.y
                + geometric_normal.y * tangent_space.z,
            tangent.z * tangent_space.x
                + bitangent.z * handedness * tangent_space.y
                + geometric_normal.z * tangent_space.z,
        ),
        geometric_normal,
    )
}

pub(super) fn metallic_roughness_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> (f32, f32) {
    let Some(texture) = material.metallic_roughness_texture() else {
        return (1.0, 1.0);
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.metallic_roughness_texture_transform()),
        )
        .map(|sample| (sample.b.clamp(0.0, 1.0), sample.g.clamp(0.0, 1.0)))
        .unwrap_or((1.0, 1.0))
}

pub(super) fn occlusion_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.occlusion_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.occlusion_texture_transform()),
        )
        .map(|sample| sample.r.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn clearcoat_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.clearcoat_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.clearcoat_texture_transform()),
        )
        .map(|sample| sample.r.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn clearcoat_roughness_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.clearcoat_roughness_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.clearcoat_roughness_texture_transform()),
        )
        .map(|sample| sample.g.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn sheen_color_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> Color {
    let Some(texture) = material.sheen_color_texture() else {
        return Color::WHITE;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.sheen_color_texture_transform()),
        )
        .unwrap_or(Color::WHITE)
}

pub(super) fn sheen_roughness_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.sheen_roughness_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.sheen_roughness_texture_transform()),
        )
        .map(|sample| sample.a.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn anisotropy_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> Vec3 {
    let Some(texture) = material.anisotropy_texture() else {
        return Vec3::new(1.0, 0.0, 1.0);
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.anisotropy_texture_transform()),
        )
        .map(|sample| {
            Vec3::new(
                sample.r.mul_add(2.0, -1.0),
                sample.g.mul_add(2.0, -1.0),
                sample.b.clamp(0.0, 1.0),
            )
        })
        .unwrap_or(Vec3::new(1.0, 0.0, 1.0))
}

pub(super) fn iridescence_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.iridescence_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.iridescence_texture_transform()),
        )
        .map(|sample| sample.r.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn iridescence_thickness_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.iridescence_thickness_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.iridescence_thickness_texture_transform()),
        )
        .map(|sample| sample.g.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn transmission_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.transmission_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.transmission_texture_transform()),
        )
        .map(|sample| sample.r.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn thickness_texture_sample(
    assets: &impl TextureSampleSource,
    material: &MaterialDesc,
    uv: [f32; 2],
) -> f32 {
    let Some(texture) = material.thickness_texture() else {
        return 1.0;
    };
    assets
        .sample_texture(
            texture,
            transform_texture_uv(uv, material.thickness_texture_transform()),
        )
        .map(|sample| sample.g.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

pub(super) fn multiply_color(left: Color, right: Color) -> Color {
    Color::from_linear_rgba(
        left.r * right.r,
        left.g * right.g,
        left.b * right.b,
        left.a * right.a,
    )
}

pub(super) fn render_material_slot(
    material: MaterialHandle,
    backend_material_slots: &[MaterialHandle],
) -> u32 {
    backend_material_slots
        .iter()
        .position(|sampled| *sampled == material)
        .map(|index| (index as u32).saturating_add(1))
        .unwrap_or(0)
}

fn transform_texture_uv(uv: [f32; 2], transform: Option<TextureTransform>) -> [f32; 2] {
    let Some(transform) = transform else {
        return uv;
    };
    let scaled = [uv[0] * transform.scale()[0], uv[1] * transform.scale()[1]];
    let sin = transform.rotation_radians().sin();
    let cos = transform.rotation_radians().cos();
    [
        scaled[0] * cos - scaled[1] * sin + transform.offset()[0],
        scaled[0] * sin + scaled[1] * cos + transform.offset()[1],
    ]
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    let length = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}

fn material_texture_slots(
    material: &MaterialDesc,
) -> impl Iterator<Item = (&'static str, TextureHandle)> {
    [
        ("base_color", material.base_color_texture()),
        ("normal", material.normal_texture()),
        ("metallic_roughness", material.metallic_roughness_texture()),
        ("occlusion", material.occlusion_texture()),
        ("emissive", material.emissive_texture()),
        ("clearcoat", material.clearcoat_texture()),
        (
            "clearcoat_roughness",
            material.clearcoat_roughness_texture(),
        ),
        ("clearcoat_normal", material.clearcoat_normal_texture()),
        ("sheen_color", material.sheen_color_texture()),
        ("sheen_roughness", material.sheen_roughness_texture()),
        ("anisotropy", material.anisotropy_texture()),
        ("iridescence", material.iridescence_texture()),
        (
            "iridescence_thickness",
            material.iridescence_thickness_texture(),
        ),
        ("transmission", material.transmission_texture()),
        ("thickness", material.thickness_texture()),
    ]
    .into_iter()
    .filter_map(|(slot, texture)| texture.map(|texture| (slot, texture)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::AssetPath;
    use crate::assets::TextureSourceFormat;
    use crate::material::TextureColorSpace;

    #[test]
    fn backend_sampled_base_color_texture_is_not_baked_twice() {
        let assets = Assets::new();
        let texture = assets
            .create_texture_for_test(
            AssetPath::from(
                "data:image/png;base64,\
                 iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
            ),
            TextureColorSpace::Srgb,
                TextureSourceFormat::Png,
                None,
            )
            .expect("inline texture loads");
        let material = MaterialDesc::unlit(Color::WHITE).with_base_color_texture(texture);

        let baked = base_color_texture_sample(&assets, &material, [0.5, 0.5], &[]);
        let backend_sampled = base_color_texture_sample(&assets, &material, [0.5, 0.5], &[texture]);

        assert_ne!(baked, Color::WHITE);
        assert_eq!(backend_sampled, Color::WHITE);
    }

    #[test]
    fn opaque_alpha_transmission_uses_scene_color_pass() {
        let material = MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.1)
            .with_transmission_factor(1.0);

        assert!(matches!(
            material_pass(NodeKey::default(), &material),
            Ok(MaterialPass::Blend)
        ));
    }
}
