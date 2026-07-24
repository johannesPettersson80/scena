#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::Path;
#[cfg(feature = "inspection")]
use std::process::Command;

use base64::Engine as _;
use scena::{AssetError, Assets, Scene};
#[cfg(feature = "scene-host")]
use scena::{Renderer, SceneHostCore, SurfaceViewport};
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

#[test]
fn gltf_instantiation_uses_only_the_declared_default_scene_roots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target/c05-gltf-scene-selection");
    fs::create_dir_all(&dir).expect("scene-selection fixture directory creates");
    let path = dir.join("default-scene.gltf");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "asset": {"version": "2.0"},
            "nodes": [
                {"name": "OtherSceneOnly"},
                {"name": "DefaultChild"},
                {"name": "DefaultRoot", "children": [1]}
            ],
            "scenes": [
                {"name": "Other", "nodes": [0]},
                {"name": "Default", "nodes": [2]}
            ],
            "scene": 1
        }))
        .expect("scene-selection fixture serializes"),
    )
    .expect("scene-selection fixture writes");

    let asset = pollster::block_on(Assets::new().load_scene(path.to_string_lossy().into_owned()))
        .expect("multi-scene glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&asset)
        .expect("default scene instantiates");

    assert_eq!(import.roots().len(), 1);
    assert_eq!(
        import.node("DefaultRoot").expect("default root resolves"),
        import.roots()[0],
    );
    assert!(import.node("DefaultChild").is_ok());
    assert!(
        import.node("OtherSceneOnly").is_err(),
        "nodes reachable only from a non-default scene must not instantiate",
    );
}

#[test]
#[cfg(feature = "scene-host")]
fn gltf_default_scene_selection_is_pinned_by_semantic_aov_pixels() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target/c05-gltf-scene-selection-semantic-aov");
    fs::create_dir_all(&dir).expect("scene-selection AOV fixture directory creates");
    let path = dir.join("default-scene-aov.gltf");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "asset": {"version": "2.0"},
            "extensionsUsed": ["KHR_materials_unlit"],
            "buffers": [{
                "byteLength": 42,
                "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA"
            }],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": 36},
                {"buffer": 0, "byteOffset": 36, "byteLength": 6}
            ],
            "accessors": [
                {
                    "bufferView": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "min": [-0.5, -0.5, 0.0],
                    "max": [0.5, 0.5, 0.0]
                },
                {
                    "bufferView": 1,
                    "componentType": 5123,
                    "count": 3,
                    "type": "SCALAR"
                }
            ],
            "materials": [{
                "pbrMetallicRoughness": {"baseColorFactor": [0.2, 0.8, 0.3, 1.0]},
                "doubleSided": true,
                "extensions": {"KHR_materials_unlit": {}}
            }],
            "meshes": [{"primitives": [{
                "attributes": {"POSITION": 0},
                "indices": 1,
                "material": 0
            }]}],
            "nodes": [
                {"name": "OtherSceneTriangle", "mesh": 0, "translation": [-0.8, 0.0, 0.0]},
                {"name": "SelectedSceneTriangle", "mesh": 0, "translation": [0.25, 0.0, 0.0]}
            ],
            "scenes": [
                {"name": "Other", "nodes": [0]},
                {"name": "Default", "nodes": [1]}
            ],
            "scene": 1
        }))
        .expect("scene-selection AOV fixture serializes"),
    )
    .expect("scene-selection AOV fixture writes");

    let assets = Assets::new();
    let asset = pollster::block_on(assets.load_scene(path.to_string_lossy().into_owned()))
        .expect("renderable multi-scene glTF loads");
    let renderer = Renderer::headless(96, 72).expect("headless semantic renderer builds");
    let viewport = SurfaceViewport::new(96.0, 72.0, 1.0).expect("semantic viewport is valid");
    let mut host = SceneHostCore::from_renderer(assets, renderer, viewport)
        .expect("semantic scene host builds");
    let root = host.scene().root();
    let import = host
        .instantiate_scene_asset_under(root, &asset)
        .expect("selected glTF scene instantiates");
    let selected = host
        .node_handle_by_name(import, "SelectedSceneTriangle")
        .expect("selected scene node is registered");
    assert!(
        host.node_handle_by_name(import, "OtherSceneTriangle")
            .is_err(),
        "the node reachable only from the non-selected scene must stay absent"
    );
    host.frame_all().expect("selected scene frames");
    host.prepare().expect("selected scene prepares");
    let aov = host
        .capture_semantic_aovs()
        .expect("selected scene semantic AOV captures");

    assert!(
        aov.id_indices.iter().any(|index| *index != 0),
        "the selected scene must produce attributed pixels"
    );
    assert_eq!(
        aov.legend.len(),
        1,
        "a compact-all-scenes regression would expose an extra semantic identity"
    );
    assert_eq!(aov.legend[0].node_handle, selected);
}

#[test]
fn gltf_scene_selection_supports_explicit_index_and_name_with_provenance() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("target/c05-gltf-explicit-scene-selection");
    fs::create_dir_all(&dir).expect("scene-selection fixture directory creates");
    let path = dir.join("explicit-scenes.gltf");
    fs::write(
        &path,
        serde_json::to_vec_pretty(&json!({
            "asset": {"version": "2.0"},
            "nodes": [
                {"name": "Shared"},
                {"name": "AssemblyA"},
                {"name": "AssemblyB"}
            ],
            "scenes": [
                {"name": "A", "nodes": [0, 1]},
                {"name": "B", "nodes": [0, 2]},
                {"name": "Empty", "nodes": []}
            ],
            "scene": 0
        }))
        .expect("explicit scene fixture serializes"),
    )
    .expect("explicit scene fixture writes");
    let assets = Assets::new();

    let by_index = pollster::block_on(assets.load_scene_with_options(
        path.to_string_lossy().into_owned(),
        scena::AssetLoadOptions::default().with_gltf_scene_index(1),
    ))
    .expect("scene index 1 loads");
    assert_eq!(by_index.selected_gltf_scene().expect("selection").index, 1);
    assert_eq!(
        by_index
            .selected_gltf_scene()
            .expect("selection")
            .name
            .as_deref(),
        Some("B")
    );
    let mut scene = Scene::new();
    let import = scene.instantiate(&by_index).expect("scene B instantiates");
    assert!(import.node("AssemblyB").is_ok());
    assert!(import.node("Shared").is_ok());
    assert!(import.node("AssemblyA").is_err());

    let by_name = pollster::block_on(assets.load_scene_with_options(
        path.to_string_lossy().into_owned(),
        scena::AssetLoadOptions::default().with_gltf_scene_name("Empty"),
    ))
    .expect("empty scene selected by name loads");
    assert_eq!(by_name.selected_gltf_scene().expect("selection").index, 2);
    let mut empty_scene = Scene::new();
    assert!(
        empty_scene
            .instantiate(&by_name)
            .expect("empty scene instantiates")
            .roots()
            .is_empty()
    );

    let error = pollster::block_on(assets.load_scene_with_options(
        path.to_string_lossy().into_owned(),
        scena::AssetLoadOptions::default().with_gltf_scene_name("Missing"),
    ))
    .expect_err("missing scene name fails closed");
    let message = error.to_string();
    assert!(
        message.contains("Missing") && message.contains("A") && message.contains("B"),
        "error must name the request and available scenes: {message}"
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
