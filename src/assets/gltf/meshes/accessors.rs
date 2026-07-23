use super::*;

/// Read a VEC3 attribute, handling normalized integer accessors that
/// the gltf crate's typed `read_positions/normals` helpers reject due
/// to their hard-coded `[f32; 3]` size assertion. This path is needed
/// for KHR_mesh_quantization where positions/normals can be normalized
/// SHORT or BYTE.
pub(super) fn read_vec3_attribute(
    path: &AssetPath,
    primitive: &Primitive<'_>,
    buffers: &ResolvedGltfBuffers,
    semantic: &Semantic,
    mesh_quantization_declared: bool,
) -> Result<Option<Vec<Vec3>>, AssetError> {
    let Some(accessor) = primitive.get(semantic) else {
        return Ok(None);
    };
    let (label, encoding) = match semantic {
        Semantic::Positions => ("POSITION attribute", Vec3Encoding::Position),
        Semantic::Normals => ("NORMAL attribute", Vec3Encoding::SignedUnit),
        other => {
            return Err(invalid_attribute(
                path,
                other,
                "is not a supported VEC3 stream".to_string(),
            ));
        }
    };
    read_vec3_accessor(
        path,
        buffers,
        accessor,
        label,
        encoding,
        mesh_quantization_declared,
    )
    .map(Some)
}

#[derive(Clone, Copy)]
pub(super) enum Vec3Encoding {
    Position,
    SignedUnit,
}

pub(super) fn read_vec3_accessor(
    path: &AssetPath,
    buffers: &ResolvedGltfBuffers,
    accessor: Accessor<'_>,
    label: &str,
    encoding: Vec3Encoding,
    mesh_quantization_declared: bool,
) -> Result<Vec<Vec3>, AssetError> {
    if accessor.dimensions() != Dimensions::Vec3 {
        return Err(invalid_accessor(
            path,
            label,
            format!("must use VEC3, found {:?}", accessor.dimensions()),
        ));
    }
    require_mesh_quantization_declaration(
        path,
        label,
        accessor.data_type(),
        mesh_quantization_declared,
    )?;
    let get_buffer = |buffer: ::gltf::Buffer<'_>| buffers.reader_buffer(buffer.index());
    macro_rules! read_vec3 {
        ($component:ty, $convert:expr) => {
            AccessorIter::<$component>::new(accessor, get_buffer)
                .map(|iter| iter.map($convert).collect::<Vec<_>>())
        };
    }
    let values = match (encoding, accessor.data_type(), accessor.normalized()) {
        (_, DataType::F32, _) => read_vec3!([f32; 3], Vec3::from_array),
        (Vec3Encoding::Position, DataType::I8, true) => {
            read_vec3!([i8; 3], normalize_i8_vec3)
        }
        (Vec3Encoding::Position, DataType::U8, true) => {
            read_vec3!([u8; 3], normalize_u8_vec3)
        }
        (Vec3Encoding::Position, DataType::I16, true) => {
            read_vec3!([i16; 3], normalize_i16_vec3)
        }
        (Vec3Encoding::Position, DataType::U16, true) => {
            read_vec3!([u16; 3], normalize_u16_vec3)
        }
        (Vec3Encoding::Position, DataType::I8, false) => {
            read_vec3!([i8; 3], raw_i8_vec3)
        }
        (Vec3Encoding::Position, DataType::U8, false) => {
            read_vec3!([u8; 3], raw_u8_vec3)
        }
        (Vec3Encoding::Position, DataType::I16, false) => {
            read_vec3!([i16; 3], raw_i16_vec3)
        }
        (Vec3Encoding::Position, DataType::U16, false) => {
            read_vec3!([u16; 3], raw_u16_vec3)
        }
        (Vec3Encoding::SignedUnit, DataType::I8, true) => {
            read_vec3!([i8; 3], normalize_i8_vec3)
        }
        (Vec3Encoding::SignedUnit, DataType::I16, true) => {
            read_vec3!([i16; 3], normalize_i16_vec3)
        }
        (Vec3Encoding::SignedUnit, data_type, normalized) => {
            return Err(invalid_accessor(
                path,
                label,
                format!(
                    "must use FLOAT or normalized signed BYTE/SHORT; found {data_type:?} with normalized={normalized}"
                ),
            ));
        }
        (_, data_type, normalized) => {
            return Err(invalid_accessor(
                path,
                label,
                format!("unsupported {data_type:?} encoding with normalized={normalized}"),
            ));
        }
    }
    .ok_or_else(|| {
        invalid_accessor(
            path,
            label,
            "buffer view could not be resolved".to_string(),
        )
    })?;
    if let Some((index, value)) = values
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(invalid_accessor(
            path,
            label,
            format!("contains non-finite decoded value at element {index}: {value:?}"),
        ));
    }
    Ok(values)
}

pub(super) fn read_tangent_attribute(
    path: &AssetPath,
    primitive: &Primitive<'_>,
    buffers: &ResolvedGltfBuffers,
    mesh_quantization_declared: bool,
) -> Result<Option<Vec<[f32; 4]>>, AssetError> {
    let Some(accessor) = primitive.get(&Semantic::Tangents) else {
        return Ok(None);
    };
    if accessor.dimensions() != Dimensions::Vec4 {
        return Err(invalid_accessor(
            path,
            "TANGENT attribute",
            format!("must use VEC4, found {:?}", accessor.dimensions()),
        ));
    }
    require_mesh_quantization_declaration(
        path,
        "TANGENT attribute",
        accessor.data_type(),
        mesh_quantization_declared,
    )?;
    let get_buffer = |buffer: ::gltf::Buffer<'_>| buffers.reader_buffer(buffer.index());
    macro_rules! read_vec4 {
        ($component:ty, $convert:expr) => {
            AccessorIter::<$component>::new(accessor, get_buffer)
                .map(|iter| iter.map($convert).collect::<Vec<_>>())
        };
    }
    let values = match (accessor.data_type(), accessor.normalized()) {
        (DataType::F32, _) => read_vec4!([f32; 4], std::convert::identity),
        (DataType::I8, true) => read_vec4!([i8; 4], normalize_i8_vec4),
        (DataType::I16, true) => read_vec4!([i16; 4], normalize_i16_vec4),
        (data_type, normalized) => {
            return Err(invalid_accessor(
                path,
                "TANGENT attribute",
                format!(
                    "must use FLOAT or normalized signed BYTE/SHORT; found {data_type:?} with normalized={normalized}"
                ),
            ));
        }
    }
    .ok_or_else(|| {
        invalid_accessor(
            path,
            "TANGENT attribute",
            "buffer view could not be resolved".to_string(),
        )
    })?;
    if let Some((index, value)) = values
        .iter()
        .enumerate()
        .find(|(_, value)| value.iter().any(|component| !component.is_finite()))
    {
        return Err(invalid_accessor(
            path,
            "TANGENT attribute",
            format!("contains non-finite decoded value at element {index}: {value:?}"),
        ));
    }
    Ok(Some(values))
}

fn require_mesh_quantization_declaration(
    path: &AssetPath,
    label: &str,
    data_type: DataType,
    declared: bool,
) -> Result<(), AssetError> {
    if data_type != DataType::F32 && !declared {
        return Err(invalid_accessor(
            path,
            label,
            "uses integer components but extensionsUsed does not declare KHR_mesh_quantization"
                .to_owned(),
        ));
    }
    Ok(())
}

fn invalid_attribute(path: &AssetPath, semantic: &Semantic, reason: String) -> AssetError {
    let semantic = match semantic {
        Semantic::Positions => "POSITION".to_string(),
        Semantic::Normals => "NORMAL".to_string(),
        other => format!("{other:?}"),
    };
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("glTF {semantic} attribute {reason}"),
    }
}

fn invalid_accessor(path: &AssetPath, label: &str, reason: String) -> AssetError {
    AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("glTF {label} {reason}"),
    }
}

fn raw_i8_vec3(values: [i8; 3]) -> Vec3 {
    Vec3::new(values[0] as f32, values[1] as f32, values[2] as f32)
}

fn raw_u8_vec3(values: [u8; 3]) -> Vec3 {
    Vec3::new(values[0] as f32, values[1] as f32, values[2] as f32)
}

fn raw_i16_vec3(values: [i16; 3]) -> Vec3 {
    Vec3::new(values[0] as f32, values[1] as f32, values[2] as f32)
}

fn raw_u16_vec3(values: [u16; 3]) -> Vec3 {
    Vec3::new(values[0] as f32, values[1] as f32, values[2] as f32)
}

pub(super) fn validate_skin_weight_accessor(
    path: &AssetPath,
    primitive: &Primitive<'_>,
    set: u32,
) -> Result<(), AssetError> {
    let Some(accessor) = primitive.get(&Semantic::Weights(set)) else {
        return Ok(());
    };
    let valid = matches!(
        (accessor.data_type(), accessor.normalized()),
        (DataType::F32, false) | (DataType::U8 | DataType::U16, true)
    );
    if valid {
        Ok(())
    } else {
        Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!(
                "glTF WEIGHTS_{set} must use FLOAT or normalized unsigned BYTE/SHORT; found {:?} with normalized={}",
                accessor.data_type(),
                accessor.normalized(),
            ),
        })
    }
}

pub(super) fn reject_skin_sets_above_one(
    path: &AssetPath,
    primitive: &Primitive<'_>,
) -> Result<(), AssetError> {
    for (semantic, _) in primitive.attributes() {
        let set = match semantic {
            Semantic::Joints(set) | Semantic::Weights(set) => set,
            _ => continue,
        };
        if set > 1 {
            return Err(AssetError::Parse {
                path: path.as_str().to_owned(),
                reason: format!(
                    "glTF skin attribute set {set} exceeds scena's supported JOINTS_0/1 and WEIGHTS_0/1 input limit"
                ),
            });
        }
    }
    Ok(())
}

pub(super) fn joint_indices(joint: [u16; 4]) -> [usize; 4] {
    [
        joint[0] as usize,
        joint[1] as usize,
        joint[2] as usize,
        joint[3] as usize,
    ]
}

fn normalize_i8_vec3(values: [i8; 3]) -> Vec3 {
    Vec3::new(
        (values[0] as f32 / 127.0).max(-1.0),
        (values[1] as f32 / 127.0).max(-1.0),
        (values[2] as f32 / 127.0).max(-1.0),
    )
}

fn normalize_u8_vec3(values: [u8; 3]) -> Vec3 {
    Vec3::new(
        values[0] as f32 / 255.0,
        values[1] as f32 / 255.0,
        values[2] as f32 / 255.0,
    )
}

fn normalize_i16_vec3(values: [i16; 3]) -> Vec3 {
    Vec3::new(
        (values[0] as f32 / 32767.0).max(-1.0),
        (values[1] as f32 / 32767.0).max(-1.0),
        (values[2] as f32 / 32767.0).max(-1.0),
    )
}

fn normalize_i8_vec4(values: [i8; 4]) -> [f32; 4] {
    values.map(|value| (value as f32 / 127.0).max(-1.0))
}

fn normalize_i16_vec4(values: [i16; 4]) -> [f32; 4] {
    values.map(|value| (value as f32 / 32767.0).max(-1.0))
}

fn normalize_u16_vec3(values: [u16; 3]) -> Vec3 {
    Vec3::new(
        values[0] as f32 / 65535.0,
        values[1] as f32 / 65535.0,
        values[2] as f32 / 65535.0,
    )
}
