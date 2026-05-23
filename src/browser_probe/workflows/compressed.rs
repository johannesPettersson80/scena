#[cfg(feature = "meshopt")]
use base64::Engine;
#[cfg(feature = "meshopt")]
use serde_json::json;
use wasm_bindgen::prelude::JsValue;

#[cfg(not(feature = "meshopt"))]
use super::WorkflowScene;
#[cfg(feature = "meshopt")]
use super::{WorkflowScene, add_default_camera};
#[cfg(feature = "meshopt")]
use crate::{Assets, Scene, TextureColorSpace};

#[cfg(feature = "meshopt")]
pub(super) async fn compressed_assets_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let ktx2_probe = match assets
        .load_texture("data:image/ktx2;base64,AAAA", TextureColorSpace::Srgb)
        .await
    {
        Ok(texture) => {
            let has_decoded_pixels = assets
                .texture(texture)
                .is_some_and(|texture| texture.has_decoded_pixels());
            json!({
                "status": "loaded",
                "has_decoded_pixels": has_decoded_pixels,
            })
        }
        Err(error) => json!({
            "status": "fail-closed",
            "error": format!("{error:?}"),
        }),
    };

    let gltf = meshopt_triangle_gltf();
    let encoded = base64::engine::general_purpose::STANDARD.encode(gltf.as_bytes());
    let source = format!("data:model/gltf+json;base64,{encoded}");
    let scene_asset = assets
        .load_scene(source.clone())
        .await
        .map_err(|error| JsValue::from_str(&format!("meshopt glTF load failed: {error:?}")))?;
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).map_err(|error| {
        JsValue::from_str(&format!("meshopt scene instantiate failed: {error:?}"))
    })?;
    let camera = add_default_camera(&mut scene)?;
    if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).map_err(|error| {
            JsValue::from_str(&format!("meshopt scene frame failed: {error:?}"))
        })?;
    }

    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-compressed-asset-runtime",
            "meshopt_required_extension": true,
            "meshopt_decoder": "EXT_meshopt_compression bufferView expansion",
            "ktx2_probe": ktx2_probe,
            "ktx2_release_evidence": false,
            "source_kind": "data-uri-gltf",
            "source_bytes": gltf.len(),
        }),
    })
}

#[cfg(not(feature = "meshopt"))]
pub(super) async fn compressed_assets_scene() -> Result<WorkflowScene, JsValue> {
    Err(JsValue::from_str(
        "compressed-assets browser proof requires the meshopt or production-assets feature",
    ))
}

#[cfg(feature = "meshopt")]
fn meshopt_triangle_gltf() -> String {
    let positions = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];
    let indices = [0_u32, 1, 2];
    let compressed_positions =
        meshopt::encode_vertex_buffer(&positions).expect("positions meshopt-encode");
    let compressed_indices =
        meshopt::encode_index_buffer(&indices, positions.len()).expect("indices meshopt-encode");
    let mut encoded = compressed_positions.clone();
    encoded.extend_from_slice(&compressed_indices);
    let decoded_len = 42;
    let decoded_uri = base64::engine::general_purpose::STANDARD.encode(vec![0_u8; decoded_len]);
    let encoded_uri = base64::engine::general_purpose::STANDARD.encode(encoded);
    let index_offset = compressed_positions.len();
    let index_len = compressed_indices.len();
    let compressed_len = compressed_positions.len() + compressed_indices.len();

    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["EXT_meshopt_compression"],
        "extensionsRequired": ["EXT_meshopt_compression"],
        "materials": [{{
            "pbrMetallicRoughness": {{ "baseColorFactor": [0.2, 0.9, 0.65, 1.0] }},
            "extensions": {{ "KHR_materials_unlit": {{}} }}
        }}],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }},
                "indices": 1,
                "material": 0
            }}]
        }}],
        "nodes": [{{ "name": "BrowserMeshoptTriangle", "mesh": 0 }}],
        "buffers": [
            {{ "byteLength": 42, "uri": "data:application/octet-stream;base64,{decoded_uri}" }},
            {{ "byteLength": {compressed_len}, "uri": "data:application/octet-stream;base64,{encoded_uri}" }}
        ],
        "bufferViews": [
            {{
                "buffer": 0,
                "byteOffset": 0,
                "byteLength": 36,
                "byteStride": 12,
                "extensions": {{
                    "EXT_meshopt_compression": {{
                        "buffer": 1,
                        "byteOffset": 0,
                        "byteLength": {position_len},
                        "byteStride": 12,
                        "count": 3,
                        "mode": "ATTRIBUTES",
                        "filter": "NONE"
                    }}
                }}
            }},
            {{
                "buffer": 0,
                "byteOffset": 36,
                "byteLength": 6,
                "extensions": {{
                    "EXT_meshopt_compression": {{
                        "buffer": 1,
                        "byteOffset": {index_offset},
                        "byteLength": {index_len},
                        "byteStride": 2,
                        "count": 3,
                        "mode": "TRIANGLES",
                        "filter": "NONE"
                    }}
                }}
            }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.5,-0.5,0.0], "max": [0.5,0.5,0.0] }},
            {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
        ]
    }}"#,
        position_len = compressed_positions.len(),
    )
}
