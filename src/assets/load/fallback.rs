use serde::{Deserialize, Serialize};

use crate::assets::AssetPath;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMaterialFallback {
    pub kind: AssetMaterialFallbackKind,
    pub material_index: Option<usize>,
    pub material_slot: String,
    pub texture_index: usize,
    pub source_path: AssetPath,
    pub fallback_path: AssetPath,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetMaterialFallbackKind {
    TextureBasisuFallback,
    MissingTextureFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMaterialFallbackV1 {
    pub kind: AssetMaterialFallbackKind,
    #[serde(default)]
    pub material_index: Option<usize>,
    pub material_slot: String,
    pub texture_index: usize,
    pub source_path: String,
    pub fallback_path: String,
    pub reason: String,
}

impl AssetMaterialFallback {
    pub fn texture_basisu_fallback(
        material_index: usize,
        material_slot: impl Into<String>,
        texture_index: usize,
        source_path: AssetPath,
        fallback_path: AssetPath,
    ) -> Self {
        Self {
            kind: AssetMaterialFallbackKind::TextureBasisuFallback,
            material_index: Some(material_index),
            material_slot: material_slot.into(),
            texture_index,
            source_path,
            fallback_path,
            reason: "KHR_texture_basisu unavailable; using authored fallback texture".to_string(),
        }
    }

    pub fn missing_texture_fallback(
        material_index: usize,
        material_slot: impl Into<String>,
        texture_index: usize,
        source_path: AssetPath,
        fallback_path: AssetPath,
    ) -> Self {
        Self {
            kind: AssetMaterialFallbackKind::MissingTextureFallback,
            material_index: Some(material_index),
            material_slot: material_slot.into(),
            texture_index,
            source_path,
            fallback_path,
            reason: "texture bytes were unavailable; renderer will bind the generated material fallback texture".to_string(),
        }
    }
}

impl From<&AssetMaterialFallback> for AssetMaterialFallbackV1 {
    fn from(fallback: &AssetMaterialFallback) -> Self {
        Self {
            kind: fallback.kind,
            material_index: fallback.material_index,
            material_slot: fallback.material_slot.clone(),
            texture_index: fallback.texture_index,
            source_path: fallback.source_path.as_str().to_owned(),
            fallback_path: fallback.fallback_path.as_str().to_owned(),
            reason: fallback.reason.clone(),
        }
    }
}
