use std::fs;
use std::path::PathBuf;

use scena::{Assets, Color, GeometryDesc, MaterialDesc, Renderer, Scene};

const README: &str = include_str!("../README.md");
const GETTING_STARTED: &str = include_str!("../docs/getting-started.md");
const FEATURE_FLAGS: &str = include_str!("../docs/feature-flags.md");
const NEXT_RELEASE: &str =
    include_str!("../docs/checklists/next-release-easy-use-and-state-of-the-art.md");
const LIB_RS: &str = include_str!("../src/lib.rs");
const CI: &str = include_str!("../.github/workflows/ci.yml");

#[test]
fn every_onboarding_rust_block_is_explicitly_compile_gated() {
    for (name, markdown) in [
        ("README.md", README),
        ("docs/getting-started.md", GETTING_STARTED),
    ] {
        let openings = markdown
            .lines()
            .filter(|line| line.trim_start().starts_with("```rust"))
            .collect::<Vec<_>>();
        assert!(
            !openings.is_empty(),
            "{name} must keep a runnable Rust example"
        );
        assert!(
            openings.iter().all(|line| line.trim() == "```rust,no_run"),
            "every {name} Rust block must compile as a no-run doctest: {openings:?}"
        );
    }
    assert!(
        LIB_RS.contains("cfg(doctest)")
            && LIB_RS.contains("include_str!(\"../README.md\")")
            && LIB_RS.contains("include_str!(\"../docs/getting-started.md\")"),
        "crate doctests must extract both onboarding documents"
    );
    assert!(
        CI.contains("cargo test --doc"),
        "CI must compile extracted onboarding snippets explicitly"
    );
}

#[test]
fn public_dependency_examples_follow_version_agnostic_policy() {
    for (name, markdown) in [
        ("README.md", README),
        ("docs/getting-started.md", GETTING_STARTED),
        ("docs/feature-flags.md", FEATURE_FLAGS),
    ] {
        assert!(
            !has_numeric_scena_dependency(markdown),
            "{name} must use cargo add scena rather than a drift-prone numeric dependency"
        );
        assert!(
            markdown.contains("cargo add scena"),
            "{name} must show cargo add scena"
        );
    }
}

#[test]
fn getting_started_snippets_pin_visible_framed_capture_lifecycles() {
    let first_scene = section(GETTING_STARTED, "## Create a first scene", "## Load a GLB");
    for required in [
        "Scene::with_default_camera()",
        "scene.frame_all_with_assets(camera, &assets)",
        "renderer.prepare_with_assets(&mut scene, &assets)",
        "renderer.render_active(&scene)",
        "renderer.capture_rgba8(&scene, Default::default())",
        "write_png(\"first-scene.png\")",
    ] {
        assert!(
            first_scene.contains(required),
            "first-scene snippet misses {required}"
        );
    }
    assert!(
        !first_scene.contains("Transform::default()"),
        "first scene must not place the camera inside its cube"
    );

    let glb = section(GETTING_STARTED, "## Load a GLB", "## Choose an output path");
    for required in [
        "pollster::block_on(assets.load_scene(path.as_str()))",
        "scene.instantiate(&asset)",
        "scene.add_default_camera()",
        "scene.frame_import(camera, &import)",
        "renderer.prepare_with_assets(&mut scene, &assets)",
        "renderer.render_active(&scene)",
        "renderer.capture_rgba8(&scene, Default::default())",
        "write_png(\"model.png\")",
        "std::io::Error::other",
    ] {
        assert!(glb.contains(required), "GLB snippet misses {required}");
    }
}

#[test]
fn onboarding_first_scene_renders_deterministic_nonblank_output() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.5, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::BLUE));
    let (mut scene, camera) = Scene::with_default_camera().expect("default camera inserts");
    scene.mesh(geometry, material).add().expect("cube inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("camera frames cube");

    let frame = render(&mut scene, &assets);
    assert_nonblank("first-scene", &frame);
    write_ppm("first-scene", &frame);
}

#[test]
fn onboarding_glb_scene_renders_deterministic_nonblank_output() {
    let path = "tests/assets/gltf/animated_triangle_scene.glb";
    let assets = Assets::new();
    let asset = pollster::block_on(assets.load_scene(path)).expect("GLB fixture loads");
    let mut scene = Scene::new();
    let import = scene.instantiate(&asset).expect("GLB fixture instantiates");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("camera frames GLB import");

    let frame = render(&mut scene, &assets);
    assert_nonblank("glb-scene", &frame);
    write_ppm("glb-scene", &frame);
}

#[test]
fn shipped_renderer_features_have_no_reverse_status_drift() {
    for stale in [
        "LTC area lights, KTX2 cubemap delivery, and clustered/tiled culling stay later",
        "clustered/tiled light\n  culling, LTC area lights, and SSR now point to future backend lanes",
    ] {
        assert!(
            !NEXT_RELEASE.contains(stale),
            "active checklist retains stale shipped-feature closeout: {stale}"
        );
    }
    for shipped in [
        "**Clustered / tiled light culling.** Status:\n  **[shipped]**",
        "**Area lights with LTC** (rect/disc/sphere). Status:\n  **[shipped]**",
        "**Screen-space reflections (SSR).** Status:\n  **[shipped]**",
    ] {
        assert!(
            NEXT_RELEASE.contains(shipped),
            "missing shipped status: {shipped}"
        );
    }
}

fn render(scene: &mut Scene, assets: &Assets) -> Vec<u8> {
    let mut renderer = Renderer::headless(96, 72).expect("renderer builds");
    renderer
        .prepare_with_assets(scene, assets)
        .expect("scene prepares");
    renderer.render_active(scene).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

fn assert_nonblank(name: &str, frame: &[u8]) {
    let visible = frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count();
    assert!(
        visible >= 128,
        "{name} must render visible pixels, got {visible}"
    );
}

fn write_ppm(name: &str, frame: &[u8]) {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/c11-onboarding");
    fs::create_dir_all(&directory).expect("artifact directory creates");
    let mut ppm = b"P6\n96 72\n255\n".to_vec();
    for pixel in frame.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(directory.join(format!("{name}.ppm")), ppm).expect("artifact writes");
}

fn section<'a>(document: &'a str, start: &str, end: &str) -> &'a str {
    let start = document.find(start).expect("section start exists");
    let tail = &document[start..];
    let end = tail.find(end).expect("section end exists");
    &tail[..end]
}

fn has_numeric_scena_dependency(markdown: &str) -> bool {
    markdown.lines().any(|line| {
        let line = line.trim();
        line.starts_with("scena =")
            && line.contains('"')
            && line.chars().any(|character| character.is_ascii_digit())
            && !line.contains("path =")
    })
}
