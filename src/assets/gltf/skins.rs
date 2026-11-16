//! glTF skin parsing. Inverse bind matrices are decoded through the checked
//! accessor reader so a malformed accessor is reported instead of misread.

use ::gltf::accessor::{DataType, Dimensions};
use ::gltf::{Document, Skin};

use crate::diagnostics::AssetError;
use crate::geometry::SkinningMatrix;

use super::super::AssetPath;
use super::buffers::ResolvedGltfBuffers;
use super::typed_accessors::{accessor_error, typed_accessor_iter};

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetSkin {
    joints: Vec<usize>,
    inverse_bind_matrices: Vec<SkinningMatrix>,
}

impl SceneAssetSkin {
    pub fn joints(&self) -> &[usize] {
        &self.joints
    }

    pub fn inverse_bind_matrices(&self) -> &[SkinningMatrix] {
        &self.inverse_bind_matrices
    }
}

pub(super) fn parse_skins(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
) -> Result<Vec<SceneAssetSkin>, AssetError> {
    document
        .skins()
        .map(|skin| {
            let joints: Vec<usize> = skin.joints().map(|joint| joint.index()).collect();
            let inverse_bind_matrices = read_inverse_bind_matrices(path, buffers, &skin)?
                .unwrap_or_else(|| vec![SkinningMatrix::IDENTITY; joints.len()]);
            if inverse_bind_matrices.len() != joints.len() {
                return Err(AssetError::Parse {
                    path: path.as_str().to_string(),
                    reason: "glTF skin inverseBindMatrices count must match joints count"
                        .to_string(),
                });
            }
            Ok(SceneAssetSkin {
                joints,
                inverse_bind_matrices,
            })
        })
        .collect()
}

/// Returns `None` only when the skin declares no `inverseBindMatrices`, which
/// glTF defines as identity matrices. A declared but unreadable accessor is an
/// error rather than a silent identity fallback.
fn read_inverse_bind_matrices(
    path: &AssetPath,
    buffers: &ResolvedGltfBuffers,
    skin: &Skin<'_>,
) -> Result<Option<Vec<SkinningMatrix>>, AssetError> {
    let Some(accessor) = skin.inverse_bind_matrices() else {
        return Ok(None);
    };
    let label = format!("skin {} inverseBindMatrices", skin.index());
    if accessor.data_type() != DataType::F32 || accessor.dimensions() != Dimensions::Mat4 {
        return Err(accessor_error(
            path,
            &label,
            format!(
                "must use FLOAT MAT4, found {:?} {:?}",
                accessor.data_type(),
                accessor.dimensions()
            ),
        ));
    }
    let matrices = typed_accessor_iter::<[[f32; 4]; 4]>(path, buffers, &accessor, &label)?
        .map(|matrix| {
            let mut flat = [0.0_f32; 16];
            for (column, source) in matrix.iter().enumerate() {
                for (row, value) in source.iter().enumerate() {
                    flat[column * 4 + row] = *value;
                }
            }
            SkinningMatrix::from_gltf_column_major(flat)
        })
        .collect();
    Ok(Some(matrices))
}
