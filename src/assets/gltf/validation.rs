use std::collections::VecDeque;

use serde_json::Value;

use crate::diagnostics::AssetError;

use super::super::AssetPath;

#[derive(Debug, Clone)]
pub(super) struct ExtensionDeclarations {
    pub(super) used: Vec<String>,
    pub(super) required: Vec<String>,
}

pub(super) fn validate_document_structure(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<(), AssetError> {
    let json_bytes = gltf_json_bytes(path, bytes)?;
    let root = serde_json::from_slice::<Value>(json_bytes)
        .map_err(|error| parse_error(path, format!("glTF JSON is not valid: {error}")))?;
    let Some(root) = root.as_object() else {
        return Err(parse_error(path, "glTF JSON root must be an object"));
    };

    let nodes = array_len(root.get("nodes"));
    let meshes = array_len(root.get("meshes"));
    let skins = array_len(root.get("skins"));
    let cameras = array_len(root.get("cameras"));
    let accessors = array_len(root.get("accessors"));
    let buffer_views = array_len(root.get("bufferViews"));
    let buffers = array_len(root.get("buffers"));
    let materials = array_len(root.get("materials"));
    let images = array_len(root.get("images"));
    let samplers = array_len(root.get("samplers"));
    let textures = array_len(root.get("textures"));
    let scenes = array_len(root.get("scenes"));

    validate_node_graph(path, root.get("nodes"), nodes, meshes, skins, cameras)?;
    validate_meshes(path, root.get("meshes"), accessors, materials)?;
    validate_accessors(path, root.get("accessors"), buffer_views)?;
    validate_buffer_views(path, root.get("bufferViews"), buffers)?;
    validate_textures(path, root.get("textures"), images, samplers)?;
    validate_material_texture_refs(path, root.get("materials"), textures)?;
    validate_images(path, root.get("images"), buffer_views)?;
    validate_skins(path, root.get("skins"), nodes, accessors)?;
    validate_animations(path, root.get("animations"), nodes, accessors)?;
    validate_scenes(path, root.get("scenes"), nodes)?;
    validate_index(path, root.get("scene"), "$.scene", scenes)?;
    Ok(())
}

fn validate_material_texture_refs(
    path: &AssetPath,
    value: Option<&Value>,
    texture_count: usize,
) -> Result<(), AssetError> {
    let Some(materials) = value.and_then(Value::as_array) else {
        return Ok(());
    };
    for material in materials {
        validate_texture_slots_recursive(path, material, texture_count)?;
    }
    Ok(())
}

fn validate_texture_slots_recursive(
    path: &AssetPath,
    value: &Value,
    texture_count: usize,
) -> Result<(), AssetError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if key.ends_with("Texture")
                    && let Some(texture_index) = child.get("index").and_then(Value::as_u64)
                    && usize::try_from(texture_index).map_or(true, |index| index >= texture_count)
                {
                    return Err(AssetError::MissingTexture {
                        path: path.as_str().to_owned(),
                        material_slot: key.clone(),
                        texture_index: usize::try_from(texture_index).unwrap_or(usize::MAX),
                        help: "export the referenced image or remove the broken material slot",
                    });
                }
                validate_texture_slots_recursive(path, child, texture_count)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                validate_texture_slots_recursive(path, child, texture_count)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn extension_declarations(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<ExtensionDeclarations, AssetError> {
    let root = serde_json::from_slice::<Value>(gltf_json_bytes(path, bytes)?)
        .map_err(|error| parse_error(path, format!("glTF JSON is not valid: {error}")))?;
    Ok(ExtensionDeclarations {
        used: string_array(root.get("extensionsUsed")),
        required: string_array(root.get("extensionsRequired")),
    })
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn gltf_json_bytes<'a>(path: &AssetPath, bytes: &'a [u8]) -> Result<&'a [u8], AssetError> {
    if !super::has_glb_magic(bytes) {
        return Ok(bytes);
    }
    if bytes.len() < 20 {
        return Err(parse_error(
            path,
            "GLB is too short to contain a JSON chunk",
        ));
    }
    let chunk_len = u32::from_le_bytes(bytes[12..16].try_into().expect("four-byte GLB field"));
    let chunk_len = usize::try_from(chunk_len)
        .map_err(|_| parse_error(path, "GLB JSON chunk length does not fit usize"))?;
    let chunk_type = u32::from_le_bytes(bytes[16..20].try_into().expect("four-byte GLB field"));
    if chunk_type != 0x4E4F_534A {
        return Err(parse_error(path, "GLB first chunk must be JSON"));
    }
    bytes
        .get(20..20_usize.saturating_add(chunk_len))
        .ok_or_else(|| parse_error(path, "GLB JSON chunk exceeds the container length"))
}

fn validate_node_graph(
    path: &AssetPath,
    value: Option<&Value>,
    node_count: usize,
    mesh_count: usize,
    skin_count: usize,
    camera_count: usize,
) -> Result<(), AssetError> {
    let Some(nodes) = value.and_then(Value::as_array) else {
        return Ok(());
    };
    let mut indegree = vec![0_usize; node_count];
    let mut edges = vec![Vec::new(); node_count];
    for (node_index, node) in nodes.iter().enumerate() {
        let Some(node) = node.as_object() else {
            continue;
        };
        validate_index(
            path,
            node.get("mesh"),
            &format!("$.nodes[{node_index}].mesh"),
            mesh_count,
        )?;
        validate_index(
            path,
            node.get("skin"),
            &format!("$.nodes[{node_index}].skin"),
            skin_count,
        )?;
        validate_index(
            path,
            node.get("camera"),
            &format!("$.nodes[{node_index}].camera"),
            camera_count,
        )?;
        let Some(children) = node.get("children").and_then(Value::as_array) else {
            continue;
        };
        for (child_offset, child) in children.iter().enumerate() {
            let child_path = format!("$.nodes[{node_index}].children[{child_offset}]");
            let child = required_index(path, child, &child_path, node_count)?;
            indegree[child] += 1;
            if indegree[child] > 1 {
                return Err(parse_error(
                    path,
                    format!(
                        "{child_path} gives node {child} multiple parents; glTF nodes must form a tree/forest, not a DAG"
                    ),
                ));
            }
            edges[node_index].push(child);
        }
    }

    let mut ready = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<VecDeque<_>>();
    let mut visited = 0_usize;
    while let Some(node) = ready.pop_front() {
        visited += 1;
        for child in &edges[node] {
            indegree[*child] -= 1;
            if indegree[*child] == 0 {
                ready.push_back(*child);
            }
        }
    }
    if visited != node_count {
        return Err(parse_error(
            path,
            "$.nodes contains a cycle; glTF node graphs must be acyclic",
        ));
    }
    Ok(())
}

fn validate_meshes(
    path: &AssetPath,
    value: Option<&Value>,
    accessor_count: usize,
    material_count: usize,
) -> Result<(), AssetError> {
    for (mesh_index, mesh) in values(value).enumerate() {
        for (primitive_index, primitive) in values(mesh.get("primitives")).enumerate() {
            let base = format!("$.meshes[{mesh_index}].primitives[{primitive_index}]");
            validate_index(
                path,
                primitive.get("indices"),
                &format!("{base}.indices"),
                accessor_count,
            )?;
            validate_index(
                path,
                primitive.get("material"),
                &format!("{base}.material"),
                material_count,
            )?;
            if let Some(attributes) = primitive.get("attributes").and_then(Value::as_object) {
                for (name, accessor) in attributes {
                    required_index(
                        path,
                        accessor,
                        &format!("{base}.attributes.{name}"),
                        accessor_count,
                    )?;
                }
            }
            for (target_index, target) in values(primitive.get("targets")).enumerate() {
                if let Some(attributes) = target.as_object() {
                    for (name, accessor) in attributes {
                        required_index(
                            path,
                            accessor,
                            &format!("{base}.targets[{target_index}].{name}"),
                            accessor_count,
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_accessors(
    path: &AssetPath,
    value: Option<&Value>,
    buffer_view_count: usize,
) -> Result<(), AssetError> {
    for (index, accessor) in values(value).enumerate() {
        let base = format!("$.accessors[{index}]");
        validate_index(
            path,
            accessor.get("bufferView"),
            &format!("{base}.bufferView"),
            buffer_view_count,
        )?;
        if let Some(sparse) = accessor.get("sparse") {
            validate_index(
                path,
                sparse
                    .get("indices")
                    .and_then(|value| value.get("bufferView")),
                &format!("{base}.sparse.indices.bufferView"),
                buffer_view_count,
            )?;
            validate_index(
                path,
                sparse
                    .get("values")
                    .and_then(|value| value.get("bufferView")),
                &format!("{base}.sparse.values.bufferView"),
                buffer_view_count,
            )?;
        }
    }
    Ok(())
}

fn validate_buffer_views(
    path: &AssetPath,
    value: Option<&Value>,
    buffer_count: usize,
) -> Result<(), AssetError> {
    for (index, view) in values(value).enumerate() {
        validate_index(
            path,
            view.get("buffer"),
            &format!("$.bufferViews[{index}].buffer"),
            buffer_count,
        )?;
    }
    Ok(())
}

fn validate_textures(
    path: &AssetPath,
    value: Option<&Value>,
    image_count: usize,
    sampler_count: usize,
) -> Result<(), AssetError> {
    for (index, texture) in values(value).enumerate() {
        validate_index(
            path,
            texture.get("source"),
            &format!("$.textures[{index}].source"),
            image_count,
        )?;
        validate_index(
            path,
            texture.get("sampler"),
            &format!("$.textures[{index}].sampler"),
            sampler_count,
        )?;
    }
    Ok(())
}

fn validate_images(
    path: &AssetPath,
    value: Option<&Value>,
    buffer_view_count: usize,
) -> Result<(), AssetError> {
    for (index, image) in values(value).enumerate() {
        validate_index(
            path,
            image.get("bufferView"),
            &format!("$.images[{index}].bufferView"),
            buffer_view_count,
        )?;
    }
    Ok(())
}

fn validate_skins(
    path: &AssetPath,
    value: Option<&Value>,
    node_count: usize,
    accessor_count: usize,
) -> Result<(), AssetError> {
    for (skin_index, skin) in values(value).enumerate() {
        validate_index(
            path,
            skin.get("inverseBindMatrices"),
            &format!("$.skins[{skin_index}].inverseBindMatrices"),
            accessor_count,
        )?;
        validate_index(
            path,
            skin.get("skeleton"),
            &format!("$.skins[{skin_index}].skeleton"),
            node_count,
        )?;
        for (joint_index, joint) in values(skin.get("joints")).enumerate() {
            required_index(
                path,
                joint,
                &format!("$.skins[{skin_index}].joints[{joint_index}]"),
                node_count,
            )?;
        }
    }
    Ok(())
}

fn validate_animations(
    path: &AssetPath,
    value: Option<&Value>,
    node_count: usize,
    accessor_count: usize,
) -> Result<(), AssetError> {
    for (animation_index, animation) in values(value).enumerate() {
        let samplers = animation
            .get("samplers")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        for (sampler_index, sampler) in values(animation.get("samplers")).enumerate() {
            validate_index(
                path,
                sampler.get("input"),
                &format!("$.animations[{animation_index}].samplers[{sampler_index}].input"),
                accessor_count,
            )?;
            validate_index(
                path,
                sampler.get("output"),
                &format!("$.animations[{animation_index}].samplers[{sampler_index}].output"),
                accessor_count,
            )?;
        }
        for (channel_index, channel) in values(animation.get("channels")).enumerate() {
            let base = format!("$.animations[{animation_index}].channels[{channel_index}]");
            validate_index(
                path,
                channel.get("sampler"),
                &format!("{base}.sampler"),
                samplers,
            )?;
            validate_index(
                path,
                channel.get("target").and_then(|target| target.get("node")),
                &format!("{base}.target.node"),
                node_count,
            )?;
        }
    }
    Ok(())
}

fn validate_scenes(
    path: &AssetPath,
    value: Option<&Value>,
    node_count: usize,
) -> Result<(), AssetError> {
    for (scene_index, scene) in values(value).enumerate() {
        for (root_index, node) in values(scene.get("nodes")).enumerate() {
            required_index(
                path,
                node,
                &format!("$.scenes[{scene_index}].nodes[{root_index}]"),
                node_count,
            )?;
        }
    }
    Ok(())
}

fn validate_index(
    path: &AssetPath,
    value: Option<&Value>,
    json_path: &str,
    length: usize,
) -> Result<(), AssetError> {
    let Some(value) = value else {
        return Ok(());
    };
    required_index(path, value, json_path, length).map(|_| ())
}

fn required_index(
    path: &AssetPath,
    value: &Value,
    json_path: &str,
    length: usize,
) -> Result<usize, AssetError> {
    let Some(index) = value.as_u64() else {
        return Err(parse_error(
            path,
            format!("{json_path} must be a non-negative integer index"),
        ));
    };
    if index >= length as u64 {
        return Err(parse_error(
            path,
            format!(
                "{json_path} references index {index}, but the target array length is {length}"
            ),
        ));
    }
    usize::try_from(index).map_err(|_| {
        parse_error(
            path,
            format!("{json_path} index {index} does not fit usize"),
        )
    })
}

fn values(value: Option<&Value>) -> impl Iterator<Item = &Value> {
    value.and_then(Value::as_array).into_iter().flatten()
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

fn parse_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}
