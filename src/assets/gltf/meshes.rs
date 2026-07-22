//! Stage C2: glTF mesh / primitive parsing now uses the `gltf` crate's
//! typed `Primitive::reader()` so all attribute / index / morph-target
//! reading is delegated to the gltf-crate util module.

use ::gltf::Document;
use ::gltf::Primitive;
use ::gltf::accessor::Iter as AccessorIter;
use ::gltf::accessor::{DataType, Dimensions};
use ::gltf::mesh::{Mode, Semantic};

use crate::diagnostics::AssetError;
use crate::geometry::{GeometryDesc, GeometryMorphTarget, GeometryTopology, GeometryVertex};
use crate::material::{Color, MaterialDesc};
use crate::scene::Vec3;

use super::super::{
    AssetLoadWarning, AssetMaterialSource, AssetPath, AssetStorage, MaterialHandle,
};
use super::SceneAssetMesh;
use super::buffers::ResolvedGltfBuffers;

mod flat_normals;
mod skin_influences;

pub(super) fn parse_meshes(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    materials: &[MaterialHandle],
    storage: &mut AssetStorage,
    load_warnings: &mut Vec<AssetLoadWarning>,
) -> Result<Vec<Vec<SceneAssetMesh>>, AssetError> {
    document
        .meshes()
        .enumerate()
        .map(|(mesh_index, mesh)| {
            let mesh_weights: Vec<f32> = mesh.weights().map(<[f32]>::to_vec).unwrap_or_default();
            mesh.primitives()
                .enumerate()
                .map(|(primitive_index, primitive)| {
                    parse_primitive(PrimitiveParseInputs {
                        path,
                        mesh_index,
                        primitive_index,
                        primitive: &primitive,
                        buffers,
                        mesh_weights: &mesh_weights,
                        materials,
                        storage,
                        load_warnings,
                    })
                })
                .collect()
        })
        .collect()
}

struct PrimitiveParseInputs<'a> {
    path: &'a AssetPath,
    mesh_index: usize,
    primitive_index: usize,
    primitive: &'a Primitive<'a>,
    buffers: &'a ResolvedGltfBuffers,
    mesh_weights: &'a [f32],
    materials: &'a [MaterialHandle],
    storage: &'a mut AssetStorage,
    load_warnings: &'a mut Vec<AssetLoadWarning>,
}

fn parse_primitive(inputs: PrimitiveParseInputs<'_>) -> Result<SceneAssetMesh, AssetError> {
    let PrimitiveParseInputs {
        path,
        mesh_index,
        primitive_index,
        primitive,
        buffers,
        mesh_weights,
        materials,
        storage,
        load_warnings,
    } = inputs;
    let reader = primitive.reader(|buffer| buffers.reader_buffer(buffer.index()));
    let positions = read_vec3_attribute(path, primitive, buffers, &Semantic::Positions)?
        .ok_or_else(|| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "glTF primitive is missing POSITION attribute".to_string(),
        })?;
    let normals = read_vec3_attribute(path, primitive, buffers, &Semantic::Normals)?;
    let vertex_colors: Option<Vec<Color>> = reader.read_colors(0).map(|colors| {
        colors
            .into_rgba_f32()
            .map(|rgba| Color::from_linear_rgba(rgba[0], rgba[1], rgba[2], rgba[3]))
            .collect()
    });
    let tex_coords0: Option<Vec<[f32; 2]>> = reader
        .read_tex_coords(0)
        .map(|tex| tex.into_f32().collect());
    let tangents: Option<Vec<[f32; 4]>> = reader.read_tangents().map(|iter| iter.collect());
    reject_skin_sets_above_one(path, primitive)?;
    validate_skin_weight_accessor(path, primitive, 0)?;
    validate_skin_weight_accessor(path, primitive, 1)?;
    let skin = skin_influences::resolve(
        path,
        skin_influences::SkinSet {
            joints: reader
                .read_joints(0)
                .map(|joints| joints.into_u16().map(joint_indices).collect::<Vec<_>>()),
            weights: reader
                .read_weights(0)
                .map(|weights| weights.into_f32().collect::<Vec<_>>()),
        },
        skin_influences::SkinSet {
            joints: reader
                .read_joints(1)
                .map(|joints| joints.into_u16().map(joint_indices).collect::<Vec<_>>()),
            weights: reader
                .read_weights(1)
                .map(|weights| weights.into_f32().collect::<Vec<_>>()),
        },
    )?;
    if skin.truncated_vertices > 0 {
        load_warnings.push(AssetLoadWarning::SkinInfluencesTruncated {
            path: path.clone(),
            mesh_index,
            primitive_index,
            affected_vertices: skin.truncated_vertices,
            source_influences: 8,
            retained_influences: 4,
        });
    }
    let skin = skin.skin;
    let vertex_count = positions.len();
    let morph_targets = reader
        .read_morph_targets()
        .map(|(positions, normals, tangents)| {
            let position_deltas = positions
                .map(|positions| positions.map(Vec3::from_array).collect::<Vec<_>>())
                .unwrap_or_else(|| vec![Vec3::ZERO; vertex_count]);
            let normal_deltas =
                normals.map(|normals| normals.map(Vec3::from_array).collect::<Vec<_>>());
            let tangent_deltas =
                tangents.map(|tangents| tangents.map(Vec3::from_array).collect::<Vec<_>>());
            GeometryMorphTarget::new_with_semantics(position_deltas, normal_deltas, tangent_deltas)
        })
        .collect::<Vec<_>>();
    let indices: Vec<u32> = reader
        .read_indices()
        .map(|reader| reader.into_u32().collect())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    let computed_flat_normals = normals.is_none();
    let source_triangle_count = indices.len() / 3;
    let flat_normals::ResolvedPrimitiveStreams {
        positions,
        normals,
        indices,
        vertex_colors,
        tex_coords0,
        tangents,
        skin,
        morph_targets,
    } = flat_normals::resolve(flat_normals::PrimitiveStreams {
        path,
        positions,
        normals,
        indices,
        vertex_colors,
        tex_coords0,
        tangents,
        skin,
        morph_targets,
    })?;
    if computed_flat_normals {
        load_warnings.push(AssetLoadWarning::ComputedFlatNormals {
            path: path.clone(),
            mesh_index,
            primitive_index,
            triangle_count: source_triangle_count,
        });
    }
    if normals.len() != positions.len() {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "NORMAL accessor count must match POSITION count".to_string(),
        });
    }
    if let Some(tangents) = tangents.as_ref()
        && tangents.len() != positions.len()
    {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "TANGENT accessor count must match POSITION count".to_string(),
        });
    }
    let topology = match primitive.mode() {
        Mode::Triangles => GeometryTopology::Triangles,
        other => {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("unsupported glTF primitive mode {other:?}"),
            });
        }
    };
    let vertices = positions
        .into_iter()
        .zip(normals)
        .map(|(position, normal)| GeometryVertex { position, normal })
        .collect::<Vec<_>>();
    let uses_vertex_colors = vertex_colors
        .as_ref()
        .is_some_and(|colors| colors.iter().any(|color| *color != Color::WHITE));
    let geometry = GeometryDesc::try_new_with_optional_vertex_attributes(
        topology,
        vertices,
        indices,
        vertex_colors,
        tex_coords0,
    )
    .and_then(|geometry| match tangents {
        Some(tangents) => geometry.with_tangents(tangents),
        None => Ok(geometry),
    })
    .and_then(|geometry| geometry.with_morph_targets(morph_targets))
    .and_then(|geometry| match skin {
        Some(skin) => geometry.with_skin(skin),
        None => Ok(geometry),
    })
    .map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: format!("invalid glTF geometry: {error:?}"),
    })?;
    let bounds = geometry.bounds();
    let geometry = storage.geometries.insert(std::sync::Arc::new(geometry));
    let material = primitive
        .material()
        .index()
        .and_then(|index| materials.get(index))
        .copied()
        .unwrap_or_else(|| {
            let handle = storage
                .materials
                .insert(std::sync::Arc::new(MaterialDesc::default()));
            storage.material_sources.insert(
                handle,
                AssetMaterialSource::generated_default(
                    path.clone(),
                    "source primitive did not reference a material; using glTF default material",
                ),
            );
            handle
        });
    let material_variant_bindings =
        super::material_variants::parse_primitive_material_variant_bindings(primitive, materials);
    Ok(SceneAssetMesh {
        geometry,
        material,
        bounds,
        uses_vertex_colors,
        morph_weights: mesh_weights.to_vec(),
        material_variant_bindings,
    })
}

/// Read a VEC3 attribute, handling normalized integer accessors that
/// the gltf crate's typed `read_positions/normals` helpers reject due
/// to their hard-coded `[f32; 3]` size assertion. This path is needed
/// for KHR_mesh_quantization where positions/normals can be normalized
/// SHORT or BYTE.
fn read_vec3_attribute(
    path: &AssetPath,
    primitive: &Primitive<'_>,
    buffers: &ResolvedGltfBuffers,
    semantic: &Semantic,
) -> Result<Option<Vec<Vec3>>, AssetError> {
    let Some(accessor) = primitive.get(semantic) else {
        return Ok(None);
    };
    if accessor.dimensions() != Dimensions::Vec3 {
        return Err(invalid_attribute(
            path,
            semantic,
            format!("must use VEC3, found {:?}", accessor.dimensions()),
        ));
    }
    let get_buffer = |buffer: ::gltf::Buffer<'_>| buffers.reader_buffer(buffer.index());
    macro_rules! read_vec3 {
        ($component:ty, $convert:expr) => {
            AccessorIter::<$component>::new(accessor, get_buffer)
                .map(|iter| iter.map($convert).collect::<Vec<_>>())
        };
    }
    let values = match (semantic, accessor.data_type(), accessor.normalized()) {
        (_, DataType::F32, _) => read_vec3!([f32; 3], Vec3::from_array),
        (Semantic::Positions, DataType::I8, true) => {
            read_vec3!([i8; 3], normalize_i8_vec3)
        }
        (Semantic::Positions, DataType::U8, true) => {
            read_vec3!([u8; 3], normalize_u8_vec3)
        }
        (Semantic::Positions, DataType::I16, true) => {
            read_vec3!([i16; 3], normalize_i16_vec3)
        }
        (Semantic::Positions, DataType::U16, true) => {
            read_vec3!([u16; 3], normalize_u16_vec3)
        }
        (Semantic::Positions, DataType::I8, false) => {
            read_vec3!([i8; 3], raw_i8_vec3)
        }
        (Semantic::Positions, DataType::U8, false) => {
            read_vec3!([u8; 3], raw_u8_vec3)
        }
        (Semantic::Positions, DataType::I16, false) => {
            read_vec3!([i16; 3], raw_i16_vec3)
        }
        (Semantic::Positions, DataType::U16, false) => {
            read_vec3!([u16; 3], raw_u16_vec3)
        }
        (Semantic::Normals, DataType::I8, true) => {
            read_vec3!([i8; 3], normalize_i8_vec3)
        }
        (Semantic::Normals, DataType::I16, true) => {
            read_vec3!([i16; 3], normalize_i16_vec3)
        }
        (Semantic::Normals, data_type, normalized) => {
            return Err(invalid_attribute(
                path,
                semantic,
                format!(
                    "must use FLOAT or normalized signed BYTE/SHORT; found {data_type:?} with normalized={normalized}"
                ),
            ));
        }
        (_, data_type, normalized) => {
            return Err(invalid_attribute(
                path,
                semantic,
                format!("unsupported {data_type:?} encoding with normalized={normalized}"),
            ));
        }
    }
    .ok_or_else(|| {
        invalid_attribute(
            path,
            semantic,
            "buffer view could not be resolved".to_string(),
        )
    })?;
    Ok(Some(values))
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

fn validate_skin_weight_accessor(
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

fn reject_skin_sets_above_one(
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

fn joint_indices(joint: [u16; 4]) -> [usize; 4] {
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

fn normalize_u16_vec3(values: [u16; 3]) -> Vec3 {
    Vec3::new(
        values[0] as f32 / 65535.0,
        values[1] as f32 / 65535.0,
        values[2] as f32 / 65535.0,
    )
}
