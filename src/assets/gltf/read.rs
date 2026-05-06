use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;
use crate::geometry::{GeometryDesc, GeometryTopology, GeometryVertex};
use crate::material::{AlphaMode, Color, MaterialDesc, TextureColorSpace};
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
                    if let Some(texture) = pbr
                        .get("baseColorTexture")
                        .and_then(|texture| optional_usize(texture, "index"))
                        .and_then(|index| textures.get(index))
                    {
                        desc = desc.with_base_color_texture(*texture);
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
) -> Result<Vec<SceneAssetMesh>, AssetError> {
    json.get("meshes")
        .and_then(JsonValue::as_array)
        .map(|meshes| {
            meshes
                .iter()
                .map(|mesh| {
                    let primitive = mesh
                        .get("primitives")
                        .and_then(JsonValue::as_array)
                        .and_then(|primitives| primitives.first())
                        .ok_or_else(|| parse_error(path, "glTF mesh has no primitives"))?;
                    parse_mesh_primitive(
                        path,
                        primitive,
                        buffers,
                        buffer_views,
                        accessors,
                        materials,
                        storage,
                    )
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn parse_mesh_primitive(
    path: &AssetPath,
    primitive: &JsonValue,
    buffers: &[Vec<u8>],
    buffer_views: &[GltfBufferView],
    accessors: &[GltfAccessor],
    materials: &[MaterialHandle],
    storage: &mut AssetStorage,
) -> Result<SceneAssetMesh, AssetError> {
    let attributes = primitive
        .get("attributes")
        .ok_or_else(|| parse_error(path, "glTF primitive is missing attributes"))?;
    let positions = read_vec3_accessor(
        path,
        required_usize(path, attributes, "POSITION")?,
        buffers,
        buffer_views,
        accessors,
    )?;
    let normals = attributes
        .get("NORMAL")
        .and_then(JsonValue::as_u64)
        .map(|index| read_vec3_accessor(path, index as usize, buffers, buffer_views, accessors))
        .transpose()?
        .unwrap_or_else(|| vec![Vec3::new(0.0, 0.0, 1.0); positions.len()]);
    let vertex_colors = attributes
        .get("COLOR_0")
        .and_then(JsonValue::as_u64)
        .map(|index| read_color_accessor(path, index as usize, buffers, buffer_views, accessors))
        .transpose()?
        .unwrap_or_else(|| vec![Color::WHITE; positions.len()]);
    let indices = primitive
        .get("indices")
        .and_then(JsonValue::as_u64)
        .map(|index| read_indices_accessor(path, index as usize, buffers, buffer_views, accessors))
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
    .map_err(|error| parse_error(path, format!("invalid glTF geometry: {error:?}")))?;
    let geometry = storage.geometries.insert(geometry);
    let material = primitive
        .get("material")
        .and_then(JsonValue::as_u64)
        .and_then(|index| materials.get(index as usize))
        .copied()
        .unwrap_or_else(|| storage.materials.insert(MaterialDesc::default()));
    Ok(SceneAssetMesh {
        geometry,
        material,
        uses_vertex_colors,
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

fn array_f32(values: &[JsonValue], index: usize) -> Option<f32> {
    values
        .get(index)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}
