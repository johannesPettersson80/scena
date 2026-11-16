//! glTF mesh / primitive parsing. The `gltf` crate supplies the document
//! model; every attribute, index, and morph-target stream is decoded by the
//! checked readers in `accessors` and `attribute_streams`, which validate the
//! accessor layout before any byte is read.

use ::gltf::Document;
use ::gltf::Primitive;
use ::gltf::accessor::Accessor;
use ::gltf::accessor::Item as AccessorItem;
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
use super::typed_accessors::{
    accessor_error, normalize_i8, normalize_i16, normalize_u8, normalize_u16, typed_accessor_iter,
};

mod accessors;
mod attribute_streams;
mod flat_normals;
use accessors::{
    PrimitiveSource, Vec3Encoding, read_tangent_attribute, read_vec3_accessor, read_vec3_attribute,
    reject_skin_sets_above_one,
};
use attribute_streams::{read_colors, read_indices, read_joints, read_tex_coords, read_weights};
mod skin_influences;

pub(super) fn parse_meshes(
    path: &AssetPath,
    document: &Document,
    buffers: &ResolvedGltfBuffers,
    materials: &[MaterialHandle],
    raw_material_variant_material_indices: &super::material_variants::RawMaterialVariantMaterialIndices,
    storage: &mut AssetStorage,
    load_warnings: &mut Vec<AssetLoadWarning>,
) -> Result<Vec<Vec<SceneAssetMesh>>, AssetError> {
    let mesh_quantization_declared = document
        .extensions_used()
        .any(|extension| extension == "KHR_mesh_quantization");
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
                        raw_material_indices: raw_material_variant_material_indices
                            .get(mesh_index)
                            .and_then(|mesh| mesh.get(primitive_index))
                            .map(Vec::as_slice)
                            .unwrap_or_default(),
                        storage,
                        load_warnings,
                        mesh_quantization_declared,
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
    raw_material_indices: &'a [Option<usize>],
    storage: &'a mut AssetStorage,
    load_warnings: &'a mut Vec<AssetLoadWarning>,
    mesh_quantization_declared: bool,
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
        raw_material_indices,
        storage,
        load_warnings,
        mesh_quantization_declared,
    } = inputs;
    let source = PrimitiveSource {
        path,
        buffers,
        mesh_index,
        primitive_index,
        mesh_quantization_declared,
    };
    let positions = read_vec3_attribute(source, primitive, &Semantic::Positions)?.ok_or_else(
        || AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!(
                "glTF mesh {mesh_index} primitive {primitive_index} is missing POSITION attribute"
            ),
        },
    )?;
    let topology = match primitive.mode() {
        Mode::Triangles => GeometryTopology::Triangles,
        Mode::Lines => GeometryTopology::Lines,
        other => {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("unsupported glTF primitive mode {other:?}"),
            });
        }
    };
    let mut normals = read_vec3_attribute(source, primitive, &Semantic::Normals)?;
    if topology == GeometryTopology::Lines && normals.is_none() {
        normals = Some(vec![Vec3::Y; positions.len()]);
    }
    let vertex_colors: Option<Vec<Color>> = read_colors(source, primitive, 0)?;
    let tex_coords0: Option<Vec<[f32; 2]>> = read_tex_coords(source, primitive, 0)?;
    let tangents = read_tangent_attribute(source, primitive)?;
    reject_skin_sets_above_one(source, primitive)?;
    let skin = skin_influences::resolve(
        path,
        skin_influences::SkinSet {
            joints: read_joints(source, primitive, 0)?,
            weights: read_weights(source, primitive, 0)?,
        },
        skin_influences::SkinSet {
            joints: read_joints(source, primitive, 1)?,
            weights: read_weights(source, primitive, 1)?,
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
    let morph_targets = primitive
        .morph_targets()
        .enumerate()
        .map(|(target_index, target)| {
            let position_deltas = match target.positions() {
                Some(accessor) => read_vec3_accessor(
                    source,
                    accessor,
                    &format!("morph target {target_index} POSITION"),
                    Vec3Encoding::Position,
                )?,
                None => vec![Vec3::ZERO; vertex_count],
            };
            let normal_deltas = target
                .normals()
                .map(|accessor| {
                    read_vec3_accessor(
                        source,
                        accessor,
                        &format!("morph target {target_index} NORMAL"),
                        Vec3Encoding::SignedUnit,
                    )
                })
                .transpose()?;
            let tangent_deltas = target
                .tangents()
                .map(|accessor| {
                    read_vec3_accessor(
                        source,
                        accessor,
                        &format!("morph target {target_index} TANGENT"),
                        Vec3Encoding::SignedUnit,
                    )
                })
                .transpose()?;
            Ok(GeometryMorphTarget::new_with_semantics(
                position_deltas,
                normal_deltas,
                tangent_deltas,
            ))
        })
        .collect::<Result<Vec<_>, AssetError>>()?;
    let indices: Vec<u32> = match read_indices(source, primitive)? {
        Some(indices) => indices,
        None => (0..positions.len() as u32).collect(),
    };
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
    let source_material = primitive
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
    let material = if topology == GeometryTopology::Lines {
        insert_line_material(storage, source_material, path)
    } else {
        source_material
    };
    let mut material_variant_bindings =
        super::material_variants::parse_primitive_material_variant_bindings(
            primitive,
            materials,
            raw_material_indices,
            path,
            mesh_index,
            primitive_index,
            load_warnings,
        );
    if topology == GeometryTopology::Lines {
        material_variant_bindings = material_variant_bindings
            .into_iter()
            .map(|binding| {
                let material = insert_line_material(storage, binding.material(), path);
                super::material_variants::MaterialVariantBinding::new(
                    binding.variants().to_vec(),
                    material,
                )
            })
            .collect();
    }
    Ok(SceneAssetMesh {
        geometry,
        material,
        bounds,
        uses_vertex_colors,
        morph_weights: mesh_weights.to_vec(),
        material_variant_bindings,
    })
}

fn insert_line_material(
    storage: &mut AssetStorage,
    source_material: MaterialHandle,
    path: &AssetPath,
) -> MaterialHandle {
    let base_color = storage
        .materials
        .get(source_material)
        .map(|material| material.base_color())
        .unwrap_or(Color::WHITE);
    let handle = storage
        .materials
        .insert(std::sync::Arc::new(MaterialDesc::line(base_color, 1.0)));
    storage.material_sources.insert(
        handle,
        AssetMaterialSource::generated_default(
            path.clone(),
            "glTF line primitive uses Scena line material",
        ),
    );
    handle
}
