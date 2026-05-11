//! Stage C2: skin parsing now uses the `gltf` crate's typed
//! `Skin::reader()` for inverse-bind-matrices accessor reading.

use ::gltf::Document;

use crate::diagnostics::AssetError;
use crate::geometry::SkinningMatrix;

use super::super::AssetPath;

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
    buffers: &[Vec<u8>],
) -> Result<Vec<SceneAssetSkin>, AssetError> {
    document
        .skins()
        .map(|skin| {
            let joints: Vec<usize> = skin.joints().map(|joint| joint.index()).collect();
            let inverse_bind_matrices = skin
                .reader(|buffer| buffers.get(buffer.index()).map(Vec::as_slice))
                .read_inverse_bind_matrices()
                .map(|reader| {
                    reader
                        .map(|matrix| {
                            let mut flat = [0.0_f32; 16];
                            for (column, source) in matrix.iter().enumerate() {
                                for (row, value) in source.iter().enumerate() {
                                    flat[column * 4 + row] = *value;
                                }
                            }
                            SkinningMatrix::from_gltf_column_major(flat)
                        })
                        .collect::<Vec<_>>()
                })
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
