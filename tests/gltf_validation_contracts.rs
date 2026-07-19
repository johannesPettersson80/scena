#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;
#[cfg(feature = "inspection")]
use std::process::Command;

use base64::Engine as _;
use scena::{AssetError, Assets};
use serde_json::{Value, json};

#[test]
#[cfg(feature = "inspection")]
fn malformed_gltf_references_and_graphs_fail_in_isolated_cli_processes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target/c01-gltf-validation/isolated-cli");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("malformed glTF fixture directory creates");

    for (name, _, document) in malformed_documents() {
        let path = dir.join(format!("{name}.gltf"));
        fs::write(
            &path,
            serde_json::to_vec_pretty(&document).expect("malformed glTF fixture serializes"),
        )
        .expect("malformed glTF fixture writes");
        let output = Command::new(env!("CARGO_BIN_EXE_scena"))
            .args(["inspect", path.to_str().expect("fixture path is UTF-8")])
            .output()
            .expect("isolated scena inspect process runs");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !output.status.success(),
            "malformed {name} must fail, stdout={}, stderr={stderr}",
            String::from_utf8_lossy(&output.stdout),
        );
        assert!(
            !stderr.contains("panicked at")
                && !stderr.contains("stack overflow")
                && !stderr.contains("fatal runtime error"),
            "malformed {name} must fail structurally without unwind/abort: {stderr}",
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "malformed {name} must emit machine-readable diagnostics: {error}; stdout={}",
                String::from_utf8_lossy(&output.stdout)
            )
        });
        assert_eq!(report["ok"], false, "malformed {name}: {report:#}");
    }
}

#[test]
fn malformed_gltf_references_and_graphs_return_structured_errors_without_unwind() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target/c01-gltf-validation/in-process");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("malformed glTF fixture directory creates");

    for (name, expected_path, document) in malformed_documents() {
        let path = dir.join(format!("{name}.gltf"));
        fs::write(
            &path,
            serde_json::to_vec_pretty(&document).expect("malformed glTF fixture serializes"),
        )
        .expect("malformed glTF fixture writes");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pollster::block_on(Assets::new().load_scene(path.to_string_lossy().into_owned()))
        }));
        let error = result
            .unwrap_or_else(|_| panic!("malformed {name} must not unwind"))
            .expect_err("malformed glTF must return AssetError");
        assert!(
            matches!(error, AssetError::Parse { ref path, ref reason }
                if path.ends_with(&format!("{name}.gltf")) && reason.contains(expected_path)),
            "malformed {name} must report stable path {expected_path}, got {error:?}",
        );
    }
}

#[test]
fn failed_gltf_parse_does_not_mutate_asset_storage() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target/c01-gltf-validation/transaction");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("transaction fixture directory creates");
    let path = dir.join("late-material-error.gltf");
    let mut buffer = Vec::new();
    for component in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        buffer.extend_from_slice(&component.to_le_bytes());
    }
    for index in [0_u16, 1] {
        buffer.extend_from_slice(&index.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(&buffer);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "asset": {"version": "2.0"},
            "buffers": [{
                "uri": format!("data:application/octet-stream;base64,{encoded}"),
                "byteLength": buffer.len()
            }],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 4}
            ],
            "accessors": [
                {
                    "bufferView": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "min": [0.0, 0.0, 0.0],
                    "max": [1.0, 1.0, 0.0]
                },
                {
                    "bufferView": 1,
                    "componentType": 5123,
                    "count": 2,
                    "type": "SCALAR"
                }
            ],
            "materials": [{
                "pbrMetallicRoughness": {"baseColorFactor": [0.4, 0.5, 0.6, 1.0]}
            }],
            "meshes": [{"primitives": [{
                "attributes": {"POSITION": 0},
                "indices": 1,
                "material": 0
            }]}],
            "nodes": [{"mesh": 0}],
            "scenes": [{"nodes": [0]}],
            "scene": 0
        }))
        .expect("transaction fixture serializes"),
    )
    .expect("transaction fixture writes");

    let assets = Assets::new();
    let error = pollster::block_on(assets.load_scene(path.to_string_lossy().into_owned()))
        .expect_err("late material error must reject the glTF");
    assert!(
        matches!(error, AssetError::Parse { .. }),
        "late geometry validation must return a structured asset error: {error:?}"
    );
    let leaked = assets.release_unreferenced();
    assert_eq!(leaked.geometries_evicted, 0, "failed parse leaked geometry");
    assert_eq!(leaked.materials_evicted, 0, "failed parse leaked material");
    assert_eq!(leaked.textures_evicted, 0, "failed parse leaked texture");
    assert_eq!(
        leaked.environments_evicted, 0,
        "failed parse leaked environment"
    );
}

fn malformed_documents() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "child-index",
            "$.nodes[0].children[0]",
            json!({
                "asset": {"version": "2.0"},
                "nodes": [{"children": [99]}],
                "scenes": [{"nodes": [0]}],
                "scene": 0
            }),
        ),
        (
            "mesh-index",
            "$.nodes[0].mesh",
            json!({
                "asset": {"version": "2.0"},
                "nodes": [{"mesh": 99}],
                "scenes": [{"nodes": [0]}],
                "scene": 0
            }),
        ),
        (
            "skin-index",
            "$.nodes[0].skin",
            json!({
                "asset": {"version": "2.0"},
                "nodes": [{"skin": 99}],
                "scenes": [{"nodes": [0]}],
                "scene": 0
            }),
        ),
        (
            "accessor-index",
            "$.meshes[0].primitives[0].attributes.POSITION",
            json!({
                "asset": {"version": "2.0"},
                "meshes": [{"primitives": [{"attributes": {"POSITION": 99}}]}],
                "nodes": [{"mesh": 0}],
                "scenes": [{"nodes": [0]}],
                "scene": 0
            }),
        ),
        (
            "sampler-index",
            "$.textures[0].sampler",
            json!({
                "asset": {"version": "2.0"},
                "images": [{"uri": "data:image/png;base64,AA=="}],
                "textures": [{"source": 0, "sampler": 99}]
            }),
        ),
        (
            "animation-reference",
            "$.animations[0].channels[0].sampler",
            json!({
                "asset": {"version": "2.0"},
                "nodes": [{}],
                "animations": [{
                    "samplers": [],
                    "channels": [{
                        "sampler": 99,
                        "target": {"node": 99, "path": "translation"}
                    }]
                }]
            }),
        ),
        (
            "node-cycle",
            "$.nodes",
            json!({
                "asset": {"version": "2.0"},
                "nodes": [{"children": [1]}, {"children": [0]}],
                "scenes": [{"nodes": [0]}],
                "scene": 0
            }),
        ),
        (
            "node-dag-multiple-parents",
            "$.nodes[1].children[0]",
            json!({
                "asset": {"version": "2.0"},
                "nodes": [{"children": [2]}, {"children": [2]}, {}],
                "scenes": [{"nodes": [0, 1]}],
                "scene": 0
            }),
        ),
    ]
}
