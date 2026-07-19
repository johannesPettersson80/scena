#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{AnimationClip, Assets, PerspectiveCamera, Renderer, Scene, Transform, Vec3};

#[path = "support/q03_visual_metrics.rs"]
mod q03_visual_metrics;
use q03_visual_metrics::{difference_metrics, foreground_metrics};

#[test]
fn m3b_headless_visual_artifacts_cover_khronos_skin_morph_and_animation() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    let skin = render_khronos_pose_pair(
        "skin",
        "m3b-khronos-simple-skin",
        "tests/assets/gltf/khronos/SimpleSkin/SimpleSkin.gltf",
    );
    let morph = render_khronos_pose_pair(
        "morph",
        "m3b-khronos-simple-morph",
        "tests/assets/gltf/khronos/MorphCube/AnimatedMorphCube.gltf",
    );
    let animation = render_khronos_pose_pair(
        "animation",
        "m3b-khronos-rigged-simple",
        "tests/assets/gltf/khronos/RiggedSimple/RiggedSimple.gltf",
    );

    let errors = evaluate_m3b_pose_truth(&morph, &skin, &animation);
    assert!(
        errors.is_empty(),
        "M3B pose truth failed {errors:?}; fixtures={:?}; skin base={:?} sampled={:?} delta={:?}",
        [morph.fixture_id, skin.fixture_id, animation.fixture_id],
        foreground_metrics(&skin.base.rgba, skin.base.width, skin.base.height),
        foreground_metrics(&skin.sampled.rgba, skin.sampled.width, skin.sampled.height,),
        difference_metrics(
            &skin.base.rgba,
            &skin.sampled.rgba,
            skin.base.width,
            skin.base.height,
            2,
        ),
    );
    let frozen_morph = PosePair {
        fixture_id: morph.fixture_id,
        base: morph.base.clone(),
        sampled: morph.base.clone(),
    };
    assert!(
        nonblack_pixel_count(&frozen_morph.sampled.rgba) > 0,
        "the old nonblack oracle would accept a frozen morph"
    );
    assert!(
        evaluate_m3b_pose_truth(&frozen_morph, &skin, &animation).contains(&"morph_pose_change"),
        "feature-specific truth must reject a frozen morph"
    );
    let collapsed_skin = PosePair {
        fixture_id: skin.fixture_id,
        base: skin.base.clone(),
        sampled: skin.base.clone(),
    };
    assert!(
        nonblack_pixel_count(&collapsed_skin.sampled.rgba) > 0,
        "the old nonblack oracle would accept collapsed skin deformation"
    );
    assert!(
        evaluate_m3b_pose_truth(&morph, &collapsed_skin, &animation).contains(&"skin_pose_change"),
        "feature-specific truth must reject collapsed skin deformation"
    );

    for artifact in [
        skin.base,
        skin.sampled,
        morph.base,
        morph.sampled,
        animation.base,
        animation.sampled,
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

fn evaluate_m3b_pose_truth(
    morph: &PosePair,
    skin: &PosePair,
    animation: &PosePair,
) -> Vec<&'static str> {
    let mut errors = Vec::new();
    for (code, pair) in [
        ("morph_pose_change", morph),
        ("skin_pose_change", skin),
        ("animation_pose_change", animation),
    ] {
        let base = foreground_metrics(&pair.base.rgba, pair.base.width, pair.base.height);
        let sampled =
            foreground_metrics(&pair.sampled.rgba, pair.sampled.width, pair.sampled.height);
        let delta = difference_metrics(
            &pair.base.rgba,
            &pair.sampled.rgba,
            pair.base.width,
            pair.base.height,
            2,
        );
        let footprint_changed = base.rect.zip(sampled.rect).is_some_and(|(left, right)| {
            left != right
                || (base.centroid_x - sampled.centroid_x).abs() >= 0.25
                || (base.centroid_y - sampled.centroid_y).abs() >= 0.25
        });
        let localized = delta.changed_pixels >= 8
            && delta.changed_pixels < pair.base.width as usize * pair.base.height as usize / 2
            && delta.rect.is_some_and(|change| {
                base.rect.is_some_and(|rect| change.intersects(rect))
                    || sampled.rect.is_some_and(|rect| change.intersects(rect))
            });
        if !footprint_changed || !localized {
            errors.push(code);
        }
    }
    errors
}

fn render_khronos_pose_pair(
    name: &'static str,
    fixture_id: &'static str,
    path: &'static str,
) -> PosePair {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(path)).expect("Khronos sample scene loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("Khronos sample instantiates");
    let (duration, channels) = {
        let clip = import
            .clips()
            .expect("sample clips remain live")
            .first()
            .expect("Khronos sample has an animation clip");
        (clip.duration_seconds(), clip.channels().to_vec())
    };
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
    let base = VisualArtifact {
        name: match name {
            "morph" => "m3b-morph-base",
            "skin" => "m3b-skin-base",
            "animation" => "m3b-animation-base",
            _ => unreachable!("known Q03 pose proof"),
        },
        width: 48,
        height: 48,
        rgba: renderer.frame_rgba8().to_vec(),
    };
    let clip = AnimationClip::authored(Some(format!("q03-{name}")), channels, duration)
        .expect("import-bound channels form an authored Q03 clip");
    let mixer = scene
        .create_authored_animation_mixer(clip)
        .expect("Q03 pose mixer creates");
    let sample_time = if name == "skin" {
        1.0_f32.min(duration)
    } else {
        duration * 0.5
    };
    scene
        .seek_animation(mixer, sample_time)
        .expect("Q03 pose samples a declared non-rest pose");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("sampled pose prepares");
    renderer
        .render(&scene, camera)
        .expect("sampled pose renders");
    let sampled = VisualArtifact {
        name: match name {
            "morph" => "m3b-morph-sampled",
            "skin" => "m3b-skin-sampled",
            "animation" => "m3b-animation-sampled",
            _ => unreachable!("known Q03 pose proof"),
        },
        width: 48,
        height: 48,
        rgba: renderer.frame_rgba8().to_vec(),
    };
    PosePair {
        fixture_id,
        base,
        sampled,
    }
}

#[derive(Clone)]
struct VisualArtifact {
    name: &'static str,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

struct PosePair {
    fixture_id: &'static str,
    base: VisualArtifact,
    sampled: VisualArtifact,
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
