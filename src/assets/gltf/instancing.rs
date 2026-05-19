use ::gltf::accessor::{DataType, Dimensions, Iter as AccessorIter};
use ::gltf::{Accessor, Document, Node};
use serde_json::Value;

use crate::diagnostics::AssetError;
use crate::scene::{Quat, Transform, Vec3};

use super::AssetPath;
use super::buffers::ResolvedGltfBuffers;

const EXTENSION: &str = "EXT_mesh_gpu_instancing";

pub(super) fn parse_node_instance_transforms(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    node: &Node<'_>,
) -> Result<Vec<Transform>, AssetError> {
    let Some(extension) = node.extension_value(EXTENSION) else {
        return Ok(Vec::new());
    };
    if node.mesh().is_none() {
        return Err(parse_error(
            path,
            format!(
                "{EXTENSION} appears on node {} without a mesh",
                node_label(node)
            ),
        ));
    }
    let attributes = extension
        .get("attributes")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            parse_error(
                path,
                format!(
                    "{EXTENSION} on node {} must contain an attributes object",
                    node_label(node)
                ),
            )
        })?;
    let translations =
        read_optional_vec3_attribute(path, document, buffers, attributes, "TRANSLATION")?;
    let rotations = read_optional_rotation_attribute(path, document, buffers, attributes)?;
    let scales = read_optional_vec3_attribute(path, document, buffers, attributes, "SCALE")?;
    let count = translations
        .as_ref()
        .map(Vec::len)
        .or_else(|| rotations.as_ref().map(Vec::len))
        .or_else(|| scales.as_ref().map(Vec::len))
        .ok_or_else(|| {
            parse_error(
                path,
                format!(
                    "{EXTENSION} on node {} must provide TRANSLATION, ROTATION, or SCALE",
                    node_label(node)
                ),
            )
        })?;
    validate_attribute_count(path, node, "TRANSLATION", translations.as_ref(), count)?;
    validate_attribute_count(path, node, "ROTATION", rotations.as_ref(), count)?;
    validate_attribute_count(path, node, "SCALE", scales.as_ref(), count)?;

    Ok((0..count)
        .map(|index| Transform {
            translation: translations
                .as_ref()
                .map_or(Vec3::ZERO, |values| values[index]),
            rotation: rotations
                .as_ref()
                .map_or(Quat::IDENTITY, |values| values[index]),
            scale: scales.as_ref().map_or(Vec3::ONE, |values| values[index]),
        })
        .collect())
}

fn read_optional_vec3_attribute(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    attributes: &serde_json::Map<String, Value>,
    semantic: &'static str,
) -> Result<Option<Vec<Vec3>>, AssetError> {
    let Some(index) = optional_accessor_index(path, attributes, semantic)? else {
        return Ok(None);
    };
    let accessor = accessor_by_index(path, document, index, semantic)?;
    if accessor.dimensions() != Dimensions::Vec3 || accessor.data_type() != DataType::F32 {
        return Err(parse_error(
            path,
            format!("{EXTENSION} {semantic} accessor must be FLOAT VEC3"),
        ));
    }
    let get_buffer = |buffer: ::gltf::Buffer<'_>| buffers.reader_buffer(buffer.index());
    let values = AccessorIter::<[f32; 3]>::new(accessor, get_buffer)
        .ok_or_else(|| {
            parse_error(
                path,
                format!("{EXTENSION} {semantic} accessor is unreadable"),
            )
        })?
        .map(Vec3::from_array)
        .collect();
    Ok(Some(values))
}

fn read_optional_rotation_attribute(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    attributes: &serde_json::Map<String, Value>,
) -> Result<Option<Vec<Quat>>, AssetError> {
    let Some(index) = optional_accessor_index(path, attributes, "ROTATION")? else {
        return Ok(None);
    };
    let accessor = accessor_by_index(path, document, index, "ROTATION")?;
    if accessor.dimensions() != Dimensions::Vec4 {
        return Err(parse_error(
            path,
            format!("{EXTENSION} ROTATION accessor must be VEC4"),
        ));
    }
    let get_buffer = |buffer: ::gltf::Buffer<'_>| buffers.reader_buffer(buffer.index());
    let rotations = match (accessor.data_type(), accessor.normalized()) {
        (DataType::F32, _) => AccessorIter::<[f32; 4]>::new(accessor, get_buffer)
            .ok_or_else(|| {
                parse_error(path, format!("{EXTENSION} ROTATION accessor is unreadable"))
            })?
            .map(|values| quat_from_xyzw(path, values))
            .collect::<Result<Vec<_>, _>>()?,
        (DataType::I8, true) => AccessorIter::<[i8; 4]>::new(accessor, get_buffer)
            .ok_or_else(|| {
                parse_error(path, format!("{EXTENSION} ROTATION accessor is unreadable"))
            })?
            .map(|values| {
                quat_from_xyzw(
                    path,
                    [
                        normalize_i8(values[0]),
                        normalize_i8(values[1]),
                        normalize_i8(values[2]),
                        normalize_i8(values[3]),
                    ],
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
        (DataType::I16, true) => AccessorIter::<[i16; 4]>::new(accessor, get_buffer)
            .ok_or_else(|| {
                parse_error(path, format!("{EXTENSION} ROTATION accessor is unreadable"))
            })?
            .map(|values| {
                quat_from_xyzw(
                    path,
                    [
                        normalize_i16(values[0]),
                        normalize_i16(values[1]),
                        normalize_i16(values[2]),
                        normalize_i16(values[3]),
                    ],
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(parse_error(
                path,
                format!(
                    "{EXTENSION} ROTATION accessor must be FLOAT, normalized BYTE, or normalized SHORT VEC4"
                ),
            ));
        }
    };
    Ok(Some(rotations))
}

fn optional_accessor_index(
    path: &AssetPath,
    attributes: &serde_json::Map<String, Value>,
    semantic: &'static str,
) -> Result<Option<usize>, AssetError> {
    let Some(value) = attributes.get(semantic) else {
        return Ok(None);
    };
    let Some(index) = value.as_u64() else {
        return Err(parse_error(
            path,
            format!("{EXTENSION} {semantic} attribute must be an accessor index"),
        ));
    };
    usize::try_from(index).map(Some).map_err(|_| {
        parse_error(
            path,
            format!("{EXTENSION} {semantic} accessor index is too large"),
        )
    })
}

fn accessor_by_index<'a>(
    path: &AssetPath,
    document: &'a Document,
    index: usize,
    semantic: &'static str,
) -> Result<Accessor<'a>, AssetError> {
    document.accessors().nth(index).ok_or_else(|| {
        parse_error(
            path,
            format!("{EXTENSION} {semantic} references missing accessor {index}"),
        )
    })
}

fn validate_attribute_count<T>(
    path: &AssetPath,
    node: &Node<'_>,
    semantic: &'static str,
    values: Option<&Vec<T>>,
    expected: usize,
) -> Result<(), AssetError> {
    let Some(values) = values else {
        return Ok(());
    };
    if values.len() == expected {
        return Ok(());
    }
    Err(parse_error(
        path,
        format!(
            "{EXTENSION} {semantic} accessor on node {} has count {}, expected {expected}",
            node_label(node),
            values.len()
        ),
    ))
}

fn quat_from_xyzw(path: &AssetPath, values: [f32; 4]) -> Result<Quat, AssetError> {
    let rotation = Quat::from_xyzw(values[0], values[1], values[2], values[3]);
    let length_squared = rotation.length_squared();
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return Err(parse_error(
            path,
            format!("{EXTENSION} ROTATION accessor contains an invalid quaternion"),
        ));
    }
    Ok(rotation.normalize())
}

fn normalize_i8(value: i8) -> f32 {
    (value as f32 / 127.0).max(-1.0)
}

fn normalize_i16(value: i16) -> f32 {
    (value as f32 / 32767.0).max(-1.0)
}

fn node_label(node: &Node<'_>) -> String {
    node.name()
        .map(str::to_string)
        .unwrap_or_else(|| format!("#{}", node.index()))
}

fn parse_error(path: &AssetPath, reason: String) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason,
    }
}
