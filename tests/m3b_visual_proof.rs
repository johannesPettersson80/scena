#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{Assets, PerspectiveCamera, Renderer, Scene, Transform, Vec3};

#[test]
fn m3b_headless_visual_artifacts_cover_khronos_skin_morph_and_animation() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    for artifact in [
        render_khronos_sample(
            "m3b-khronos-simple-skin",
            "tests/assets/gltf/khronos/SimpleSkin/SimpleSkin.gltf",
            None,
        ),
        render_khronos_sample(
            "m3b-khronos-simple-morph",
            "tests/assets/gltf/khronos/MorphCube/AnimatedMorphCube.gltf",
            Some("Square"),
        ),
        render_khronos_sample(
            "m3b-khronos-rigged-simple",
            "tests/assets/gltf/khronos/RiggedSimple/RiggedSimple.gltf",
            None,
        ),
    ] {
        assert!(
            nonblack_pixel_count(&artifact.rgba) > 0,
            "{} should have visible output",
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

fn render_khronos_sample(
    name: &'static str,
    path: &'static str,
    clip_name: Option<&str>,
) -> VisualArtifact {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(path)).expect("Khronos sample scene loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("Khronos sample instantiates");
    if let Some(clip_name) = clip_name {
        let mixer = scene
            .create_animation_mixer(&import, clip_name)
            .expect("sample animation mixer creates");
        scene
            .seek_animation(mixer, scene_asset.clips()[0].duration_seconds() * 0.5)
            .expect("sample animation seeks");
    }
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform {
                translation: Vec3::new(0.0, 0.0, 3.0),
                ..Transform::default()
            },
        )
        .expect("camera inserts");
    if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).expect("camera frames sample");
    }
    let mut renderer = Renderer::headless(48, 48).expect("renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("sample prepares");
    renderer.render(&scene, camera).expect("sample renders");
    VisualArtifact {
        name,
        width: 48,
        height: 48,
        rgba: renderer.frame_rgba8().to_vec(),
    }
}

struct VisualArtifact {
    name: &'static str,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn artifact_dir() -> PathBuf {
    PathBuf::from("target/gate-artifacts/m3b-visual")
}

fn write_ppm_artifact(path: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[0..3]);
    }
    fs::write(path.join(format!("{name}.ppm")), ppm).expect("visual artifact can be written");
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}
