use crate::assets::{
    AssetMaterialFallback, AssetMaterialSource, Assets, MaterialHandle, TextureHandle,
};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};

use super::SceneTextureInspection;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneMaterialInspection {
    material: MaterialHandle,
    source: Option<AssetMaterialSource>,
    kind: MaterialKind,
    base_color: Color,
    alpha_mode: AlphaMode,
    base_color_texture: Option<SceneTextureInspection>,
    normal_texture: Option<SceneTextureInspection>,
    metallic_roughness_texture: Option<SceneTextureInspection>,
    occlusion_texture: Option<SceneTextureInspection>,
    emissive_texture: Option<SceneTextureInspection>,
    clearcoat_texture: Option<SceneTextureInspection>,
    clearcoat_roughness_texture: Option<SceneTextureInspection>,
    clearcoat_normal_texture: Option<SceneTextureInspection>,
    sheen_color_texture: Option<SceneTextureInspection>,
    sheen_roughness_texture: Option<SceneTextureInspection>,
    anisotropy_texture: Option<SceneTextureInspection>,
    iridescence_texture: Option<SceneTextureInspection>,
    iridescence_thickness_texture: Option<SceneTextureInspection>,
    transmission_texture: Option<SceneTextureInspection>,
    thickness_texture: Option<SceneTextureInspection>,
}

impl SceneMaterialInspection {
    pub(super) fn new<F>(material: MaterialHandle, desc: MaterialDesc, assets: &Assets<F>) -> Self {
        let source = assets.material_source(material);
        Self {
            material,
            source,
            kind: desc.kind(),
            base_color: desc.base_color(),
            alpha_mode: desc.alpha_mode(),
            base_color_texture: texture_preview(desc.base_color_texture(), assets),
            normal_texture: texture_preview(desc.normal_texture(), assets),
            metallic_roughness_texture: texture_preview(desc.metallic_roughness_texture(), assets),
            occlusion_texture: texture_preview(desc.occlusion_texture(), assets),
            emissive_texture: texture_preview(desc.emissive_texture(), assets),
            clearcoat_texture: texture_preview(desc.clearcoat_texture(), assets),
            clearcoat_roughness_texture: texture_preview(
                desc.clearcoat_roughness_texture(),
                assets,
            ),
            clearcoat_normal_texture: texture_preview(desc.clearcoat_normal_texture(), assets),
            sheen_color_texture: texture_preview(desc.sheen_color_texture(), assets),
            sheen_roughness_texture: texture_preview(desc.sheen_roughness_texture(), assets),
            anisotropy_texture: texture_preview(desc.anisotropy_texture(), assets),
            iridescence_texture: texture_preview(desc.iridescence_texture(), assets),
            iridescence_thickness_texture: texture_preview(
                desc.iridescence_thickness_texture(),
                assets,
            ),
            transmission_texture: texture_preview(desc.transmission_texture(), assets),
            thickness_texture: texture_preview(desc.thickness_texture(), assets),
        }
    }

    pub const fn material(&self) -> MaterialHandle {
        self.material
    }

    pub fn source(&self) -> Option<&AssetMaterialSource> {
        self.source.as_ref()
    }

    pub fn fallbacks(&self) -> &[AssetMaterialFallback] {
        self.source
            .as_ref()
            .map(AssetMaterialSource::fallbacks)
            .unwrap_or(&[])
    }

    pub const fn kind(&self) -> MaterialKind {
        self.kind
    }

    pub const fn base_color(&self) -> Color {
        self.base_color
    }

    pub const fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    pub const fn has_base_color_texture(&self) -> bool {
        self.base_color_texture.is_some()
    }

    pub fn base_color_texture(&self) -> Option<SceneTextureInspection> {
        self.base_color_texture.clone()
    }

    pub const fn has_normal_texture(&self) -> bool {
        self.normal_texture.is_some()
    }

    pub fn normal_texture(&self) -> Option<SceneTextureInspection> {
        self.normal_texture.clone()
    }

    pub const fn has_metallic_roughness_texture(&self) -> bool {
        self.metallic_roughness_texture.is_some()
    }

    pub fn metallic_roughness_texture(&self) -> Option<SceneTextureInspection> {
        self.metallic_roughness_texture.clone()
    }

    pub const fn has_occlusion_texture(&self) -> bool {
        self.occlusion_texture.is_some()
    }

    pub fn occlusion_texture(&self) -> Option<SceneTextureInspection> {
        self.occlusion_texture.clone()
    }

    pub const fn has_emissive_texture(&self) -> bool {
        self.emissive_texture.is_some()
    }

    pub fn emissive_texture(&self) -> Option<SceneTextureInspection> {
        self.emissive_texture.clone()
    }

    pub const fn has_clearcoat_texture(&self) -> bool {
        self.clearcoat_texture.is_some()
    }

    pub fn clearcoat_texture(&self) -> Option<SceneTextureInspection> {
        self.clearcoat_texture.clone()
    }

    pub const fn has_clearcoat_roughness_texture(&self) -> bool {
        self.clearcoat_roughness_texture.is_some()
    }

    pub fn clearcoat_roughness_texture(&self) -> Option<SceneTextureInspection> {
        self.clearcoat_roughness_texture.clone()
    }

    pub const fn has_clearcoat_normal_texture(&self) -> bool {
        self.clearcoat_normal_texture.is_some()
    }

    pub fn clearcoat_normal_texture(&self) -> Option<SceneTextureInspection> {
        self.clearcoat_normal_texture.clone()
    }

    pub const fn has_sheen_color_texture(&self) -> bool {
        self.sheen_color_texture.is_some()
    }

    pub fn sheen_color_texture(&self) -> Option<SceneTextureInspection> {
        self.sheen_color_texture.clone()
    }

    pub const fn has_sheen_roughness_texture(&self) -> bool {
        self.sheen_roughness_texture.is_some()
    }

    pub fn sheen_roughness_texture(&self) -> Option<SceneTextureInspection> {
        self.sheen_roughness_texture.clone()
    }

    pub const fn has_anisotropy_texture(&self) -> bool {
        self.anisotropy_texture.is_some()
    }

    pub fn anisotropy_texture(&self) -> Option<SceneTextureInspection> {
        self.anisotropy_texture.clone()
    }

    pub const fn has_iridescence_texture(&self) -> bool {
        self.iridescence_texture.is_some()
    }

    pub fn iridescence_texture(&self) -> Option<SceneTextureInspection> {
        self.iridescence_texture.clone()
    }

    pub const fn has_iridescence_thickness_texture(&self) -> bool {
        self.iridescence_thickness_texture.is_some()
    }

    pub fn iridescence_thickness_texture(&self) -> Option<SceneTextureInspection> {
        self.iridescence_thickness_texture.clone()
    }

    pub const fn has_transmission_texture(&self) -> bool {
        self.transmission_texture.is_some()
    }

    pub fn transmission_texture(&self) -> Option<SceneTextureInspection> {
        self.transmission_texture.clone()
    }

    pub const fn has_thickness_texture(&self) -> bool {
        self.thickness_texture.is_some()
    }

    pub fn thickness_texture(&self) -> Option<SceneTextureInspection> {
        self.thickness_texture.clone()
    }
}

fn texture_preview<F>(
    texture: Option<TextureHandle>,
    assets: &Assets<F>,
) -> Option<SceneTextureInspection> {
    let texture = texture?;
    assets
        .texture(texture)
        .map(|desc| SceneTextureInspection::new(texture, desc))
}
