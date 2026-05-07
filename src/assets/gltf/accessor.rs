use base64::Engine;
use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::material::Color;
use crate::scene::Vec3;

use super::super::AssetPath;

pub(super) fn parse_buffers(
    path: &AssetPath,
    json: &JsonValue,
    binary_chunk: Option<&[u8]>,
) -> Result<Vec<Vec<u8>>, AssetError> {
    json.get("buffers")
        .and_then(JsonValue::as_array)
        .map(|buffers| {
            buffers
                .iter()
                .enumerate()
                .map(|(index, buffer)| parse_buffer(path, index, buffer, binary_chunk))
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn parse_buffer(
    path: &AssetPath,
    index: usize,
    buffer: &JsonValue,
    binary_chunk: Option<&[u8]>,
) -> Result<Vec<u8>, AssetError> {
    if let Some(uri) = buffer.get("uri").and_then(JsonValue::as_str) {
        return decode_data_uri(path, uri);
    }
    let byte_length = required_usize(path, buffer, "byteLength")?;
    let Some(binary_chunk) = binary_chunk else {
        return Err(parse_error(
            path,
            "glTF buffer without uri requires a GLB binary chunk",
        ));
    };
    if index != 0 {
        return Err(parse_error(
            path,
            "only the first GLB buffer may be backed by the binary chunk",
        ));
    }
    let bytes = binary_chunk
        .get(..byte_length)
        .ok_or_else(|| parse_error(path, "GLB binary chunk is shorter than buffer byteLength"))?;
    Ok(bytes.to_vec())
}

pub(super) fn parse_buffer_views(
    path: &AssetPath,
    json: &JsonValue,
) -> Result<Vec<GltfBufferView>, AssetError> {
    json.get("bufferViews")
        .and_then(JsonValue::as_array)
        .map(|views| {
            views
                .iter()
                .map(|view| {
                    Ok(GltfBufferView {
                        buffer: optional_usize(view, "buffer").unwrap_or(0),
                        byte_offset: optional_usize(view, "byteOffset").unwrap_or(0),
                        byte_length: required_usize(path, view, "byteLength")?,
                        byte_stride: optional_usize(view, "byteStride"),
                    })
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

pub(super) fn parse_accessors(
    path: &AssetPath,
    json: &JsonValue,
) -> Result<Vec<GltfAccessor>, AssetError> {
    json.get("accessors")
        .and_then(JsonValue::as_array)
        .map(|accessors| {
            accessors
                .iter()
                .map(|accessor| {
                    Ok(GltfAccessor {
                        buffer_view: optional_usize(accessor, "bufferView"),
                        byte_offset: optional_usize(accessor, "byteOffset").unwrap_or(0),
                        component_type: required_u32(path, accessor, "componentType")?,
                        count: required_usize(path, accessor, "count")?,
                        kind: required_string(path, accessor, "type")?.to_string(),
                        normalized: accessor
                            .get("normalized")
                            .and_then(JsonValue::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

pub(super) fn read_vec3_accessor(
    path: &AssetPath,
    accessor_index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<Vec3>, AssetError> {
    let accessor = required_accessor(path, accessor_index, accessors)?;
    if accessor.kind != "VEC3" {
        return Err(parse_error(path, "expected VEC3 accessor"));
    }
    (0..accessor.count)
        .map(|index| {
            let values = read_vec3_components(path, accessor, index, buffers, buffer_views)?;
            Ok(Vec3::new(values[0], values[1], values[2]))
        })
        .collect()
}

pub(super) fn read_color_accessor(
    path: &AssetPath,
    accessor_index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<Color>, AssetError> {
    let accessor = required_accessor(path, accessor_index, accessors)?;
    if accessor.component_type != GL_FLOAT || !matches!(accessor.kind.as_str(), "VEC3" | "VEC4") {
        return Err(parse_error(
            path,
            "expected FLOAT VEC3 or VEC4 COLOR_0 accessor",
        ));
    }
    let component_count = if accessor.kind == "VEC4" { 4 } else { 3 };
    (0..accessor.count)
        .map(|index| {
            let values = read_f32_components(
                path,
                accessor,
                index,
                component_count,
                buffers,
                buffer_views,
            )?;
            Ok(Color::from_linear_rgba(
                values[0],
                values[1],
                values[2],
                values.get(3).copied().unwrap_or(1.0),
            ))
        })
        .collect()
}

pub(super) fn read_indices_accessor(
    path: &AssetPath,
    accessor_index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<u32>, AssetError> {
    let accessor = required_accessor(path, accessor_index, accessors)?;
    if accessor.kind != "SCALAR" {
        return Err(parse_error(path, "expected SCALAR index accessor"));
    }
    (0..accessor.count)
        .map(|index| read_index_component(path, accessor, index, buffers, buffer_views))
        .collect()
}

fn decode_data_uri(path: &AssetPath, uri: &str) -> Result<Vec<u8>, AssetError> {
    let Some((_, encoded)) = uri.split_once(";base64,") else {
        return Err(parse_error(
            path,
            "only embedded base64 glTF buffers are supported in this loader slice",
        ));
    };
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| parse_error(path, format!("invalid embedded buffer base64: {error}")))
}

fn read_f32_components(
    path: &AssetPath,
    accessor: &GltfAccessor,
    index: usize,
    component_count: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
) -> Result<Vec<f32>, AssetError> {
    let offset = accessor_element_offset(path, accessor, index, component_count * 4, buffer_views)?;
    let view = required_buffer_view(path, accessor, buffer_views)?;
    let buffer = buffers
        .get(view.buffer)
        .ok_or_else(|| parse_error(path, "bufferView references missing buffer"))?;
    (0..component_count)
        .map(|component| {
            let start = offset + component * 4;
            let bytes = buffer
                .get(start..start + 4)
                .ok_or_else(|| parse_error(path, "accessor reads past buffer end"))?;
            Ok(f32::from_le_bytes(
                bytes.try_into().expect("slice length checked above"),
            ))
        })
        .collect()
}

fn read_vec3_components(
    path: &AssetPath,
    accessor: &GltfAccessor,
    index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
) -> Result<Vec<f32>, AssetError> {
    match accessor.component_type {
        GL_FLOAT => read_f32_components(path, accessor, index, 3, buffers, buffer_views),
        GL_BYTE | GL_UNSIGNED_BYTE | GL_SHORT | GL_UNSIGNED_SHORT if accessor.normalized => {
            read_normalized_components(path, accessor, index, 3, buffers, buffer_views)
        }
        _ => Err(parse_error(
            path,
            "expected FLOAT VEC3 accessor or normalized integer VEC3 accessor",
        )),
    }
}

fn read_normalized_components(
    path: &AssetPath,
    accessor: &GltfAccessor,
    index: usize,
    component_count: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
) -> Result<Vec<f32>, AssetError> {
    let component_size = match accessor.component_type {
        GL_BYTE | GL_UNSIGNED_BYTE => 1,
        GL_SHORT | GL_UNSIGNED_SHORT => 2,
        _ => return Err(parse_error(path, "unsupported normalized component type")),
    };
    let offset = accessor_element_offset(
        path,
        accessor,
        index,
        component_count * component_size,
        buffer_views,
    )?;
    let view = required_buffer_view(path, accessor, buffer_views)?;
    let buffer = buffers
        .get(view.buffer)
        .ok_or_else(|| parse_error(path, "bufferView references missing buffer"))?;
    (0..component_count)
        .map(|component| {
            let start = offset + component * component_size;
            let bytes = buffer
                .get(start..start + component_size)
                .ok_or_else(|| parse_error(path, "accessor reads past buffer end"))?;
            Ok(match accessor.component_type {
                GL_BYTE => f32::from(i8::from_le_bytes([bytes[0]])).max(-127.0) / 127.0,
                GL_UNSIGNED_BYTE => f32::from(bytes[0]) / 255.0,
                GL_SHORT => {
                    f32::from(i16::from_le_bytes(
                        bytes.try_into().expect("slice length checked above"),
                    ))
                    .max(-32767.0)
                        / 32767.0
                }
                GL_UNSIGNED_SHORT => {
                    f32::from(u16::from_le_bytes(
                        bytes.try_into().expect("slice length checked above"),
                    )) / 65535.0
                }
                _ => unreachable!("component type checked above"),
            })
        })
        .collect()
}

fn read_index_component(
    path: &AssetPath,
    accessor: &GltfAccessor,
    index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
) -> Result<u32, AssetError> {
    let component_size = match accessor.component_type {
        GL_UNSIGNED_BYTE => 1,
        GL_UNSIGNED_SHORT => 2,
        GL_UNSIGNED_INT => 4,
        _ => return Err(parse_error(path, "unsupported index component type")),
    };
    let offset = accessor_element_offset(path, accessor, index, component_size, buffer_views)?;
    let view = required_buffer_view(path, accessor, buffer_views)?;
    let buffer = buffers
        .get(view.buffer)
        .ok_or_else(|| parse_error(path, "bufferView references missing buffer"))?;
    let bytes = buffer
        .get(offset..offset + component_size)
        .ok_or_else(|| parse_error(path, "index accessor reads past buffer end"))?;
    Ok(match accessor.component_type {
        GL_UNSIGNED_BYTE => u32::from(bytes[0]),
        GL_UNSIGNED_SHORT => u32::from(u16::from_le_bytes(
            bytes.try_into().expect("slice length checked above"),
        )),
        GL_UNSIGNED_INT => {
            u32::from_le_bytes(bytes.try_into().expect("slice length checked above"))
        }
        _ => unreachable!("component type checked above"),
    })
}

fn accessor_element_offset(
    path: &AssetPath,
    accessor: &GltfAccessor,
    index: usize,
    packed_size: usize,
    buffer_views: &[GltfBufferView],
) -> Result<usize, AssetError> {
    let view = required_buffer_view(path, accessor, buffer_views)?;
    let stride = view.byte_stride.unwrap_or(packed_size);
    let view_relative_end = accessor.byte_offset + index * stride + packed_size;
    if view_relative_end > view.byte_length {
        return Err(parse_error(path, "accessor reads past bufferView end"));
    }
    Ok(view.byte_offset + accessor.byte_offset + index * stride)
}

fn required_accessor<'a>(
    path: &AssetPath,
    index: usize,
    accessors: &'a [GltfAccessor],
) -> Result<&'a GltfAccessor, AssetError> {
    accessors
        .get(index)
        .ok_or_else(|| parse_error(path, format!("missing accessor {index}")))
}

fn required_buffer_view<'a>(
    path: &AssetPath,
    accessor: &GltfAccessor,
    buffer_views: &'a [GltfBufferView],
) -> Result<&'a GltfBufferView, AssetError> {
    let view = accessor
        .buffer_view
        .ok_or_else(|| parse_error(path, "accessor without bufferView is not supported"))?;
    buffer_views
        .get(view)
        .ok_or_else(|| parse_error(path, format!("missing bufferView {view}")))
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GltfBufferView {
    buffer: usize,
    byte_offset: usize,
    byte_length: usize,
    byte_stride: Option<usize>,
}

#[derive(Debug, Clone)]
pub(super) struct GltfAccessor {
    buffer_view: Option<usize>,
    byte_offset: usize,
    component_type: u32,
    count: usize,
    kind: String,
    normalized: bool,
}

const GL_FLOAT: u32 = 5126;
const GL_BYTE: u32 = 5120;
const GL_UNSIGNED_BYTE: u32 = 5121;
const GL_SHORT: u32 = 5122;
const GL_UNSIGNED_SHORT: u32 = 5123;
const GL_UNSIGNED_INT: u32 = 5125;

fn required_string<'a>(
    path: &AssetPath,
    value: &'a JsonValue,
    field: &str,
) -> Result<&'a str, AssetError> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| parse_error(path, format!("missing string field {field}")))
}

pub(super) fn required_usize(
    path: &AssetPath,
    value: &JsonValue,
    field: &str,
) -> Result<usize, AssetError> {
    optional_usize(value, field)
        .ok_or_else(|| parse_error(path, format!("missing usize field {field}")))
}

fn required_u32(path: &AssetPath, value: &JsonValue, field: &str) -> Result<u32, AssetError> {
    value
        .get(field)
        .and_then(JsonValue::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| parse_error(path, format!("missing u32 field {field}")))
}

pub(super) fn optional_usize(value: &JsonValue, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(JsonValue::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

pub(super) fn parse_error(path: &AssetPath, reason: impl Into<String>) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: reason.into(),
    }
}
