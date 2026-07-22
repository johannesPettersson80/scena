use ::gltf::{Document, Node};

use crate::assets::AssetPath;
use crate::diagnostics::AssetError;

use super::anchors::parse_node_anchors;
use super::buffers::ResolvedGltfBuffers;
use super::connectors::parse_node_connectors;
use super::instancing::parse_node_instance_transforms;
use super::scene_asset::{SceneAssetLight, SceneAssetMesh, SceneAssetNode};
use super::transform::from_gltf_transform;

pub(super) fn parse_gltf_nodes(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    meshes: &[Vec<SceneAssetMesh>],
    lights: &[SceneAssetLight],
) -> Result<Vec<SceneAssetNode>, AssetError> {
    document
        .nodes()
        .map(|node| {
            Ok(SceneAssetNode {
                name: node.name().map(str::to_string),
                children: node.children().map(|child| child.index()).collect(),
                transform: from_gltf_transform(node.transform()),
                meshes: parse_node_meshes(path, &node, meshes)?,
                instance_transforms: parse_node_instance_transforms(
                    path, document, buffers, &node,
                )?,
                skin: node.skin().map(|skin| skin.index()),
                light: node
                    .light()
                    .and_then(|light| lights.get(light.index()).copied()),
                anchors: parse_node_anchors(path, &node)?,
                connectors: parse_node_connectors(path, &node)?,
            })
        })
        .collect()
}

fn parse_node_meshes(
    path: &AssetPath,
    node: &Node<'_>,
    meshes: &[Vec<SceneAssetMesh>],
) -> Result<Vec<SceneAssetMesh>, AssetError> {
    let Some(source_mesh) = node.mesh() else {
        if node.weights().is_some() {
            return Err(AssetError::Parse {
                path: path.as_str().to_owned(),
                reason: format!(
                    "glTF node {} defines morph weights without a mesh",
                    node.index()
                ),
            });
        }
        return Ok(Vec::new());
    };
    let mut node_meshes = meshes.get(source_mesh.index()).cloned().unwrap_or_default();
    let Some(weights) = node.weights() else {
        return Ok(node_meshes);
    };
    if weights.iter().any(|weight| !weight.is_finite()) {
        return Err(AssetError::Parse {
            path: path.as_str().to_owned(),
            reason: format!("glTF node {} morph weights must be finite", node.index()),
        });
    }
    for (primitive_index, (primitive, mesh)) in
        source_mesh.primitives().zip(&mut node_meshes).enumerate()
    {
        let target_count = primitive.morph_targets().count();
        if weights.len() != target_count {
            return Err(AssetError::Parse {
                path: path.as_str().to_owned(),
                reason: format!(
                    "glTF node {} morph weight count {} must match mesh {} primitive {primitive_index} target count {target_count}",
                    node.index(),
                    weights.len(),
                    source_mesh.index(),
                ),
            });
        }
        mesh.morph_weights = weights.to_vec();
    }
    Ok(node_meshes)
}
