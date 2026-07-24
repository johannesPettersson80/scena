use serde::{Deserialize, Serialize};

use crate::assets::AssetPath;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetLoadWarning {
    ExternalBufferMissing {
        path: AssetPath,
        index: usize,
        reason: String,
    },
    ExternalImageMissing {
        path: AssetPath,
        reason: String,
    },
    ComputedFlatNormals {
        path: AssetPath,
        mesh_index: usize,
        primitive_index: usize,
        triangle_count: usize,
    },
    SkinInfluencesTruncated {
        path: AssetPath,
        mesh_index: usize,
        primitive_index: usize,
        affected_vertices: usize,
        source_influences: usize,
        retained_influences: usize,
    },
    InvalidMaterialVariantMapping {
        path: AssetPath,
        mesh_index: usize,
        primitive_index: usize,
        mapping_index: usize,
        material_index: Option<usize>,
        variant_indices: Vec<u32>,
        material_count: usize,
    },
    TextureDownscaled {
        path: AssetPath,
        original_width: u32,
        original_height: u32,
        decoded_width: u32,
        decoded_height: u32,
        maximum_dimension: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AssetLoadWarningV1 {
    ExternalBufferMissing {
        path: String,
        index: usize,
        reason: String,
    },
    ExternalImageMissing {
        path: String,
        reason: String,
    },
    ComputedFlatNormals {
        path: String,
        mesh_index: usize,
        primitive_index: usize,
        triangle_count: usize,
    },
    SkinInfluencesTruncated {
        path: String,
        mesh_index: usize,
        primitive_index: usize,
        affected_vertices: usize,
        source_influences: usize,
        retained_influences: usize,
    },
    InvalidMaterialVariantMapping {
        path: String,
        mesh_index: usize,
        primitive_index: usize,
        mapping_index: usize,
        material_index: Option<usize>,
        variant_indices: Vec<u32>,
        material_count: usize,
    },
    TextureDownscaled {
        path: String,
        original_width: u32,
        original_height: u32,
        decoded_width: u32,
        decoded_height: u32,
        maximum_dimension: u32,
    },
}

impl From<&AssetLoadWarning> for AssetLoadWarningV1 {
    fn from(warning: &AssetLoadWarning) -> Self {
        match warning {
            AssetLoadWarning::ExternalBufferMissing {
                path,
                index,
                reason,
            } => Self::ExternalBufferMissing {
                path: path.as_str().to_owned(),
                index: *index,
                reason: reason.clone(),
            },
            AssetLoadWarning::ExternalImageMissing { path, reason } => Self::ExternalImageMissing {
                path: path.as_str().to_owned(),
                reason: reason.clone(),
            },
            AssetLoadWarning::ComputedFlatNormals {
                path,
                mesh_index,
                primitive_index,
                triangle_count,
            } => Self::ComputedFlatNormals {
                path: path.as_str().to_owned(),
                mesh_index: *mesh_index,
                primitive_index: *primitive_index,
                triangle_count: *triangle_count,
            },
            AssetLoadWarning::SkinInfluencesTruncated {
                path,
                mesh_index,
                primitive_index,
                affected_vertices,
                source_influences,
                retained_influences,
            } => Self::SkinInfluencesTruncated {
                path: path.as_str().to_owned(),
                mesh_index: *mesh_index,
                primitive_index: *primitive_index,
                affected_vertices: *affected_vertices,
                source_influences: *source_influences,
                retained_influences: *retained_influences,
            },
            AssetLoadWarning::InvalidMaterialVariantMapping {
                path,
                mesh_index,
                primitive_index,
                mapping_index,
                material_index,
                variant_indices,
                material_count,
            } => Self::InvalidMaterialVariantMapping {
                path: path.as_str().to_owned(),
                mesh_index: *mesh_index,
                primitive_index: *primitive_index,
                mapping_index: *mapping_index,
                material_index: *material_index,
                variant_indices: variant_indices.clone(),
                material_count: *material_count,
            },
            AssetLoadWarning::TextureDownscaled {
                path,
                original_width,
                original_height,
                decoded_width,
                decoded_height,
                maximum_dimension,
            } => Self::TextureDownscaled {
                path: path.as_str().to_owned(),
                original_width: *original_width,
                original_height: *original_height,
                decoded_width: *decoded_width,
                decoded_height: *decoded_height,
                maximum_dimension: *maximum_dimension,
            },
        }
    }
}
