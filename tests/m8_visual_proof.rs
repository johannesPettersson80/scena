#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::fs;
use std::future::{Ready, ready};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use base64::Engine;
use scena::{
    AlphaMode, AssetError, AssetFetcher, AssetPath, Assets, Color, DirectionalLight,
    EnvironmentHandle, GeometryDesc, GeometryTopology, MaterialDesc, PerspectiveCamera, Renderer,
    Scene, TextureColorSpace, Transform, Vec3,
};

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;

fn ndc_fixture_camera_transform() -> Transform {
    Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES))
}

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
        render_clearcoat_material_feature(),
        render_sheen_material_feature(),
        render_anisotropy_material_feature(),
        render_iridescence_material_feature(),
        render_dispersion_material_feature(),
        render_transmission_volume_material_feature(),
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
        "m8-clearcoat-material-feature",
        "m8-sheen-material-feature",
        "m8-anisotropy-material-feature",
        "m8-iridescence-material-feature",
        "m8-dispersion-material-feature",
        "m8-transmission-volume-material-feature",
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
        if artifact.name == "m8-clearcoat-material-feature" {
            let matte_highlight =
                max_luminance_in_region(&artifact.rgba, artifact.width, 0, artifact.width / 2);
            let clearcoat_highlight = max_luminance_in_region(
                &artifact.rgba,
                artifact.width,
                artifact.width / 2,
                artifact.width,
            );
            assert!(
                clearcoat_highlight > matte_highlight + 1,
                "clearcoat visual proof must brighten the right-side specular response; \
                 matte={matte_highlight:?} clearcoat={clearcoat_highlight:?}"
            );
        }
        if artifact.name == "m8-sheen-material-feature" {
            let black_sheen =
                max_luminance_in_region(&artifact.rgba, artifact.width, 0, artifact.width / 2);
            let red_sheen = max_luminance_in_region(
                &artifact.rgba,
                artifact.width,
                artifact.width / 2,
                artifact.width,
            );
            assert!(
                red_sheen > black_sheen + 2,
                "sheen visual proof must brighten the right-side texture/factor response; \
                 black={black_sheen:?} red={red_sheen:?}"
            );
        }
        if artifact.name == "m8-anisotropy-material-feature" {
            let off =
                max_luminance_in_region(&artifact.rgba, artifact.width, 0, artifact.width / 2);
            let on = max_luminance_in_region(
                &artifact.rgba,
                artifact.width,
                artifact.width / 2,
                artifact.width,
            );
            assert!(
                on > off,
                "anisotropy visual proof must brighten the right-side direction/strength response; \
                 off={off:?} on={on:?}"
            );
        }
        if artifact.name == "m8-iridescence-material-feature" {
            let off = max_rgb_in_region(&artifact.rgba, artifact.width, 0, artifact.width / 2);
            let on = max_rgb_in_region(
                &artifact.rgba,
                artifact.width,
                artifact.width / 2,
                artifact.width,
            );
            assert!(
                on[2] > off[2] && on[2] >= on[0],
                "iridescence visual proof must add a thickness-driven colored lobe; \
                 off={off:?} on={on:?}"
            );
        }
        if artifact.name == "m8-dispersion-material-feature" {
            let off = max_rgb_in_region(&artifact.rgba, artifact.width, 0, artifact.width / 2);
            let on = max_rgb_in_region(
                &artifact.rgba,
                artifact.width,
                artifact.width / 2,
                artifact.width,
            );
            assert!(
                on[0] > off[0] || on[2] > off[2],
                "dispersion visual proof must separate red/blue channel response; \
                 off={off:?} on={on:?}"
            );
        }
        if artifact.name == "m8-transmission-volume-material-feature" {
            let off = max_rgb_in_region(&artifact.rgba, artifact.width, 0, artifact.width / 2);
            let on = max_rgb_in_region(
                &artifact.rgba,
                artifact.width,
                artifact.width / 2,
                artifact.width,
            );
            assert!(
                on[2] > off[2] + 1 && on[2] > on[0] && on[2] > on[1],
                "transmission/volume visual proof must tint the transmitted glass path; \
                 off={off:?} on={on:?}"
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
        render_lighting_sensitivity_frame(Color::from_linear_rgb(1.0, 0.0, 0.0)),
        render_lighting_sensitivity_frame(Color::from_linear_rgb(0.0, 1.0, 0.0)),
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
}

impl SensitivityOptions {
    fn unlit(material_color: Color) -> Self {
        Self {
            camera_x: 0.0,
            mesh_x: 0.0,
            material_color,
            texture_pixel: None,
        }
    }
}

fn render_sensitivity_box(options: SensitivityOptions) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let mut material = MaterialDesc::unlit(options.material_color);
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

fn render_lighting_sensitivity_frame(light_color: Color) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(light_color)
                .with_illuminance_lux(12_000.0),
        )
        .add()
        .expect("directional light inserts");
    scene.mesh(geometry, material).add().expect("mesh inserts");
    render_sensitivity_scene(scene, camera, &assets)
}

fn fullscreen_triangle_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-1.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(3.0, -1.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-1.0, 3.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2],
    )
    .expect("fullscreen test geometry is valid")
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

fn render_clearcoat_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let matte = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62);
    let clearcoat = matte
        .clone()
        .with_clearcoat_factor(0.9)
        .with_clearcoat_roughness_factor(0.12);
    let mut left = render_material_box(
        "m8-clearcoat-left",
        &assets,
        matte,
        None,
        true,
        "clearcoat-before",
    );
    let right = render_material_box(
        "m8-clearcoat-right",
        &assets,
        clearcoat,
        None,
        true,
        "clearcoat-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-clearcoat-material-feature";
    left.proof_class = "clearcoat-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-clearcoat-material-feature:clearcoat-before-after",
    ));
    left
}

fn render_sheen_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let black = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Srgb);
    let red = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Srgb);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(104, 96, 92), 0.0, 0.72)
        .with_sheen_color_factor(Color::WHITE)
        .with_sheen_roughness_factor(0.35);
    let no_sheen = base.clone().with_sheen_color_texture(black);
    let red_sheen = base.with_sheen_color_texture(red);
    let mut left = render_material_box(
        "m8-sheen-left",
        &assets,
        no_sheen,
        None,
        true,
        "sheen-before",
    );
    let right = render_material_box(
        "m8-sheen-right",
        &assets,
        red_sheen,
        None,
        true,
        "sheen-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-sheen-material-feature";
    left.proof_class = "sheen-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-sheen-material-feature:sheen-before-after",
    ));
    left
}

fn render_anisotropy_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let off_texture = load_pixel_texture(&assets, [255, 128, 0, 255], TextureColorSpace::Linear);
    let on_texture = load_pixel_texture(&assets, [255, 128, 255, 255], TextureColorSpace::Linear);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(150, 150, 150), 1.0, 0.42)
        .with_anisotropy_strength_factor(1.0);
    let off = base.clone().with_anisotropy_texture(off_texture);
    let on = base.with_anisotropy_texture(on_texture);
    let mut left = render_material_box(
        "m8-anisotropy-left",
        &assets,
        off,
        None,
        true,
        "anisotropy-before",
    );
    let right = render_material_box(
        "m8-anisotropy-right",
        &assets,
        on,
        None,
        true,
        "anisotropy-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-anisotropy-material-feature";
    left.proof_class = "anisotropy-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-anisotropy-material-feature:anisotropy-before-after",
    ));
    left
}

fn render_iridescence_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let off_texture = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Linear);
    let on_texture = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Linear);
    let thickness_texture =
        load_pixel_texture(&assets, [0, 255, 0, 255], TextureColorSpace::Linear);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(180, 180, 180), 1.0, 0.18)
        .with_iridescence_factor(1.0)
        .with_iridescence_ior(1.45)
        .with_iridescence_thickness_range_nm(120.0, 520.0)
        .with_iridescence_thickness_texture(thickness_texture);
    let off = base.clone().with_iridescence_texture(off_texture);
    let on = base.with_iridescence_texture(on_texture);
    let mut left = render_material_box(
        "m8-iridescence-left",
        &assets,
        off,
        None,
        true,
        "iridescence-before",
    );
    let right = render_material_box(
        "m8-iridescence-right",
        &assets,
        on,
        None,
        true,
        "iridescence-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-iridescence-material-feature";
    left.proof_class = "iridescence-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-iridescence-material-feature:iridescence-before-after",
    ));
    left
}

fn render_dispersion_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let off = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(165, 165, 165), 0.0, 0.24)
        .with_dispersion_factor(0.0);
    let on = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(165, 165, 165), 0.0, 0.24)
        .with_dispersion_factor(1.0);
    let mut left = render_material_box(
        "m8-dispersion-left",
        &assets,
        off,
        None,
        true,
        "dispersion-before",
    );
    let right = render_material_box(
        "m8-dispersion-right",
        &assets,
        on,
        None,
        true,
        "dispersion-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-dispersion-material-feature";
    left.proof_class = "dispersion-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-dispersion-material-feature:dispersion-before-after",
    ));
    left
}

fn render_transmission_volume_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let blocked_texture = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Linear);
    let transmission_texture =
        load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Linear);
    let thickness_texture =
        load_pixel_texture(&assets, [0, 255, 0, 255], TextureColorSpace::Linear);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 205, 230), 0.0, 0.08)
        .with_transmission_factor(1.0)
        .with_ior(1.7)
        .with_thickness_factor(2.0)
        .with_thickness_texture(thickness_texture)
        .with_attenuation_distance(1.0)
        .with_attenuation_color(Color::from_linear_rgb(0.08, 0.35, 1.0));
    let off = base.clone().with_transmission_texture(blocked_texture);
    let on = base.with_transmission_texture(transmission_texture);
    let mut left = render_material_box(
        "m8-transmission-volume-left",
        &assets,
        off,
        None,
        true,
        "transmission-volume-before",
    );
    let right = render_material_box(
        "m8-transmission-volume-right",
        &assets,
        on,
        None,
        true,
        "transmission-volume-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-transmission-volume-material-feature";
    left.proof_class = "transmission-volume-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-transmission-volume-material-feature:transmission-volume-before-after",
    ));
    left
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
            .directional_light(DirectionalLight::key_light().with_illuminance_lux(12_000.0))
            .add()
            .expect("light inserts");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    let environment = environment.or_else(|| add_light.then(|| assets.default_environment()));
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

fn max_luminance_in_region(rgba: &[u8], width: u32, min_x: u32, max_x: u32) -> u16 {
    let height = rgba.len() as u32 / width / 4;
    let mut max_luminance = 0;
    for y in 0..height {
        for x in min_x..max_x {
            let pixel = pixel_at(rgba, width, x, y);
            max_luminance =
                max_luminance.max(u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]));
        }
    }
    max_luminance
}

fn max_rgb_in_region(rgba: &[u8], width: u32, min_x: u32, max_x: u32) -> [u16; 3] {
    let height = rgba.len() as u32 / width / 4;
    let mut max_rgb = [0, 0, 0];
    for y in 0..height {
        for x in min_x..max_x {
            let pixel = pixel_at(rgba, width, x, y);
            max_rgb[0] = max_rgb[0].max(u16::from(pixel[0]));
            max_rgb[1] = max_rgb[1].max(u16::from(pixel[1]));
            max_rgb[2] = max_rgb[2].max(u16::from(pixel[2]));
        }
    }
    max_rgb
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
