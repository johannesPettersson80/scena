use std::collections::BTreeMap;
use std::future::{Ready, ready};

use scena::{
    AlphaMode, AssetError, AssetFetcher, AssetLoadControl, AssetLoadProgress, AssetPath, Assets,
    GltfDecoderPolicy, GltfExtensionStatus, MaterialDesc, NotPreparedReason, RenderError, Renderer,
    RetainPolicy, Scene, TextureColorSpace, TextureFilter, TextureWrap, Transform,
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
