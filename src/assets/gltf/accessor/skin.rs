use crate::diagnostics::AssetError;
use crate::geometry::SkinningMatrix;

use super::super::super::AssetPath;
use super::{
    GL_FLOAT, GL_UNSIGNED_BYTE, GL_UNSIGNED_SHORT, GltfAccessor, GltfBufferView,
    accessor_element_offset, parse_error, read_f32_components, read_normalized_components,
    required_accessor, required_buffer_view,
};

pub(in crate::assets::gltf) fn read_mat4_accessor(
    path: &AssetPath,
    accessor_index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<SkinningMatrix>, AssetError> {
    let accessor = required_accessor(path, accessor_index, accessors)?;
    if accessor.component_type != GL_FLOAT || accessor.kind != "MAT4" {
        return Err(parse_error(path, "expected FLOAT MAT4 accessor"));
    }
    (0..accessor.count)
        .map(|index| {
            let values = read_f32_components(path, accessor, index, 16, buffers, buffer_views)?;
            Ok(SkinningMatrix::from_gltf_column_major(
                values
                    .try_into()
                    .expect("MAT4 component count checked above"),
            ))
        })
        .collect()
}

pub(in crate::assets::gltf) fn read_joints_accessor(
    path: &AssetPath,
    accessor_index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<[usize; 4]>, AssetError> {
    let accessor = required_accessor(path, accessor_index, accessors)?;
    if !matches!(
        accessor.component_type,
        GL_UNSIGNED_BYTE | GL_UNSIGNED_SHORT
    ) || accessor.kind != "VEC4"
    {
        return Err(parse_error(
            path,
            "expected unsigned integer VEC4 JOINTS_0 accessor",
        ));
    }
    (0..accessor.count)
        .map(|index| {
            let values = read_unsigned_components(path, accessor, index, 4, buffers, buffer_views)?;
            Ok([
                values[0] as usize,
                values[1] as usize,
                values[2] as usize,
                values[3] as usize,
            ])
        })
        .collect()
}

pub(in crate::assets::gltf) fn read_weights_accessor(
    path: &AssetPath,
    accessor_index: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
) -> Result<Vec<[f32; 4]>, AssetError> {
    let accessor = required_accessor(path, accessor_index, accessors)?;
    if accessor.kind != "VEC4" {
        return Err(parse_error(path, "expected VEC4 WEIGHTS_0 accessor"));
    }
    let values = match accessor.component_type {
        GL_FLOAT => (0..accessor.count)
            .map(|index| read_f32_components(path, accessor, index, 4, buffers, buffer_views))
            .collect::<Result<Vec<_>, _>>()?,
        GL_UNSIGNED_BYTE | GL_UNSIGNED_SHORT if accessor.normalized => (0..accessor.count)
            .map(|index| {
                read_normalized_components(path, accessor, index, 4, buffers, buffer_views)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(parse_error(
                path,
                "expected FLOAT or normalized unsigned integer VEC4 WEIGHTS_0 accessor",
            ));
        }
    };
    Ok(values
        .into_iter()
        .map(|values| [values[0], values[1], values[2], values[3]])
        .collect())
}

fn read_unsigned_components(
    path: &AssetPath,
    accessor: &GltfAccessor,
    index: usize,
    component_count: usize,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
) -> Result<Vec<u32>, AssetError> {
    let component_size = match accessor.component_type {
        GL_UNSIGNED_BYTE => 1,
        GL_UNSIGNED_SHORT => 2,
        _ => return Err(parse_error(path, "unsupported unsigned component type")),
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
                GL_UNSIGNED_BYTE => u32::from(bytes[0]),
                GL_UNSIGNED_SHORT => u32::from(u16::from_le_bytes(
                    bytes.try_into().expect("slice length checked above"),
                )),
                _ => unreachable!("component type checked above"),
            })
        })
        .collect()
}
