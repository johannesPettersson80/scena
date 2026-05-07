use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::geometry::SkinningMatrix;

use super::super::AssetPath;
use super::accessor::{GltfAccessor, GltfBufferView, read_mat4_accessor};

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
    json: &JsonValue,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<SceneAssetSkin>, AssetError> {
    json.get("skins")
        .and_then(JsonValue::as_array)
        .map(|skins| {
            skins
                .iter()
                .map(|skin| parse_skin(path, skin, buffers, buffer_views, accessors))
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn parse_skin(
    path: &AssetPath,
    skin: &JsonValue,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<SceneAssetSkin, AssetError> {
    let joints = skin
        .get("joints")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| parse_skin_error(path, "glTF skin is missing joints"))?
        .iter()
        .map(|joint| {
            joint
                .as_u64()
                .and_then(|joint| usize::try_from(joint).ok())
                .ok_or_else(|| parse_skin_error(path, "glTF skin joint must be a node index"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let inverse_bind_matrices = skin
        .get("inverseBindMatrices")
        .and_then(JsonValue::as_u64)
        .map(|accessor| {
            read_mat4_accessor(path, accessor as usize, buffers, buffer_views, accessors)
        })
        .transpose()?
        .unwrap_or_else(|| vec![SkinningMatrix::IDENTITY; joints.len()]);
    if inverse_bind_matrices.len() != joints.len() {
        return Err(parse_skin_error(
            path,
            "glTF skin inverseBindMatrices count must match joints count",
        ));
    }
    Ok(SceneAssetSkin {
        joints,
        inverse_bind_matrices,
    })
}

fn parse_skin_error(path: &AssetPath, reason: &'static str) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.to_string(),
    }
}
