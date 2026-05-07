#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    AlphaMode, Assets, Color, DirectionalLight, EnvironmentHandle, GeometryDesc, MaterialDesc,
    Renderer, Scene, TextureColorSpace, TextureTransform, Transform, Vec3,
};

#[test]
fn m8_headless_visual_artifacts_cover_material_texture_environment_paths() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    for artifact in [
        render_base_color_alpha(),
        render_texture_slots(),
        render_environment_color_management(),
    ] {
        assert!(
            nonblack_pixel_count(&artifact.rgba) > 0,
            "{} should render visible nonblack pixels",
            artifact.name
        );
        write_ppm_artifact(
            &artifact_dir,
            artifact.name,
            artifact.width,
            artifact.height,
            &artifact.rgba,
        );
    }
}

fn render_base_color_alpha() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let material = assets.create_material(
        MaterialDesc::unlit(Color::from_linear_rgba(0.1, 0.45, 1.0, 0.72))
            .with_alpha_mode(AlphaMode::Blend)
            .with_double_sided(true),
    );
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    render_scene_with_assets("m8-base-color-alpha", scene, camera, &assets, None)
}

fn render_texture_slots() -> VisualArtifact {
    let assets = Assets::new();
    let base =
        pollster::block_on(assets.load_texture("textures/m8-base.png", TextureColorSpace::Srgb))
            .expect("base texture records");
    let normal = pollster::block_on(
        assets.load_texture("textures/m8-normal.png", TextureColorSpace::Linear),
    )
    .expect("normal texture records");
    let metallic_roughness = pollster::block_on(assets.load_texture(
        "textures/m8-metallic-roughness.png",
        TextureColorSpace::Linear,
    ))
    .expect("metallic roughness texture records");
    let occlusion = pollster::block_on(
        assets.load_texture("textures/m8-occlusion.png", TextureColorSpace::Linear),
    )
    .expect("occlusion texture records");
    let emissive = pollster::block_on(
        assets.load_texture("textures/m8-emissive.png", TextureColorSpace::Srgb),
    )
    .expect("emissive texture records");
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(180, 220, 255), 0.2, 0.6)
            .with_base_color_texture(base)
            .with_base_color_texture_transform(TextureTransform::new(
                [0.25, 0.5],
                0.0,
                [1.0, 1.0],
                None,
            ))
            .with_normal_texture(normal)
            .with_normal_texture_transform(TextureTransform::new(
                [0.0, 0.0],
                0.0,
                [1.0, 1.0],
                Some(1),
            ))
            .with_metallic_roughness_texture(metallic_roughness)
            .with_occlusion_texture(occlusion)
            .with_emissive_texture(emissive)
            .with_emissive(Color::from_linear_rgb(0.02, 0.04, 0.08))
            .with_emissive_strength(1.5)
            .with_alpha_mode(AlphaMode::Mask { cutoff: 0.35 })
            .with_double_sided(true),
    );
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let artifact = render_scene_with_assets("m8-texture-slots", scene, camera, &assets, None);
    assert_eq!(artifact.stats_textures, 5);
    artifact
}

fn render_environment_color_management() -> VisualArtifact {
    let assets = Assets::new();
    let environment = assets.default_environment();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(220, 220, 220),
        0.0,
        0.55,
    ));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let artifact = render_scene_with_assets(
        "m8-environment-color-management",
        scene,
        camera,
        &assets,
        Some(environment),
    );
    assert_eq!(artifact.stats_environments, 1);
    artifact
}

fn render_scene_with_assets<F>(
    name: &'static str,
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets<F>,
    environment: Option<EnvironmentHandle>,
) -> VisualArtifact {
    let mut renderer = Renderer::headless(56, 56).expect("headless renderer builds");
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
        width: 56,
        height: 56,
        rgba: renderer.frame_rgba8().to_vec(),
        stats_textures: stats.textures,
        stats_environments: stats.environments,
    }
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact can be written");
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\nname = \"{name}\"\nformat = \"ppm\"\nencoding = \"srgb8\"\nwidth = {width}\nheight = {height}\ntolerance = \"nonblack-material-smoke\"\ncolor_management = \"linear-material-to-aces-srgb-output\"\n"
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
}
