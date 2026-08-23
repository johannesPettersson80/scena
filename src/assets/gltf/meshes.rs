//! Stage C2: glTF mesh / primitive parsing now uses the `gltf` crate's
//! typed `Primitive::reader()` so all attribute / index / morph-target
//! reading is delegated to the gltf-crate util module.

use ::gltf::Document;
use ::gltf::Primitive;
use ::gltf::accessor::Accessor;
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

mod accessors;
mod flat_normals;
use accessors::{
    Vec3Encoding, joint_indices, read_tangent_attribute, read_vec3_accessor, read_vec3_attribute,
    reject_skin_sets_above_one, validate_skin_weight_accessor,
};
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
    let reader = primitive.reader(|buffer| buffers.reader_buffer(buffer.index()));
    let positions = read_vec3_attribute(
        path,
        primitive,
        buffers,
        &Semantic::Positions,
        mesh_quantization_declared,
    )?
    .ok_or_else(|| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: "glTF primitive is missing POSITION attribute".to_string(),
    })?;
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
    let mut normals = read_vec3_attribute(
        path,
        primitive,
        buffers,
        &Semantic::Normals,
        mesh_quantization_declared,
    )?;
    if topology == GeometryTopology::Lines && normals.is_none() {
        normals = Some(vec![Vec3::Y; positions.len()]);
    }
    let vertex_colors: Option<Vec<Color>> = reader.read_colors(0).map(|colors| {
        colors
            .into_rgba_f32()
            .map(|rgba| Color::from_linear_rgba(rgba[0], rgba[1], rgba[2], rgba[3]))
            .collect()
    });
    let tex_coords0: Option<Vec<[f32; 2]>> = reader
        .read_tex_coords(0)
        .map(|tex| tex.into_f32().collect());
    let tangents = read_tangent_attribute(path, primitive, buffers, mesh_quantization_declared)?;
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
    let morph_targets = primitive
        .morph_targets()
        .enumerate()
        .map(|(target_index, target)| {
            let position_deltas = match target.positions() {
                Some(accessor) => read_vec3_accessor(
                    path,
                    buffers,
                    accessor,
                    &format!("morph target {target_index} POSITION"),
                    Vec3Encoding::Position,
                    mesh_quantization_declared,
                )?,
                None => vec![Vec3::ZERO; vertex_count],
            };
            let normal_deltas = target
                .normals()
                .map(|accessor| {
                    read_vec3_accessor(
                        path,
                        buffers,
                        accessor,
                        &format!("morph target {target_index} NORMAL"),
                        Vec3Encoding::SignedUnit,
                        mesh_quantization_declared,
                    )
                })
                .transpose()?;
            let tangent_deltas = target
                .tangents()
                .map(|accessor| {
                    read_vec3_accessor(
                        path,
                        buffers,
                        accessor,
                        &format!("morph target {target_index} TANGENT"),
                        Vec3Encoding::SignedUnit,
                        mesh_quantization_declared,
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
