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
    /// Triangles of a primitive without `NORMAL` that have no area and so no
    /// face normal. They were given the +Y fallback normal. `first_triangle`
    /// indexes the primitive's triangle list.
    DegenerateTriangleNormals {
        path: AssetPath,
        mesh_index: usize,
        primitive_index: usize,
        affected_triangles: usize,
        first_triangle: usize,
    },
    /// Vertices of a skinned primitive whose weights are all zero. They are
    /// bound to no joint and keep their authored position. `first_vertex`
    /// indexes the primitive's source vertices.
    SkinVerticesUnweighted {
        path: AssetPath,
        mesh_index: usize,
        primitive_index: usize,
        affected_vertices: usize,
        first_vertex: usize,
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
    DegenerateTriangleNormals {
        path: String,
        mesh_index: usize,
        primitive_index: usize,
        affected_triangles: usize,
        first_triangle: usize,
    },
    SkinVerticesUnweighted {
        path: String,
        mesh_index: usize,
        primitive_index: usize,
        affected_vertices: usize,
        first_vertex: usize,
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
            AssetLoadWarning::DegenerateTriangleNormals {
                path,
                mesh_index,
                primitive_index,
                affected_triangles,
                first_triangle,
            } => Self::DegenerateTriangleNormals {
                path: path.as_str().to_owned(),
                mesh_index: *mesh_index,
                primitive_index: *primitive_index,
                affected_triangles: *affected_triangles,
                first_triangle: *first_triangle,
            },
            AssetLoadWarning::SkinVerticesUnweighted {
                path,
                mesh_index,
                primitive_index,
                affected_vertices,
                first_vertex,
            } => Self::SkinVerticesUnweighted {
                path: path.as_str().to_owned(),
                mesh_index: *mesh_index,
                primitive_index: *primitive_index,
                affected_vertices: *affected_vertices,
                first_vertex: *first_vertex,
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
