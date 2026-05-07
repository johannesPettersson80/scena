use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::geometry::{GeometryDesc, GeometryMorphTarget, GeometryTopology, GeometryVertex};
use crate::material::{AlphaMode, Color, MaterialDesc, TextureColorSpace, TextureTransform};
use crate::scene::Vec3;

use super::super::{
    AssetPath, AssetStorage, MaterialHandle, TextureCacheKey, TextureDesc, TextureHandle,
};
use super::SceneAssetMesh;
use super::accessor::{
    GltfAccessor, GltfBufferView, optional_usize, parse_error, read_color_accessor,
    read_indices_accessor, read_vec3_accessor, required_usize,
};

pub(super) fn parse_textures(
    path: &AssetPath,
    json: &JsonValue,
    storage: &mut AssetStorage,
) -> Vec<TextureHandle> {
    let images = json
        .get("images")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    json.get("textures")
        .and_then(JsonValue::as_array)
        .map(|textures| {
            textures
                .iter()
                .filter_map(|texture| {
                    let source = optional_usize(texture, "source")?;
                    let uri = images
                        .get(source)
                        .and_then(|image| image.get("uri"))
                        .and_then(JsonValue::as_str)?;
                    Some(insert_texture(storage, resolve_relative_path(path, uri)))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn parse_materials(
    json: &JsonValue,
    storage: &mut AssetStorage,
    textures: &[TextureHandle],
) -> Vec<MaterialHandle> {
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
                        if let Some(texture) = base_color_texture
                            .get("index")
                            .and_then(JsonValue::as_u64)
                            .and_then(|index| textures.get(index as usize))
                        {
                            desc = desc.with_base_color_texture(*texture);
                        }
                        if let Some(transform) = parse_texture_transform(base_color_texture) {
                            desc = desc.with_base_color_texture_transform(transform);
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
                    storage.materials.insert(desc)
                })
                .collect()
        })
        .unwrap_or_default()
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

fn insert_texture(storage: &mut AssetStorage, path: AssetPath) -> TextureHandle {
    let cache_key = TextureCacheKey {
        path,
        color_space: TextureColorSpace::Srgb,
    };
    if let Some(handle) = storage.texture_lookup.get(&cache_key) {
        return *handle;
    }
    let texture = TextureDesc {
        path: cache_key.path.clone(),
        color_space: cache_key.color_space,
    };
    let handle = storage.textures.insert(texture);
    storage.texture_lookup.insert(cache_key, handle);
    handle
}

fn resolve_relative_path(base: &AssetPath, uri: &str) -> AssetPath {
    if uri.starts_with("data:") || uri.starts_with('/') || uri.contains("://") {
        return AssetPath::from(uri);
    }
    let Some((directory, _file)) = base.as_str().rsplit_once('/') else {
        return AssetPath::from(uri);
    };
    AssetPath::from(format!("{directory}/{uri}"))
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
