use super::*;

/// Identifies the primitive whose accessors are being decoded, so every stream
/// error names the mesh, the primitive, and the attribute that caused it.
#[derive(Clone, Copy)]
pub(super) struct PrimitiveSource<'a> {
    pub(super) path: &'a AssetPath,
    pub(super) buffers: &'a ResolvedGltfBuffers,
    pub(super) mesh_index: usize,
    pub(super) primitive_index: usize,
    pub(super) mesh_quantization_declared: bool,
}

impl<'a> PrimitiveSource<'a> {
    pub(super) fn label(&self, stream: &str) -> String {
        format!(
            "mesh {} primitive {} {stream}",
            self.mesh_index, self.primitive_index
        )
    }

    pub(super) fn error(&self, stream: &str, reason: String) -> AssetError {
        accessor_error(self.path, &self.label(stream), reason)
    }

    /// Opens one accessor of this primitive as a checked stream of `T`.
    pub(super) fn stream<T: AccessorItem>(
        &self,
        accessor: &Accessor<'_>,
        stream: &str,
    ) -> Result<AccessorIter<'a, T>, AssetError> {
        typed_accessor_iter(self.path, self.buffers, accessor, &self.label(stream))
    }

    /// Integer encodings other than the core-specification ones are legal only
    /// when the document declares `KHR_mesh_quantization`.
    pub(super) fn require_mesh_quantization(&self, stream: &str) -> Result<(), AssetError> {
        if self.mesh_quantization_declared {
            return Ok(());
        }
        Err(self.error(
            stream,
            "uses integer components but extensionsUsed does not declare KHR_mesh_quantization"
                .to_owned(),
        ))
    }
}

/// Read a VEC3 attribute, handling normalized integer accessors that
/// the gltf crate's typed `read_positions/normals` helpers reject due
/// to their hard-coded `[f32; 3]` size assertion. This path is needed
/// for KHR_mesh_quantization where positions/normals can be normalized
/// SHORT or BYTE.
pub(super) fn read_vec3_attribute(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
    semantic: &Semantic,
) -> Result<Option<Vec<Vec3>>, AssetError> {
    let Some(accessor) = primitive.get(semantic) else {
        return Ok(None);
    };
    let (stream, encoding) = match semantic {
        Semantic::Positions => ("POSITION attribute", Vec3Encoding::Position),
        Semantic::Normals => ("NORMAL attribute", Vec3Encoding::SignedUnit),
        other => {
            return Err(source.error(
                &format!("{other:?} attribute"),
                "is not a supported VEC3 stream".to_owned(),
            ));
        }
    };
    read_vec3_accessor(source, accessor, stream, encoding).map(Some)
}

#[derive(Clone, Copy)]
pub(super) enum Vec3Encoding {
    Position,
    SignedUnit,
}

pub(super) fn read_vec3_accessor(
    source: PrimitiveSource<'_>,
    accessor: Accessor<'_>,
    stream: &str,
    encoding: Vec3Encoding,
) -> Result<Vec<Vec3>, AssetError> {
    if accessor.dimensions() != Dimensions::Vec3 {
        return Err(source.error(
            stream,
            format!("must use VEC3, found {:?}", accessor.dimensions()),
        ));
    }
    if accessor.data_type() != DataType::F32 {
        source.require_mesh_quantization(stream)?;
    }
    macro_rules! read_vec3 {
        ($component:ty, $convert:expr) => {
            source
                .stream::<$component>(&accessor, stream)?
                .map($convert)
                .collect::<Vec<_>>()
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
            return Err(source.error(
                stream,
                format!(
                    "must use FLOAT or normalized signed BYTE/SHORT; found {data_type:?} with normalized={normalized}"
                ),
            ));
        }
        (_, data_type, normalized) => {
            return Err(source.error(
                stream,
                format!("unsupported {data_type:?} encoding with normalized={normalized}"),
            ));
        }
    };
    if let Some((index, value)) = values
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(source.error(
            stream,
            format!("contains non-finite decoded value at element {index}: {value:?}"),
        ));
    }
    Ok(values)
}

pub(super) fn read_tangent_attribute(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
) -> Result<Option<Vec<[f32; 4]>>, AssetError> {
    const STREAM: &str = "TANGENT attribute";
    let Some(accessor) = primitive.get(&Semantic::Tangents) else {
        return Ok(None);
    };
    if accessor.dimensions() != Dimensions::Vec4 {
        return Err(source.error(
            STREAM,
            format!("must use VEC4, found {:?}", accessor.dimensions()),
        ));
    }
    if accessor.data_type() != DataType::F32 {
        source.require_mesh_quantization(STREAM)?;
    }
    macro_rules! read_vec4 {
        ($component:ty, $convert:expr) => {
            source
                .stream::<$component>(&accessor, STREAM)?
                .map($convert)
                .collect::<Vec<_>>()
        };
    }
    let values = match (accessor.data_type(), accessor.normalized()) {
        (DataType::F32, _) => read_vec4!([f32; 4], std::convert::identity),
        (DataType::I8, true) => read_vec4!([i8; 4], normalize_i8_vec4),
        (DataType::I16, true) => read_vec4!([i16; 4], normalize_i16_vec4),
        (data_type, normalized) => {
            return Err(source.error(
                STREAM,
                format!(
                    "must use FLOAT or normalized signed BYTE/SHORT; found {data_type:?} with normalized={normalized}"
                ),
            ));
        }
    };
    if let Some((index, value)) = values
        .iter()
        .enumerate()
        .find(|(_, value)| value.iter().any(|component| !component.is_finite()))
    {
        return Err(source.error(
            STREAM,
            format!("contains non-finite decoded value at element {index}: {value:?}"),
        ));
    }
    Ok(Some(values))
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

pub(super) fn reject_skin_sets_above_one(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
) -> Result<(), AssetError> {
    for (semantic, _) in primitive.attributes() {
        let (name, set) = match semantic {
            Semantic::Joints(set) => ("JOINTS", set),
            Semantic::Weights(set) => ("WEIGHTS", set),
            _ => continue,
        };
        if set > 1 {
            return Err(source.error(
                &format!("{name}_{set} attribute"),
                format!(
                    "uses skin attribute set {set}, which exceeds scena's supported JOINTS_0/1 and WEIGHTS_0/1 input limit"
                ),
            ));
        }
    }
    Ok(())
}

fn normalize_i8_vec3(values: [i8; 3]) -> Vec3 {
    Vec3::from_array(values.map(normalize_i8))
}

fn normalize_u8_vec3(values: [u8; 3]) -> Vec3 {
    Vec3::from_array(values.map(normalize_u8))
}

fn normalize_i16_vec3(values: [i16; 3]) -> Vec3 {
    Vec3::from_array(values.map(normalize_i16))
}

fn normalize_i8_vec4(values: [i8; 4]) -> [f32; 4] {
    values.map(normalize_i8)
}

fn normalize_i16_vec4(values: [i16; 4]) -> [f32; 4] {
    values.map(normalize_i16)
}

fn normalize_u16_vec3(values: [u16; 3]) -> Vec3 {
    Vec3::from_array(values.map(normalize_u16))
}
