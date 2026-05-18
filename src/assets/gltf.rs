//! Stage C2: glTF import now uses the canonical `gltf` crate as its
//! parser. Scena retains the typed public surface — `SceneAsset`,
//! `SceneAssetNode`, `SceneAssetMesh`, `SceneAssetSkin`,
//! `SceneAssetClip`, `SceneAssetLight`, `SceneAssetAnchor`,
//! `SceneAssetConnector`, `MaterialVariantBinding` — but every JSON
//! walk + accessor read is delegated to the gltf crate. Scena-specific
//! extras (anchors, connectors) are still parsed by scena since they
//! live in the freeform `extras` slot.

use std::collections::BTreeMap;
use std::sync::Arc;

use ::gltf::Gltf;
use ::gltf::buffer::Source as BufferSource;

use crate::diagnostics::AssetError;

pub use self::anchors::SceneAssetAnchor;
use self::anchors::parse_node_anchors;
use self::animation::parse_gltf_clips;
pub use self::connectors::SceneAssetConnector;
use self::connectors::parse_node_connectors;
pub use self::extensions::{GltfDecoderPolicy, GltfExtensionDiagnostic, GltfExtensionStatus};
use self::extensions::{collect_extension_diagnostics, is_v1_required_gltf_extension};
use self::external::{external_buffer_paths, external_image_paths, resolve_relative_path};
use self::lights::parse_punctual_lights;
pub use self::material_variants::MaterialVariantBinding;
use self::materials::parse_materials;
use self::meshes::parse_meshes;
use self::scene_asset::SceneAssetData;
pub use self::scene_asset::{
    SceneAsset, SceneAssetClip, SceneAssetLight, SceneAssetMesh, SceneAssetNode,
};
pub use self::skins::SceneAssetSkin;
use self::skins::parse_skins;
use self::textures::parse_textures;
use self::transform::from_gltf_transform;
use super::{AssetPath, AssetStorage};

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn gltf_now_ms() -> f64 {
    js_sys::Date::now()
}

#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
fn log_gltf_step(label: &str, start_ms: f64) -> f64 {
    let now = gltf_now_ms();
    if crate::diagnostics::browser_timing_enabled() {
        web_sys::console::log_1(
            &format!("[scena-demo] glTF {label}: {:.1}ms", now - start_ms).into(),
        );
    }
    now
}

mod anchors;
mod animation;
mod buffers;
mod connectors;
mod extensions;
mod external;
mod lights;
mod material_variants;
mod materials;
mod meshes;
mod meshopt;
mod scene_asset;
mod skins;
mod textures;
mod transform;

impl SceneAsset {
    pub(super) fn from_gltf_bytes(
        path: AssetPath,
        bytes: &[u8],
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        Self::from_gltf_bytes_with_external_resources(
            path,
            bytes,
            &BTreeMap::new(),
            &BTreeMap::new(),
            storage,
        )
    }

    pub(super) fn from_gltf_bytes_with_external_resources(
        path: AssetPath,
        bytes: &[u8],
        external_buffers: &BTreeMap<usize, Vec<u8>>,
        external_images: &BTreeMap<AssetPath, Vec<u8>>,
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let total_start = gltf_now_ms();
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let mut step_start = total_start;

        let gltf = open_gltf_with_massage(&path, bytes)?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("open_gltf_with_massage", step_start);
        }
        let blob = gltf.blob.clone();
        let scene = Self::from_gltf_document(
            &path,
            &gltf,
            blob.as_deref(),
            external_buffers,
            external_images,
            storage,
        )?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            log_gltf_step("from_gltf_document wrapper", step_start);
            log_gltf_step("from_gltf_bytes_with_external_resources total", total_start);
        }
        Ok(scene)
    }

    pub(super) fn external_buffer_paths(
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Vec<(usize, AssetPath)>, AssetError> {
        external_buffer_paths(path, bytes)
    }

    pub(super) fn external_image_paths(
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Vec<AssetPath>, AssetError> {
        external_image_paths(path, bytes)
    }

    fn from_gltf_document(
        path: &AssetPath,
        gltf: &Gltf,
        binary_chunk: Option<&[u8]>,
        external_buffers: &BTreeMap<usize, Vec<u8>>,
        external_images: &BTreeMap<AssetPath, Vec<u8>>,
        storage: &mut AssetStorage,
    ) -> Result<Self, AssetError> {
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let total_start = gltf_now_ms();
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        let mut step_start = total_start;

        validate_gltf_version(path, gltf)?;
        let extensions_used: Vec<String> = gltf
            .document
            .extensions_used()
            .map(str::to_string)
            .collect();
        let extensions_required: Vec<String> = gltf
            .document
            .extensions_required()
            .map(str::to_string)
            .collect();
        for extension in &extensions_required {
            if !is_v1_required_gltf_extension(extension) {
                return Err(AssetError::UnsupportedRequiredExtension {
                    path: path.as_str().to_string(),
                    extension: extension.clone(),
                });
            }
        }
        let extension_diagnostics = collect_extension_diagnostics(&extensions_used);
        let material_variants = material_variants::parse_material_variant_names(&gltf.document);
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("metadata + extensions", step_start);
        }

        let mut buffers = buffers::ResolvedGltfBuffers::new(resolve_buffers(
            path,
            gltf,
            binary_chunk,
            external_buffers,
        )?);
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("resolve_buffers", step_start);
        }
        meshopt::decode_meshopt_buffer_views(path, &gltf.document, &mut buffers)?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("decode_meshopt_buffer_views", step_start);
        }
        let textures = parse_textures(path, &gltf.document, &buffers, external_images, storage);
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("parse_textures", step_start);
        }
        let materials = parse_materials(path, &gltf.document, storage, &textures)?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("parse_materials", step_start);
        }
        let meshes = parse_meshes(path, &gltf.document, &buffers, &materials, storage)?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("parse_meshes", step_start);
        }
        let skins = parse_skins(path, &gltf.document, &buffers)?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("parse_skins", step_start);
        }
        let lights = parse_punctual_lights(&gltf.document);
        let nodes = parse_gltf_nodes(&gltf.document, &meshes, &lights);
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            step_start = log_gltf_step("parse_lights_nodes", step_start);
        }
        let clips = parse_gltf_clips(path, &gltf.document, &buffers)?;
        #[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
        {
            log_gltf_step("parse_clips", step_start);
            log_gltf_step("from_gltf_document total", total_start);
        }
        let node_count = nodes.len();
        let mesh_count = meshes.iter().map(Vec::len).sum();
        Ok(Self {
            inner: Arc::new(SceneAssetData {
                path: path.clone(),
                node_count,
                mesh_count,
                nodes,
                skins,
                clips,
                extensions_used,
                extensions_required,
                extension_diagnostics,
                material_variants,
                retained_source_bytes: None,
            }),
        })
    }
}

fn validate_gltf_version(path: &AssetPath, gltf: &Gltf) -> Result<(), AssetError> {
    if gltf.document.as_json().asset.version == "2.0" {
        Ok(())
    } else {
        Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "expected glTF asset.version \"2.0\"".to_string(),
        })
    }
}

fn resolve_buffers(
    path: &AssetPath,
    gltf: &Gltf,
    binary_chunk: Option<&[u8]>,
    external_buffers: &BTreeMap<usize, Vec<u8>>,
) -> Result<Vec<Vec<u8>>, AssetError> {
    gltf.document
        .buffers()
        .map(|buffer| {
            let byte_length = buffer.length();
            match buffer.source() {
                BufferSource::Bin => {
                    let bin = binary_chunk.ok_or_else(|| AssetError::Parse {
                        path: path.as_str().to_string(),
                        reason: "glTF buffer without uri requires a GLB binary chunk".to_string(),
                    })?;
                    Ok(bin
                        .get(..byte_length)
                        .ok_or_else(|| AssetError::Parse {
                            path: path.as_str().to_string(),
                            reason: "GLB binary chunk is shorter than buffer byteLength".to_string(),
                        })?
                        .to_vec())
                }
                BufferSource::Uri(uri) => {
                    if uri.starts_with("data:") {
                        let (_, encoded) =
                            uri.split_once(";base64,").ok_or_else(|| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason:
                                    "only embedded base64 glTF buffers are supported in this loader slice"
                                        .to_string(),
                            })?;
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD
                            .decode(encoded)
                            .map_err(|error| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason: format!("invalid embedded buffer base64: {error}"),
                            })
                    } else {
                        let _resolved = resolve_relative_path(path, uri);
                        let bytes = external_buffers
                            .get(&buffer.index())
                            .ok_or_else(|| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason: "external glTF buffer was not fetched".to_string(),
                            })?;
                        bytes
                            .get(..byte_length)
                            .map(<[u8]>::to_vec)
                            .ok_or_else(|| AssetError::Parse {
                                path: path.as_str().to_string(),
                                reason: "external glTF buffer is shorter than byteLength"
                                    .to_string(),
                            })
                    }
                }
            }
        })
        .collect()
}

fn parse_gltf_nodes(
    document: &::gltf::Document,
    meshes: &[Vec<SceneAssetMesh>],
    lights: &[SceneAssetLight],
) -> Vec<SceneAssetNode> {
    document
        .nodes()
        .map(|node| SceneAssetNode {
            name: node.name().map(str::to_string),
            children: node.children().map(|child| child.index()).collect(),
            transform: from_gltf_transform(node.transform()),
            meshes: node
                .mesh()
                .and_then(|mesh| meshes.get(mesh.index()))
                .cloned()
                .unwrap_or_default(),
            skin: node.skin().map(|skin| skin.index()),
            light: node
                .light()
                .and_then(|light| lights.get(light.index()).copied()),
            anchors: parse_node_anchors(&node),
            connectors: parse_node_connectors(&node),
        })
        .collect()
}

/// Helper exposed to anchor/connector parsers: convert the `gltf` crate's
/// `Extras = Option<Box<RawValue>>` into a `serde_json::Value` so the
/// existing scena-specific JSON-walking validators can inspect it.
pub(super) fn extras_to_value(extras: &::gltf::json::Extras) -> Option<serde_json::Value> {
    let raw = extras.as_ref()?;
    serde_json::from_str(raw.get()).ok()
}

pub(super) fn has_glb_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(&0x4654_6C67_u32.to_le_bytes())
}

/// Parse glTF bytes (JSON or GLB) into a `Gltf` value, applying scena's
/// lenient JSON pre-massage for JSON inputs. Stage C2: scena keeps the
/// pre-existing tolerance for animations that omit `channels`/`samplers`
/// arrays — the gltf crate's strict serde derive would otherwise reject
/// those fixtures even though the spec considers them malformed.
pub(super) fn open_gltf_with_massage(path: &AssetPath, bytes: &[u8]) -> Result<Gltf, AssetError> {
    let parse = |slice: &[u8]| {
        Gltf::from_slice_without_validation(slice).map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: error.to_string(),
        })
    };
    if has_glb_magic(bytes) {
        return parse(bytes);
    }
    if let Some(massaged) = massage_json_for_gltf_crate(bytes) {
        return parse(&massaged);
    }
    parse(bytes)
}

/// Pre-process JSON-form glTF to add empty `channels: []` and
/// `samplers: []` arrays to any animation entry that omits them. The
/// previous scena parser tolerated those omissions (a clip with just a
/// `name` produced an empty `SceneAssetClip`); the gltf crate's
/// strict serde derive rejects them. Returns `None` when no change is
/// needed.
pub(super) fn massage_json_for_gltf_crate(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let mut changed = false;

    if let Some(animations) = value.get_mut("animations").and_then(|v| v.as_array_mut()) {
        for animation in animations.iter_mut() {
            let Some(object) = animation.as_object_mut() else {
                continue;
            };
            if !object.contains_key("channels") {
                object.insert("channels".to_string(), serde_json::Value::Array(Vec::new()));
                changed = true;
            }
            if !object.contains_key("samplers") {
                object.insert("samplers".to_string(), serde_json::Value::Array(Vec::new()));
                changed = true;
            }
        }
    }

    // `KHR_materials_variants` requires `variants: [...]`. Scena
    // tolerates an empty extension block so a fixture that only
    // declares the extension surfaces as zero variants.
    if let Some(ext) = value
        .get_mut("extensions")
        .and_then(|v| v.get_mut("KHR_materials_variants"))
        .and_then(|v| v.as_object_mut())
        && !ext.contains_key("variants")
    {
        ext.insert("variants".to_string(), serde_json::Value::Array(Vec::new()));
        changed = true;
    }

    if changed {
        serde_json::to_vec(&value).ok()
    } else {
        None
    }
}
