use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use base64::Engine;
use scena::{
    AlphaMode, Angle, AssetError, AssetFetcher, AssetLoadControl, AssetLoadProgress, AssetPath,
    Assets, Color, DirectionalLight, GeometryDesc, GltfDecoderPolicy, GltfExtensionStatus,
    MaterialDesc, MaterialKind, NotPreparedReason, PointLight, RenderError, Renderer, RetainPolicy,
    Scene, SpotLight, TextureColorSpace, TextureFilter, TextureSourceFormat, TextureWrap,
    Transform, Vec3,
};

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
        Some(&GltfExtensionStatus::Degraded)
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
    assert_eq!(
        degraded.get("KHR_draco_mesh_compression"),
        Some(&GltfExtensionStatus::Degraded)
    );
    assert_eq!(
        degraded.get("EXT_meshopt_compression"),
        Some(&GltfExtensionStatus::Degraded)
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.help().contains("structured degradation")
                || (cfg!(feature = "ktx2")
                    && diagnostic.extension() == "KHR_texture_basisu"
                    && diagnostic.help().contains("enabled by the ktx2 feature"))
        }),
        "each optional unsupported extension needs an actionable degradation hint and enabled features need explicit support metadata",
    );
    assert_eq!(
        diagnostics
            .iter()
            .find(|diagnostic| diagnostic.extension() == "KHR_texture_basisu")
            .expect("basisu diagnostic exists")
            .decoder_policy(),
        GltfDecoderPolicy::FeatureFlag {
            feature: "ktx2",
            crate_name: "basis-universal",
            license: "Apache-2.0 OR MIT-compatible decoder required"
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
                    "index": 1,
                    "extensions": {
                        "KHR_texture_transform": { "texCoord": 1 }
                    }
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
    assert_eq!(
        material
            .normal_texture_transform()
            .expect("normal transform")
            .tex_coord(),
        Some(1)
    );
    assert_eq!(
        material
            .emissive_texture_transform()
            .expect("emissive transform")
            .scale(),
        [0.5, 0.5]
    );
}

#[test]
fn m8_gltf_data_uri_image_texture_descriptor_is_preserved() {
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
    assert_eq!(texture.path().as_str(), image_uri);
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
            "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }]
        }"#
        .to_vec(),
    )]));

    let error = pollster::block_on(assets.load_scene("memory://missing-texture.gltf"))
        .expect_err("missing texture index must not silently fall back");
    assert!(matches!(
        error,
        AssetError::MissingTexture {
            ref material_slot,
            texture_index: 9,
            ..
        } if material_slot == "baseColorTexture"
    ));
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
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(48, 48).expect("renderer builds");

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
            "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }]
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

#[cfg(feature = "ktx2")]
#[test]
fn m8_ktx2_basisu_feature_loads_compressed_texture_descriptor() {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(
        AssetPath::from("memory://basisu.gltf"),
        basisu_material_gltf().to_vec(),
    )]));

    let scene_asset =
        pollster::block_on(assets.load_scene("memory://basisu.gltf")).expect("glTF loads");
    let mesh = scene_asset.nodes()[0].mesh().expect("mesh exists");
    let material = assets.material(mesh.material()).expect("material exists");
    let texture = assets
        .texture(material.base_color_texture().expect("base texture exists"))
        .expect("texture descriptor exists");

    assert_eq!(texture.path().as_str(), "memory://albedo.ktx2");
    assert_eq!(
        texture.source_format(),
        scena::TextureSourceFormat::Ktx2Basisu
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

    for (path, extension) in [
        (
            "memory://real-world/draco-required.gltf",
            "KHR_draco_mesh_compression",
        ),
        (
            "memory://real-world/meshopt-required.gltf",
            "EXT_meshopt_compression",
        ),
    ] {
        let error = pollster::block_on(assets.load_scene(path))
            .expect_err("required compressed mesh extension must fail explicitly");
        assert!(matches!(
            error,
            AssetError::UnsupportedRequiredExtension {
                extension: ref rejected,
                ..
            } if rejected == extension
        ));
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
    assert!(external.fetched_bytes() > first.fetched_bytes());
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

fn render_center_rgb_with_assets(assets: &Assets, material: MaterialDesc) -> [u8; 3] {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.75, 0.75, 0.75));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless(48, 48).expect("renderer builds");

    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");

    let center = ((48 / 2) * 48 + (48 / 2)) as usize * 4;
    let frame = renderer.frame_rgba8();
    [frame[center], frame[center + 1], frame[center + 2]]
}

fn sample_rgb(frame: &[u8], width: u32, height: u32, x: u32, y: u32) -> [u8; 3] {
    assert!(x < width);
    assert!(y < height);
    let offset = ((y * width + x) as usize) * 4;
    [frame[offset], frame[offset + 1], frame[offset + 2]]
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
        "accessors": [{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }]
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
