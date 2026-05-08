#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::fs;
use std::future::{Ready, ready};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use base64::Engine;
use scena::{
    AlphaMode, AssetError, AssetFetcher, AssetPath, Assets, Color, DirectionalLight,
    EnvironmentHandle, GeometryDesc, MaterialDesc, Renderer, Scene, TextureColorSpace, Transform,
    Vec3,
};

#[test]
fn m8_headless_visual_artifacts_cover_material_texture_environment_paths() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    let artifacts = [
        render_unlit_textured_asset(),
        render_metallic_roughness_asset(),
        render_normal_mapped_asset(),
        render_emissive_asset(),
        render_alpha_mask(),
        render_base_color_alpha(),
        render_texture_slots(),
        render_environment_color_management(),
    ];
    let expected_artifacts = [
        "m8-unlit-textured-asset",
        "m8-metallic-roughness-asset",
        "m8-normal-mapped-asset",
        "m8-emissive-asset",
        "m8-alpha-mask",
        "m8-alpha-blend",
        "m8-texture-slots",
        "m8-environment-color-management",
    ];
    for expected in expected_artifacts {
        assert!(
            artifacts.iter().any(|artifact| artifact.name == expected),
            "missing M8 visual material proof artifact {expected}"
        );
    }

    for artifact in artifacts {
        assert!(
            nonblack_pixel_count(&artifact.rgba) > 0,
            "{} should render visible nonblack pixels",
            artifact.name
        );
        assert_eq!(
            (artifact.width, artifact.height),
            (256, 256),
            "{} must be a 256x256 visual proof artifact",
            artifact.name
        );
        if artifact.name == "m8-texture-slots" {
            let center = artifact.center_pixel();
            assert!(
                center[0] > 150 && center[1] < 80 && center[2] < 80,
                "m8-texture-slots must prove decoded texture pixels affect output, got {center:?}"
            );
        }
        write_ppm_artifact(&artifact_dir, &artifact);
        let metadata = fs::read_to_string(artifact_dir.join(format!("{}.toml", artifact.name)))
            .expect("visual artifact metadata is readable");
        for key in [
            "backend =",
            "adapter =",
            "renderer_settings =",
            "source_hash =",
            "tolerance =",
            "color_management =",
        ] {
            assert!(
                metadata.contains(key),
                "{} metadata must include {key}",
                artifact.name
            );
        }
    }
}

#[test]
fn m8_visual_reference_sensitivity_covers_camera_transform_depth_material_texture_and_lighting() {
    assert_visual_change(
        "camera",
        render_sensitivity_box(SensitivityOptions {
            camera_x: 0.0,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
        render_sensitivity_box(SensitivityOptions {
            camera_x: 0.45,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
    );
    assert_visual_change(
        "transform",
        render_sensitivity_box(SensitivityOptions {
            mesh_x: 0.0,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
        render_sensitivity_box(SensitivityOptions {
            mesh_x: 0.35,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
    );
    assert_visual_change(
        "material",
        render_sensitivity_box(SensitivityOptions::unlit(Color::from_srgb_u8(220, 32, 24))),
        render_sensitivity_box(SensitivityOptions::unlit(Color::from_srgb_u8(24, 190, 72))),
    );
    assert_visual_change(
        "texture",
        render_sensitivity_box(SensitivityOptions {
            texture_pixel: Some([240, 32, 24, 255]),
            ..SensitivityOptions::unlit(Color::WHITE)
        }),
        render_sensitivity_box(SensitivityOptions {
            texture_pixel: Some([24, 72, 240, 255]),
            ..SensitivityOptions::unlit(Color::WHITE)
        }),
    );
    assert_visual_change(
        "lighting",
        render_sensitivity_box(SensitivityOptions::lit(Color::from_srgb_u8(255, 0, 0))),
        render_sensitivity_box(SensitivityOptions::lit(Color::from_srgb_u8(0, 255, 0))),
    );

    let depth = render_depth_sensitivity_scene();
    let center = pixel_at(&depth, 64, 32, 32);
    assert!(
        center[0] > center[2],
        "depth-sensitive visual fixture must keep the nearer red surface visible when the \
         farther blue surface is submitted later, got {center:?}"
    );
}

fn render_unlit_textured_asset() -> VisualArtifact {
    let assets = Assets::new();
    let base = load_pixel_texture(&assets, [240, 32, 24, 255], TextureColorSpace::Srgb);
    render_material_box(
        "m8-unlit-textured-asset",
        &assets,
        MaterialDesc::unlit(Color::WHITE).with_base_color_texture(base),
        None,
        false,
        "unlit-textured-cpu-headless-256",
    )
}

#[derive(Clone, Copy)]
struct SensitivityOptions {
    camera_x: f32,
    mesh_x: f32,
    material_color: Color,
    texture_pixel: Option<[u8; 4]>,
    light_color: Option<Color>,
}

impl SensitivityOptions {
    fn unlit(material_color: Color) -> Self {
        Self {
            camera_x: 0.0,
            mesh_x: 0.0,
            material_color,
            texture_pixel: None,
            light_color: None,
        }
    }

    fn lit(light_color: Color) -> Self {
        Self {
            camera_x: 0.0,
            mesh_x: 0.0,
            material_color: Color::WHITE,
            texture_pixel: None,
            light_color: Some(light_color),
        }
    }
}

fn render_sensitivity_box(options: SensitivityOptions) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let mut material = if options.light_color.is_some() {
        MaterialDesc::pbr_metallic_roughness(options.material_color, 0.0, 0.75)
    } else {
        MaterialDesc::unlit(options.material_color)
    };
    if let Some(pixel) = options.texture_pixel {
        material = material.with_base_color_texture(load_pixel_texture(
            &assets,
            pixel,
            TextureColorSpace::Srgb,
        ));
    }
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(options.mesh_x, 0.0, 0.0)))
        .add()
        .expect("sensitivity mesh inserts");
    if let Some(light_color) = options.light_color {
        scene
            .directional_light(
                DirectionalLight::default()
                    .with_color(light_color)
                    .with_illuminance_lux(10_000.0),
            )
            .add()
            .expect("sensitivity light inserts");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    let camera_node = scene.camera_node(camera).expect("camera node exists");
    scene
        .set_transform(
            camera_node,
            Transform::at(Vec3::new(options.camera_x, 0.0, 2.0)),
        )
        .expect("camera moves");
    scene
        .look_at_point(camera, Vec3::ZERO)
        .expect("camera looks at origin");
    render_sensitivity_scene(scene, camera, &assets)
}

fn render_depth_sensitivity_scene() -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.7, 0.08));
    let red = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(230, 16, 16)));
    let blue = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(16, 48, 230)));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, red)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.08)))
        .add()
        .expect("near red mesh inserts");
    scene
        .mesh(geometry, blue)
        .transform(Transform::at(Vec3::new(0.0, 0.0, -0.08)))
        .add()
        .expect("far blue mesh inserts after near red");
    let camera = scene.add_default_camera().expect("camera inserts");
    render_sensitivity_scene(scene, camera, &assets)
}

fn render_sensitivity_scene<F>(
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets<F>,
) -> Vec<u8> {
    let mut renderer = Renderer::headless(64, 64).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("sensitivity scene prepares");
    renderer
        .render(&scene, camera)
        .expect("sensitivity scene renders");
    renderer.frame_rgba8().to_vec()
}

fn assert_visual_change(label: &str, left: Vec<u8>, right: Vec<u8>) {
    assert_ne!(
        fnv1a64_hex(&left),
        fnv1a64_hex(&right),
        "{label} visual sensitivity must change the rendered frame hash"
    );
}

fn render_metallic_roughness_asset() -> VisualArtifact {
    let assets = Assets::new();
    let metallic_roughness =
        load_pixel_texture(&assets, [0, 32, 255, 255], TextureColorSpace::Linear);
    render_material_box(
        "m8-metallic-roughness-asset",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 1.0, 1.0)
            .with_metallic_roughness_texture(metallic_roughness),
        None,
        true,
        "metallic-roughness-cpu-headless-256",
    )
}

fn render_normal_mapped_asset() -> VisualArtifact {
    let assets = Assets::new();
    let normal = load_pixel_texture(&assets, [128, 128, 255, 255], TextureColorSpace::Linear);
    render_material_box(
        "m8-normal-mapped-asset",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 0.0, 0.75)
            .with_normal_texture(normal),
        None,
        true,
        "normal-mapped-cpu-headless-256",
    )
}

fn render_emissive_asset() -> VisualArtifact {
    let assets = Assets::new();
    let emissive = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Srgb);
    render_material_box(
        "m8-emissive-asset",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(20, 20, 20), 0.0, 0.75)
            .with_emissive(Color::WHITE)
            .with_emissive_texture(emissive),
        None,
        false,
        "emissive-textured-cpu-headless-256",
    )
}

fn render_alpha_mask() -> VisualArtifact {
    let assets = Assets::new();
    render_material_box(
        "m8-alpha-mask",
        &assets,
        MaterialDesc::unlit(Color::from_linear_rgba(0.1, 0.85, 0.2, 0.85))
            .with_alpha_mode(AlphaMode::Mask { cutoff: 0.5 }),
        None,
        false,
        "alpha-mask-cpu-headless-256",
    )
}

fn render_base_color_alpha() -> VisualArtifact {
    let assets = Assets::new();
    render_material_box(
        "m8-alpha-blend",
        &assets,
        MaterialDesc::unlit(Color::from_linear_rgba(0.1, 0.45, 1.0, 0.72))
            .with_alpha_mode(AlphaMode::Blend)
            .with_double_sided(true),
        None,
        false,
        "alpha-blend-cpu-headless-256",
    )
}

fn render_texture_slots() -> VisualArtifact {
    let red_png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let red_png = base64::engine::general_purpose::STANDARD
        .decode(red_png_base64)
        .expect("fixture PNG base64 is valid");
    let gltf = textured_material_gltf();
    let source_hash = fnv1a64_hex(gltf.as_bytes());
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://m8-visual-textures/scene.gltf"),
            gltf.into_bytes(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/base.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/normal.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/metallic_roughness.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/occlusion.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/emissive.png"),
            red_png,
        ),
    ]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://m8-visual-textures/scene.gltf"))
            .expect("textured visual glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("textured visual glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut artifact = render_scene_with_assets("m8-texture-slots", scene, camera, &assets, None);
    artifact.proof_class = "decoded-texture-pixels-256";
    artifact.source = "memory://m8-visual-textures/scene.gltf".to_string();
    artifact.source_hash = Some(source_hash);
    assert_eq!(artifact.stats_textures, 5);
    artifact
}

fn render_environment_color_management() -> VisualArtifact {
    let assets = Assets::new();
    let environment = assets.default_environment();
    let artifact = render_material_box(
        "m8-environment-color-management",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(220, 220, 220), 0.0, 0.55),
        Some(environment),
        true,
        "environment-handle-color-management-cpu-headless-256",
    );
    assert_eq!(artifact.stats_environments, 1);
    artifact
}

fn render_material_box<F>(
    name: &'static str,
    assets: &Assets<F>,
    material: MaterialDesc,
    environment: Option<EnvironmentHandle>,
    add_light: bool,
    proof_class: &'static str,
) -> VisualArtifact {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    if add_light {
        scene
            .directional_light(DirectionalLight::default())
            .add()
            .expect("light inserts");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut artifact = render_scene_with_assets(name, scene, camera, assets, environment);
    artifact.proof_class = proof_class;
    artifact.source_hash = Some(fnv1a64_hex(
        format!("generated-rust-scene:{name}:{proof_class}").as_bytes(),
    ));
    artifact
}

fn render_scene_with_assets<F>(
    name: &'static str,
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets<F>,
    environment: Option<EnvironmentHandle>,
) -> VisualArtifact {
    let (width, height) = (256, 256);
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    if let Some(environment) = environment {
        renderer.set_environment(environment);
    }
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("asset scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let stats = renderer.stats();
    VisualArtifact {
        name,
        width,
        height,
        rgba: renderer.frame_rgba8().to_vec(),
        stats_textures: stats.textures,
        stats_environments: stats.environments,
        proof_class: "headless-material-smoke",
        source: "generated-rust-scene".to_string(),
        source_hash: None,
    }
}

fn load_pixel_texture<F: AssetFetcher>(
    assets: &Assets<F>,
    pixel: [u8; 4],
    color_space: TextureColorSpace,
) -> scena::TextureHandle {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    pollster::block_on(assets.load_texture(uri, color_space)).expect("pixel texture loads")
}

fn textured_material_gltf() -> String {
    let mut buffer = Vec::new();
    for value in [-0.65_f32, -0.65, 0.0, 0.65, -0.65, 0.0, 0.0, 0.65, 0.0] {
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
            "extensionsUsed": [
                "KHR_materials_unlit",
                "KHR_texture_transform",
                "KHR_materials_emissive_strength"
            ],
            "extensionsRequired": [
                "KHR_materials_unlit",
                "KHR_texture_transform",
                "KHR_materials_emissive_strength"
            ],
            "images": [
                {{ "uri": "base.png" }},
                {{ "uri": "normal.png" }},
                {{ "uri": "metallic_roughness.png" }},
                {{ "uri": "occlusion.png" }},
                {{ "uri": "emissive.png" }}
            ],
            "textures": [
                {{ "source": 0, "sampler": 0 }},
                {{ "source": 1, "sampler": 0 }},
                {{ "source": 2, "sampler": 0 }},
                {{ "source": 3, "sampler": 0 }},
                {{ "source": 4, "sampler": 0 }}
            ],
            "samplers": [
                {{ "magFilter": 9728, "minFilter": 9728, "wrapS": 10497, "wrapT": 10497 }}
            ],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                    "baseColorTexture": {{
                        "index": 0,
                        "extensions": {{
                            "KHR_texture_transform": {{ "offset": [0.0, 0.0], "scale": [1.0, 1.0] }}
                        }}
                    }},
                    "metallicRoughnessTexture": {{ "index": 2 }},
                    "metallicFactor": 0.2,
                    "roughnessFactor": 0.6
                }},
                "normalTexture": {{ "index": 1 }},
                "occlusionTexture": {{ "index": 3 }},
                "emissiveTexture": {{ "index": 4 }},
                "emissiveFactor": [0.0, 0.0, 0.0],
                "extensions": {{
                    "KHR_materials_unlit": {{}},
                    "KHR_materials_emissive_strength": {{ "emissiveStrength": 1.0 }}
                }},
                "alphaMode": "OPAQUE",
                "doubleSided": true
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "TexturedVisualProof", "mesh": 0 }}],
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
    )
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn pixel_at(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    rgba[offset..offset + 4]
        .try_into()
        .expect("pixel slice has four channels")
}

fn write_ppm_artifact(dir: &Path, artifact: &VisualArtifact) {
    let mut ppm = format!("P6\n{} {}\n255\n", artifact.width, artifact.height).into_bytes();
    for pixel in artifact.rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{}.ppm", artifact.name)), ppm)
        .expect("PPM artifact can be written");
    let source_hash = artifact
        .source_hash
        .as_deref()
        .unwrap_or("generated-scene-no-source-bytes");
    fs::write(
        dir.join(format!("{}.toml", artifact.name)),
        format!(
            "[artifact]\nname = \"{}\"\nformat = \"ppm\"\nencoding = \"srgb8\"\nwidth = {}\nheight = {}\nbackend = \"Headless\"\nadapter = \"cpu-headless-no-gpu-adapter\"\nrenderer_settings = \"Renderer::headless {}x{} default render mode\"\nproof_class = \"{}\"\nsource = \"{}\"\nsource_hash = \"{}\"\ntolerance = \"material-visible-output-smoke\"\ncolor_management = \"linear-material-to-aces-srgb-output\"\n",
            artifact.name,
            artifact.width,
            artifact.height,
            artifact.width,
            artifact.height,
            artifact.proof_class,
            artifact.source,
            source_hash,
        ),
    )
    .expect("artifact metadata can be written");
}

fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m8-visual")
}

struct VisualArtifact {
    name: &'static str,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    stats_textures: u64,
    stats_environments: u64,
    proof_class: &'static str,
    source: String,
    source_hash: Option<String>,
}

impl VisualArtifact {
    fn center_pixel(&self) -> [u8; 4] {
        let center = ((self.height / 2) * self.width + (self.width / 2)) as usize * 4;
        [
            self.rgba[center],
            self.rgba[center + 1],
            self.rgba[center + 2],
            self.rgba[center + 3],
        ]
    }
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
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

#[derive(Debug, Clone)]
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
