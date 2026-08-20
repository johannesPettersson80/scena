use serde::{Deserialize, Serialize};

#[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
use super::PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2;
use super::{
    PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1, PhotographicMaterialCategoryV1,
    PhotographicMaterialResolutionV1, PhotographicSurfaceKind,
};
use crate::{AssetPath, MaterialHandle, TextureHandle};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum PhotographicMaterialPackMapRoleV1 {
    BaseColor,
    NormalGl,
    OcclusionRoughnessMetallic,
}

impl PhotographicMaterialPackMapRoleV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BaseColor => "base_color",
            Self::NormalGl => "normal_gl",
            Self::OcclusionRoughnessMetallic => "occlusion_roughness_metallic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialPackSourceV1 {
    pub provider: String,
    pub provider_asset_id: String,
    pub source_page: String,
    pub archive_uri: String,
    pub archive_sha256: String,
    pub archive_bytes: u64,
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialPackMapV1 {
    pub role: PhotographicMaterialPackMapRoleV1,
    pub path: String,
    pub color_space: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialPackV1 {
    pub schema: String,
    pub id: String,
    pub label: String,
    pub category: PhotographicMaterialCategoryV1,
    pub surface_kind: PhotographicSurfaceKind,
    pub recommended_tile_size_m: f32,
    pub source: PhotographicMaterialPackSourceV1,
    pub maps: Vec<PhotographicMaterialPackMapV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PhotographicMaterialPackV2 {
    pub schema: String,
    pub id: String,
    pub label: String,
    pub category: PhotographicMaterialCategoryV1,
    pub surface_kind: PhotographicSurfaceKind,
    pub resolution: PhotographicMaterialResolutionV1,
    pub recommended_tile_size_m: f32,
    pub source: PhotographicMaterialPackSourceV1,
    pub maps: Vec<PhotographicMaterialPackMapV1>,
}

impl PhotographicMaterialPackV2 {
    #[cfg(all(feature = "material-library", not(target_arch = "wasm32")))]
    pub(crate) fn from_v1(
        pack: PhotographicMaterialPackV1,
        resolution: PhotographicMaterialResolutionV1,
    ) -> Self {
        Self {
            schema: PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V2.to_owned(),
            id: pack.id,
            label: pack.label,
            category: pack.category,
            surface_kind: pack.surface_kind,
            resolution,
            recommended_tile_size_m: pack.recommended_tile_size_m,
            source: pack.source,
            maps: pack.maps,
        }
    }

    pub(crate) fn as_v1_compatibility_pack(&self) -> PhotographicMaterialPackV1 {
        PhotographicMaterialPackV1 {
            schema: PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1.to_owned(),
            id: self.id.clone(),
            label: self.label.clone(),
            category: self.category,
            surface_kind: self.surface_kind,
            recommended_tile_size_m: self.recommended_tile_size_m,
            source: self.source.clone(),
            maps: self.maps.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PhotographicMaterialPackAssets {
    pub(super) pack: PhotographicMaterialPackV1,
    pub(super) manifest_schema: String,
    pub(super) manifest_path: AssetPath,
    pub(super) resolution: PhotographicMaterialResolutionV1,
    pub(super) material: MaterialHandle,
    pub(super) base_color_texture: TextureHandle,
    pub(super) normal_texture: TextureHandle,
    pub(super) metallic_roughness_texture: TextureHandle,
}

impl PhotographicMaterialPackAssets {
    pub const fn material(&self) -> MaterialHandle {
        self.material
    }
    pub const fn base_color_texture(&self) -> TextureHandle {
        self.base_color_texture
    }
    pub const fn normal_texture(&self) -> TextureHandle {
        self.normal_texture
    }
    pub const fn metallic_roughness_texture(&self) -> TextureHandle {
        self.metallic_roughness_texture
    }
    pub const fn pack(&self) -> &PhotographicMaterialPackV1 {
        &self.pack
    }
    pub fn manifest_schema(&self) -> &str {
        &self.manifest_schema
    }
    pub fn manifest_path(&self) -> &AssetPath {
        &self.manifest_path
    }
    pub const fn resolution(&self) -> PhotographicMaterialResolutionV1 {
        self.resolution
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn texture_resource_identity(&self, map: &PhotographicMaterialPackMapV1) -> String {
        if self.manifest_schema == PHOTOGRAPHIC_MATERIAL_PACK_SCHEMA_V1 {
            format!(
                "scena/material-pack/v1/{}/{}",
                map.role.as_str(),
                map.sha256
            )
        } else {
            format!(
                "scena/material-pack/v2/{}/{}/{}/{}",
                self.pack.id,
                self.resolution.as_str(),
                map.role.as_str(),
                map.sha256
            )
        }
    }
}

#[cfg(feature = "scene-host")]
#[derive(Debug, Clone)]
pub(crate) struct PhotographicMaterialPackBinding {
    pub(crate) manifest_path: AssetPath,
    pub(crate) pack_id: String,
    pub(crate) resolution: PhotographicMaterialResolutionV1,
}
