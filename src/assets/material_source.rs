use serde::{Deserialize, Serialize};

use super::{AssetMaterialFallback, AssetPath};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMaterialSource {
    kind: AssetMaterialSourceKind,
    asset_path: Option<AssetPath>,
    material_index: Option<usize>,
    material_name: Option<String>,
    reason: Option<String>,
    fallbacks: Vec<AssetMaterialFallback>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetMaterialSourceKind {
    UserCreated,
    SourceMaterial,
    GeneratedDefault,
}

impl AssetMaterialSource {
    pub fn user_created() -> Self {
        Self {
            kind: AssetMaterialSourceKind::UserCreated,
            asset_path: None,
            material_index: None,
            material_name: None,
            reason: Some("material was created by the host application".to_string()),
            fallbacks: Vec::new(),
        }
    }

    pub fn source_material(
        asset_path: AssetPath,
        material_index: usize,
        fallbacks: Vec<AssetMaterialFallback>,
    ) -> Self {
        Self::source_material_named(asset_path, material_index, None, fallbacks)
    }

    pub fn source_material_named(
        asset_path: AssetPath,
        material_index: usize,
        material_name: Option<String>,
        fallbacks: Vec<AssetMaterialFallback>,
    ) -> Self {
        Self {
            kind: AssetMaterialSourceKind::SourceMaterial,
            asset_path: Some(asset_path),
            material_index: Some(material_index),
            material_name,
            reason: None,
            fallbacks,
        }
    }

    pub fn generated_default(asset_path: AssetPath, reason: impl Into<String>) -> Self {
        Self {
            kind: AssetMaterialSourceKind::GeneratedDefault,
            asset_path: Some(asset_path),
            material_index: None,
            material_name: None,
            reason: Some(reason.into()),
            fallbacks: Vec::new(),
        }
    }

    pub const fn kind(&self) -> AssetMaterialSourceKind {
        self.kind
    }

    pub fn asset_path(&self) -> Option<&AssetPath> {
        self.asset_path.as_ref()
    }

    pub const fn material_index(&self) -> Option<usize> {
        self.material_index
    }

    pub fn material_name(&self) -> Option<&str> {
        self.material_name.as_deref()
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub fn fallbacks(&self) -> &[AssetMaterialFallback] {
        &self.fallbacks
    }
}
