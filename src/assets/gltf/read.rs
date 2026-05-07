use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::geometry::{
    GeometryDesc, GeometryMorphTarget, GeometrySkin, GeometryTopology, GeometryVertex,
};
use crate::material::{AlphaMode, Color, MaterialDesc, TextureColorSpace, TextureTransform};
use crate::scene::Vec3;

use self::textures::{GltfTexture, texture_slot};
use super::super::{AssetPath, AssetStorage, MaterialHandle};
use super::SceneAssetMesh;
use super::accessor::{
    GltfAccessor, GltfBufferView, parse_error, read_color_accessor, read_indices_accessor,
    read_joints_accessor, read_vec3_accessor, read_weights_accessor, required_usize,
};
pub(super) use textures::parse_textures;

mod textures;

pub(super) fn parse_materials(
    path: &AssetPath,
    json: &JsonValue,
    storage: &mut AssetStorage,
    textures: &[GltfTexture],
) -> Result<Vec<MaterialHandle>, AssetError> {
    json.get("materials")
        .and_then(JsonValue::as_array)
        .map(|materials| {
            materials
                .iter()
                .map(|material| {
                    let pbr = material
                        .get("pbrMetallicRoughness")
                        .unwrap_or(&JsonValue::Null);
                    let base_color = color_factor(pbr, "baseColorFactor", Color::WHITE);
                    let metallic = number_field(pbr, "metallicFactor").unwrap_or(1.0);
                    let roughness = number_field(pbr, "roughnessFactor").unwrap_or(1.0);
                    let unlit = material
                        .get("extensions")
                        .and_then(|extensions| extensions.get("KHR_materials_unlit"))
                        .is_some();
                    let mut desc = if unlit {
                        MaterialDesc::unlit(base_color)
                    } else {
                        MaterialDesc::pbr_metallic_roughness(base_color, metallic, roughness)
                    };
                    if let Some(base_color_texture) = pbr.get("baseColorTexture") {
                        if let Some(texture) = texture_slot(
                            path,
                            "baseColorTexture",
                            base_color_texture,
                            textures,
                            storage,
                            TextureColorSpace::Srgb,
                        )? {
                            desc = desc.with_base_color_texture(texture);
                        }
                        if let Some(transform) = parse_texture_transform(base_color_texture) {
                            desc = desc.with_base_color_texture_transform(transform);
                        }
                    }
                    if let Some(metallic_roughness_texture) = pbr.get("metallicRoughnessTexture") {
                        if let Some(texture) = texture_slot(
                            path,
                            "metallicRoughnessTexture",
                            metallic_roughness_texture,
                            textures,
                            storage,
                            TextureColorSpace::Linear,
                        )? {
                            desc = desc.with_metallic_roughness_texture(texture);
                        }
                        if let Some(transform) = parse_texture_transform(metallic_roughness_texture)
                        {
                            desc = desc.with_metallic_roughness_texture_transform(transform);
                        }
                    }
                    if let Some(normal_texture) = material.get("normalTexture") {
                        if let Some(texture) = texture_slot(
                            path,
                            "normalTexture",
                            normal_texture,
                            textures,
                            storage,
                            TextureColorSpace::Linear,
                        )? {
                            desc = desc.with_normal_texture(texture);
                        }
                        if let Some(transform) = parse_texture_transform(normal_texture) {
                            desc = desc.with_normal_texture_transform(transform);
                        }
                    }
                    if let Some(occlusion_texture) = material.get("occlusionTexture") {
                        if let Some(texture) = texture_slot(
                            path,
                            "occlusionTexture",
                            occlusion_texture,
                            textures,
                            storage,
                            TextureColorSpace::Linear,
                        )? {
                            desc = desc.with_occlusion_texture(texture);
                        }
                        if let Some(transform) = parse_texture_transform(occlusion_texture) {
                            desc = desc.with_occlusion_texture_transform(transform);
                        }
                    }
                    if let Some(emissive_texture) = material.get("emissiveTexture") {
                        if let Some(texture) = texture_slot(
                            path,
                            "emissiveTexture",
                            emissive_texture,
                            textures,
                            storage,
                            TextureColorSpace::Srgb,
                        )? {
                            desc = desc.with_emissive_texture(texture);
                        }
                        if let Some(transform) = parse_texture_transform(emissive_texture) {
                            desc = desc.with_emissive_texture_transform(transform);
                        }
                    }
                    if let Some(emissive) = color3_factor(material, "emissiveFactor") {
                        desc = desc.with_emissive(emissive);
                    }
                    if let Some(strength) = material
                        .get("extensions")
                        .and_then(|extensions| extensions.get("KHR_materials_emissive_strength"))
                        .and_then(|extension| number_field(extension, "emissiveStrength"))
                    {
                        desc = desc.with_emissive_strength(strength);
                    }
                    desc = match material.get("alphaMode").and_then(JsonValue::as_str) {
                        Some("BLEND") => desc.with_alpha_mode(AlphaMode::Blend),
                        Some("MASK") => desc.with_alpha_mode(AlphaMode::Mask {
                            cutoff: number_field(material, "alphaCutoff").unwrap_or(0.5),
                        }),
                        _ => desc,
                    };
                    if material
                        .get("doubleSided")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(false)
                    {
                        desc = desc.with_double_sided(true);
                    }
                    Ok(storage.materials.insert(desc))
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

pub(super) fn parse_meshes(
    path: &AssetPath,
    json: &JsonValue,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
    materials: &[MaterialHandle],
    storage: &mut AssetStorage,
) -> Result<Vec<Vec<SceneAssetMesh>>, AssetError> {
    json.get("meshes")
        .and_then(JsonValue::as_array)
        .map(|meshes| {
            meshes
                .iter()
                .map(|mesh| {
                    let mesh_weights = mesh
                        .get("weights")
                        .and_then(JsonValue::as_array)
                        .map(|weights| {
                            weights
                                .iter()
                                .filter_map(JsonValue::as_f64)
                                .map(|value| value as f32)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let primitives = mesh
                        .get("primitives")
                        .and_then(JsonValue::as_array)
                        .ok_or_else(|| parse_error(path, "glTF mesh has no primitives"))?;
                    primitives
                        .iter()
                        .map(|primitive| {
                            let mut inputs = PrimitiveParseInputs {
                                path,
                                buffers,
                                buffer_views,
                                accessors,
                                materials,
                                mesh_weights: &mesh_weights,
                                storage,
                            };
                            parse_mesh_primitive(primitive, &mut inputs)
                        })
                        .collect()
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

struct PrimitiveParseInputs<'a, 'storage> {
    path: &'a AssetPath,
    buffers: &'a [Vec<u8>],
    buffer_views: &'a [GltfBufferView],
    accessors: &'a [GltfAccessor],
    materials: &'a [MaterialHandle],
    mesh_weights: &'a [f32],
    storage: &'storage mut AssetStorage,
}

fn parse_mesh_primitive(
    primitive: &JsonValue,
    inputs: &mut PrimitiveParseInputs<'_, '_>,
) -> Result<SceneAssetMesh, AssetError> {
    let path = inputs.path;
    let attributes = primitive
        .get("attributes")
        .ok_or_else(|| parse_error(path, "glTF primitive is missing attributes"))?;
    let positions = read_vec3_accessor(
        path,
        required_usize(path, attributes, "POSITION")?,
        inputs.buffers,
        inputs.buffer_views,
        inputs.accessors,
    )?;
    let normals = attributes
        .get("NORMAL")
        .and_then(JsonValue::as_u64)
        .map(|index| {
            read_vec3_accessor(
                path,
                index as usize,
                inputs.buffers,
                inputs.buffer_views,
                inputs.accessors,
            )
        })
        .transpose()?
        .unwrap_or_else(|| vec![Vec3::new(0.0, 0.0, 1.0); positions.len()]);
    let vertex_colors = attributes
        .get("COLOR_0")
        .and_then(JsonValue::as_u64)
        .map(|index| {
            read_color_accessor(
                path,
                index as usize,
                inputs.buffers,
                inputs.buffer_views,
                inputs.accessors,
            )
        })
        .transpose()?
        .unwrap_or_else(|| vec![Color::WHITE; positions.len()]);
    let skin = match (
        attributes.get("JOINTS_0").and_then(JsonValue::as_u64),
        attributes.get("WEIGHTS_0").and_then(JsonValue::as_u64),
    ) {
        (Some(joints), Some(weights)) => Some(GeometrySkin::new(
            read_joints_accessor(
                path,
                joints as usize,
                inputs.buffers,
                inputs.buffer_views,
                inputs.accessors,
            )?,
            read_weights_accessor(
                path,
                weights as usize,
                inputs.buffers,
                inputs.buffer_views,
                inputs.accessors,
            )?,
        )),
        (None, None) => None,
        _ => {
            return Err(parse_error(
                path,
                "JOINTS_0 and WEIGHTS_0 must be provided together for skinned geometry",
            ));
        }
    };
    let morph_targets = primitive
        .get("targets")
        .and_then(JsonValue::as_array)
        .map(|targets| {
            targets
                .iter()
                .map(|target| {
                    let position_accessor = required_usize(path, target, "POSITION")?;
                    read_vec3_accessor(
                        path,
                        position_accessor,
                        inputs.buffers,
                        inputs.buffer_views,
                        inputs.accessors,
                    )
                    .map(GeometryMorphTarget::new)
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))?;
    let indices = primitive
        .get("indices")
        .and_then(JsonValue::as_u64)
        .map(|index| {
            read_indices_accessor(
                path,
                index as usize,
                inputs.buffers,
                inputs.buffer_views,
                inputs.accessors,
            )
        })
        .transpose()?
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    if normals.len() != positions.len() {
        return Err(parse_error(
            path,
            "NORMAL accessor count must match POSITION count",
        ));
    }

    let vertices = positions
        .into_iter()
        .zip(normals)
        .map(|(position, normal)| GeometryVertex { position, normal })
        .collect::<Vec<_>>();
    let uses_vertex_colors = vertex_colors.iter().any(|color| *color != Color::WHITE);
    let geometry = GeometryDesc::try_new_with_vertex_colors(
        GeometryTopology::Triangles,
        vertices,
        indices,
        vertex_colors,
    )
    .and_then(|geometry| geometry.with_morph_targets(morph_targets))
    .and_then(|geometry| match skin {
        Some(skin) => geometry.with_skin(skin),
        None => Ok(geometry),
    })
    .map_err(|error| parse_error(path, format!("invalid glTF geometry: {error:?}")))?;
    let bounds = geometry.bounds();
    let geometry = inputs.storage.geometries.insert(geometry);
    let material = primitive
        .get("material")
        .and_then(JsonValue::as_u64)
        .and_then(|index| inputs.materials.get(index as usize))
        .copied()
        .unwrap_or_else(|| inputs.storage.materials.insert(MaterialDesc::default()));
    Ok(SceneAssetMesh {
        geometry,
        material,
        bounds,
        uses_vertex_colors,
        morph_weights: inputs.mesh_weights.to_vec(),
    })
}

fn number_field(value: &JsonValue, field: &str) -> Option<f32> {
    value
        .get(field)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}

fn parse_texture_transform(texture_info: &JsonValue) -> Option<TextureTransform> {
    let extension = texture_info
        .get("extensions")?
        .get("KHR_texture_transform")?;
    Some(TextureTransform::new(
        vec2_factor(extension, "offset", [0.0, 0.0]),
        number_field(extension, "rotation").unwrap_or(0.0),
        vec2_factor(extension, "scale", [1.0, 1.0]),
        extension
            .get("texCoord")
            .and_then(JsonValue::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
    ))
}

fn color_factor(value: &JsonValue, field: &str, fallback: Color) -> Color {
    let Some(values) = value.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    Color::from_linear_rgba(
        array_f32(values, 0).unwrap_or(fallback.r),
        array_f32(values, 1).unwrap_or(fallback.g),
        array_f32(values, 2).unwrap_or(fallback.b),
        array_f32(values, 3).unwrap_or(fallback.a),
    )
}

fn color3_factor(value: &JsonValue, field: &str) -> Option<Color> {
    let values = value.get(field).and_then(JsonValue::as_array)?;
    Some(Color::from_linear_rgb(
        array_f32(values, 0).unwrap_or(0.0),
        array_f32(values, 1).unwrap_or(0.0),
        array_f32(values, 2).unwrap_or(0.0),
    ))
}

fn vec2_factor(value: &JsonValue, field: &str, fallback: [f32; 2]) -> [f32; 2] {
    let Some(values) = value.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    [
        array_f32(values, 0).unwrap_or(fallback[0]),
        array_f32(values, 1).unwrap_or(fallback[1]),
    ]
}

fn array_f32(values: &[JsonValue], index: usize) -> Option<f32> {
    values
        .get(index)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}
