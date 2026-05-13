//! Stage C2: glTF mesh / primitive parsing now uses the `gltf` crate's
//! typed `Primitive::reader()` so all attribute / index / morph-target
//! reading is delegated to the gltf-crate util module.

use ::gltf::Document;
use ::gltf::Primitive;
use ::gltf::accessor::Iter as AccessorIter;
use ::gltf::accessor::{DataType, Dimensions};
use ::gltf::mesh::{Mode, Semantic};

use crate::diagnostics::AssetError;
use crate::geometry::{
    GeometryDesc, GeometryMorphTarget, GeometrySkin, GeometryTopology, GeometryVertex,
};
use crate::material::{Color, MaterialDesc};
use crate::scene::Vec3;

use super::super::{AssetPath, AssetStorage, MaterialHandle};
use super::SceneAssetMesh;
use super::buffers::ResolvedGltfBuffers;

pub(super) fn parse_meshes(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    materials: &[MaterialHandle],
    storage: &mut AssetStorage,
) -> Result<Vec<Vec<SceneAssetMesh>>, AssetError> {
    document
        .meshes()
        .map(|mesh| {
            let mesh_weights: Vec<f32> = mesh.weights().map(<[f32]>::to_vec).unwrap_or_default();
            mesh.primitives()
                .map(|primitive| {
                    parse_primitive(path, &primitive, buffers, &mesh_weights, materials, storage)
                })
                .collect()
        })
        .collect()
}

fn parse_primitive(
    path: &AssetPath,
    primitive: &Primitive<'_>,
    buffers: &ResolvedGltfBuffers,
    mesh_weights: &[f32],
    materials: &[MaterialHandle],
    storage: &mut AssetStorage,
) -> Result<SceneAssetMesh, AssetError> {
    let reader = primitive.reader(|buffer| buffers.reader_buffer(buffer.index()));
    let positions =
        read_vec3_attribute(primitive, buffers, &Semantic::Positions)?.ok_or_else(|| {
            AssetError::Parse {
                path: path.as_str().to_string(),
                reason: "glTF primitive is missing POSITION attribute".to_string(),
            }
        })?;
    let normals = read_vec3_attribute(primitive, buffers, &Semantic::Normals)?
        .unwrap_or_else(|| vec![Vec3::new(0.0, 0.0, 1.0); positions.len()]);
    let vertex_colors: Vec<Color> = reader
        .read_colors(0)
        .map(|colors| {
            colors
                .into_rgba_f32()
                .map(|rgba| Color::from_linear_rgba(rgba[0], rgba[1], rgba[2], rgba[3]))
                .collect()
        })
        .unwrap_or_else(|| vec![Color::WHITE; positions.len()]);
    let tex_coords0: Vec<[f32; 2]> = reader
        .read_tex_coords(0)
        .map(|tex| tex.into_f32().collect())
        .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
    let tangents: Option<Vec<[f32; 4]>> = reader.read_tangents().map(|iter| iter.collect());
    let skin = match (reader.read_joints(0), reader.read_weights(0)) {
        (Some(joints), Some(weights)) => {
            let joints: Vec<[usize; 4]> = joints
                .into_u16()
                .map(|joint| {
                    [
                        joint[0] as usize,
                        joint[1] as usize,
                        joint[2] as usize,
                        joint[3] as usize,
                    ]
                })
                .collect();
            let weights: Vec<[f32; 4]> = weights.into_f32().collect();
            Some(GeometrySkin::new(joints, weights))
        }
        (None, None) => None,
        _ => {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: "JOINTS_0 and WEIGHTS_0 must be provided together for skinned geometry"
                    .to_string(),
            });
        }
    };
    let morph_targets = reader
        .read_morph_targets()
        .filter_map(|(positions, _normals, _tangents)| {
            positions.map(|iter| {
                GeometryMorphTarget::new(iter.map(Vec3::from_array).collect::<Vec<_>>())
            })
        })
        .collect::<Vec<_>>();
    let indices: Vec<u32> = reader
        .read_indices()
        .map(|reader| reader.into_u32().collect())
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
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
    let uses_vertex_colors = vertex_colors.iter().any(|color| *color != Color::WHITE);
    let geometry = GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
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
    let geometry = storage.geometries.insert(geometry);
    let material = primitive
        .material()
        .index()
        .and_then(|index| materials.get(index))
        .copied()
        .unwrap_or_else(|| storage.materials.insert(MaterialDesc::default()));
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
    primitive: &Primitive<'_>,
    buffers: &ResolvedGltfBuffers,
    semantic: &Semantic,
) -> Result<Option<Vec<Vec3>>, AssetError> {
    let Some(accessor) = primitive.get(semantic) else {
        return Ok(None);
    };
    if accessor.dimensions() != Dimensions::Vec3 {
        return Ok(None);
    }
    let get_buffer = |buffer: ::gltf::Buffer<'_>| buffers.reader_buffer(buffer.index());
    let values: Vec<Vec3> = match (accessor.data_type(), accessor.normalized()) {
        (DataType::F32, _) => AccessorIter::<[f32; 3]>::new(accessor, get_buffer)
            .map(|iter| iter.map(Vec3::from_array).collect())
            .unwrap_or_default(),
        (DataType::I8, true) => AccessorIter::<[i8; 3]>::new(accessor, get_buffer)
            .map(|iter| iter.map(normalize_i8_vec3).collect())
            .unwrap_or_default(),
        (DataType::U8, true) => AccessorIter::<[u8; 3]>::new(accessor, get_buffer)
            .map(|iter| iter.map(normalize_u8_vec3).collect())
            .unwrap_or_default(),
        (DataType::I16, true) => AccessorIter::<[i16; 3]>::new(accessor, get_buffer)
            .map(|iter| iter.map(normalize_i16_vec3).collect())
            .unwrap_or_default(),
        (DataType::U16, true) => AccessorIter::<[u16; 3]>::new(accessor, get_buffer)
            .map(|iter| iter.map(normalize_u16_vec3).collect())
            .unwrap_or_default(),
        _ => return Ok(None),
    };
    Ok(Some(values))
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
