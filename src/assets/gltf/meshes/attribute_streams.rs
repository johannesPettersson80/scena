//! Typed decoding of the texture-coordinate, color, skin, and index streams.
//!
//! Each reader accepts exactly the layouts the glTF 2.0 specification and
//! `KHR_mesh_quantization` define for its stream and reports anything else
//! with the mesh, primitive, and attribute. The `gltf` crate's own
//! `read_tex_coords`/`read_colors`/`read_joints`/`read_weights`/`read_indices`
//! helpers are deliberately not used: they reach `unreachable!()` for an
//! unexpected component type and treat every integer as normalized.

use super::*;

/// Reads `TEXCOORD_<set>`.
///
/// FLOAT and normalized UNSIGNED_BYTE/UNSIGNED_SHORT are core glTF. Signed and
/// non-normalized integer layouts come from `KHR_mesh_quantization`; a
/// non-normalized value keeps its integer magnitude because the dequantization
/// scale lives in `KHR_texture_transform`, not in the accessor.
pub(super) fn read_tex_coords(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
    set: u32,
) -> Result<Option<Vec<[f32; 2]>>, AssetError> {
    let Some(accessor) = primitive.get(&Semantic::TexCoords(set)) else {
        return Ok(None);
    };
    let stream = format!("TEXCOORD_{set} attribute");
    if accessor.dimensions() != Dimensions::Vec2 {
        return Err(source.error(
            &stream,
            format!("must use VEC2, found {:?}", accessor.dimensions()),
        ));
    }
    let data_type = accessor.data_type();
    let normalized = accessor.normalized();
    let core_layout = matches!(
        (data_type, normalized),
        (DataType::F32, _) | (DataType::U8 | DataType::U16, true)
    );
    if !core_layout && data_type != DataType::U32 {
        source.require_mesh_quantization(&stream)?;
    }
    macro_rules! read_vec2 {
        ($component:ty, $convert:expr) => {
            source
                .stream::<[$component; 2]>(&accessor, &stream)?
                .map(|value| value.map($convert))
                .collect::<Vec<_>>()
        };
    }
    let values = match (data_type, normalized) {
        (DataType::F32, _) => read_vec2!(f32, std::convert::identity),
        (DataType::I8, true) => read_vec2!(i8, normalize_i8),
        (DataType::U8, true) => read_vec2!(u8, normalize_u8),
        (DataType::I16, true) => read_vec2!(i16, normalize_i16),
        (DataType::U16, true) => read_vec2!(u16, normalize_u16),
        (DataType::I8, false) => read_vec2!(i8, f32::from),
        (DataType::U8, false) => read_vec2!(u8, f32::from),
        (DataType::I16, false) => read_vec2!(i16, f32::from),
        (DataType::U16, false) => read_vec2!(u16, f32::from),
        (DataType::U32, _) => {
            return Err(source.error(
                &stream,
                "must use FLOAT, BYTE, UNSIGNED_BYTE, SHORT, or UNSIGNED_SHORT components; found UNSIGNED_INT"
                    .to_owned(),
            ));
        }
    };
    Ok(Some(values))
}

/// Reads `COLOR_<set>` as linear RGBA; RGB accessors get an alpha of one.
pub(super) fn read_colors(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
    set: u32,
) -> Result<Option<Vec<Color>>, AssetError> {
    let Some(accessor) = primitive.get(&Semantic::Colors(set)) else {
        return Ok(None);
    };
    let stream = format!("COLOR_{set} attribute");
    macro_rules! read_color {
        ($element:ty, $convert:expr) => {
            source
                .stream::<$element>(&accessor, &stream)?
                .map(|value| value.map($convert))
                .map(color_from_components)
                .collect::<Vec<_>>()
        };
    }
    let values = match (
        accessor.data_type(),
        accessor.normalized(),
        accessor.dimensions(),
    ) {
        (DataType::F32, _, Dimensions::Vec3) => read_color!([f32; 3], std::convert::identity),
        (DataType::F32, _, Dimensions::Vec4) => read_color!([f32; 4], std::convert::identity),
        (DataType::U8, true, Dimensions::Vec3) => read_color!([u8; 3], normalize_u8),
        (DataType::U8, true, Dimensions::Vec4) => read_color!([u8; 4], normalize_u8),
        (DataType::U16, true, Dimensions::Vec3) => read_color!([u16; 3], normalize_u16),
        (DataType::U16, true, Dimensions::Vec4) => read_color!([u16; 4], normalize_u16),
        (data_type, normalized, dimensions) => {
            return Err(source.error(
                &stream,
                format!(
                    "must use VEC3 or VEC4 with FLOAT or normalized UNSIGNED_BYTE/UNSIGNED_SHORT components; found {data_type:?} {dimensions:?} with normalized={normalized}"
                ),
            ));
        }
    };
    Ok(Some(values))
}

/// Reads `JOINTS_<set>` joint indices.
pub(super) fn read_joints(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
    set: u32,
) -> Result<Option<Vec<[usize; 4]>>, AssetError> {
    let Some(accessor) = primitive.get(&Semantic::Joints(set)) else {
        return Ok(None);
    };
    let stream = format!("JOINTS_{set} attribute");
    let values = match (accessor.data_type(), accessor.dimensions()) {
        (DataType::U8, Dimensions::Vec4) => source
            .stream::<[u8; 4]>(&accessor, &stream)?
            .map(|joints| joints.map(usize::from))
            .collect::<Vec<_>>(),
        (DataType::U16, Dimensions::Vec4) => source
            .stream::<[u16; 4]>(&accessor, &stream)?
            .map(|joints| joints.map(usize::from))
            .collect::<Vec<_>>(),
        (data_type, dimensions) => {
            return Err(source.error(
                &stream,
                format!(
                    "must use VEC4 with UNSIGNED_BYTE or UNSIGNED_SHORT components; found {data_type:?} {dimensions:?}"
                ),
            ));
        }
    };
    Ok(Some(values))
}

/// Reads `WEIGHTS_<set>` skin weights without renormalizing them.
pub(super) fn read_weights(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
    set: u32,
) -> Result<Option<Vec<[f32; 4]>>, AssetError> {
    let Some(accessor) = primitive.get(&Semantic::Weights(set)) else {
        return Ok(None);
    };
    let stream = format!("WEIGHTS_{set} attribute");
    let values = match (
        accessor.data_type(),
        accessor.normalized(),
        accessor.dimensions(),
    ) {
        (DataType::F32, false, Dimensions::Vec4) => source
            .stream::<[f32; 4]>(&accessor, &stream)?
            .collect::<Vec<_>>(),
        (DataType::U8, true, Dimensions::Vec4) => source
            .stream::<[u8; 4]>(&accessor, &stream)?
            .map(|weights| weights.map(normalize_u8))
            .collect::<Vec<_>>(),
        (DataType::U16, true, Dimensions::Vec4) => source
            .stream::<[u16; 4]>(&accessor, &stream)?
            .map(|weights| weights.map(normalize_u16))
            .collect::<Vec<_>>(),
        (data_type, normalized, dimensions) => {
            return Err(source.error(
                &stream,
                format!(
                    "must use VEC4 with FLOAT or normalized unsigned BYTE/SHORT components; found {data_type:?} {dimensions:?} with normalized={normalized}"
                ),
            ));
        }
    };
    Ok(Some(values))
}

/// Reads the primitive's index accessor, widening every legal width to `u32`.
pub(super) fn read_indices(
    source: PrimitiveSource<'_>,
    primitive: &Primitive<'_>,
) -> Result<Option<Vec<u32>>, AssetError> {
    const STREAM: &str = "indices";
    let Some(accessor) = primitive.indices() else {
        return Ok(None);
    };
    let values = match (accessor.data_type(), accessor.dimensions()) {
        (DataType::U8, Dimensions::Scalar) => source
            .stream::<u8>(&accessor, STREAM)?
            .map(u32::from)
            .collect::<Vec<_>>(),
        (DataType::U16, Dimensions::Scalar) => source
            .stream::<u16>(&accessor, STREAM)?
            .map(u32::from)
            .collect::<Vec<_>>(),
        (DataType::U32, Dimensions::Scalar) => {
            source.stream::<u32>(&accessor, STREAM)?.collect::<Vec<_>>()
        }
        (data_type, dimensions) => {
            return Err(source.error(
                STREAM,
                format!(
                    "must use SCALAR with UNSIGNED_BYTE, UNSIGNED_SHORT, or UNSIGNED_INT components; found {data_type:?} {dimensions:?}"
                ),
            ));
        }
    };
    Ok(Some(values))
}

fn color_from_components<const N: usize>(components: [f32; N]) -> Color {
    let alpha = components.get(3).copied().unwrap_or(1.0);
    Color::from_linear_rgba(components[0], components[1], components[2], alpha)
}
