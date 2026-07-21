use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use base64::Engine;
use scena::{
    ASSET_LOAD_REPORT_SCHEMA_V1, AlphaMode, Angle, AssetError, AssetFetcher, AssetLoadControl,
    AssetLoadOptions, AssetLoadProgress, AssetLoadReportV1, AssetLoadWarning, AssetLoadWarningV1,
    AssetPath, Assets, Backend, Color, DiagnosticCode, DirectionalLight, EnvironmentPreset,
    EnvironmentSourceKind, GeometryDesc, GltfDecoderPolicy, GltfExtensionStatus, MaterialDesc,
    MaterialKind, NodeKind, NotPreparedReason, PointLight, RenderError, Renderer, RetainPolicy,
    Scene, SpotLight, TextureColorSpace, TextureFilter, TextureSourceFormat, TextureWrap,
    Transform, Vec3,
};

fn unstable_headless_gpu_release_tests_enabled() -> bool {
    std::env::var_os("SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS").is_some()
}

fn record_fail_closed_headless_gpu_lane(test_name: &str, reason: &str) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/gate-artifacts/gpu-release-gaps");
    std::fs::create_dir_all(&dir).expect("gpu-release-gaps artifact dir");
    let artifact = serde_json::json!({
        "schema": "scena.gpu_release_gap.v1",
        "test_name": test_name,
        "status": "fail-closed",
        "release_evidence": false,
        "reason": reason,
        "run_hint": "Set SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1 on an approved visual lane to run the local headless-GPU assertion.",
    });
    std::fs::write(
        dir.join(format!("{test_name}.json")),
        serde_json::to_vec_pretty(&artifact).expect("gpu gap artifact serializes"),
    )
    .expect("gpu gap artifact writes");
}

fn record_headless_gpu_release_evidence(test_name: &str, artifact: serde_json::Value) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m8-gpu");
    std::fs::create_dir_all(&dir).expect("m8-gpu artifact dir");
    std::fs::write(
        dir.join(format!("{test_name}.json")),
        serde_json::to_vec_pretty(&artifact).expect("headless GPU artifact serializes"),
    )
    .expect("headless GPU artifact writes");
}

fn skip_unstable_headless_gpu_release_lane(test_name: &str, reason: &str) -> bool {
    if unstable_headless_gpu_release_tests_enabled() {
        false
    } else {
        record_fail_closed_headless_gpu_lane(test_name, reason);
        true
    }
}

/// Phase 5.1: scena's glTF parser must propagate `normalTexture.scale`
/// and `occlusionTexture.strength` from the asset to the typed
/// `MaterialDesc`. Default 1.0 when omitted (per glTF spec). The
/// previous parser dropped both — assets that authored a custom scale
/// or strength rendered at always-1.0.
#[test]
fn m8_normal_texture_scale_and_occlusion_strength_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://normal-scale.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "materials": [
                {
                    "pbrMetallicRoughness": {
                        "baseColorTexture": { "index": 0 }
                    },
                    "normalTexture":    { "index": 0, "scale": 2.5 },
                    "occlusionTexture": { "index": 0, "strength": 0.4 }
                },
                {
                    "pbrMetallicRoughness": {
                        "baseColorTexture": { "index": 0 }
                    },
                    "normalTexture":    { "index": 0 },
                    "occlusionTexture": { "index": 0 }
                }
            ],
            "textures": [{ "source": 0 }],
            "images":   [{ "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==" }],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 },
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 1 }
                ]
            }],
            "nodes": [{ "name": "ScaledMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://normal-scale.gltf")).expect("loads");
    let meshes: Vec<_> = scene_asset.nodes()[0].meshes().to_vec();
    assert_eq!(meshes.len(), 2, "two primitives, two materials");

    // Material 0: custom scale=2.5, strength=0.4
    let mat0 = assets.material(meshes[0].material()).expect("mat 0");
    assert_eq!(
        mat0.normal_scale(),
        2.5,
        "normalTexture.scale must propagate from glTF to MaterialDesc \
         (was previously dropped entirely)"
    );
    assert_eq!(
        mat0.occlusion_strength(),
        0.4,
        "occlusionTexture.strength must propagate from glTF to MaterialDesc"
    );

    // Material 1: defaults
    let mat1 = assets.material(meshes[1].material()).expect("mat 1");
    assert_eq!(mat1.normal_scale(), 1.0, "default normal scale = 1.0");
    assert_eq!(
        mat1.occlusion_strength(),
        1.0,
        "default occlusion strength = 1.0"
    );
}

#[test]
fn m8_clearcoat_material_factors_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://clearcoat-factors.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_clearcoat"],
            "materials": [
                {
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [0.8, 0.1, 0.05, 1.0],
                        "metallicFactor": 0.0,
                        "roughnessFactor": 0.72
                    },
                    "extensions": {
                        "KHR_materials_clearcoat": {
                            "clearcoatFactor": 0.85,
                            "clearcoatRoughnessFactor": 0.18
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "ClearcoatMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://clearcoat-factors.gltf")).expect("loads");
    let meshes: Vec<_> = scene_asset.nodes()[0].meshes().to_vec();
    assert_eq!(meshes.len(), 1);
    let material = assets.material(meshes[0].material()).expect("material");

    assert_eq!(
        material.clearcoat_factor(),
        0.85,
        "KHR_materials_clearcoat.clearcoatFactor must propagate into MaterialDesc"
    );
    assert_eq!(
        material.clearcoat_roughness_factor(),
        0.18,
        "KHR_materials_clearcoat.clearcoatRoughnessFactor must propagate into MaterialDesc"
    );
}

#[test]
fn m8_clearcoat_texture_slots_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://clearcoat-textures.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_clearcoat", "KHR_texture_transform"],
            "images": [
                { "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==" }
            ],
            "textures": [{ "source": 0 }],
            "materials": [
                {
                    "extensions": {
                        "KHR_materials_clearcoat": {
                            "clearcoatFactor": 0.8,
                            "clearcoatTexture": {
                                "index": 0,
                                "extensions": {
                                    "KHR_texture_transform": { "offset": [0.1, 0.2] }
                                }
                            },
                            "clearcoatRoughnessFactor": 0.4,
                            "clearcoatRoughnessTexture": {
                                "index": 0,
                                "extensions": {
                                    "KHR_texture_transform": { "scale": [0.25, 0.5] }
                                }
                            },
                            "clearcoatNormalTexture": {
                                "index": 0,
                                "scale": 1.75
                            }
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "ClearcoatTextureMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://clearcoat-textures.gltf")).expect("loads");
    let meshes: Vec<_> = scene_asset.nodes()[0].meshes().to_vec();
    let material = assets.material(meshes[0].material()).expect("material");

    let clearcoat = material.clearcoat_texture().expect("clearcoat texture");
    let roughness = material
        .clearcoat_roughness_texture()
        .expect("clearcoat roughness texture");
    let normal = material
        .clearcoat_normal_texture()
        .expect("clearcoat normal texture");
    assert_eq!(
        assets
            .texture(clearcoat)
            .expect("clearcoat texture descriptor")
            .color_space(),
        TextureColorSpace::Linear
    );
    assert_eq!(
        assets
            .texture(roughness)
            .expect("clearcoat roughness texture descriptor")
            .color_space(),
        TextureColorSpace::Linear
    );
    assert_eq!(
        assets
            .texture(normal)
            .expect("clearcoat normal texture descriptor")
            .color_space(),
        TextureColorSpace::Linear
    );
    assert_eq!(
        material
            .clearcoat_texture_transform()
            .expect("clearcoat transform")
            .offset(),
        [0.1, 0.2]
    );
    assert_eq!(
        material
            .clearcoat_roughness_texture_transform()
            .expect("roughness transform")
            .scale(),
        [0.25, 0.5]
    );
    assert!(material.clearcoat_normal_texture_transform().is_none());
    assert_eq!(material.clearcoat_normal_scale(), 1.75);
}

#[test]
fn m8_sheen_material_factors_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://sheen-factors.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_sheen"],
            "materials": [
                {
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [0.5, 0.5, 0.5, 1.0],
                        "metallicFactor": 0.0,
                        "roughnessFactor": 0.7
                    },
                    "extensions": {
                        "KHR_materials_sheen": {
                            "sheenColorFactor": [0.7, 0.2, 0.1],
                            "sheenRoughnessFactor": 0.42
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "SheenMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://sheen-factors.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    assert_eq!(
        material.sheen_color_factor(),
        Color::from_linear_rgb(0.7, 0.2, 0.1),
        "KHR_materials_sheen.sheenColorFactor must propagate into MaterialDesc",
    );
    assert_eq!(
        material.sheen_roughness_factor(),
        0.42,
        "KHR_materials_sheen.sheenRoughnessFactor must propagate into MaterialDesc",
    );
}

#[test]
fn m8_sheen_texture_slots_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://sheen-textures.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_sheen", "KHR_texture_transform"],
            "images": [
                { "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==" }
            ],
            "textures": [{ "source": 0 }],
            "materials": [
                {
                    "extensions": {
                        "KHR_materials_sheen": {
                            "sheenColorFactor": [0.6, 0.4, 0.2],
                            "sheenColorTexture": {
                                "index": 0,
                                "extensions": {
                                    "KHR_texture_transform": { "offset": [0.3, 0.4] }
                                }
                            },
                            "sheenRoughnessFactor": 0.75,
                            "sheenRoughnessTexture": {
                                "index": 0,
                                "extensions": {
                                    "KHR_texture_transform": { "scale": [0.5, 0.25] }
                                }
                            }
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "SheenTextureMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://sheen-textures.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    let sheen_color = material.sheen_color_texture().expect("sheen color texture");
    let sheen_roughness = material
        .sheen_roughness_texture()
        .expect("sheen roughness texture");
    assert_eq!(
        assets
            .texture(sheen_color)
            .expect("sheen color texture descriptor")
            .color_space(),
        TextureColorSpace::Srgb
    );
    assert_eq!(
        assets
            .texture(sheen_roughness)
            .expect("sheen roughness texture descriptor")
            .color_space(),
        TextureColorSpace::Linear
    );
    assert_eq!(
        material
            .sheen_color_texture_transform()
            .expect("sheen color transform")
            .offset(),
        [0.3, 0.4]
    );
    assert_eq!(
        material
            .sheen_roughness_texture_transform()
            .expect("sheen roughness transform")
            .scale(),
        [0.5, 0.25]
    );
}

#[test]
fn m8_anisotropy_material_factors_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://anisotropy-factors.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_anisotropy"],
            "materials": [
                {
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [0.5, 0.5, 0.5, 1.0],
                        "metallicFactor": 1.0,
                        "roughnessFactor": 0.38
                    },
                    "extensions": {
                        "KHR_materials_anisotropy": {
                            "anisotropyStrength": 0.8,
                            "anisotropyRotation": 1.57
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "AnisotropyMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://anisotropy-factors.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    assert_eq!(
        material.anisotropy_strength_factor(),
        0.8,
        "KHR_materials_anisotropy.anisotropyStrength must propagate into MaterialDesc",
    );
    assert_eq!(
        material.anisotropy_rotation_radians(),
        1.57,
        "KHR_materials_anisotropy.anisotropyRotation must propagate into MaterialDesc",
    );
}

#[test]
fn m8_anisotropy_texture_slot_is_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://anisotropy-texture.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_anisotropy", "KHR_texture_transform"],
            "images": [
                { "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==" }
            ],
            "textures": [{ "source": 0 }],
            "materials": [
                {
                    "extensions": {
                        "KHR_materials_anisotropy": {
                            "anisotropyStrength": 0.9,
                            "anisotropyRotation": 0.25,
                            "anisotropyTexture": {
                                "index": 0,
                                "extensions": {
                                    "KHR_texture_transform": { "offset": [0.2, 0.3] }
                                }
                            }
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "AnisotropyTextureMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://anisotropy-texture.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    let anisotropy = material
        .anisotropy_texture()
        .expect("anisotropy texture is parsed");
    assert_eq!(
        assets
            .texture(anisotropy)
            .expect("anisotropy texture descriptor")
            .color_space(),
        TextureColorSpace::Linear
    );
    assert_eq!(
        material
            .anisotropy_texture_transform()
            .expect("anisotropy transform")
            .offset(),
        [0.2, 0.3]
    );
}

#[test]
fn m8_iridescence_material_factors_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://iridescence-factors.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_iridescence"],
            "materials": [
                {
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [0.7, 0.7, 0.7, 1.0],
                        "metallicFactor": 0.0,
                        "roughnessFactor": 0.35
                    },
                    "extensions": {
                        "KHR_materials_iridescence": {
                            "iridescenceFactor": 0.65,
                            "iridescenceIor": 1.42,
                            "iridescenceThicknessMinimum": 120.0,
                            "iridescenceThicknessMaximum": 520.0
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "IridescenceMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://iridescence-factors.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    assert_eq!(
        material.iridescence_factor(),
        0.65,
        "KHR_materials_iridescence.iridescenceFactor must propagate into MaterialDesc"
    );
    assert_eq!(
        material.iridescence_ior(),
        1.42,
        "KHR_materials_iridescence.iridescenceIor must propagate into MaterialDesc"
    );
    assert_eq!(
        material.iridescence_thickness_minimum_nm(),
        120.0,
        "KHR_materials_iridescence.iridescenceThicknessMinimum must propagate"
    );
    assert_eq!(
        material.iridescence_thickness_maximum_nm(),
        520.0,
        "KHR_materials_iridescence.iridescenceThicknessMaximum must propagate"
    );
}

#[test]
fn m8_iridescence_texture_slots_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://iridescence-textures.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_iridescence", "KHR_texture_transform"],
            "images": [
                { "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==" }
            ],
            "textures": [{ "source": 0 }],
            "materials": [
                {
                    "extensions": {
                        "KHR_materials_iridescence": {
                            "iridescenceFactor": 0.9,
                            "iridescenceTexture": {
                                "index": 0,
                                "extensions": {
                                    "KHR_texture_transform": { "offset": [0.2, 0.3] }
                                }
                            },
                            "iridescenceThicknessMinimum": 100.0,
                            "iridescenceThicknessMaximum": 650.0,
                            "iridescenceThicknessTexture": {
                                "index": 0,
                                "extensions": {
                                    "KHR_texture_transform": { "scale": [0.5, 0.75] }
                                }
                            }
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "IridescenceTextureMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://iridescence-textures.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    let iridescence = material
        .iridescence_texture()
        .expect("iridescence texture is parsed");
    let thickness = material
        .iridescence_thickness_texture()
        .expect("iridescence thickness texture is parsed");
    assert_eq!(
        assets
            .texture(iridescence)
            .expect("iridescence texture descriptor")
            .color_space(),
        TextureColorSpace::Linear
    );
    assert_eq!(
        assets
            .texture(thickness)
            .expect("iridescence thickness texture descriptor")
            .color_space(),
        TextureColorSpace::Linear
    );
    assert_eq!(
        material
            .iridescence_texture_transform()
            .expect("iridescence transform")
            .offset(),
        [0.2, 0.3]
    );
    assert_eq!(
        material
            .iridescence_thickness_texture_transform()
            .expect("iridescence thickness transform")
            .scale(),
        [0.5, 0.75]
    );
}

#[test]
fn m8_dispersion_material_factor_is_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://dispersion-factor.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": ["KHR_materials_dispersion"],
            "materials": [
                {
                    "pbrMetallicRoughness": {
                        "baseColorFactor": [0.72, 0.72, 0.72, 1.0],
                        "metallicFactor": 0.0,
                        "roughnessFactor": 0.28
                    },
                    "extensions": {
                        "KHR_materials_dispersion": {
                            "dispersion": 0.36
                        }
                    }
                }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "DispersionMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://dispersion-factor.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    assert_eq!(
        material.dispersion_factor(),
        0.36,
        "KHR_materials_dispersion.dispersion must propagate into MaterialDesc"
    );
}

#[test]
fn m8_transmission_ior_volume_material_factors_are_parsed_from_gltf() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://transmission-volume.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": [
                "KHR_materials_transmission",
                "KHR_materials_ior",
                "KHR_materials_volume",
                "KHR_texture_transform"
            ],
            "materials": [{
                "pbrMetallicRoughness": {
                    "baseColorFactor": [0.85, 0.92, 1.0, 0.62],
                    "metallicFactor": 0.0,
                    "roughnessFactor": 0.08
                },
                "extensions": {
                    "KHR_materials_transmission": {
                        "transmissionFactor": 0.72,
                        "transmissionTexture": {
                            "index": 0,
                            "extensions": {
                                "KHR_texture_transform": {
                                    "offset": [0.1, 0.2],
                                    "scale": [0.5, 0.75]
                                }
                            }
                        }
                    },
                    "KHR_materials_ior": {
                        "ior": 1.7
                    },
                    "KHR_materials_volume": {
                        "thicknessFactor": 0.45,
                        "thicknessTexture": {
                            "index": 1,
                            "extensions": {
                                "KHR_texture_transform": {
                                    "offset": [0.3, 0.4],
                                    "scale": [0.25, 0.5]
                                }
                            }
                        },
                        "attenuationDistance": 2.5,
                        "attenuationColor": [0.3, 0.55, 0.9]
                    }
                }
            }],
            "textures": [{ "source": 0 }, { "source": 1 }],
            "images": [
                { "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==" },
                { "uri": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==" }
            ],
            "meshes": [{
                "primitives": [
                    { "attributes": { "POSITION": 0 }, "indices": 1, "material": 0 }
                ]
            }],
            "nodes": [{ "name": "TransmissionVolumeMat", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://transmission-volume.gltf")).expect("loads");
    let material = assets
        .material(scene_asset.nodes()[0].meshes()[0].material())
        .expect("material");

    assert_eq!(material.transmission_factor(), 0.72);
    assert_eq!(material.ior(), 1.7);
    assert_eq!(material.thickness_factor(), 0.45);
    assert_eq!(material.attenuation_distance(), 2.5);
    assert_eq!(
        material.attenuation_color(),
        Color::from_linear_rgb(0.3, 0.55, 0.9)
    );

    let transmission = material
        .transmission_texture()
        .expect("transmission texture is parsed");
    let thickness = material
        .thickness_texture()
        .expect("thickness texture is parsed");
    assert!(assets.texture(transmission).is_some());
    assert!(assets.texture(thickness).is_some());
    assert_eq!(
        material
            .transmission_texture_transform()
            .expect("transmission transform")
            .offset(),
        [0.1, 0.2]
    );
    assert_eq!(
        material
            .thickness_texture_transform()
            .expect("thickness transform")
            .scale(),
        [0.25, 0.5]
    );
}

#[test]
fn m8_optional_real_world_gltf_extensions_report_degradation_metadata() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://extensions.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": [
                "KHR_materials_clearcoat",
                "KHR_materials_transmission",
                "KHR_materials_ior",
                "KHR_materials_volume",
                "KHR_materials_variants",
                "KHR_texture_basisu",
                "KHR_draco_mesh_compression",
                "EXT_meshopt_compression"
            ],
            "nodes": [{ "name": "Root" }]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://extensions.gltf")).expect("glTF loads");
    let diagnostics = scene_asset.extension_diagnostics();
    let degraded = diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.extension(), diagnostic.status()))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        degraded.get("KHR_materials_clearcoat"),
        Some(&GltfExtensionStatus::Degraded)
    );
    assert_eq!(
        degraded.get("KHR_materials_transmission"),
        Some(&GltfExtensionStatus::Degraded)
    );
    assert_eq!(
        degraded.get("KHR_materials_ior"),
        Some(&GltfExtensionStatus::Degraded)
    );
    assert_eq!(
        degraded.get("KHR_materials_volume"),
        Some(&GltfExtensionStatus::Degraded)
    );
    assert_eq!(
        degraded.get("KHR_materials_variants"),
        Some(&GltfExtensionStatus::Supported)
    );
    #[cfg(not(feature = "ktx2"))]
    assert_eq!(
        degraded.get("KHR_texture_basisu"),
        Some(&GltfExtensionStatus::Degraded)
    );
    #[cfg(feature = "ktx2")]
    assert_eq!(
        degraded.get("KHR_texture_basisu"),
        Some(&GltfExtensionStatus::Supported)
    );
    #[cfg(not(feature = "meshopt"))]
    assert_eq!(
        degraded.get("EXT_meshopt_compression"),
        Some(&GltfExtensionStatus::Degraded)
    );
    #[cfg(feature = "meshopt")]
    assert_eq!(
        degraded.get("EXT_meshopt_compression"),
        Some(&GltfExtensionStatus::Supported)
    );
    assert_eq!(
        degraded.get("KHR_draco_mesh_compression"),
        Some(&GltfExtensionStatus::Degraded)
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.help().contains("structured degradation")
                || (diagnostic.extension() == "KHR_materials_variants"
                    && diagnostic.status() == GltfExtensionStatus::Supported
                    && diagnostic.decoder_policy() == GltfDecoderPolicy::BuiltIn)
                || (diagnostic.extension() == "KHR_texture_basisu"
                    && diagnostic.status() == GltfExtensionStatus::Supported
                    && diagnostic.decoder_policy()
                        == (GltfDecoderPolicy::FeatureFlag {
                            feature: "ktx2",
                            crate_name: "basisu_c_sys",
                            license: "MIT OR Apache-2.0",
                        }))
                || (diagnostic.extension() == "EXT_meshopt_compression"
                    && diagnostic.status() == GltfExtensionStatus::Supported
                    && diagnostic.decoder_policy()
                        == (GltfDecoderPolicy::FeatureFlag {
                            feature: "meshopt",
                            crate_name: "meshopt",
                            license: "MIT",
                        }))
        }),
        "each optional unsupported extension needs an actionable degradation hint and enabled features need explicit support metadata",
    );
    assert_eq!(
        diagnostics
            .iter()
            .find(|diagnostic| diagnostic.extension() == "KHR_materials_variants")
            .expect("variants diagnostic exists")
            .decoder_policy(),
        GltfDecoderPolicy::BuiltIn
    );
    assert_eq!(
        diagnostics
            .iter()
            .find(|diagnostic| diagnostic.extension() == "KHR_texture_basisu")
            .expect("basisu diagnostic exists")
            .decoder_policy(),
        GltfDecoderPolicy::FeatureFlag {
            feature: "ktx2",
            crate_name: "basisu_c_sys",
            license: "MIT OR Apache-2.0"
        }
    );
    assert_eq!(
        diagnostics
            .iter()
            .find(|diagnostic| diagnostic.extension() == "EXT_meshopt_compression")
            .expect("meshopt diagnostic exists")
            .decoder_policy(),
        GltfDecoderPolicy::FeatureFlag {
            feature: "meshopt",
            crate_name: "meshopt",
            license: "MIT"
        }
    );
    assert_eq!(
        diagnostics
            .iter()
            .find(|diagnostic| diagnostic.extension() == "KHR_draco_mesh_compression")
            .expect("draco diagnostic exists")
            .decoder_policy(),
        GltfDecoderPolicy::External {
            feature: "draco",
            crate_name: "draco",
            license: "Apache-2.0-compatible decoder required"
        }
    );
    let clearcoat = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.extension() == "KHR_materials_clearcoat")
        .expect("clearcoat diagnostic exists");
    assert!(
        clearcoat.suggested_fix().contains("fallback material"),
        "clearcoat degradation should tell users what to export instead: {:?}",
        clearcoat.suggested_fix()
    );
    let transmission = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.extension() == "KHR_materials_transmission")
        .expect("transmission diagnostic exists");
    assert!(
        transmission.help().contains("physical_glass_transmission")
            && transmission.help().contains("attached GPU"),
        "transmission degradation should point to the backend capability proof: {:?}",
        transmission.help()
    );
    assert!(
        !transmission.help().contains("not release-proven"),
        "transmission degradation must not keep stale pre-proof wording: {:?}",
        transmission.help()
    );
    assert!(
        transmission.suggested_fix().contains("capability report")
            && transmission.suggested_fix().contains("fallback material"),
        "transmission fix should name both the capability-check and fallback paths: {:?}",
        transmission.suggested_fix()
    );
    let draco = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.extension() == "KHR_draco_mesh_compression")
        .expect("draco diagnostic exists");
    assert!(
        draco.suggested_fix().contains("EXT_meshopt_compression"),
        "Draco degradation should point users to the maintained compression path: {:?}",
        draco.suggested_fix()
    );
}

#[test]
fn m8_modern_optional_extensions_have_explicit_v1x_defer_metadata() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://modern-optional-extensions.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "extensionsUsed": [
                "KHR_materials_sheen",
                "KHR_materials_specular",
                "KHR_materials_iridescence",
                "KHR_materials_dispersion",
                "EXT_texture_webp"
            ],
            "nodes": [{ "name": "Root" }]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://modern-optional-extensions.gltf"))
            .expect("optional modern extensions load with degradation metadata");

    for (extension, help_fragment) in [
        ("KHR_materials_sheen", "material extension"),
        ("KHR_materials_specular", "material extension"),
        ("KHR_materials_iridescence", "material extension"),
        ("KHR_materials_dispersion", "material extension"),
        ("EXT_texture_webp", "WebP texture extension"),
    ] {
        let diagnostic = scene_asset
            .extension_diagnostics()
            .iter()
            .find(|diagnostic| diagnostic.extension() == extension)
            .unwrap_or_else(|| panic!("{extension} diagnostic exists"));
        assert_eq!(diagnostic.status(), GltfExtensionStatus::Degraded);
        assert_eq!(diagnostic.decoder_policy(), GltfDecoderPolicy::V1xDeferred);
        assert!(
            diagnostic.help().contains(help_fragment),
            "{extension} needs extension-specific deferral help, got {:?}",
            diagnostic.help()
        );
    }

    for extension in [
        "KHR_materials_sheen",
        "KHR_materials_specular",
        "KHR_materials_iridescence",
        "KHR_materials_dispersion",
        "EXT_texture_webp",
    ] {
        let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
            AssetPath::from(format!("memory://required-{extension}.gltf")),
            required_extension_gltf(extension).into_bytes(),
        )]));
        let error =
            pollster::block_on(assets.load_scene(format!("memory://required-{extension}.gltf")))
                .expect_err("required v1.x extension must fail explicitly");
        assert!(matches!(
            error,
            AssetError::UnsupportedRequiredExtension {
                extension: ref rejected,
                ..
            } if rejected == extension
        ));
    }
}

#[test]
fn m8_texture_transform_nonzero_texcoord_fails_closed() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://unsupported-texcoord.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "images": [{ "uri": "base.png" }],
            "textures": [{ "source": 0 }],
            "materials": [{
                "pbrMetallicRoughness": {
                    "baseColorTexture": {
                        "index": 0,
                        "extensions": {
                            "KHR_texture_transform": { "texCoord": 1 }
                        }
                    }
                }
            }],
            "meshes": [{
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "indices": 1,
                    "material": 0
                }]
            }],
            "nodes": [{ "name": "Root", "mesh": 0 }],
            "buffers": [{ "byteLength": 42, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0,  "byteLength": 36 },
                { "buffer": 0, "byteOffset": 36, "byteLength": 6  }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let error = pollster::block_on(assets.load_scene("memory://unsupported-texcoord.gltf"))
        .expect_err("nonzero texture-transform texCoord must fail closed");
    assert!(matches!(
        error,
        AssetError::Parse {
            reason,
            ..
        } if reason.contains("supports only TEXCOORD_0")
    ));
}

#[test]
fn m8_common_gltf_texture_slots_and_material_flags_are_preserved() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://textures.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "images": [
                { "uri": "base.png" },
                { "uri": "normal.png" },
                { "uri": "metallic_roughness.png" },
                { "uri": "occlusion.png" },
                { "uri": "emissive.png" }
            ],
            "textures": [
                { "source": 0, "sampler": 0 },
                { "source": 1, "sampler": 1 },
                { "source": 2 },
                { "source": 3 },
                { "source": 4 }
            ],
            "samplers": [
                { "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 33648 },
                { "magFilter": 9728, "minFilter": 9728, "wrapS": 33071, "wrapT": 33071 }
            ],
            "materials": [{
                "pbrMetallicRoughness": {
                    "baseColorTexture": {
                        "index": 0,
                        "extensions": {
                            "KHR_texture_transform": { "offset": [0.25, 0.5] }
                        }
                    },
                    "metallicRoughnessTexture": { "index": 2 },
                    "metallicFactor": 0.25,
                    "roughnessFactor": 0.75
                },
                "normalTexture": {
                    "index": 1
                },
                "occlusionTexture": { "index": 3 },
                "emissiveTexture": {
                    "index": 4,
                    "extensions": {
                        "KHR_texture_transform": { "scale": [0.5, 0.5] }
                    }
                },
                "emissiveFactor": [0.1, 0.2, 0.3],
                "extensions": {
                    "KHR_materials_emissive_strength": { "emissiveStrength": 2.5 }
                },
                "alphaMode": "MASK",
                "alphaCutoff": 0.3,
                "doubleSided": true
            }],
            "meshes": [{
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "indices": 1,
                    "material": 0
                }]
            }],
            "nodes": [{ "name": "Root", "mesh": 0 }],
            "buffers": [{ "byteLength": 126, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AACAPwAAAAAAAAAAAACAPwAAAAAAAIA/AAAAAAAAgD8AAAAAAAAAAAAAgD8AAIA/AAABAAIA" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
                { "buffer": 0, "byteOffset": 120, "byteLength": 6 }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] },
                { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
            ]
        }"#
        .to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://textures.gltf")).expect("glTF loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let material = assets.material(mesh.material()).expect("material exists");

    assert!(material.base_color_texture().is_some());
    assert!(material.normal_texture().is_some());
    assert!(material.metallic_roughness_texture().is_some());
    assert!(material.occlusion_texture().is_some());
    assert!(material.emissive_texture().is_some());
    assert_eq!(material.alpha_mode(), AlphaMode::Mask { cutoff: 0.3 });
    assert!(material.double_sided());
    assert_eq!(material.emissive(), Color::from_linear_rgb(0.1, 0.2, 0.3));
    assert_eq!(material.emissive_strength(), 2.5);
    assert_eq!(material.metallic_factor(), 0.25);
    assert_eq!(material.roughness_factor(), 0.75);

    let base = assets
        .texture(material.base_color_texture().expect("base texture"))
        .expect("base texture exists");
    let normal = assets
        .texture(material.normal_texture().expect("normal texture"))
        .expect("normal texture exists");
    assert_eq!(base.color_space(), TextureColorSpace::Srgb);
    assert_eq!(normal.color_space(), TextureColorSpace::Linear);
    assert_eq!(base.path().as_str(), "memory://base.png");
    assert_eq!(base.sampler().mag_filter(), Some(TextureFilter::Linear));
    assert_eq!(
        base.sampler().min_filter(),
        Some(TextureFilter::LinearMipmapLinear)
    );
    assert_eq!(base.sampler().wrap_s(), TextureWrap::Repeat);
    assert_eq!(base.sampler().wrap_t(), TextureWrap::MirroredRepeat);
    assert_eq!(normal.sampler().mag_filter(), Some(TextureFilter::Nearest));
    assert_eq!(normal.sampler().wrap_s(), TextureWrap::ClampToEdge);

    assert_eq!(
        material
            .base_color_texture_transform()
            .expect("base transform")
            .offset(),
        [0.25, 0.5]
    );
    assert!(material.normal_texture_transform().is_none());
    assert_eq!(
        material
            .emissive_texture_transform()
            .expect("emissive transform")
            .scale(),
        [0.5, 0.5]
    );
}

#[test]
fn m8_gltf_data_uri_image_texture_uses_bounded_content_identity() {
    let image_uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "images": [{{ "uri": "{image_uri}" }}],
            "textures": [{{ "source": 0, "sampler": 0 }}],
            "samplers": [
                {{ "magFilter": 9729, "minFilter": 9729, "wrapS": 10497, "wrapT": 10497 }}
            ],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{ "index": 0 }}
                }},
                "emissiveTexture": {{ "index": 0 }},
                "emissiveFactor": [1.0, 1.0, 1.0]
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0 }},
                    "indices": 1,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "EmbeddedTexture", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 126, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AACAPwAAAAAAAAAAAACAPwAAAAAAAIA/AAAAAAAAgD8AAAAAAAAAAAAAgD8AAIA/AAABAAIA" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 120, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] }},
                {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://embedded-texture.gltf"),
        gltf.into_bytes(),
    )]));

    let scene_asset = pollster::block_on(assets.load_scene("memory://embedded-texture.gltf"))
        .expect("glTF with data URI image loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let material = assets.material(mesh.material()).expect("material exists");

    assert_eq!(material.base_color_texture(), material.emissive_texture());
    let texture = assets
        .texture(
            material
                .base_color_texture()
                .expect("base color texture handle"),
        )
        .expect("texture descriptor exists");
    assert!(
        texture.path().as_str().starts_with("memory:image-sha256-")
            && texture.path().as_str().ends_with(".png")
            && texture.path().as_str().len() < 128,
        "data URI images must use a bounded content-addressed identity, got {}",
        texture.path().as_str()
    );
    assert_ne!(texture.path().as_str(), image_uri);
    assert_eq!(texture.color_space(), TextureColorSpace::Srgb);
    assert_eq!(texture.source_format(), TextureSourceFormat::Png);
    assert_eq!(texture.sampler().mag_filter(), Some(TextureFilter::Linear));
    assert_eq!(texture.sampler().wrap_s(), TextureWrap::Repeat);
}

#[test]
fn m8_gltf_texcoord0_is_preserved_for_material_texture_sampling_contract() {
    let mut buffer = Vec::new();
    for value in [-0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "images": [{{ "uri": "albedo.png" }}],
            "textures": [{{ "source": 0 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{ "index": 0 }}
                }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "TexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 66, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://texcoord0.gltf"),
        gltf.into_bytes(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://texcoord0.gltf")).expect("glTF loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry exists");

    assert_eq!(
        geometry.tex_coords0(),
        &[[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]]
    );
}

#[test]
fn m8_gltf_tangent_attribute_is_preserved_with_handedness() {
    let mut buffer = Vec::new();
    for value in [-0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [
        1.0_f32, 0.0, 0.0, -1.0, //
        0.0, 1.0, 0.0, 1.0, //
        1.0, 0.0, 0.0, -1.0,
    ] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TANGENT": 1 }},
                    "indices": 2
                }}]
            }}],
            "nodes": [{{ "name": "TangentTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 90, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 48 }},
                {{ "buffer": 0, "byteOffset": 84, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC4" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://tangent.gltf"),
        gltf.into_bytes(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://tangent.gltf")).expect("glTF loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let geometry = assets.geometry(mesh.geometry()).expect("geometry exists");

    assert_eq!(
        geometry
            .tangents()
            .expect("authored tangents are preserved"),
        &[
            [1.0, 0.0, 0.0, -1.0],
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, -1.0],
        ]
    );
}

#[test]
fn m8_data_uri_base_color_texture_affects_cpu_preview_pixels() {
    let red_png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit"],
            "extensionsRequired": ["KHR_materials_unlit"],
            "images": [{{ "uri": "{red_png}" }}],
            "textures": [{{ "source": 0 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{ "index": 0 }}
                }},
                "extensions": {{ "KHR_materials_unlit": {{}} }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 }},
                    "indices": 3,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "TexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 102, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 72, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }},
                {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://red-texture.gltf"),
        gltf.into_bytes(),
    )]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://red-texture.gltf")).expect("glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("textured scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("textured scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 150 && frame[center + 1] < 80 && frame[center + 2] < 80,
        "embedded red base-color texture should visibly affect CPU preview center pixel, got {:?}",
        &frame[center..center + 4]
    );
}

#[test]
fn m8_external_png_base_color_texture_affects_cpu_preview_pixels() {
    let red_png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let red_png = base64::engine::general_purpose::STANDARD
        .decode(red_png_base64)
        .expect("fixture PNG base64 is valid");
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit"],
            "extensionsRequired": ["KHR_materials_unlit"],
            "images": [{{ "uri": "red.png" }}],
            "textures": [{{ "source": 0 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{ "index": 0 }}
                }},
                "extensions": {{ "KHR_materials_unlit": {{}} }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "TexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 66, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://external-texture/scene.gltf"),
            gltf.into_bytes(),
        ),
        (
            AssetPath::from("memory://external-texture/red.png"),
            red_png,
        ),
    ]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://external-texture/scene.gltf"))
        .expect("glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("textured scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("textured scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 150 && frame[center + 1] < 80 && frame[center + 2] < 80,
        "external red base-color texture should visibly affect CPU preview center pixel, got {:?}",
        &frame[center..center + 4]
    );
}

#[test]
fn m8_reload_promotes_cached_texture_descriptor_when_external_png_arrives() {
    let red_png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let red_png = base64::engine::general_purpose::STANDARD
        .decode(red_png_base64)
        .expect("fixture PNG base64 is valid");
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit"],
            "extensionsRequired": ["KHR_materials_unlit"],
            "images": [{{ "uri": "red.png" }}],
            "textures": [{{ "source": 0 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{ "index": 0 }}
                }},
                "extensions": {{ "KHR_materials_unlit": {{}} }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "ReloadTexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 66, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let fetcher = MutableMemoryFetcher::new(vec![(
        AssetPath::from("memory://reload-texture/scene.gltf"),
        gltf.into_bytes(),
    )]);
    let mut assets = Assets::with_fetcher(fetcher.clone());
    assets.set_retain_policy(RetainPolicy::Always);

    let first = pollster::block_on(assets.load_scene("memory://reload-texture/scene.gltf"))
        .expect("scene loads without optional external image bytes");
    let first_material = assets
        .material(first.nodes()[0].mesh().expect("mesh exists").material())
        .expect("material exists");
    let first_texture = first_material
        .base_color_texture()
        .expect("base texture handle exists");
    assert!(
        !assets
            .texture(first_texture)
            .expect("texture descriptor exists")
            .has_decoded_pixels(),
        "first descriptor should be cached without decoded pixels when the external image is missing",
    );

    fetcher.insert(AssetPath::from("memory://reload-texture/red.png"), red_png);
    let reloaded = pollster::block_on(assets.reload_scene(&first))
        .expect("retained reload reparses after image bytes arrive");
    let reloaded_material = assets
        .material(
            reloaded.nodes()[0]
                .mesh()
                .expect("reloaded mesh exists")
                .material(),
        )
        .expect("reloaded material exists");
    let reloaded_texture = reloaded_material
        .base_color_texture()
        .expect("reloaded base texture handle exists");

    assert_eq!(
        first_texture, reloaded_texture,
        "reload should preserve texture cache identity while promoting decoded pixels",
    );
    assert!(
        assets
            .texture(reloaded_texture)
            .expect("reloaded texture descriptor exists")
            .has_decoded_pixels(),
        "reload with available external PNG bytes must update the cached descriptor instead of keeping a silent descriptor-only fallback",
    );

    let mut scene = Scene::new();
    scene
        .instantiate(&reloaded)
        .expect("reloaded textured scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("reloaded textured scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 150 && frame[center + 1] < 80 && frame[center + 2] < 80,
        "reloaded decoded texture should visibly affect CPU preview center pixel, got {:?}",
        &frame[center..center + 4]
    );
}

#[test]
fn m8_missing_external_image_records_load_warning() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://missing-external-image/scene.gltf"),
        textured_triangle_gltf("missing.png").into_bytes(),
    )]));

    let report = pollster::block_on(
        assets.load_scene_with_report("memory://missing-external-image/scene.gltf"),
    )
    .expect("scene still loads with a structured missing-image warning");

    assert!(
        report.warnings().iter().any(|warning| matches!(
            warning,
            AssetLoadWarning::ExternalImageMissing { path, reason }
                if path.as_str() == "memory://missing-external-image/missing.png"
                    && reason.contains("not found")
        )),
        "missing external image must be surfaced in AssetLoadReport warnings instead of being silently skipped: {:?}",
        report.warnings(),
    );
}

#[test]
fn m8_asset_load_report_schema_serializes_warnings_geometry_and_cache_contract() {
    let path = "memory://asset-report/missing-image-scene.gltf";
    let scene_bytes = textured_triangle_gltf("missing.png").into_bytes();
    let expected_sha = sha256_hex(&scene_bytes);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from(path),
        scene_bytes,
    )]));

    let first =
        pollster::block_on(assets.load_scene_with_report(path)).expect("first load reports");
    let schema_json = first.to_schema_json();

    assert_eq!(schema_json["schema"], ASSET_LOAD_REPORT_SCHEMA_V1);
    assert_eq!(schema_json["path"], path);
    assert_eq!(schema_json["cache_hit"], false);
    assert_eq!(
        schema_json["requested_options"],
        serde_json::json!({
            "strict_textures": false,
            "strict_external_resources": false,
            "fetch_byte_limit": null
        })
    );
    assert_eq!(
        schema_json["cache_entry_options"],
        schema_json["requested_options"]
    );
    assert_eq!(schema_json["geometry"]["node_count"], 1);
    assert_eq!(schema_json["geometry"]["mesh_count"], 1);
    assert_eq!(schema_json["geometry"]["primitive_count"], 1);
    assert_eq!(schema_json["provenance"]["source_path"], path);
    assert_eq!(schema_json["provenance"]["source_sha256"], expected_sha);
    assert_eq!(
        schema_json["geometry"]["provenance"],
        schema_json["provenance"]
    );
    assert_eq!(schema_json["warnings"][0]["kind"], "external_image_missing");
    assert_eq!(
        schema_json["warnings"][0]["path"],
        "memory://asset-report/missing.png"
    );
    assert_eq!(
        schema_json["external_resources"],
        serde_json::json!([
            {
                "kind": "image",
                "path": "memory://asset-report/missing.png",
                "index": null,
                "status": "missing",
                "bytes": null,
                "reason": "not found"
            }
        ]),
        "asset reports must record missing external resources in a status table for browser proof"
    );
    assert_eq!(
        schema_json["material_fallbacks"],
        serde_json::json!([
            {
                "kind": "missing_texture_fallback",
                "material_index": 0,
                "material_slot": "baseColorTexture",
                "texture_index": 0,
                "source_path": "memory://asset-report/missing.png",
                "fallback_path": "scena.material.fallback_texture",
                "reason": "texture bytes were unavailable; renderer will bind the generated material fallback texture"
            }
        ]),
        "asset reports must expose fallback provenance when a material texture binds generated renderer fallback pixels"
    );

    let decoded: AssetLoadReportV1 =
        serde_json::from_value(schema_json.clone()).expect("asset load report schema decodes");
    assert_eq!(decoded.schema, ASSET_LOAD_REPORT_SCHEMA_V1);
    assert_eq!(decoded.requested_options, AssetLoadOptions::default());
    assert_eq!(decoded.cache_entry_options, AssetLoadOptions::default());
    assert_eq!(decoded.provenance.source_path().as_str(), path);
    assert_eq!(
        decoded.provenance.source_sha256(),
        Some(expected_sha.as_str())
    );
    assert_eq!(
        decoded.geometry.provenance.source_sha256(),
        Some(expected_sha.as_str())
    );
    assert!(
        matches!(
            decoded.warnings.as_slice(),
            [AssetLoadWarningV1::ExternalImageMissing { path, reason }]
                if path == "memory://asset-report/missing.png" && reason.contains("not found")
        ),
        "decoded warning contract drifted: {:?}",
        decoded.warnings
    );

    let cached =
        pollster::block_on(assets.load_scene_with_report(path)).expect("cached load reports");
    assert!(cached.cache_hit());
    assert!(
        cached.warnings().iter().any(|warning| matches!(
            warning,
            AssetLoadWarning::ExternalImageMissing { path, reason }
                if path.as_str() == "memory://asset-report/missing.png"
                    && reason.contains("not found")
        )),
        "cache-hit reports must preserve load warnings needed for browser proof"
    );
    let cached_schema = cached.to_schema_report();
    assert!(cached_schema.cache_hit);
    assert!(matches!(
        cached_schema.warnings.as_slice(),
        [AssetLoadWarningV1::ExternalImageMissing { path, reason }]
            if path == "memory://asset-report/missing.png" && reason.contains("not found")
    ));
    let cached_json =
        serde_json::to_value(&cached_schema).expect("cached asset load report serializes");
    assert_eq!(
        cached_json["external_resources"], schema_json["external_resources"],
        "cache-hit reports must preserve external-resource status evidence"
    );
}

#[test]
fn m8_asset_load_report_records_external_image_fetch_status() {
    let red_png = png_rgba8(1, 1, &[[255, 0, 0, 255]]);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://asset-report-image-fetch/scene.gltf"),
            textured_triangle_gltf("red.png").into_bytes(),
        ),
        (
            AssetPath::from("memory://asset-report-image-fetch/red.png"),
            red_png.clone(),
        ),
    ]));

    let report = pollster::block_on(
        assets.load_scene_with_report("memory://asset-report-image-fetch/scene.gltf"),
    )
    .expect("external image scene loads");
    let schema_json = report.to_schema_json();

    assert_eq!(
        schema_json["external_resources"],
        serde_json::json!([
            {
                "kind": "image",
                "path": "memory://asset-report-image-fetch/red.png",
                "index": null,
                "status": "fetched",
                "bytes": red_png.len(),
                "reason": null
            }
        ])
    );
    assert!(
        report.progress_events().iter().any(|event| matches!(
            event,
            AssetLoadProgress::ExternalImageFetched { path, bytes }
                if path.as_str() == "memory://asset-report-image-fetch/red.png"
                    && *bytes == red_png.len()
        )),
        "external image fetches must be visible in progress events for browser proof"
    );

    let old_shape = serde_json::json!({
        "schema": ASSET_LOAD_REPORT_SCHEMA_V1,
        "path": "memory://old-report/scene.gltf",
        "cache_hit": false,
        "fetched_bytes": 0,
        "external_buffers": 0,
        "external_images": 0,
        "provenance": {
            "source_path": "memory://old-report/scene.gltf",
            "source_sha256": null,
            "license": null,
            "generator": null,
            "derivatives": []
        },
        "geometry": {
            "schema": "scena.asset_geometry_summary.v1",
            "node_count": 0,
            "mesh_count": 0,
            "primitive_count": 0,
            "bounds": null,
            "provenance": {
                "source_path": "memory://old-report/scene.gltf",
                "source_sha256": null,
                "license": null,
                "generator": null,
                "derivatives": []
            },
            "source_units": [],
            "source_coordinate_systems": []
        },
        "warnings": [],
        "progress_events": []
    });
    let decoded: AssetLoadReportV1 =
        serde_json::from_value(old_shape).expect("old additive report shape still decodes");
    assert_eq!(decoded.schema, ASSET_LOAD_REPORT_SCHEMA_V1);
    assert_eq!(decoded.requested_options, AssetLoadOptions::default());
    assert_eq!(decoded.cache_entry_options, AssetLoadOptions::default());
    assert!(
        serde_json::to_value(decoded)
            .expect("decoded old report reserializes")["external_resources"]
            .as_array()
            .expect("external_resources is an array")
            .is_empty(),
        "new v1 report fields must default empty for old fixtures"
    );
}

#[test]
fn m8_missing_external_buffer_records_typed_load_warning() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://missing-external-buffer/scene.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "nodes": [{ "name": "Root" }],
            "buffers": [{ "byteLength": 0, "uri": "missing.bin" }]
        }"#
        .to_vec(),
    )]));

    let report = pollster::block_on(
        assets.load_scene_with_report("memory://missing-external-buffer/scene.gltf"),
    )
    .expect("unused zero-length missing external buffer loads with typed warning");

    assert!(report.warnings().iter().any(|warning| matches!(
        warning,
        AssetLoadWarning::ExternalBufferMissing { path, index, reason }
            if path.as_str() == "memory://missing-external-buffer/missing.bin"
                && *index == 0
                && reason.contains("not found")
    )));
    assert!(matches!(
        report.to_schema_report().warnings.as_slice(),
        [AssetLoadWarningV1::ExternalBufferMissing { path, index, reason }]
            if path == "memory://missing-external-buffer/missing.bin"
                && *index == 0
                && reason.contains("not found")
    ));
}

#[test]
fn m8_strict_scene_load_promotes_missing_external_buffer_to_error() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://strict-missing-external-buffer/scene.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "nodes": [{ "name": "Root" }],
            "buffers": [{ "byteLength": 0, "uri": "missing.bin" }]
        }"#
        .to_vec(),
    )]));

    let error = pollster::block_on(assets.load_scene_with_report_options(
        "memory://strict-missing-external-buffer/scene.gltf",
        AssetLoadOptions::default().with_strict_external_resources(true),
    ))
    .expect_err("strict external resources must fail when a referenced buffer is missing");

    assert!(matches!(
        error,
        AssetError::NotFound { ref path }
            if path == "memory://strict-missing-external-buffer/missing.bin"
    ));
}

#[test]
fn m8_strict_scene_load_promotes_missing_external_image_to_error() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://strict-missing-external-image/scene.gltf"),
        textured_triangle_gltf("missing.png").into_bytes(),
    )]));

    let error = pollster::block_on(assets.load_scene_with_options(
        "memory://strict-missing-external-image/scene.gltf",
        AssetLoadOptions::default().with_strict_textures(true),
    ))
    .expect_err("strict texture loading must fail when a referenced external image is missing");

    assert!(
        matches!(
            error,
            AssetError::NotFound { ref path }
                if path == "memory://strict-missing-external-image/missing.png"
        ),
        "strict texture loading should preserve the missing external image path in the hard error, got {error:?}",
    );
}

#[test]
fn scene_cache_lenient_then_strict_does_not_bypass_texture_policy() {
    let path = "memory://cache-policy-lenient-strict/scene.gltf";
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from(path),
        textured_triangle_gltf("missing.png").into_bytes(),
    )]));

    let lenient = pollster::block_on(assets.load_scene_with_report(path))
        .expect("lenient load records the missing image");
    assert_eq!(lenient.options(), AssetLoadOptions::default());
    assert_eq!(lenient.cache_entry_options(), AssetLoadOptions::default());
    assert!(
        lenient
            .warnings()
            .iter()
            .any(|warning| matches!(warning, AssetLoadWarning::ExternalImageMissing { .. }))
    );

    let strict = pollster::block_on(assets.load_scene_with_report_options(
        path,
        AssetLoadOptions::default().with_strict_textures(true),
    ));
    assert!(
        matches!(strict, Err(AssetError::NotFound { ref path }) if path == "memory://cache-policy-lenient-strict/missing.png"),
        "strict request must not reuse lenient cached evidence: {strict:?}"
    );
}

#[test]
fn scene_cache_lenient_then_strict_does_not_bypass_external_buffer_policy() {
    let path = "memory://cache-policy-buffer/scene.gltf";
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from(path),
        br#"{
            "asset": { "version": "2.0" },
            "nodes": [{ "name": "Root" }],
            "buffers": [{ "byteLength": 0, "uri": "missing.bin" }]
        }"#
        .to_vec(),
    )]));

    let lenient = pollster::block_on(assets.load_scene_with_report(path))
        .expect("lenient load records the missing empty buffer");
    assert!(
        lenient
            .warnings()
            .iter()
            .any(|warning| matches!(warning, AssetLoadWarning::ExternalBufferMissing { .. }))
    );

    let strict = pollster::block_on(assets.load_scene_with_report_options(
        path,
        AssetLoadOptions::default().with_strict_external_resources(true),
    ));
    assert!(
        matches!(strict, Err(AssetError::NotFound { ref path }) if path == "memory://cache-policy-buffer/missing.bin"),
        "strict buffer policy must not reuse lenient cached evidence: {strict:?}"
    );
}

#[test]
fn scene_cache_strict_then_lenient_keeps_policy_specific_evidence() {
    let path = "memory://cache-policy-strict-lenient/scene.gltf";
    let image = AssetPath::from("memory://cache-policy-strict-lenient/present.png");
    let fetcher = MutableMemoryFetcher::new(vec![
        (
            AssetPath::from(path),
            textured_triangle_gltf("present.png").into_bytes(),
        ),
        (image.clone(), png_rgba8(1, 1, &[[220, 30, 40, 255]])),
    ]);
    let assets = Assets::with_fetcher(fetcher.clone());

    let strict_options = AssetLoadOptions::default().with_strict_textures(true);
    let strict = pollster::block_on(assets.load_scene_with_report_options(path, strict_options))
        .expect("strict load validates the present image");
    assert!(!strict.cache_hit());
    assert_eq!(strict.options(), strict_options);
    assert_eq!(strict.cache_entry_options(), strict_options);
    assert!(
        strict.warnings().is_empty(),
        "strict present-image warnings drifted: {:?}",
        strict.warnings()
    );

    fetcher.remove(&image);
    let lenient = pollster::block_on(assets.load_scene_with_report(path))
        .expect("strict cache evidence satisfies the later lenient request");
    assert!(
        lenient.cache_hit(),
        "validated strict evidence should avoid a duplicate lenient cache entry"
    );
    assert!(lenient.warnings().is_empty());
    assert_eq!(lenient.options(), AssetLoadOptions::default());
    assert_eq!(lenient.cache_entry_options(), strict_options);
    let schema = lenient.to_schema_report();
    assert_eq!(schema.requested_options, AssetLoadOptions::default());
    assert_eq!(schema.cache_entry_options, strict_options);
    let lenient_cached = pollster::block_on(assets.load_scene_with_report(path))
        .expect("second lenient request reuses the same compatible evidence");
    assert!(lenient_cached.cache_hit());
    assert_eq!(lenient_cached.cache_entry_options(), strict_options);
}

#[test]
fn scene_cache_unlimited_then_bounded_does_not_bypass_fetch_limit() {
    let path = "memory://cache-policy-fetch-limit/scene.gltf";
    let bytes = br#"{
        "asset": { "version": "2.0" },
        "nodes": [{ "name": "LargerThanTinyLimit" }]
    }"#
    .to_vec();
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(AssetPath::from(path), bytes)]));

    pollster::block_on(assets.load_scene(path)).expect("unlimited load succeeds");
    let bounded = pollster::block_on(assets.load_scene_with_report_options(
        path,
        AssetLoadOptions::default().with_fetch_byte_limit(16),
    ));
    assert!(
        matches!(bounded, Err(AssetError::PolicyViolation { .. })),
        "bounded request must enforce its limit instead of reusing unlimited cache evidence: {bounded:?}"
    );
}

#[test]
fn m8_scene_asset_provenance_records_source_hash_and_round_trips() {
    let path = "memory://provenance/scene.gltf";
    let scene_bytes = textured_triangle_gltf("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==")
        .replace(
            r#""asset": { "version": "2.0" }"#,
            r#""asset": { "version": "2.0", "generator": "scena-test-generator", "copyright": "CC0-1.0" }"#,
        )
        .into_bytes();
    let expected_sha = sha256_hex(&scene_bytes);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from(path),
        scene_bytes,
    )]));

    let scene_asset = pollster::block_on(assets.load_scene(path)).expect("scene loads");
    let provenance = scene_asset.provenance();

    assert_eq!(provenance.source_path().as_str(), path);
    assert_eq!(provenance.source_sha256(), Some(expected_sha.as_str()));
    assert_eq!(provenance.license(), Some("CC0-1.0"));
    assert_eq!(provenance.generator(), Some("scena-test-generator"));
    assert!(provenance.derivatives().is_empty());

    let json = serde_json::to_value(provenance).expect("provenance serializes");
    assert_eq!(json["source_path"], path);
    assert_eq!(json["source_sha256"], expected_sha);
    assert_eq!(json["license"], "CC0-1.0");
    assert_eq!(json["generator"], "scena-test-generator");
    let decoded: scena::AssetProvenance =
        serde_json::from_value(json).expect("provenance deserializes");
    assert_eq!(decoded, provenance.clone());
}

#[test]
fn m8_texture_provenance_records_direct_and_external_image_source_hashes() {
    let direct_path = "memory://provenance/direct.png";
    let direct_png = png_rgba8(1, 1, &[[255, 0, 0, 255]]);
    let expected_direct_sha = sha256_hex(&direct_png);
    let external_png = png_rgba8(1, 1, &[[0, 0, 255, 255]]);
    let expected_external_sha = sha256_hex(&external_png);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (AssetPath::from(direct_path), direct_png),
        (
            AssetPath::from("memory://provenance-texture/scene.gltf"),
            textured_triangle_gltf("blue.png").into_bytes(),
        ),
        (
            AssetPath::from("memory://provenance-texture/blue.png"),
            external_png,
        ),
    ]));

    let direct = pollster::block_on(assets.load_texture(direct_path, TextureColorSpace::Srgb))
        .expect("direct texture loads");
    let direct_texture = assets.texture(direct).expect("direct texture descriptor");
    assert_eq!(
        direct_texture.provenance().source_path().as_str(),
        direct_path
    );
    assert_eq!(
        direct_texture.provenance().source_sha256(),
        Some(expected_direct_sha.as_str())
    );

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://provenance-texture/scene.gltf"))
            .expect("scene with external image loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh");
    let material = assets.material(mesh.material()).expect("material");
    let external_texture = material
        .base_color_texture()
        .expect("base color texture handle");
    let external_texture = assets
        .texture(external_texture)
        .expect("external texture descriptor");
    assert_eq!(
        external_texture.provenance().source_path().as_str(),
        "memory://provenance-texture/blue.png"
    );
    assert_eq!(
        external_texture.provenance().source_sha256(),
        Some(expected_external_sha.as_str())
    );
}

#[test]
fn m8_environment_provenance_uses_generic_asset_provenance_contract() {
    let assets = Assets::new();
    let environment = assets
        .environment(assets.default_environment())
        .expect("default environment exists");
    let provenance = environment.provenance();

    assert_eq!(
        provenance.source_path().as_str(),
        "tests/assets/environment/neutral-studio.fixture.txt"
    );
    assert_eq!(
        provenance.source_sha256(),
        environment.source_sha256(),
        "legacy environment hash accessor should delegate to generic provenance"
    );
    assert_eq!(provenance.license(), Some("CC0-1.0"));
    assert_eq!(
        provenance.generator(),
        Some(
            "xtask generate-default-env-fixture --input tests/assets/environment/neutral-studio.fixture.txt"
        )
    );
    assert_eq!(
        environment.source_kind(),
        EnvironmentSourceKind::BundledPreviewFixture
    );
    assert_eq!(provenance.derivatives().len(), 2);

    let json = serde_json::to_value(provenance).expect("environment provenance serializes");
    assert_eq!(
        json["derivatives"][0]["path"],
        "tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml"
    );
}

#[test]
fn m8_prepare_reports_material_texture_handles_without_decoded_pixels() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(Vec::new()));
    let texture = pollster::block_on(assets.load_texture(
        "memory://missing-texture-diagnostic/base.png",
        TextureColorSpace::Srgb,
    ))
    .expect("lenient texture load creates descriptor-only texture");
    assert!(
        !assets
            .texture(texture)
            .expect("texture descriptor exists")
            .has_decoded_pixels(),
        "test setup needs a texture handle whose descriptor has no decoded pixels",
    );
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::WHITE).with_base_color_texture(texture));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("descriptor-only texture should stay lenient but diagnosed");

    assert_eq!(
        renderer.stats().material_textures_missing_decoded_pixels,
        1,
        "renderer stats must count material texture handles that fell back because decoded pixels are missing",
    );
    assert!(
        renderer.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == DiagnosticCode::MaterialTextureMissingDecodedPixels
                && diagnostic.message().contains("base_color")
        }),
        "prepare diagnostics must surface the missing decoded material texture instead of silently binding a fallback: {:?}",
        renderer.diagnostics(),
    );

    renderer
        .render(&scene, camera)
        .expect("render remains lenient");
}

#[test]
fn m8_emissive_png_texture_affects_cpu_preview_pixels() {
    let red_png = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "images": [{{ "uri": "{red_png}" }}],
            "textures": [{{ "source": 0 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorFactor": [0.0, 0.0, 0.0, 1.0]
                }},
                "emissiveTexture": {{ "index": 0 }},
                "emissiveFactor": [1.0, 1.0, 1.0]
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "EmissiveTexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 66, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://emissive-texture.gltf"),
        gltf.into_bytes(),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://emissive-texture.gltf"))
        .expect("emissive texture glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("emissive texture scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("emissive texture scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 150 && frame[center + 1] < 80 && frame[center + 2] < 80,
        "emissive red texture should modulate emissive output in the CPU preview path, got {:?}",
        &frame[center..center + 4]
    );
}

#[test]
fn m8_retained_scene_source_bytes_allow_reload_when_fetcher_goes_offline() {
    let scene_bytes = br#"{
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Root", "children": [1] },
            { "name": "Child" }
        ],
        "scenes": [{ "nodes": [0] }],
        "scene": 0
    }"#
    .to_vec();
    let fetcher = MutableMemoryFetcher::new(vec![(
        AssetPath::from("memory://retained-source/scene.gltf"),
        scene_bytes.clone(),
    )]);
    let mut assets = Assets::with_fetcher(fetcher.clone());
    assets.set_retain_policy(RetainPolicy::Always);

    let first = pollster::block_on(assets.load_scene("memory://retained-source/scene.gltf"))
        .expect("initial retained-source scene loads");
    assert_eq!(first.retained_source_bytes_len(), Some(scene_bytes.len()));

    fetcher.remove(&AssetPath::from("memory://retained-source/scene.gltf"));
    let reloaded =
        pollster::block_on(assets.reload_scene(&first)).expect("retained source bytes reload");

    assert_eq!(reloaded.path(), first.path());
    assert_eq!(reloaded.node_count(), first.node_count());
    assert_eq!(
        reloaded.retained_source_bytes_len(),
        Some(scene_bytes.len())
    );
}

#[test]
fn m8_direct_load_texture_decodes_png_for_cpu_preview_pixels() {
    let red_png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let red_png = base64::engine::general_purpose::STANDARD
        .decode(red_png_base64)
        .expect("fixture PNG base64 is valid");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://direct-texture/red.png"),
        red_png,
    )]));
    let texture = pollster::block_on(
        assets.load_texture("memory://direct-texture/red.png", TextureColorSpace::Srgb),
    )
    .expect("direct texture load succeeds");
    assert!(
        assets
            .texture(texture)
            .expect("texture descriptor exists")
            .has_decoded_pixels(),
        "direct load_texture should decode PNG bytes supplied by the asset fetcher",
    );
    let geometry = assets.create_geometry(
        GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
            scena::GeometryTopology::Triangles,
            vec![
                scena::GeometryVertex {
                    position: Vec3::new(-0.6, -0.6, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                scena::GeometryVertex {
                    position: Vec3::new(0.6, -0.6, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                scena::GeometryVertex {
                    position: Vec3::new(0.0, 0.6, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
            ],
            vec![0, 1, 2],
            vec![Color::WHITE; 3],
            vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
        )
        .expect("textured triangle geometry is valid"),
    );
    let material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE)
            .with_base_color_texture(texture)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("direct textured scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 150 && frame[center + 1] < 80 && frame[center + 2] < 80,
        "directly loaded red base-color texture should affect CPU preview pixels, got {:?}",
        &frame[center..center + 4]
    );
}

#[test]
fn m8_headless_gpu_samples_multiple_base_color_material_slots_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_samples_multiple_base_color_material_slots_when_available",
        "local headless GPU material-texture readback can intermittently return black frames under sustained adapter load",
    ) {
        return;
    }

    let red_png = png_rgba8(1, 1, &[[255, 0, 0, 255]]);
    let blue_png = png_rgba8(1, 1, &[[0, 0, 255, 255]]);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (AssetPath::from("memory://gpu-slots/red.png"), red_png),
        (AssetPath::from("memory://gpu-slots/blue.png"), blue_png),
    ]));
    let red_texture = pollster::block_on(
        assets.load_texture("memory://gpu-slots/red.png", TextureColorSpace::Srgb),
    )
    .expect("red texture loads");
    let blue_texture = pollster::block_on(
        assets.load_texture("memory://gpu-slots/blue.png", TextureColorSpace::Srgb),
    )
    .expect("blue texture loads");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.05));
    let red_material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE)
            .with_base_color_texture(red_texture)
            .with_double_sided(true),
    );
    let blue_material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE)
            .with_base_color_texture(blue_texture)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, red_material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .expect("red mesh inserts");
    scene
        .mesh(geometry, blue_material)
        .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .add()
        .expect("blue mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(96, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU textured scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let frame = renderer.frame_rgba8();
    let left = sample_rgb(frame, 96, 64, 36, 32);
    let right = sample_rgb(frame, 96, 64, 60, 32);
    assert!(
        left[0] > left[2] + 40,
        "left material slot should sample the red texture on GPU, got {left:?}"
    );
    assert!(
        right[2] > right[0] + 40,
        "right material slot should sample the blue texture on GPU, got {right:?}"
    );
}

#[test]
fn m8_headless_gpu_applies_base_color_texture_transform_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_applies_base_color_texture_transform_when_available",
        "local headless GPU texture-transform readback can intermittently return black frames under sustained adapter load",
    ) {
        return;
    }

    let strip_png = png_rgba8(2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.25_f32, 0.5, 0.25, 0.5, 0.25, 0.5] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit", "KHR_texture_transform"],
            "extensionsRequired": ["KHR_materials_unlit", "KHR_texture_transform"],
            "images": [{{ "uri": "strip.png" }}],
            "textures": [{{ "source": 0, "sampler": 0 }}],
            "samplers": [{{ "magFilter": 9728, "minFilter": 9728, "wrapS": 33071, "wrapT": 33071 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{
                        "index": 0,
                        "extensions": {{ "KHR_texture_transform": {{ "offset": [0.5, 0.0] }} }}
                    }}
                }},
                "extensions": {{ "KHR_materials_unlit": {{}} }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "TransformedTexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 66, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://gpu-transform/scene.gltf"),
            gltf.into_bytes(),
        ),
        (
            AssetPath::from("memory://gpu-transform/strip.png"),
            strip_png,
        ),
    ]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://gpu-transform/scene.gltf"))
        .expect("texture transform glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("texture transform scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(64, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU texture transform scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let center = sample_rgb(renderer.frame_rgba8(), 64, 64, 32, 32);
    assert!(
        center[2] > center[0] + 40,
        "GPU material uniform should apply KHR_texture_transform and sample the blue texel, got {center:?}"
    );
}

#[test]
fn m8_headless_gpu_samples_occlusion_and_emissive_material_slots_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_samples_occlusion_and_emissive_material_slots_when_available",
        "local GPU occlusion/emissive material-role readback is not trusted as release evidence until approved backend screenshots exist",
    ) {
        return;
    }

    let occlusion_black = png_rgba8(1, 1, &[[0, 0, 0, 255]]);
    let emissive_red = png_rgba8(1, 1, &[[255, 0, 0, 255]]);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://gpu-material-roles/occlusion.png"),
            occlusion_black,
        ),
        (
            AssetPath::from("memory://gpu-material-roles/emissive.png"),
            emissive_red,
        ),
    ]));
    let occlusion = pollster::block_on(assets.load_texture(
        "memory://gpu-material-roles/occlusion.png",
        TextureColorSpace::Linear,
    ))
    .expect("occlusion texture loads");
    let emissive = pollster::block_on(assets.load_texture(
        "memory://gpu-material-roles/emissive.png",
        TextureColorSpace::Srgb,
    ))
    .expect("emissive texture loads");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.05));
    let occluded_material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE)
            .with_occlusion_texture(occlusion)
            .with_double_sided(true),
    );
    let emissive_material = assets.create_material(
        MaterialDesc::unlit(Color::BLACK)
            .with_emissive(Color::from_linear_rgb(1.0, 0.0, 0.0))
            .with_emissive_strength(4.0)
            .with_emissive_texture(emissive)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, occluded_material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .expect("occluded mesh inserts");
    scene
        .mesh(geometry, emissive_material)
        .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .add()
        .expect("emissive mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(96, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU non-base texture role scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let frame = renderer.frame_rgba8();
    let occluded = sample_rgb(frame, 96, 64, 36, 32);
    let emissive = sample_rgb(frame, 96, 64, 60, 32);
    assert!(
        occluded[0] < 20 && occluded[1] < 20 && occluded[2] < 20,
        "GPU shader should darken the left material through the occlusion texture, got {occluded:?}"
    );
    assert!(
        emissive[0] > emissive[1] + 40 && emissive[0] > emissive[2] + 40,
        "GPU shader should add the right material's emissive texture contribution, got {emissive:?}"
    );
}

#[test]
fn m8_headless_gpu_directional_light_uniform_tints_pbr_output_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_directional_light_uniform_tints_pbr_output_when_available",
        "local headless GPU PBR light readback can intermittently return black frames under sustained adapter load",
    ) {
        return;
    }

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8).with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("PBR mesh inserts");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))
                .with_illuminance_lux(20_000.0),
        )
        .add()
        .expect("directional light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(64, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU lit PBR scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let center = sample_rgb(renderer.frame_rgba8(), 64, 64, 32, 32);
    assert!(
        center[0] > center[1] + 30 && center[0] > center[2] + 30,
        "prepared GPU directional light uniform should tint PBR output red, got {center:?}"
    );
}

#[test]
fn m8_headless_gpu_point_light_uniform_tints_pbr_output_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_point_light_uniform_tints_pbr_output_when_available",
        "local headless GPU PBR light readback can intermittently return black frames under sustained adapter load",
    ) {
        return;
    }

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8).with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("PBR mesh inserts");
    scene
        .point_light(
            PointLight::default()
                .with_color(Color::from_linear_rgb(0.0, 1.0, 0.0))
                .with_intensity_candela(800.0)
                .with_range(5.0),
        )
        .transform(Transform::at(Vec3::new(0.0, 0.0, 1.0)))
        .add()
        .expect("point light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(64, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU point-lit PBR scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let center = sample_rgb(renderer.frame_rgba8(), 64, 64, 32, 32);
    assert!(
        center[1] > center[0] + 30 && center[1] > center[2] + 30,
        "prepared GPU point light uniform should tint PBR output green, got {center:?}"
    );
}

#[test]
fn m8_headless_gpu_spot_light_uniform_tints_pbr_output_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_spot_light_uniform_tints_pbr_output_when_available",
        "local headless GPU PBR light readback can intermittently return black frames under sustained adapter load",
    ) {
        return;
    }

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8).with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("PBR mesh inserts");
    scene
        .spot_light(
            SpotLight::default()
                .with_color(Color::from_linear_rgb(0.0, 0.0, 1.0))
                .with_intensity_candela(900.0)
                .with_range(5.0)
                .with_inner_cone_angle(Angle::from_degrees(20.0))
                .with_outer_cone_angle(Angle::from_degrees(35.0)),
        )
        .transform(Transform::at(Vec3::new(0.0, 0.0, 1.0)))
        .add()
        .expect("spot light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(64, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU spot-lit PBR scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let center = sample_rgb(renderer.frame_rgba8(), 64, 64, 32, 32);
    assert!(
        center[2] > center[0] + 30 && center[2] > center[1] + 30,
        "prepared GPU spot light uniform should tint PBR output blue, got {center:?}"
    );
}

#[test]
fn m8_headless_gpu_tangent_space_normal_map_changes_pbr_lighting_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_tangent_space_normal_map_changes_pbr_lighting_when_available",
        "local headless GPU normal-map readback can intermittently return black frames under sustained adapter load",
    ) {
        return;
    }

    let flat_normal = png_rgba8(1, 1, &[[128, 128, 255, 255]]);
    let inverted_normal = png_rgba8(1, 1, &[[128, 128, 0, 255]]);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://gpu-normal-map/flat.png"),
            flat_normal,
        ),
        (
            AssetPath::from("memory://gpu-normal-map/inverted.png"),
            inverted_normal,
        ),
    ]));
    let flat = pollster::block_on(assets.load_texture(
        "memory://gpu-normal-map/flat.png",
        TextureColorSpace::Linear,
    ))
    .expect("flat normal texture loads");
    let inverted = pollster::block_on(assets.load_texture(
        "memory://gpu-normal-map/inverted.png",
        TextureColorSpace::Linear,
    ))
    .expect("inverted normal texture loads");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.05));
    let lit_material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8)
            .with_normal_texture(flat)
            .with_double_sided(true),
    );
    let inverted_material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.8)
            .with_normal_texture(inverted)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, lit_material)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .expect("lit normal-map mesh inserts");
    scene
        .mesh(geometry, inverted_material)
        .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .add()
        .expect("inverted normal-map mesh inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(20_000.0))
        .add()
        .expect("directional light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(96, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU normal-map PBR scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let frame = renderer.frame_rgba8();
    let flat = sample_rgb(frame, 96, 64, 36, 32);
    let inverted = sample_rgb(frame, 96, 64, 60, 32);
    assert!(
        flat[0] > inverted[0] + 30 && flat[1] > inverted[1] + 30 && flat[2] > inverted[2] + 30,
        "tangent-space normal map should turn the inverted-normal material away from the light; flat={flat:?} inverted={inverted:?}"
    );
}

#[test]
fn m8_headless_gpu_environment_uniform_tints_pbr_output_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_environment_uniform_tints_pbr_output_when_available",
        "local headless GPU environment-light readback can intermittently return black frames under sustained adapter load",
    ) {
        return;
    }

    let environment_path = AssetPath::from("memory://gpu-studio-blue_2x1.hdr");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        environment_path.clone(),
        tiny_radiance_hdr_rgbe(2, 1, &[[16, 32, 255, 132], [16, 32, 255, 132]]),
    )]));
    let environment = pollster::block_on(assets.load_environment(environment_path.as_str()))
        .expect("HDR environment loads");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.04, 0.04, 0.04), 0.0, 0.7)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("environment-lit PBR mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(64, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };
    renderer.set_environment(environment);

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU environment-lit PBR scene prepares");
    renderer.render(&scene, camera).expect("GPU scene renders");

    let center = sample_rgb(renderer.frame_rgba8(), 64, 64, 32, 32);
    assert!(
        center[2] > center[0] + 20 && center[2] > center[1] + 10,
        "prepared GPU environment uniform should tint PBR output blue, got {center:?}"
    );
}

#[test]
fn m8_headless_gpu_transmission_volume_ibl_capability_when_available() {
    const TEST_NAME: &str = "m8_headless_gpu_transmission_volume_ibl_capability_when_available";
    if skip_unstable_headless_gpu_release_lane(
        TEST_NAME,
        "approved headless GPU transmission/volume plus IBL evidence is still required; current default lane records this as a fail-closed proof gap",
    ) {
        return;
    }

    let environment_path = AssetPath::from("memory://gpu-transmission-ibl-blue_2x1.hdr");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        environment_path.clone(),
        tiny_radiance_hdr_rgbe(2, 1, &[[16, 32, 255, 132], [16, 32, 255, 132]]),
    )]));
    let environment = pollster::block_on(assets.load_environment(environment_path.as_str()))
        .expect("HDR environment loads");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.05));
    let backdrop_geometry = assets.create_geometry(GeometryDesc::box_xyz(2.6, 1.6, 0.02));
    let backdrop = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let render_glass = |attenuation_color: Color| -> ([u8; 3], Backend, String, String) {
        let material = assets.create_material(
            MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 205, 230), 0.0, 0.08)
                .with_transmission_factor(1.0)
                .with_ior(1.7)
                .with_thickness_factor(2.0)
                .with_attenuation_distance(1.0)
                .with_attenuation_color(attenuation_color)
                .with_double_sided(true),
        );
        let mut scene = Scene::new();
        scene
            .mesh(backdrop_geometry, backdrop)
            .transform(Transform::at(Vec3::new(0.0, 0.0, -0.28)))
            .add()
            .expect("opaque backdrop mesh inserts");
        scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::ZERO))
            .add()
            .expect("glass mesh inserts");
        let camera = scene.add_default_camera().expect("camera inserts");
        let mut renderer = match Renderer::headless_gpu(96, 64) {
            Ok(renderer) => renderer,
            Err(error) => {
                let reason = format!("Renderer::headless_gpu unavailable on this host: {error:?}");
                record_fail_closed_headless_gpu_lane(TEST_NAME, &reason);
                panic!(
                    "{TEST_NAME} cannot produce approved release evidence with \
                     SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS set: {reason}"
                );
            }
        };
        renderer.set_environment(environment);
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("GPU transmission+IBL scene prepares");
        renderer
            .render(&scene, camera)
            .expect("GPU transmission+IBL scene renders");
        let capabilities = renderer.capabilities();
        (
            sample_rgb(renderer.frame_rgba8(), 96, 64, 48, 32),
            capabilities.backend,
            format!("{:?}", capabilities.forward_pbr),
            format!("{:?}", capabilities.readback_headless_screenshots),
        )
    };

    let (red_glass, backend, forward_pbr, readback_headless_screenshots) =
        render_glass(Color::from_linear_rgb(1.0, 0.08, 0.08));
    let (blue_glass, _, _, _) = render_glass(Color::from_linear_rgb(0.08, 0.35, 1.0));
    let red_r = i16::from(red_glass[0]);
    let red_b = i16::from(red_glass[2]);
    let blue_r = i16::from(blue_glass[0]);
    let blue_b = i16::from(blue_glass[2]);
    assert!(
        blue_b > red_b + 10 && red_r > blue_r + 10 && blue_b > blue_r + 10,
        "headless GPU scalar transmission/volume under IBL should tint transmitted glass by attenuation color; red={red_glass:?} blue={blue_glass:?}",
    );
    record_headless_gpu_release_evidence(
        TEST_NAME,
        serde_json::json!({
            "schema": "scena.m8.headless_gpu_material_ibl.v1",
            "test_name": TEST_NAME,
            "status": "passed",
            "release_evidence": true,
            "backend": format!("{backend:?}"),
            "capabilities": {
                "forward_pbr": forward_pbr,
                "readback_headless_screenshots": readback_headless_screenshots,
            },
            "red_volume_rgb": red_glass,
            "transmission_volume_ibl_rgb": blue_glass,
        }),
    );
}

#[test]
fn m8_headless_gpu_clearcoat_texture_lobe_brightens_pbr_output_when_available() {
    if skip_unstable_headless_gpu_release_lane(
        "m8_headless_gpu_clearcoat_texture_lobe_brightens_pbr_output_when_available",
        "local headless GPU clearcoat readback is not trusted as release evidence until approved backend screenshots exist",
    ) {
        return;
    }

    let clearcoat_off = png_rgba8(1, 1, &[[0, 255, 128, 255]]);
    let clearcoat_on = png_rgba8(1, 1, &[[255, 255, 128, 255]]);
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://gpu-clearcoat/off.png"),
            clearcoat_off,
        ),
        (
            AssetPath::from("memory://gpu-clearcoat/on.png"),
            clearcoat_on,
        ),
    ]));
    let off = pollster::block_on(
        assets.load_texture("memory://gpu-clearcoat/off.png", TextureColorSpace::Linear),
    )
    .expect("clearcoat off texture loads");
    let on = pollster::block_on(
        assets.load_texture("memory://gpu-clearcoat/on.png", TextureColorSpace::Linear),
    )
    .expect("clearcoat on texture loads");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.05));
    let matte = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62)
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(0.12)
            .with_clearcoat_texture(off)
            .with_double_sided(true),
    );
    let coated = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62)
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(0.12)
            .with_clearcoat_texture(on)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, matte)
        .transform(Transform::at(Vec3::new(-0.4, 0.0, 0.0)))
        .add()
        .expect("clearcoat-off mesh inserts");
    scene
        .mesh(geometry, coated)
        .transform(Transform::at(Vec3::new(0.4, 0.0, 0.0)))
        .add()
        .expect("clearcoat-on mesh inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(24_000.0))
        .add()
        .expect("directional light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = match Renderer::headless_gpu(96, 64) {
        Ok(renderer) => renderer,
        Err(_) => return,
    };

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GPU clearcoat scene prepares");
    renderer
        .render(&scene, camera)
        .expect("GPU clearcoat scene renders");

    let frame = renderer.frame_rgba8();
    let matte = max_luminance_in_region(frame, 96, 0, 48);
    let coated = max_luminance_in_region(frame, 96, 48, 96);
    assert!(
        coated > matte + 8,
        "GPU clearcoat texture R channel should brighten the coated material; matte={matte} coated={coated}",
    );
}

#[test]
fn m8_environment_hdr_lights_pbr_preview_pixels() {
    let environment_path = AssetPath::from("memory://studio-blue_2x1.hdr");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        environment_path.clone(),
        tiny_radiance_hdr_rgbe(2, 1, &[[24, 48, 128, 129], [24, 48, 128, 129]]),
    )]));
    let environment = pollster::block_on(assets.load_environment(environment_path.as_str()))
        .expect("HDR environment loads");
    let without_environment = render_environment_preview_center(&assets, None);
    let with_environment = render_environment_preview_center(&assets, Some(environment));

    assert!(
        with_environment[2] > without_environment[2] + 10
            && with_environment[2] > with_environment[0] + 10,
        "active HDR environment should contribute blue IBL to PBR preview pixels, without={without_environment:?} with={with_environment:?}"
    );
}

#[test]
fn m8_environment_hdr_data_uri_lights_pbr_preview_pixels() {
    let hdr_bytes = tiny_radiance_hdr_rgbe(2, 1, &[[24, 48, 128, 129], [24, 48, 128, 129]]);
    let environment_path = format!(
        "data:application/radiance-hdr;base64,{}#studio-blue_2x1.hdr",
        base64::engine::general_purpose::STANDARD.encode(hdr_bytes)
    );
    let assets = Assets::new();
    let environment = pollster::block_on(assets.load_environment(environment_path.as_str()))
        .expect("inline HDR environment loads");
    let without_environment = render_environment_preview_center(&assets, None);
    let with_environment = render_environment_preview_center(&assets, Some(environment));

    assert!(
        with_environment[2] > without_environment[2] + 10
            && with_environment[2] > with_environment[0] + 10,
        "inline HDR environments should contribute blue IBL to PBR preview pixels, without={without_environment:?} with={with_environment:?}"
    );
}

#[test]
fn m8_direct_load_texture_decodes_jpeg_for_cpu_preview_pixels() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://direct-texture/base-color.jpg"),
        include_bytes!("assets/gltf/khronos/AlphaBlendModeTest/MatBed_baseColor.jpg").to_vec(),
    )]));
    let texture = pollster::block_on(assets.load_texture(
        "memory://direct-texture/base-color.jpg",
        TextureColorSpace::Srgb,
    ))
    .expect("direct JPEG texture load succeeds");
    let desc = assets.texture(texture).expect("texture descriptor exists");
    assert_eq!(desc.source_format(), TextureSourceFormat::Jpeg);
    assert!(
        desc.has_decoded_pixels(),
        "direct load_texture should decode JPEG bytes supplied by the asset fetcher",
    );
}

#[test]
fn m8_texture_sampler_clamp_to_edge_affects_cpu_preview_pixels() {
    let strip_png = png_rgba8(2, 1, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [-0.25_f32, 0.5, -0.25, 0.5, -0.25, 0.5] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    let gltf = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit"],
            "extensionsRequired": ["KHR_materials_unlit"],
            "images": [{{ "uri": "strip.png" }}],
            "textures": [{{ "source": 0, "sampler": 0 }}],
            "samplers": [{{ "magFilter": 9728, "minFilter": 9728, "wrapS": 33071, "wrapT": 33071 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{ "index": 0 }}
                }},
                "extensions": {{ "KHR_materials_unlit": {{}} }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "ClampTexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 66, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    );
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://sampler-clamp/scene.gltf"),
            gltf.into_bytes(),
        ),
        (
            AssetPath::from("memory://sampler-clamp/strip.png"),
            strip_png,
        ),
    ]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://sampler-clamp/scene.gltf"))
        .expect("sampler clamp glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("sampler clamp scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("sampler clamp scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 150 && frame[center + 1] < 80 && frame[center + 2] < 80,
        "CLAMP_TO_EDGE sampler should clamp negative U to the red edge texel, got {:?}",
        &frame[center..center + 4]
    );
}

#[test]
fn m8_metallic_roughness_factors_affect_cpu_preview_pixels() {
    let dielectric = render_center_rgb_for_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(190, 190, 190),
        0.0,
        0.95,
    ));
    let polished_metal = render_center_rgb_for_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(190, 190, 190),
        1.0,
        0.15,
    ));

    assert_ne!(
        dielectric, polished_metal,
        "metallic and roughness factors must visibly affect rendered pixels even in the degraded CPU preview path",
    );
}

#[test]
fn m8_normal_png_texture_affects_cpu_preview_pixels() {
    let flat = render_center_rgb_for_normal_texture([128, 128, 255, 255]);
    let inverted = render_center_rgb_for_normal_texture([128, 128, 0, 255]);

    assert_ne!(
        flat, inverted,
        "normal texture pixels must affect CPU preview lighting instead of being silently ignored",
    );
    assert!(
        flat[0] > inverted[0],
        "front-facing normal map should receive more directional light than an inverted normal, flat={flat:?}, inverted={inverted:?}",
    );
}

#[test]
fn m8_metallic_roughness_png_texture_affects_cpu_preview_pixels() {
    let rough_dielectric = render_center_rgb_for_metallic_roughness_texture([0, 255, 0, 255]);
    let polished_metal = render_center_rgb_for_metallic_roughness_texture([0, 32, 255, 255]);

    assert_ne!(
        rough_dielectric, polished_metal,
        "metallic-roughness texture G/B channels must affect CPU preview lighting instead of being silently ignored",
    );
}

#[test]
fn m8_occlusion_png_texture_affects_cpu_preview_pixels() {
    let unoccluded = render_center_rgb_for_occlusion_texture([255, 255, 255, 255]);
    let occluded = render_center_rgb_for_occlusion_texture([0, 0, 0, 255]);

    assert_ne!(
        unoccluded, occluded,
        "occlusion texture pixels must affect the degraded CPU preview instead of being silently ignored",
    );
    assert!(
        unoccluded[0] > occluded[0],
        "white occlusion should keep more light than black occlusion, unoccluded={unoccluded:?}, occluded={occluded:?}",
    );
}

#[test]
fn m8_clearcoat_png_textures_affect_cpu_preview_pixels() {
    let no_clearcoat = render_max_luminance_for_clearcoat_texture([0, 0, 0, 255]);
    let full_clearcoat = render_max_luminance_for_clearcoat_texture([255, 0, 0, 255]);

    assert!(
        full_clearcoat > no_clearcoat,
        "clearcoat texture R channel must affect CPU preview lighting instead of being silently ignored: off={no_clearcoat} on={full_clearcoat}",
    );

    let polished = render_max_luminance_for_clearcoat_roughness_texture([0, 0, 0, 255]);
    let rough = render_max_luminance_for_clearcoat_roughness_texture([0, 255, 0, 255]);
    assert_ne!(
        polished, rough,
        "clearcoat roughness texture G channel must affect CPU preview lighting instead of being silently ignored",
    );
}

#[test]
fn m8_clearcoat_normal_texture_affects_cpu_preview_pixels() {
    let flat = render_max_luminance_for_clearcoat_normal_texture([128, 128, 255, 255]);
    let tilted = render_max_luminance_for_clearcoat_normal_texture([255, 128, 128, 255]);

    assert_ne!(
        flat, tilted,
        "clearcoat normal texture pixels must affect CPU preview lighting instead of being silently ignored",
    );
}

#[test]
fn m8_sheen_png_textures_affect_cpu_preview_pixels() {
    let black_sheen = render_center_rgb_for_sheen_color_texture([0, 0, 0, 255]);
    let red_sheen = render_center_rgb_for_sheen_color_texture([255, 0, 0, 255]);

    assert!(
        red_sheen[0] > black_sheen[0],
        "sheen color texture RGB must affect CPU preview lighting instead of being silently ignored: black={black_sheen:?} red={red_sheen:?}",
    );

    let polished = render_max_luminance_for_sheen_roughness_texture([0, 0, 0, 0]);
    let rough = render_max_luminance_for_sheen_roughness_texture([0, 0, 0, 255]);
    assert_ne!(
        polished, rough,
        "sheen roughness texture alpha channel must affect CPU preview lighting instead of being silently ignored",
    );
}

#[test]
fn m8_anisotropy_png_texture_affects_cpu_preview_pixels() {
    let no_anisotropy = render_max_luminance_for_anisotropy_texture([255, 128, 0, 255]);
    let full_anisotropy = render_max_luminance_for_anisotropy_texture([255, 128, 255, 255]);

    assert!(
        full_anisotropy > no_anisotropy,
        "anisotropy texture B channel must multiply anisotropyStrength and affect CPU preview lighting: off={no_anisotropy} on={full_anisotropy}",
    );
}

#[test]
fn m8_iridescence_png_textures_affect_cpu_preview_pixels() {
    let off = render_center_rgb_for_iridescence_textures([0, 0, 0, 255], [0, 255, 0, 255]);
    let thin = render_center_rgb_for_iridescence_textures([255, 0, 0, 255], [0, 0, 0, 255]);
    let thick = render_center_rgb_for_iridescence_textures([255, 0, 0, 255], [0, 255, 0, 255]);

    assert_ne!(
        off, thin,
        "iridescenceTexture R channel must multiply iridescenceFactor and affect CPU preview pixels",
    );
    assert_ne!(
        dominant_rgb_channel(thin),
        dominant_rgb_channel(thick),
        "iridescenceThicknessTexture G channel must shift the thin-film hue in CPU preview pixels: thin={thin:?} thick={thick:?}",
    );
}

#[test]
fn m8_dispersion_factor_affects_cpu_preview_pixels() {
    let off = render_center_rgb_for_dispersion_factor(0.0);
    let on = render_center_rgb_for_dispersion_factor(1.0);

    assert_ne!(
        dominant_rgb_channel(off),
        dominant_rgb_channel(on),
        "dispersion must separate the visible channel response instead of being silently ignored: off={off:?} on={on:?}",
    );
}

#[test]
fn m8_transmission_volume_textures_affect_cpu_preview_pixels() {
    let blocked =
        render_center_rgb_for_transmission_volume_textures([0, 0, 0, 255], [0, 255, 0, 255]);
    let blue_glass =
        render_center_rgb_for_transmission_volume_textures([255, 0, 0, 255], [0, 255, 0, 255]);

    assert!(
        rgb_distance_u8(blocked, blue_glass) > 120,
        "transmissionTexture R must gate physical transmission and change CPU preview pixels substantially: blocked={blocked:?} blue_glass={blue_glass:?}",
    );
    assert!(
        rgb_sum(blocked) > rgb_sum(blue_glass) + 180,
        "blocked transmission should stay opaque/bright while transmitted volume is absorbed by glass: blocked={blocked:?} blue_glass={blue_glass:?}",
    );
    assert!(
        blue_glass[2] > blue_glass[0] + 12 && blue_glass[2] > blue_glass[1] + 6,
        "volume attenuation color should tint transmitted glass toward blue: {blue_glass:?}",
    );
}

#[test]
fn m8_missing_texture_slots_fail_with_actionable_asset_error() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://missing-texture.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "materials": [{
                "pbrMetallicRoughness": {
                    "baseColorTexture": { "index": 9 }
                }
            }],
            "meshes": [{
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "material": 0
                }]
            }],
            "nodes": [{ "name": "Root", "mesh": 0 }],
            "buffers": [{ "byteLength": 36, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAA" }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
            "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }]
        }"#
        .to_vec(),
    )]));

    let error = pollster::block_on(assets.load_scene("memory://missing-texture.gltf"))
        .expect_err("missing texture index must not silently fall back");
    assert!(
        matches!(
            error,
            AssetError::MissingTexture {
                ref material_slot,
                texture_index: 9,
                ..
            } if material_slot == "baseColorTexture"
        ),
        "unexpected error: {error:?}"
    );
    assert!(error.help().contains("material slot"));
}

fn render_center_rgb_for_material(material: MaterialDesc) -> [u8; 3] {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.75, 0.75, 0.75));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    scene
        .directional_light(DirectionalLight::key_light().with_illuminance_lux(12_000.0))
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(48, 48).expect("renderer builds");
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::NeutralStudio))
            .expect("neutral studio environment loads");
    renderer.set_environment(environment);

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let center = ((48 / 2) * 48 + (48 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    [frame[center], frame[center + 1], frame[center + 2]]
}

#[test]
fn m8_unsupported_texture_formats_fail_before_silent_handles_are_created() {
    let assets = Assets::new();
    let error =
        pollster::block_on(assets.load_texture("textures/albedo.tga", TextureColorSpace::Srgb))
            .expect_err("unsupported texture format should not create a handle");

    assert!(matches!(
        error,
        AssetError::UnsupportedTextureFormat { ref path, .. } if path == "textures/albedo.tga"
    ));
    assert!(error.help().contains("supported texture format"));

    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://unsupported-texture.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "images": [{ "uri": "albedo.tga" }],
            "textures": [{ "source": 0 }],
            "materials": [{
                "pbrMetallicRoughness": {
                    "baseColorTexture": { "index": 0 }
                }
            }],
            "meshes": [{
                "primitives": [{
                    "attributes": { "POSITION": 0 },
                    "material": 0
                }]
            }],
            "nodes": [{ "name": "Root", "mesh": 0 }],
            "buffers": [{ "byteLength": 36, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAA" }],
            "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
            "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }]
        }"#
        .to_vec(),
    )]));

    let error = pollster::block_on(assets.load_scene("memory://unsupported-texture.gltf"))
        .expect_err("unsupported glTF texture format must fail during asset load");
    assert!(matches!(
        error,
        AssetError::UnsupportedTextureFormat { ref path, .. } if path == "memory://albedo.tga"
    ));
}

#[test]
fn m8_scene_load_reports_cache_fetch_and_external_buffer_metadata() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://reported.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "nodes": [{ "name": "Root" }]
        }"#
        .to_vec(),
    )]));

    let first = pollster::block_on(assets.load_scene_with_report("memory://reported.gltf"))
        .expect("first load reports telemetry");
    assert_eq!(first.path().as_str(), "memory://reported.gltf");
    assert!(!first.cache_hit());
    assert!(first.fetched_bytes() > 0);
    assert_eq!(first.external_buffers(), 0);
    assert_eq!(first.asset().nodes()[0].name(), Some("Root"));

    let second = pollster::block_on(assets.load_scene_with_report("memory://reported.gltf"))
        .expect("cached load reports cache hit");
    assert!(second.cache_hit());
    assert_eq!(second.fetched_bytes(), 0);
}

#[test]
fn m8_scene_load_progress_reports_fetch_parse_cache_and_external_buffers() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://progress/scene.gltf"),
            br#"{
                "asset": { "version": "2.0" },
                "nodes": [{ "name": "ProgressRoot" }],
                "buffers": [{ "byteLength": 4, "uri": "buffer.bin" }]
            }"#
            .to_vec(),
        ),
        (
            AssetPath::from("memory://progress/buffer.bin"),
            vec![1, 2, 3, 4],
        ),
    ]));

    let mut observed = Vec::new();
    let report = pollster::block_on(assets.load_scene_with_progress(
        "memory://progress/scene.gltf",
        |event| {
            observed.push(event.clone());
        },
    ))
    .expect("progress load succeeds");

    assert_eq!(report.progress_events(), observed.as_slice());
    assert!(observed.iter().any(|event| matches!(
        event,
        AssetLoadProgress::LoadStarted { path }
            if path.as_str() == "memory://progress/scene.gltf"
    )));
    assert!(observed.iter().any(|event| matches!(
        event,
        AssetLoadProgress::AssetFetched { path, bytes }
            if path.as_str() == "memory://progress/scene.gltf" && *bytes > 0
    )));
    assert!(observed.iter().any(|event| matches!(
        event,
        AssetLoadProgress::ExternalBufferFetched { path, index: 0, bytes }
            if path.as_str() == "memory://progress/buffer.bin" && *bytes == 4
    )));
    assert!(observed.iter().any(|event| matches!(
        event,
        AssetLoadProgress::Parsed { path, nodes: 1, meshes: 0 }
            if path.as_str() == "memory://progress/scene.gltf"
    )));
    assert!(observed.iter().any(|event| matches!(
        event,
        AssetLoadProgress::Cached { path }
            if path.as_str() == "memory://progress/scene.gltf"
    )));

    let mut cached = Vec::new();
    let cached_report = pollster::block_on(assets.load_scene_with_progress(
        "memory://progress/scene.gltf",
        |event| {
            cached.push(event.clone());
        },
    ))
    .expect("cached progress load succeeds");
    assert!(cached_report.cache_hit());
    assert_eq!(
        cached,
        vec![
            AssetLoadProgress::LoadStarted {
                path: AssetPath::from("memory://progress/scene.gltf")
            },
            AssetLoadProgress::CacheHit {
                path: AssetPath::from("memory://progress/scene.gltf")
            }
        ]
    );
}

#[cfg(not(feature = "ktx2"))]
#[test]
fn m8_ktx2_basisu_texture_requires_feature_or_explicit_decoder_policy() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://basisu.gltf"),
        basisu_material_gltf().to_vec(),
    )]));

    let error = pollster::block_on(assets.load_scene("memory://basisu.gltf"))
        .expect_err("KTX2/Basis must not silently create a texture without the feature");
    assert!(matches!(
        error,
        AssetError::UnsupportedOptionalExtensionUsed { ref extension, ref help, .. }
            if extension == "KHR_texture_basisu" && help.contains("ktx2")
    ));
}

#[cfg(not(feature = "ktx2"))]
#[test]
fn m8_direct_load_texture_ktx2_fails_closed_without_feature() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://direct/albedo.ktx2"),
        vec![0, 1, 2, 3],
    )]));
    let error = pollster::block_on(
        assets.load_texture("memory://direct/albedo.ktx2", TextureColorSpace::Srgb),
    )
    .expect_err("direct KTX2 texture load must fail without the ktx2 feature");
    assert!(matches!(
        error,
        AssetError::UnsupportedTextureFormat { ref path, .. }
            if path == "memory://direct/albedo.ktx2"
    ));
}

#[cfg(feature = "ktx2")]
#[test]
fn m8_ktx2_basisu_feature_rejects_descriptor_only_texture_load() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://basisu.gltf"),
        basisu_material_gltf().to_vec(),
    )]));

    let error = pollster::block_on(assets.load_scene("memory://basisu.gltf"))
        .expect_err("KTX2/Basis support must not pass from descriptor metadata alone");
    assert!(matches!(
        error,
        AssetError::UnsupportedOptionalExtensionUsed { ref extension, ref help, .. }
            if extension == "KHR_texture_basisu" && help.contains("decodable KTX2")
    ));
}

#[cfg(feature = "ktx2")]
#[test]
fn m8_ktx2_basisu_feature_rejects_malformed_ktx2_bytes_at_container_boundary() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://basisu.gltf"),
            basisu_material_gltf().to_vec(),
        ),
        (
            AssetPath::from("memory://albedo.ktx2"),
            b"not a ktx2 container".to_vec(),
        ),
    ]));

    let error = pollster::block_on(assets.load_scene("memory://basisu.gltf"))
        .expect_err("malformed KTX2 bytes must fail at the KTX2 container boundary");
    assert!(matches!(
        error,
        AssetError::Parse { ref path, ref reason }
            if path == "memory://albedo.ktx2" && reason.contains("invalid KTX2 container")
    ));
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn m8_ktx2_basisu_feature_decodes_basisu_ktx2_rgba_pixels() {
    let ktx2 = tiny_basisu_ktx2_solid_red();
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://basisu.gltf"),
            basisu_material_gltf().to_vec(),
        ),
        (AssetPath::from("memory://albedo.ktx2"), ktx2),
    ]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://basisu.gltf")).expect("glTF loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let material = assets.material(mesh.material()).expect("material exists");
    let texture = assets
        .texture(material.base_color_texture().expect("base texture exists"))
        .expect("texture descriptor exists");

    let (width, height, rgba) = texture
        .decoded_rgba8()
        .expect("KTX2/Basis texture produces decoded RGBA8 pixels");
    assert_eq!((width, height), (4, 4));
    assert_eq!(rgba.len(), 4 * 4 * 4);
    for pixel in rgba.chunks_exact(4) {
        assert!(
            pixel[0] > 160 && pixel[1] < 96 && pixel[2] < 96 && pixel[3] > 240,
            "decoded solid-red BasisU pixel must stay in the authored color family, got {pixel:?}",
        );
    }
    assert_eq!(
        texture.decoded_mip_metadata(),
        Some(vec![(4, 4, 4 * 4 * 4)]),
        "KTX2 metadata must expose decoded mip dimensions and byte lengths",
    );
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn m8_direct_load_texture_ktx2_decodes_rgba_pixels() {
    let ktx2 = tiny_basisu_ktx2_solid_red();
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://direct/albedo.ktx2"),
        ktx2,
    )]));
    let texture = pollster::block_on(
        assets.load_texture("memory://direct/albedo.ktx2", TextureColorSpace::Srgb),
    )
    .expect("direct KTX2 load succeeds with decoder feature");
    let texture = assets.texture(texture).expect("texture exists");
    let (width, height, rgba) = texture
        .decoded_rgba8()
        .expect("direct KTX2 load decodes RGBA8 pixels");
    assert_eq!((width, height), (4, 4));
    assert!(
        rgba.chunks_exact(4)
            .all(|pixel| pixel[0] > 160 && pixel[1] < 96 && pixel[2] < 96 && pixel[3] > 240)
    );
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn m8_data_uri_ktx2_decodes_rgba_pixels() {
    let encoded = base64::engine::general_purpose::STANDARD.encode(tiny_basisu_ktx2_solid_red());
    let uri = format!("data:image/ktx2;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Srgb))
        .expect("KTX2 data URI texture loads");
    let texture = assets.texture(texture).expect("texture exists");
    assert_eq!(texture.decoded_dimensions(), Some((4, 4)));
    assert!(texture.has_decoded_pixels());
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn m8_gltf_buffer_view_ktx2_image_decodes_rgba_pixels() {
    let ktx2 = tiny_basisu_ktx2_solid_red();
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://basisu-buffer-view.gltf"),
        basisu_buffer_view_gltf(&ktx2),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://basisu-buffer-view.gltf"))
        .expect("bufferView KTX2 glTF loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let material = assets.material(mesh.material()).expect("material exists");
    let texture = assets
        .texture(material.base_color_texture().expect("base texture exists"))
        .expect("texture descriptor exists");
    assert_eq!(texture.source_format(), TextureSourceFormat::Ktx2Basisu);
    assert_eq!(texture.decoded_dimensions(), Some((4, 4)));
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
#[test]
fn m8_ktx2_base_color_texture_affects_cpu_preview_pixels() {
    let ktx2 = tiny_basisu_ktx2_solid_red();
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://basisu.gltf"),
            basisu_material_gltf().to_vec(),
        ),
        (AssetPath::from("memory://albedo.ktx2"), ktx2),
    ]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://basisu.gltf")).expect("glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("KTX2 textured scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("KTX2 textured scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 120 && frame[center + 1] < 100 && frame[center + 2] < 100,
        "KTX2 base-color texture should visibly affect CPU preview pixels, got {:?}",
        &frame[center..center + 4]
    );
}

#[cfg(not(feature = "ktx2"))]
#[test]
fn m8_optional_basisu_texture_uses_png_fallback_without_ktx2_feature() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://basisu-fallback.gltf"),
        basisu_with_png_fallback_gltf().into_bytes(),
    )]));
    let report = pollster::block_on(assets.load_scene_with_report("memory://basisu-fallback.gltf"))
        .expect("optional KHR_texture_basisu with PNG fallback loads without ktx2");
    let scene_asset = report.asset();
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let material = assets.material(mesh.material()).expect("material exists");
    let texture = assets
        .texture(material.base_color_texture().expect("base texture exists"))
        .expect("fallback texture descriptor exists");
    assert_eq!(texture.source_format(), TextureSourceFormat::Png);
    assert_eq!(texture.decoded_dimensions(), Some((1, 1)));
    let schema_json = report.to_schema_json();
    let fallback = schema_json["material_fallbacks"]
        .as_array()
        .and_then(|fallbacks| fallbacks.first())
        .expect("optional texture fallback is reported");
    assert_eq!(fallback["kind"], "texture_basisu_fallback");
    assert_eq!(fallback["material_slot"], "baseColorTexture");
    assert_eq!(fallback["texture_index"], 0);
    assert!(
        fallback["source_path"]
            .as_str()
            .expect("source_path is a string")
            .ends_with("missing-albedo.ktx2"),
        "fallback source should name the skipped Basis source, got {fallback:?}"
    );
    let fallback_path = fallback["fallback_path"]
        .as_str()
        .expect("fallback_path is a string");
    assert!(
        fallback_path.starts_with("memory:image-sha256-")
            && fallback_path.ends_with(".png")
            && fallback_path.len() < 128,
        "fallback path should retain the authored PNG as a bounded content identity, got {fallback:?}"
    );
    assert_eq!(
        fallback["reason"], "KHR_texture_basisu unavailable; using authored fallback texture",
        "optional texture fallbacks must be explicit instead of silently looking source-authored"
    );
}

#[cfg(feature = "inspection")]
#[cfg(not(feature = "ktx2"))]
#[test]
fn m8_scene_inspection_reports_material_fallback_and_source_provenance() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://basisu-inspection.gltf"),
        basisu_with_png_fallback_gltf().into_bytes(),
    )]));
    let report =
        pollster::block_on(assets.load_scene_with_report("memory://basisu-inspection.gltf"))
            .expect("optional KHR_texture_basisu with PNG fallback loads without ktx2");
    let mut scene = Scene::new();
    scene
        .instantiate(report.asset())
        .expect("fallback material scene instantiates");

    let schema_json = scene.inspect_with_assets(&assets).to_schema_json();
    let draw_material = &schema_json["draw_list"][0]["material"];

    assert_eq!(draw_material["source"]["kind"], "source_material");
    assert_eq!(
        draw_material["source"]["asset_path"],
        "memory://basisu-inspection.gltf"
    );
    assert_eq!(draw_material["source"]["material_index"], 0);
    assert_eq!(draw_material["source"]["reason"], serde_json::Value::Null);
    assert_eq!(draw_material["kind"], "pbr_metallic_roughness");

    let texture = draw_material["textures"]
        .as_array()
        .and_then(|textures| {
            textures
                .iter()
                .find(|entry| entry["slot"] == "baseColorTexture")
        })
        .expect("base-color texture evidence is exported");
    assert_eq!(texture["source_format"], "png");
    assert!(
        texture["source_path"]
            .as_str()
            .expect("source_path is a string")
            .starts_with("data:image/png;base64,"),
        "texture source path should name the authored fallback PNG, got {texture:?}"
    );
    assert_eq!(texture["fallback"]["kind"], "texture_basisu_fallback");
    assert_eq!(texture["fallback"]["material_index"], 0);
    assert!(
        texture["fallback"]["source_path"]
            .as_str()
            .expect("source_path is a string")
            .ends_with("missing-albedo.ktx2"),
        "fallback source should name the skipped Basis source, got {texture:?}"
    );

    let fallback = draw_material["fallbacks"]
        .as_array()
        .and_then(|fallbacks| fallbacks.first())
        .expect("material fallback provenance is exported in inspection");
    assert_eq!(fallback["kind"], "texture_basisu_fallback");
    assert_eq!(fallback["material_index"], 0);
    assert_eq!(fallback["material_slot"], "baseColorTexture");
}

#[cfg(feature = "inspection")]
#[test]
fn m8_scene_inspection_reports_generated_default_materials() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://generated-material.gltf"),
        materialless_triangle_gltf().into_bytes(),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://generated-material.gltf"))
        .expect("materialless primitive loads with a generated default material");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("materialless primitive instantiates");

    let schema_json = scene.inspect_with_assets(&assets).to_schema_json();
    let draw_material = &schema_json["draw_list"][0]["material"];

    assert_eq!(draw_material["source"]["kind"], "generated_default");
    assert_eq!(
        draw_material["source"]["asset_path"],
        "memory://generated-material.gltf"
    );
    assert_eq!(
        draw_material["source"]["material_index"],
        serde_json::Value::Null
    );
    assert!(
        draw_material["source"]["reason"]
            .as_str()
            .expect("generated material reason is a string")
            .contains("source primitive did not reference a material"),
        "generated default material must explain why it exists: {draw_material:?}"
    );
    assert_eq!(draw_material["fallbacks"], serde_json::json!([]));
}

#[cfg(feature = "meshopt")]
#[test]
fn m8_meshopt_feature_decodes_required_compressed_buffer_views() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://meshopt-required.gltf"),
        meshopt_compressed_triangle_gltf().into_bytes(),
    )]));

    let scene_asset = pollster::block_on(assets.load_scene("memory://meshopt-required.gltf"))
        .expect("required EXT_meshopt_compression fixture loads with meshopt feature");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let bounds = mesh.bounds();

    assert_eq!(bounds.min, Vec3::new(-0.5, -0.5, 0.0));
    assert_eq!(bounds.max, Vec3::new(0.5, 0.5, 0.0));
}

#[cfg(feature = "meshopt")]
#[test]
fn m8_meshopt_feature_decodes_index_sequence_mode() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://meshopt-indices.gltf"),
        meshopt_index_sequence_gltf().into_bytes(),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://meshopt-indices.gltf"))
        .expect("meshopt INDICES fixture loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    assert_eq!(mesh.bounds().min, Vec3::new(-0.5, -0.5, 0.0));
    assert_eq!(mesh.bounds().max, Vec3::new(0.5, 0.5, 0.0));
}

#[cfg(feature = "meshopt")]
#[test]
fn m8_meshopt_optional_extension_uses_compressed_data_when_feature_enabled() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://meshopt-optional.gltf"),
        meshopt_optional_fallback_gltf(true).into_bytes(),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://meshopt-optional.gltf"))
        .expect("optional meshopt fixture loads with feature");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    assert_eq!(mesh.bounds().min, Vec3::new(-0.5, -0.5, 0.0));
    assert_eq!(mesh.bounds().max, Vec3::new(0.5, 0.5, 0.0));
}

#[cfg(not(feature = "meshopt"))]
#[test]
fn m8_meshopt_optional_extension_uses_raw_fallback_without_feature() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://meshopt-optional.gltf"),
        meshopt_optional_fallback_gltf(false).into_bytes(),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://meshopt-optional.gltf"))
        .expect("optional meshopt fixture loads from fallback without feature");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    assert_eq!(mesh.bounds().min, Vec3::new(-0.5, -0.5, 0.0));
    assert_eq!(mesh.bounds().max, Vec3::new(0.5, 0.5, 0.0));
}

#[cfg(feature = "meshopt")]
#[test]
fn m8_meshopt_malformed_buffer_views_fail_with_structured_errors() {
    for (case, gltf) in [
        (
            "bad source buffer",
            meshopt_malformed_triangle_gltf("buffer", "99"),
        ),
        (
            "bad byte offset",
            meshopt_malformed_triangle_gltf("byteOffset", "999"),
        ),
        (
            "bad byte length",
            meshopt_malformed_triangle_gltf("byteLength", "999"),
        ),
        (
            "bad stride",
            meshopt_malformed_triangle_gltf("byteStride", "3"),
        ),
        (
            "bad mode",
            meshopt_malformed_triangle_gltf("mode", "\"BOGUS\""),
        ),
        (
            "bad filter",
            meshopt_malformed_triangle_gltf("filter", "\"BOGUS\""),
        ),
        (
            "bad count overflow",
            meshopt_malformed_triangle_gltf("count", "18446744073709551615"),
        ),
    ] {
        let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
            AssetPath::from(format!("memory://meshopt-malformed-{case}.gltf")),
            gltf.into_bytes(),
        )]));
        let error = pollster::block_on(
            assets.load_scene(format!("memory://meshopt-malformed-{case}.gltf")),
        )
        .expect_err("malformed meshopt view must fail");
        assert!(
            matches!(error, AssetError::Parse { ref reason, .. }
                if reason.contains("EXT_meshopt_compression")
                    || reason.contains("decompressed bufferView")
                    || reason.contains("meshopt")),
            "{case} should fail at the meshopt decoder boundary, got {error:?}",
        );
    }
}

#[cfg(feature = "meshopt")]
#[test]
fn m8_meshopt_decoded_geometry_affects_cpu_rendered_silhouette() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://meshopt-render.gltf"),
        meshopt_compressed_triangle_gltf().into_bytes(),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://meshopt-render.gltf"))
        .expect("meshopt render fixture loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("meshopt scene instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("meshopt scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let center = ((64 / 2) * 64 + (64 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    assert!(
        frame[center] > 16 || frame[center + 1] > 16 || frame[center + 2] > 16,
        "decoded meshopt triangle should produce non-empty CPU silhouette at center, got {:?}",
        &frame[center..center + 4]
    );
}

#[test]
fn m8_ext_mesh_gpu_instancing_imports_node_as_instance_set() {
    let gltf = ext_mesh_gpu_instancing_triangle_gltf();
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://instanced-triangle.gltf"),
        gltf.into_bytes(),
    )]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://instanced-triangle.gltf"))
        .expect("EXT_mesh_gpu_instancing glTF loads");

    let diagnostic = scene_asset
        .extension_diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.extension() == "EXT_mesh_gpu_instancing")
        .expect("instancing extension is diagnosed");
    assert_eq!(diagnostic.status(), GltfExtensionStatus::Supported);
    assert_eq!(diagnostic.decoder_policy(), GltfDecoderPolicy::BuiltIn);
    assert_eq!(scene_asset.nodes()[0].instance_transforms().len(), 2);

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("instanced scene instantiates");
    let node = import
        .node("InstancedTriangle")
        .expect("import node exists");
    let instance_set = match scene.node(node).expect("instanced node exists").kind() {
        NodeKind::InstanceSet(instance_set) => scene
            .instance_set(*instance_set)
            .expect("instance set handle resolves"),
        other => panic!("instanced mesh must import as an InstanceSet node, got {other:?}"),
    };
    let translations = instance_set
        .instances()
        .map(|instance| instance.transform().translation)
        .collect::<Vec<_>>();
    assert_eq!(
        translations,
        vec![Vec3::new(-0.7, 0.0, 0.0), Vec3::new(0.7, 0.0, 0.0)]
    );
}

#[test]
fn m8_cancelled_scene_load_does_not_cache_partial_asset_state() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://cancel.gltf"),
        br#"{
            "asset": { "version": "2.0" },
            "nodes": [{ "name": "LoadedAfterCancel" }]
        }"#
        .to_vec(),
    )]));
    let control = AssetLoadControl::cancelled();

    let cancelled =
        pollster::block_on(assets.load_scene_controlled("memory://cancel.gltf", &control))
            .expect_err("cancelled load should fail explicitly");
    assert!(matches!(
        cancelled,
        AssetError::Cancelled { ref path, .. } if path == "memory://cancel.gltf"
    ));

    let loaded = pollster::block_on(assets.load_scene_with_report("memory://cancel.gltf"))
        .expect("later uncancelled load should fetch and cache normally");
    assert!(!loaded.cache_hit());
    assert_eq!(loaded.asset().nodes()[0].name(), Some("LoadedAfterCancel"));
}

#[test]
fn m8_asset_resource_lifetime_counters_return_to_baseline_after_reload_cycle() {
    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let albedo = pollster::block_on(
        assets.load_texture("textures/m8-lifetime-albedo.png", TextureColorSpace::Srgb),
    )
    .expect("albedo texture records");
    let normal = pollster::block_on(
        assets.load_texture("textures/m8-lifetime-normal.png", TextureColorSpace::Linear),
    )
    .expect("normal texture records");
    let metallic_roughness = pollster::block_on(assets.load_texture(
        "textures/m8-lifetime-metallic-roughness.png",
        TextureColorSpace::Linear,
    ))
    .expect("metallic roughness texture records");
    let occlusion = pollster::block_on(assets.load_texture(
        "textures/m8-lifetime-occlusion.png",
        TextureColorSpace::Linear,
    ))
    .expect("occlusion texture records");
    let emissive = pollster::block_on(
        assets.load_texture("textures/m8-lifetime-emissive.png", TextureColorSpace::Srgb),
    )
    .expect("emissive texture records");
    let environment = assets.default_environment();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("scene fixture loads");
    let reloaded = pollster::block_on(assets.reload_scene(&scene_asset))
        .expect("retained scene fixture reloads");

    let geometry = assets.create_geometry(scena::GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(scena::Color::WHITE, 0.1, 0.8)
            .with_base_color_texture(albedo)
            .with_normal_texture(normal)
            .with_metallic_roughness_texture(metallic_roughness)
            .with_occlusion_texture(occlusion)
            .with_emissive_texture(emissive),
    );
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("scene fixture instantiates");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(scena::Vec3::new(0.25, 0.0, 0.0)))
        .add()
        .expect("textured mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("renderer builds");
    let baseline = renderer.stats();

    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("asset-heavy scene prepares");
    let prepared = renderer.stats();
    assert!(prepared.textures >= 5);
    assert!(prepared.materials >= 1);
    assert!(
        prepared.material_bindings >= 1,
        "prepared PBR materials must create renderer-visible material binding records"
    );
    assert!(
        prepared.material_texture_bindings >= 5,
        "each PBR texture slot must become a renderer-visible texture binding record"
    );
    assert!(
        prepared.material_sampler_bindings >= 5,
        "each PBR texture slot must carry a sampler binding record"
    );
    assert_eq!(prepared.environments, 1);
    assert!(prepared.live_logical_handles > baseline.live_logical_handles);

    scene
        .replace_import(&import, &reloaded)
        .expect("reload replacement succeeds");
    assert!(matches!(
        renderer.render(&scene, camera),
        Err(RenderError::NotPrepared {
            reason: NotPreparedReason::SceneChanged { .. }
        })
    ));
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("reloaded scene prepares");

    renderer.clear_environment();
    let mut empty_scene = Scene::new();
    empty_scene.add_default_camera().expect("camera inserts");
    renderer
        .prepare(&mut empty_scene)
        .expect("empty scene prepares after resource release");
    let released = renderer.stats();
    assert_eq!(released.textures, baseline.textures);
    assert_eq!(released.materials, baseline.materials);
    assert_eq!(released.environments, baseline.environments);
    assert_eq!(released.live_logical_handles, baseline.live_logical_handles);
    assert_eq!(released.pending_destructions, baseline.pending_destructions);
}

#[test]
fn m8_khronos_material_texture_samples_cover_promoted_extensions() {
    let assets = Assets::new();

    let alpha = pollster::block_on(assets.load_scene_with_report(
        "tests/assets/gltf/khronos/AlphaBlendModeTest/AlphaBlendModeTest.gltf",
    ))
    .expect("Khronos alpha material sample loads");
    assert_eq!(alpha.external_buffers(), 1);
    let alpha_materials = scene_materials(alpha.asset(), &assets);
    assert!(
        alpha_materials
            .iter()
            .any(|material| material.alpha_mode() == AlphaMode::Blend)
    );
    assert!(
        alpha_materials
            .iter()
            .any(|material| material.alpha_mode() == AlphaMode::Mask { cutoff: 0.25 })
    );
    assert!(
        alpha_materials
            .iter()
            .any(|material| material.alpha_mode() == AlphaMode::Mask { cutoff: 0.75 })
    );
    assert!(
        alpha_materials
            .iter()
            .any(|material| material.normal_texture().is_some())
    );
    assert!(
        alpha_materials
            .iter()
            .any(|material| material.occlusion_texture().is_some())
    );
    assert!(
        alpha_materials
            .iter()
            .any(|material| material.metallic_roughness_texture().is_some())
    );

    let settings = pollster::block_on(assets.load_scene_with_report(
        "tests/assets/gltf/khronos/TextureSettingsTest/TextureSettingsTest.gltf",
    ))
    .expect("Khronos texture settings sample loads");
    assert_eq!(settings.external_buffers(), 1);
    let settings_textures = scene_texture_descs(settings.asset(), &assets);
    assert!(settings_textures.iter().any(|texture| {
        texture.sampler().wrap_t() == TextureWrap::MirroredRepeat
            && texture.sampler().min_filter() == Some(TextureFilter::NearestMipmapLinear)
    }));
    assert!(
        settings_textures
            .iter()
            .any(|texture| texture.sampler().wrap_s() == TextureWrap::ClampToEdge)
    );

    let transform = pollster::block_on(assets.load_scene_with_report(
        "tests/assets/gltf/khronos/TextureTransformTest/TextureTransformTest.gltf",
    ))
    .expect("Khronos texture transform sample loads");
    assert_eq!(transform.external_buffers(), 1);
    assert!(
        transform
            .asset()
            .extensions_used()
            .iter()
            .any(|extension| extension == "KHR_texture_transform")
    );
    let transform_materials = scene_materials(transform.asset(), &assets);
    assert!(transform_materials.iter().any(|material| {
        material
            .base_color_texture_transform()
            .is_some_and(|transform| transform.offset() == [0.5, 0.0])
    }));
    assert!(transform_materials.iter().any(|material| {
        material
            .base_color_texture_transform()
            .is_some_and(|transform| transform.rotation_radians() > 0.29)
    }));
    assert!(transform_materials.iter().any(|material| {
        material
            .base_color_texture_transform()
            .is_some_and(|transform| transform.scale() == [1.5, 1.5])
    }));

    let unlit = pollster::block_on(
        assets.load_scene_with_report("tests/assets/gltf/khronos/UnlitTest/UnlitTest.gltf"),
    )
    .expect("Khronos unlit sample loads");
    assert_eq!(unlit.external_buffers(), 1);
    assert!(
        unlit
            .asset()
            .extensions_required()
            .iter()
            .any(|extension| extension == "KHR_materials_unlit")
    );
    assert!(
        scene_materials(unlit.asset(), &assets)
            .iter()
            .any(|material| material.kind() == MaterialKind::Unlit)
    );
}

#[test]
fn m8_khronos_jpeg_textures_decode_for_degraded_material_preview() {
    let assets = Assets::new();
    let alpha = pollster::block_on(assets.load_scene_with_report(
        "tests/assets/gltf/khronos/AlphaBlendModeTest/AlphaBlendModeTest.gltf",
    ))
    .expect("Khronos alpha material sample loads");

    let jpeg_textures = scene_texture_descs(alpha.asset(), &assets)
        .into_iter()
        .filter(|texture| texture.source_format() == TextureSourceFormat::Jpeg)
        .collect::<Vec<_>>();
    assert!(
        !jpeg_textures.is_empty(),
        "AlphaBlendModeTest should exercise external JPEG material textures"
    );
    assert!(
        jpeg_textures
            .iter()
            .all(scena::TextureDesc::has_decoded_pixels),
        "external JPEG material textures must decode into CPU/degraded preview pixels"
    );
}

#[test]
fn m8_real_world_fixture_matrix_covers_asset_edge_cases() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://real-world/material-degradation.gltf"),
            br#"{
                "asset": { "version": "2.0" },
                "extensionsUsed": [
                    "KHR_materials_clearcoat",
                    "KHR_materials_transmission",
                    "KHR_materials_ior",
                    "KHR_materials_volume",
                    "KHR_materials_variants",
                    "KHR_texture_basisu",
                    "KHR_draco_mesh_compression",
                    "EXT_meshopt_compression"
                ],
                "nodes": [{ "name": "RealWorldOptionalExtensions" }]
            }"#
            .to_vec(),
        ),
        (
            AssetPath::from("memory://real-world/draco-required.gltf"),
            required_extension_gltf("KHR_draco_mesh_compression").into_bytes(),
        ),
        (
            AssetPath::from("memory://real-world/meshopt-required.gltf"),
            required_extension_gltf("EXT_meshopt_compression").into_bytes(),
        ),
        (
            AssetPath::from("memory://real-world/missing-texture.gltf"),
            missing_texture_gltf().to_vec(),
        ),
        (
            AssetPath::from("memory://real-world/external/scene.gltf"),
            external_buffer_gltf("triangle.bin").into_bytes(),
        ),
        (
            AssetPath::from("memory://real-world/external/triangle.bin"),
            external_triangle_buffer(),
        ),
        (
            AssetPath::from("memory://real-world/embedded.glb"),
            minimal_glb_triangle_scene(),
        ),
    ]));

    let degraded =
        pollster::block_on(assets.load_scene("memory://real-world/material-degradation.gltf"))
            .expect("optional real-world extension fixture loads with diagnostics");
    for extension in [
        "KHR_materials_clearcoat",
        "KHR_materials_transmission",
        "KHR_materials_ior",
        "KHR_materials_volume",
        "KHR_materials_variants",
        "KHR_texture_basisu",
        "KHR_draco_mesh_compression",
        "EXT_meshopt_compression",
    ] {
        assert!(
            degraded
                .extension_diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.extension() == extension),
            "{extension} should have explicit degradation/support metadata",
        );
    }

    let draco_error =
        pollster::block_on(assets.load_scene("memory://real-world/draco-required.gltf"))
            .expect_err("required Draco compressed mesh extension must fail explicitly");
    assert!(matches!(
        draco_error,
        AssetError::UnsupportedRequiredExtension {
            extension: ref rejected,
            ..
        } if rejected == "KHR_draco_mesh_compression"
    ));

    #[cfg(not(feature = "meshopt"))]
    {
        let meshopt_error = pollster::block_on(
            assets.load_scene("memory://real-world/meshopt-required.gltf"),
        )
        .expect_err(
            "required meshopt compressed mesh extension must fail without the decoder feature",
        );
        assert!(matches!(
            meshopt_error,
            AssetError::UnsupportedRequiredExtension {
                extension: ref rejected,
                ..
            } if rejected == "EXT_meshopt_compression"
        ));
    }
    #[cfg(feature = "meshopt")]
    {
        let meshopt_asset =
            pollster::block_on(assets.load_scene("memory://real-world/meshopt-required.gltf"))
                .expect("required meshopt extension loads when the decoder feature is enabled");
        assert!(
            meshopt_asset
                .extension_diagnostics()
                .iter()
                .any(
                    |diagnostic| diagnostic.extension() == "EXT_meshopt_compression"
                        && diagnostic.status() == GltfExtensionStatus::Supported
                ),
            "enabled meshopt feature must expose supported decoder metadata",
        );
    }

    let missing = pollster::block_on(assets.load_scene("memory://real-world/missing-texture.gltf"))
        .expect_err("missing texture slot must fail explicitly");
    assert!(matches!(missing, AssetError::MissingTexture { .. }));

    let external = pollster::block_on(
        assets.load_scene_with_report("memory://real-world/external/scene.gltf"),
    )
    .expect("relative external-buffer fixture loads");
    assert_eq!(external.external_buffers(), 1);
    assert_eq!(external.asset().mesh_count(), 1);

    let embedded = pollster::block_on(assets.load_scene("memory://real-world/embedded.glb"))
        .expect("embedded GLB fixture loads");
    assert_eq!(embedded.mesh_count(), 1);
}

#[test]
fn m8_native_fetcher_cache_dedup_reload_retain_and_external_buffers_are_explicit() {
    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);

    let first = pollster::block_on(
        assets.load_scene_with_report("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("native file fetcher loads fixture");
    assert!(!first.cache_hit());
    assert!(first.fetched_bytes() > 0);

    let cached = pollster::block_on(
        assets.load_scene_with_report("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("native file fetcher reuses cached scene");
    assert!(cached.cache_hit());
    assert_eq!(cached.fetched_bytes(), 0);
    assert_eq!(cached.asset().path(), first.asset().path());

    let reloaded =
        pollster::block_on(assets.reload_scene(first.asset())).expect("retained source reloads");
    assert_eq!(reloaded.path(), first.asset().path());
    assert_eq!(reloaded.node_count(), first.asset().node_count());

    let albedo_a = pollster::block_on(
        assets.load_texture("textures/native-cache.png", TextureColorSpace::Srgb),
    )
    .expect("texture descriptor loads");
    let albedo_b = pollster::block_on(
        assets.load_texture("textures/native-cache.png", TextureColorSpace::Srgb),
    )
    .expect("texture descriptor cache hit");
    let albedo_linear = pollster::block_on(
        assets.load_texture("textures/native-cache.png", TextureColorSpace::Linear),
    )
    .expect("same texture path under linear color space has separate cache identity");
    assert_eq!(albedo_a, albedo_b);
    assert_ne!(albedo_a, albedo_linear);

    let external = pollster::block_on(assets.load_scene_with_report(
        "tests/assets/gltf/khronos/TextureTransformTest/TextureTransformTest.gltf",
    ))
    .expect("native file fetcher reports relative external buffer");
    assert_eq!(external.external_buffers(), 1);
    assert!(external.progress_events().iter().any(|event| matches!(
        event,
        AssetLoadProgress::ExternalBufferFetched { path, index: 0, bytes }
            if path.as_str()
                == "tests/assets/gltf/khronos/TextureTransformTest/TextureTransformTest.bin"
                && *bytes > 0
    )));
}

#[test]
fn m8_checked_asset_lookups_report_typed_missing_handles() {
    let owner = Assets::new();
    let other = Assets::new();
    let geometry = owner.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let material = owner.create_material(MaterialDesc::unlit(Color::WHITE));
    let texture = pollster::block_on(owner.load_texture(
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
        TextureColorSpace::Srgb,
    ))
    .expect("owner texture loads");
    let environment = owner.default_environment();

    assert!(owner.try_geometry(geometry).is_ok());
    assert!(owner.try_material(material).is_ok());
    assert!(owner.try_texture(texture).is_ok());
    assert!(owner.try_environment(environment).is_ok());

    assert!(matches!(
        other.try_geometry(geometry),
        Err(AssetError::GeometryHandleNotFound { geometry: missing }) if missing == geometry
    ));
    assert!(matches!(
        other.try_material(material),
        Err(AssetError::MaterialHandleNotFound { material: missing }) if missing == material
    ));
    assert!(matches!(
        other.try_texture(texture),
        Err(AssetError::TextureHandleNotFound { texture: missing }) if missing == texture
    ));
    assert!(matches!(
        other.try_environment(environment),
        Err(AssetError::EnvironmentHandleNotFound { environment: missing }) if missing == environment
    ));
}

#[test]
fn m8_prepare_rejects_material_texture_handles_from_wrong_assets() {
    let texture_owner = Assets::new();
    let foreign_texture = pollster::block_on(texture_owner.load_texture(
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==",
        TextureColorSpace::Srgb,
    ))
    .expect("foreign texture loads");

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE).with_base_color_texture(foreign_texture),
    );
    let mut scene = Scene::new();
    let node = scene.mesh(geometry, material).add().expect("mesh inserts");
    scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds");

    let error = renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect_err("foreign texture handles must not silently sample as white");

    assert!(matches!(
        error,
        scena::PrepareError::TextureNotFound {
            node: missing_node,
            material: missing_material,
            texture: missing_texture,
            slot: "base_color",
        } if missing_node == node && missing_material == material && missing_texture == foreign_texture
    ));
}

fn render_center_rgb_for_normal_texture(pixel: [u8; 4]) -> [u8; 3] {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let normal = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("normal texture loads");
    render_center_rgb_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 0.0, 0.75)
            .with_normal_texture(normal),
    )
}

fn render_center_rgb_for_metallic_roughness_texture(pixel: [u8; 4]) -> [u8; 3] {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("metallic-roughness texture loads");
    render_center_rgb_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 1.0, 1.0)
            .with_metallic_roughness_texture(texture),
    )
}

fn render_center_rgb_for_occlusion_texture(pixel: [u8; 4]) -> [u8; 3] {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("occlusion texture loads");
    render_center_rgb_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 0.0, 0.65)
            .with_occlusion_texture(texture),
    )
}

fn render_max_luminance_for_clearcoat_texture(pixel: [u8; 4]) -> u8 {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("clearcoat texture loads");
    render_max_luminance_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62)
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(0.12)
            .with_clearcoat_texture(texture),
    )
}

fn render_max_luminance_for_clearcoat_roughness_texture(pixel: [u8; 4]) -> u8 {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("clearcoat roughness texture loads");
    render_max_luminance_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62)
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(1.0)
            .with_clearcoat_roughness_texture(texture),
    )
}

fn render_max_luminance_for_clearcoat_normal_texture(pixel: [u8; 4]) -> u8 {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("clearcoat normal texture loads");
    render_max_luminance_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62)
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(0.12)
            .with_clearcoat_normal_texture(texture),
    )
}

fn render_center_rgb_for_sheen_color_texture(pixel: [u8; 4]) -> [u8; 3] {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Srgb))
        .expect("sheen color texture loads");
    render_center_rgb_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(104, 96, 92), 0.0, 0.72)
            .with_sheen_color_factor(Color::WHITE)
            .with_sheen_roughness_factor(0.35)
            .with_sheen_color_texture(texture),
    )
}

fn render_max_luminance_for_sheen_roughness_texture(pixel: [u8; 4]) -> u8 {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("sheen roughness texture loads");
    render_max_luminance_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(104, 96, 92), 0.0, 0.72)
            .with_sheen_color_factor(Color::WHITE)
            .with_sheen_roughness_factor(1.0)
            .with_sheen_roughness_texture(texture),
    )
}

fn render_max_luminance_for_anisotropy_texture(pixel: [u8; 4]) -> u8 {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    let assets = Assets::new();
    let texture = pollster::block_on(assets.load_texture(uri, TextureColorSpace::Linear))
        .expect("anisotropy texture loads");
    render_max_luminance_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(150, 150, 150), 1.0, 0.42)
            .with_anisotropy_strength_factor(1.0)
            .with_anisotropy_texture(texture),
    )
}

fn render_center_rgb_for_iridescence_textures(
    iridescence_pixel: [u8; 4],
    thickness_pixel: [u8; 4],
) -> [u8; 3] {
    let iridescence_png = png_rgba8(1, 1, &[iridescence_pixel]);
    let thickness_png = png_rgba8(1, 1, &[thickness_pixel]);
    let iridescence_uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(iridescence_png)
    );
    let thickness_uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(thickness_png)
    );
    let assets = Assets::new();
    let iridescence =
        pollster::block_on(assets.load_texture(iridescence_uri, TextureColorSpace::Linear))
            .expect("iridescence texture loads");
    let thickness =
        pollster::block_on(assets.load_texture(thickness_uri, TextureColorSpace::Linear))
            .expect("iridescence thickness texture loads");
    render_center_rgb_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(150, 150, 150), 0.0, 0.35)
            .with_iridescence_factor(1.0)
            .with_iridescence_ior(1.3)
            .with_iridescence_thickness_range_nm(100.0, 650.0)
            .with_iridescence_texture(iridescence)
            .with_iridescence_thickness_texture(thickness),
    )
}

fn render_center_rgb_for_dispersion_factor(dispersion: f32) -> [u8; 3] {
    let assets = Assets::new();
    render_center_rgb_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(165, 165, 165), 0.0, 0.24)
            .with_dispersion_factor(dispersion),
    )
}

fn render_center_rgb_for_transmission_volume_textures(
    transmission_pixel: [u8; 4],
    thickness_pixel: [u8; 4],
) -> [u8; 3] {
    let assets = Assets::new();
    let transmission_png = png_rgba8(1, 1, &[transmission_pixel]);
    let transmission_uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(transmission_png)
    );
    let thickness_png = png_rgba8(1, 1, &[thickness_pixel]);
    let thickness_uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(thickness_png)
    );
    let transmission =
        pollster::block_on(assets.load_texture(transmission_uri, TextureColorSpace::Linear))
            .expect("transmission texture loads");
    let thickness =
        pollster::block_on(assets.load_texture(thickness_uri, TextureColorSpace::Linear))
            .expect("thickness texture loads");
    render_center_rgb_with_assets(
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 205, 230), 0.0, 0.08)
            .with_transmission_factor(1.0)
            .with_transmission_texture(transmission)
            .with_ior(1.7)
            .with_thickness_factor(2.0)
            .with_thickness_texture(thickness)
            .with_attenuation_distance(1.0)
            .with_attenuation_color(Color::from_linear_rgb(0.08, 0.35, 1.0)),
    )
}

fn dominant_rgb_channel(value: [u8; 3]) -> usize {
    if value[0] >= value[1] && value[0] >= value[2] {
        0
    } else if value[1] >= value[2] {
        1
    } else {
        2
    }
}

fn rgb_sum(value: [u8; 3]) -> u16 {
    u16::from(value[0]) + u16::from(value[1]) + u16::from(value[2])
}

fn rgb_distance_u8(left: [u8; 3], right: [u8; 3]) -> u16 {
    left.into_iter()
        .zip(right)
        .map(|(left, right)| u16::from(left.abs_diff(right)))
        .sum()
}

fn render_center_rgb_with_assets(assets: &Assets, material: MaterialDesc) -> [u8; 3] {
    let frame = render_frame_with_assets(assets, material);
    let center = ((48 / 2) * 48 + (48 / 2)) as usize * 4;
    [frame[center], frame[center + 1], frame[center + 2]]
}

fn render_max_luminance_with_assets(assets: &Assets, material: MaterialDesc) -> u8 {
    render_frame_with_assets(assets, material)
        .chunks_exact(4)
        .map(|rgba| rgba[0].max(rgba[1]).max(rgba[2]))
        .max()
        .unwrap_or(0)
}

fn render_frame_with_assets(assets: &Assets, material: MaterialDesc) -> Vec<u8> {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.75, 0.75, 0.75));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    scene
        .directional_light(DirectionalLight::key_light().with_illuminance_lux(12_000.0))
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(48, 48).expect("renderer builds");
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::NeutralStudio))
            .expect("neutral studio environment loads");
    renderer.set_environment(environment);

    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

fn sample_rgb(frame: &[u8], width: u32, height: u32, x: u32, y: u32) -> [u8; 3] {
    assert!(x < width);
    assert!(y < height);
    let offset = ((y * width + x) as usize) * 4;
    [frame[offset], frame[offset + 1], frame[offset + 2]]
}

fn max_luminance_in_region(rgba: &[u8], width: u32, min_x: u32, max_x: u32) -> u16 {
    rgba.chunks_exact(4)
        .enumerate()
        .filter_map(|(index, pixel)| {
            let x = (index as u32) % width;
            (x >= min_x && x < max_x).then_some(u16::from(pixel[0].max(pixel[1]).max(pixel[2])))
        })
        .max()
        .unwrap_or(0)
}

fn render_environment_preview_center<F>(
    assets: &Assets<F>,
    environment: Option<scena::EnvironmentHandle>,
) -> [u8; 3] {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.05));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_linear_rgb(0.04, 0.04, 0.04), 0.0, 0.7)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("environment preview mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(64, 64).expect("CPU renderer builds");
    if let Some(environment) = environment {
        renderer.set_environment(environment);
    }
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("environment preview prepares");
    renderer
        .render(&scene, camera)
        .expect("environment preview renders");
    sample_rgb(renderer.frame_rgba8(), 64, 64, 32, 32)
}

fn tiny_radiance_hdr_rgbe(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(pixels.len(), (width * height) as usize);
    let mut bytes =
        format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
    for pixel in pixels {
        bytes.extend_from_slice(pixel);
    }
    bytes
}

fn scene_materials<F>(scene: &scena::SceneAsset, assets: &Assets<F>) -> Vec<MaterialDesc> {
    scene
        .nodes()
        .iter()
        .flat_map(|node| node.meshes())
        .filter_map(|mesh| assets.material(mesh.material()))
        .collect()
}

fn scene_texture_descs<F>(
    scene: &scena::SceneAsset,
    assets: &Assets<F>,
) -> Vec<scena::TextureDesc> {
    scene_materials(scene, assets)
        .into_iter()
        .flat_map(|material| {
            [
                material.base_color_texture(),
                material.normal_texture(),
                material.metallic_roughness_texture(),
                material.occlusion_texture(),
                material.emissive_texture(),
                material.clearcoat_texture(),
                material.clearcoat_roughness_texture(),
                material.clearcoat_normal_texture(),
                material.sheen_color_texture(),
                material.sheen_roughness_texture(),
            ]
        })
        .flatten()
        .filter_map(|texture| assets.texture(texture))
        .collect()
}

fn required_extension_gltf(extension: &str) -> String {
    format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["{extension}"],
            "extensionsRequired": ["{extension}"],
            "nodes": [{{ "name": "RequiredExtension" }}]
        }}"#
    )
}

fn missing_texture_gltf() -> &'static [u8] {
    br#"{
        "asset": { "version": "2.0" },
        "materials": [{
            "pbrMetallicRoughness": {
                "baseColorTexture": { "index": 4 }
            }
        }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0 },
                "material": 0
            }]
        }],
        "nodes": [{ "name": "MissingTexture", "mesh": 0 }],
        "buffers": [{ "byteLength": 36, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAA" }],
        "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }],
        "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }]
    }"#
}

fn external_buffer_gltf(buffer_uri: &str) -> String {
    format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "nodes": [{{ "name": "ExternalTriangle", "mesh": 0 }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0 }},
                    "indices": 1
                }}]
            }}],
            "buffers": [{{ "byteLength": 42, "uri": "{buffer_uri}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    )
}

fn external_triangle_buffer() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [-0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn minimal_glb_triangle_scene() -> Vec<u8> {
    let mut bin = external_triangle_buffer();
    let buffer_byte_length = bin.len();
    pad_to_four(&mut bin, 0);

    let json = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "buffers": [{{ "byteLength": {buffer_byte_length} }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ],
            "meshes": [
                {{ "primitives": [{{ "attributes": {{ "POSITION": 0 }}, "indices": 1 }}] }}
            ],
            "nodes": [{{ "name": "EmbeddedGlbTriangle", "mesh": 0 }}]
        }}"#
    );
    let mut json = json.into_bytes();
    pad_to_four(&mut json, b' ');

    let length = 12 + 8 + json.len() + 8 + bin.len();
    let mut glb = Vec::with_capacity(length);
    glb.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(length as u32).to_le_bytes());
    glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
    glb.extend_from_slice(&json);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
    glb.extend_from_slice(&bin);
    glb
}

fn pad_to_four(bytes: &mut Vec<u8>, pad: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(pad);
    }
}

#[derive(Clone)]
struct MemoryFetcher {
    files: BTreeMap<AssetPath, Vec<u8>>,
}

impl MemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: files.into_iter().collect(),
        }
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}

#[derive(Clone)]
struct MutableMemoryFetcher {
    files: Arc<Mutex<BTreeMap<AssetPath, Vec<u8>>>>,
}

impl MutableMemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: Arc::new(Mutex::new(files.into_iter().collect())),
        }
    }

    fn insert(&self, path: AssetPath, bytes: Vec<u8>) {
        self.files
            .lock()
            .expect("test fetcher mutex should not be poisoned")
            .insert(path, bytes);
    }

    fn remove(&self, path: &AssetPath) {
        self.files
            .lock()
            .expect("test fetcher mutex should not be poisoned")
            .remove(path);
    }
}

impl AssetFetcher for MutableMemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.files
                .lock()
                .expect("test fetcher mutex should not be poisoned")
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}

fn textured_triangle_gltf(image_uri: &str) -> String {
    let mut buffer = Vec::new();
    for value in [-0.6_f32, -0.6, 0.0, 0.6, -0.6, 0.0, 0.0, 0.6, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": ["KHR_materials_unlit"],
            "extensionsRequired": ["KHR_materials_unlit"],
            "images": [{{ "uri": "{image_uri}" }}],
            "textures": [{{ "source": 0 }}],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorTexture": {{ "index": 0 }}
                }},
                "extensions": {{ "KHR_materials_unlit": {{}} }}
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 }},
                    "indices": 3,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "TexturedTriangle", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 102, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 72, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 96, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" }},
                {{ "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 3, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    )
}

fn png_rgba8(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(pixels.len(), (width * height) as usize);
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header writes");
        let raw = pixels
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .collect::<Vec<_>>();
        writer.write_image_data(&raw).expect("PNG payload writes");
    }
    bytes
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn basisu_material_gltf() -> &'static [u8] {
    br#"{
        "asset": { "version": "2.0" },
        "extensionsUsed": ["KHR_texture_basisu"],
        "images": [{ "uri": "albedo.ktx2" }],
        "textures": [{
            "extensions": {
                "KHR_texture_basisu": { "source": 0 }
            }
        }],
        "materials": [{
            "pbrMetallicRoughness": {
                "baseColorTexture": { "index": 0 }
            }
        }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0 },
                "indices": 1,
                "material": 0
            }]
        }],
        "nodes": [{ "name": "Root", "mesh": 0 }],
        "buffers": [{ "byteLength": 126, "uri": "data:application/octet-stream;base64,AAAAvwAAAL8AAAAAAAAAPwAAAL8AAAAAAAAAAAAAAD8AAAAAAAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AAAAAAAAAAAAAIA/AACAPwAAAAAAAAAAAACAPwAAAAAAAIA/AAAAAAAAgD8AAAAAAAAAAAAAgD8AAIA/AAABAAIA" }],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 120, "byteLength": 6 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] },
            { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
        ]
    }"#
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
fn basisu_buffer_view_gltf(ktx2: &[u8]) -> Vec<u8> {
    let mut buffer = triangle_position_index_buffer(
        [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        [0, 1, 2],
    );
    let ktx2_offset = buffer.len();
    buffer.extend_from_slice(ktx2);
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["KHR_texture_basisu"],
        "images": [{{ "bufferView": 2, "mimeType": "image/ktx2" }}],
        "textures": [{{
            "extensions": {{ "KHR_texture_basisu": {{ "source": 0 }} }}
        }}],
        "materials": [{{
            "pbrMetallicRoughness": {{ "baseColorTexture": {{ "index": 0 }} }}
        }}],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }},
                "indices": 1,
                "material": 0
            }}]
        }}],
        "nodes": [{{ "name": "Root", "mesh": 0 }}],
        "buffers": [{{ "byteLength": {buffer_len}, "uri": "data:application/octet-stream;base64,{encoded}" }}],
        "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }},
            {{ "buffer": 0, "byteOffset": {ktx2_offset}, "byteLength": {ktx2_len} }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] }},
            {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
        ]
    }}"#,
        buffer_len = ktx2_offset + ktx2.len(),
        ktx2_len = ktx2.len(),
    )
    .into_bytes()
}

#[cfg(not(feature = "ktx2"))]
fn basisu_with_png_fallback_gltf() -> String {
    let red_png = png_rgba8(1, 1, &[[255, 0, 0, 255]]);
    let red_png = base64::engine::general_purpose::STANDARD.encode(red_png);
    let geometry =
        base64::engine::general_purpose::STANDARD.encode(triangle_position_index_buffer(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [0, 1, 2],
        ));
    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["KHR_texture_basisu"],
        "images": [
            {{ "uri": "data:image/png;base64,{red_png}" }},
            {{ "uri": "missing-albedo.ktx2" }}
        ],
        "textures": [{{
            "source": 0,
            "extensions": {{ "KHR_texture_basisu": {{ "source": 1 }} }}
        }}],
        "materials": [{{
            "pbrMetallicRoughness": {{ "baseColorTexture": {{ "index": 0 }} }}
        }}],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }},
                "indices": 1,
                "material": 0
            }}]
        }}],
        "nodes": [{{ "name": "Root", "mesh": 0 }}],
        "buffers": [{{ "byteLength": 42, "uri": "data:application/octet-stream;base64,{geometry}" }}],
        "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] }},
            {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
        ]
    }}"#
    )
}

#[cfg(feature = "inspection")]
fn materialless_triangle_gltf() -> String {
    let geometry =
        base64::engine::general_purpose::STANDARD.encode(triangle_position_index_buffer(
            [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            [0, 1, 2],
        ));
    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }},
                "indices": 1
            }}]
        }}],
        "nodes": [{{ "name": "GeneratedMaterialRoot", "mesh": 0 }}],
        "buffers": [{{ "byteLength": 42, "uri": "data:application/octet-stream;base64,{geometry}" }}],
        "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] }},
            {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
        ]
    }}"#
    )
}

#[cfg(all(feature = "ktx2", not(target_arch = "wasm32")))]
fn tiny_basisu_ktx2_solid_red() -> Vec<u8> {
    use basisu_c_sys::BasisTextureFormat;
    use basisu_c_sys::common;
    use basisu_c_sys::extra::{
        BasisuEncoder, BasisuEncoderParams, SourceImage, SourceImageData, basisu_encoder_init,
    };

    pollster::block_on(basisu_encoder_init());
    let mut encoder = BasisuEncoder::new();
    let pixels = [255_u8, 0, 0, 255].repeat(16);
    encoder
        .set_image(SourceImage {
            data: SourceImageData::Rgba8(&pixels),
            size: wgpu::Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
        })
        .expect("solid-red image is accepted by the Basis Universal encoder");
    encoder
        .compress(BasisuEncoderParams {
            basis_tex_format: BasisTextureFormat::UastcLdr4x4,
            quality_level: 75,
            effort_level: 2,
            flags_and_quality: common::BU_COMP_FLAGS_SRGB
                | common::BU_COMP_FLAGS_KTX2_OUTPUT
                | common::BU_COMP_FLAGS_TEXTURE_TYPE_2D,
            low_level_uastc_rdo_or_dct_quality: 0.0,
        })
        .expect("solid-red texture compresses to a KTX2/Basis Universal payload")
}

fn ext_mesh_gpu_instancing_triangle_gltf() -> String {
    let mut buffer = Vec::with_capacity(60);
    push_vec3_f32(
        &mut buffer,
        [[-0.25, -0.25, 0.0], [0.25, -0.25, 0.0], [0.0, 0.25, 0.0]],
    );
    push_vec3_f32(&mut buffer, [[-0.7, 0.0, 0.0], [0.7, 0.0, 0.0]]);
    let bytes = base64::engine::general_purpose::STANDARD.encode(&buffer);
    let byte_length = buffer.len();

    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["EXT_mesh_gpu_instancing"],
        "extensionsRequired": ["EXT_mesh_gpu_instancing"],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }}
            }}]
        }}],
        "nodes": [{{
            "name": "InstancedTriangle",
            "mesh": 0,
            "extensions": {{
                "EXT_mesh_gpu_instancing": {{
                    "attributes": {{ "TRANSLATION": 1 }}
                }}
            }}
        }}],
        "buffers": [{{ "byteLength": {byte_length}, "uri": "data:application/octet-stream;base64,{bytes}" }}],
        "bufferViews": [
            {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
            {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }}
        ],
        "accessors": [
            {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.25,-0.25,0.0], "max": [0.25,0.25,0.0] }},
            {{ "bufferView": 1, "componentType": 5126, "count": 2, "type": "VEC3", "min": [-1,-1,-1], "max": [1,1,1] }}
        ]
    }}"#
    )
}

fn push_vec3_f32<const N: usize>(buffer: &mut Vec<u8>, values: [[f32; 3]; N]) {
    for vector in values {
        for value in vector {
            buffer.extend_from_slice(&value.to_le_bytes());
        }
    }
}

#[cfg(feature = "meshopt")]
fn meshopt_compressed_triangle_gltf() -> String {
    let positions = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];
    let indices = [0_u32, 1, 2];
    let compressed_positions =
        meshopt::encode_vertex_buffer(&positions).expect("positions meshopt-encode");
    let compressed_indices =
        meshopt::encode_index_buffer(&indices, positions.len()).expect("indices meshopt-encode");
    let mut compressed = compressed_positions.clone();
    compressed.extend_from_slice(&compressed_indices);
    let decoded_placeholder = vec![0_u8; 42];
    let decoded_uri = base64::engine::general_purpose::STANDARD.encode(decoded_placeholder);
    let compressed_uri = base64::engine::general_purpose::STANDARD.encode(compressed);
    let index_offset = compressed_positions.len();
    let index_len = compressed_indices.len();

    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["EXT_meshopt_compression"],
        "extensionsRequired": ["EXT_meshopt_compression"],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }},
                "indices": 1
            }}]
        }}],
        "nodes": [{{ "name": "MeshoptRoot", "mesh": 0 }}],
        "buffers": [
            {{ "byteLength": 42, "uri": "data:application/octet-stream;base64,{decoded_uri}" }},
            {{ "byteLength": {compressed_len}, "uri": "data:application/octet-stream;base64,{compressed_uri}" }}
        ],
        "bufferViews": [
            {{
                "buffer": 0,
                "byteOffset": 0,
                "byteLength": 36,
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
        compressed_len = compressed_positions.len() + compressed_indices.len(),
        position_len = compressed_positions.len(),
    )
}

#[cfg(feature = "meshopt")]
fn meshopt_index_sequence_gltf() -> String {
    let positions = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];
    let indices = [0_u32, 1, 2];
    let compressed_positions =
        meshopt::encode_vertex_buffer(&positions).expect("positions meshopt-encode");
    let compressed_indices = meshopt_encode_index_sequence(&indices, positions.len());
    meshopt_gltf_from_encoded(
        &compressed_positions,
        &compressed_indices,
        "INDICES",
        [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
    )
}

#[cfg(feature = "meshopt")]
fn meshopt_encode_index_sequence(indices: &[u32], vertex_count: usize) -> Vec<u8> {
    let bound =
        unsafe { meshopt::ffi::meshopt_encodeIndexSequenceBound(indices.len(), vertex_count) };
    let mut result = vec![0; bound];
    let size = unsafe {
        meshopt::ffi::meshopt_encodeIndexSequence(
            result.as_mut_ptr(),
            result.len(),
            indices.as_ptr(),
            indices.len(),
        )
    };
    assert!(size > 0, "meshopt index-sequence encoding succeeds");
    result.truncate(size);
    result
}

#[cfg(feature = "meshopt")]
fn meshopt_optional_fallback_gltf(encode_compressed: bool) -> String {
    let positions = [[-0.5_f32, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]];
    let indices = [0_u32, 1, 2];
    let compressed_positions =
        meshopt::encode_vertex_buffer(&positions).expect("positions meshopt-encode");
    let compressed_indices =
        meshopt::encode_index_buffer(&indices, positions.len()).expect("indices meshopt-encode");
    if encode_compressed {
        meshopt_gltf_from_encoded(
            &compressed_positions,
            &compressed_indices,
            "TRIANGLES",
            [[2.0, 2.0, 0.0], [3.0, 2.0, 0.0], [2.0, 3.0, 0.0]],
        )
        .replace(
            r#""extensionsRequired": ["EXT_meshopt_compression"],"#,
            r#""extensionsRequired": [],"#,
        )
    } else {
        unreachable!("feature-enabled optional fallback helper should encode compressed bytes")
    }
}

#[cfg(not(feature = "meshopt"))]
fn meshopt_optional_fallback_gltf(_encode_compressed: bool) -> String {
    let fallback = triangle_position_index_buffer(
        [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [-0.5, 0.5, 0.0]],
        [0, 1, 2],
    );
    let fallback_uri = base64::engine::general_purpose::STANDARD.encode(fallback);
    r#"{
        "asset": { "version": "2.0" },
        "extensionsUsed": ["EXT_meshopt_compression"],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0 },
                "indices": 1
            }]
        }],
        "nodes": [{ "name": "MeshoptFallback", "mesh": 0 }],
        "buffers": [
            { "byteLength": 42, "uri": "data:application/octet-stream;base64,FALLBACK_URI" }
        ],
        "bufferViews": [
            {
                "buffer": 0,
                "byteOffset": 0,
                "byteLength": 36,
                "extensions": {
                    "EXT_meshopt_compression": {
                        "buffer": 99,
                        "byteOffset": 0,
                        "byteLength": 1,
                        "byteStride": 12,
                        "count": 3,
                        "mode": "ATTRIBUTES",
                        "filter": "NONE"
                    }
                }
            },
            { "buffer": 0, "byteOffset": 36, "byteLength": 6 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.5,-0.5,0], "max": [0.5,0.5,0] },
            { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }
        ]
    }"#
    .replace("FALLBACK_URI", &fallback_uri)
}

#[cfg(feature = "meshopt")]
fn meshopt_malformed_triangle_gltf(field: &str, value: &str) -> String {
    let mut gltf = meshopt_compressed_triangle_gltf();
    match field {
        "buffer" => {
            gltf = gltf.replacen(
                r#""EXT_meshopt_compression": {
                        "buffer": 1,"#,
                r#""EXT_meshopt_compression": {
                        "buffer": 99,"#,
                1,
            );
        }
        "byteOffset" => {
            gltf = gltf.replacen(r#""byteOffset": 0,"#, r#""byteOffset": 999,"#, 2);
        }
        "byteLength" => {
            gltf = gltf.replacen(r#""byteLength": 36"#, r#""byteLength": 999"#, 1);
        }
        "byteStride" => {
            gltf = gltf.replacen(r#""byteStride": 12,"#, r#""byteStride": 3,"#, 1);
        }
        "mode" => {
            gltf = gltf.replacen(r#""mode": "ATTRIBUTES","#, r#""mode": "BOGUS","#, 1);
        }
        "filter" => {
            gltf = gltf.replacen(r#""filter": "NONE""#, r#""filter": "BOGUS""#, 1);
        }
        "count" => {
            gltf = gltf.replacen(r#""count": 3,"#, &format!(r#""count": {value},"#), 1);
        }
        _ => unreachable!("unknown malformed meshopt field {field}={value}"),
    }
    gltf
}

#[cfg(feature = "meshopt")]
fn meshopt_gltf_from_encoded(
    compressed_positions: &[u8],
    compressed_indices: &[u8],
    index_mode: &str,
    fallback_positions: [[f32; 3]; 3],
) -> String {
    let mut compressed = compressed_positions.to_vec();
    compressed.extend_from_slice(compressed_indices);
    let decoded_placeholder = triangle_position_index_buffer(fallback_positions, [0, 1, 2]);
    let decoded_uri = base64::engine::general_purpose::STANDARD.encode(decoded_placeholder);
    let compressed_uri = base64::engine::general_purpose::STANDARD.encode(compressed);
    let index_offset = compressed_positions.len();
    let index_len = compressed_indices.len();

    format!(
        r#"{{
        "asset": {{ "version": "2.0" }},
        "extensionsUsed": ["EXT_meshopt_compression"],
        "extensionsRequired": ["EXT_meshopt_compression"],
        "meshes": [{{
            "primitives": [{{
                "attributes": {{ "POSITION": 0 }},
                "indices": 1
            }}]
        }}],
        "nodes": [{{ "name": "MeshoptRoot", "mesh": 0 }}],
        "buffers": [
            {{ "byteLength": 42, "uri": "data:application/octet-stream;base64,{decoded_uri}" }},
            {{ "byteLength": {compressed_len}, "uri": "data:application/octet-stream;base64,{compressed_uri}" }}
        ],
        "bufferViews": [
            {{
                "buffer": 0,
                "byteOffset": 0,
                "byteLength": 36,
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
                        "mode": "{index_mode}",
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
        compressed_len = compressed_positions.len() + compressed_indices.len(),
        position_len = compressed_positions.len(),
    )
}

fn triangle_position_index_buffer(positions: [[f32; 3]; 3], indices: [u16; 3]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(42);
    for vertex in positions {
        for value in vertex {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes
}
