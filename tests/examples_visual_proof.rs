//! Phase 5A — visual proof artifacts for the public examples that aren't already
//! covered by the M7 / M8 / M3a / M3b visual suites. The contract is
//! "every public example whose output is meant to be visible on screen has a
//! 256x256 headless PPM proof under `target/gate-artifacts/examples-visual/`",
//! per state-of-art-threejs-replacement-plan.md line 1396.
//!
//! Native-window, browser-canvas, and inspection-only examples are deliberately
//! skipped: they describe runtime patterns (event loop, attached canvas,
//! diagnostic introspection) that don't have a single deterministic frame to
//! capture.

use std::fs;
use std::path::PathBuf;

use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene, Transform, Vec3,
};

const ARTIFACT_WIDTH: u32 = 256;
const ARTIFACT_HEIGHT: u32 = 256;

fn artifact_dir() -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/examples-visual");
    fs::create_dir_all(&dir).expect("examples-visual artifact directory");
    dir
}

fn count_nonblack_pixels(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn write_artifact(name: &str, width: u32, height: u32, rgba: &[u8]) {
    let dir = artifact_dir();
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact can be written");
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\n\
             name = \"{name}\"\n\
             example_source = \"examples/{name}.rs\"\n\
             format = \"ppm\"\n\
             encoding = \"srgb8\"\n\
             width = {width}\n\
             height = {height}\n\
             tolerance = \"nonblack-smoke\"\n\
             proof_class = \"example-visual\"\n"
        ),
    )
    .expect("artifact metadata can be written");
}

#[test]
fn examples_visual_primitive_shapes_renders_box_to_ppm() {
    // Mirror examples/primitive_shapes.rs at the same headless renderer scale.
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(90, 148, 255),
        0.0,
        0.55,
    ));

    let mut scene = Scene::new();
    scene.mesh(cube, material).add().expect("box mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("primitive_shapes scene prepares");
    renderer
        .render_active(&scene)
        .expect("primitive_shapes scene renders");

    let frame = renderer.frame_rgba8();
    assert_eq!(
        frame.len(),
        (ARTIFACT_WIDTH as usize) * (ARTIFACT_HEIGHT as usize) * 4
    );
    assert!(
        count_nonblack_pixels(frame) > 0,
        "primitive_shapes example must render at least one nonblack pixel"
    );

    write_artifact("primitive_shapes", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_beginner_diagnostics_renders_recovery_scene_to_ppm() {
    // examples/beginner_diagnostics.rs walks the user through diagnostic
    // recovery; the visible scene after recovery is a single colored quad.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.5, 1.5, 0.05));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(220, 80, 80)));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("recovery mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("beginner diagnostics recovery scene prepares");
    renderer
        .render_active(&scene)
        .expect("beginner diagnostics scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "beginner diagnostics recovery scene must render at least one nonblack pixel"
    );

    write_artifact(
        "beginner_diagnostics",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_headless_ci_renders_default_scene_to_ppm() {
    // examples/headless_ci.rs exercises the deterministic headless rendering
    // path; the proof is "the deterministic frame produces non-black pixels".
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.8, 0.8));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(120, 200, 140),
        0.1,
        0.6,
    ));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("headless ci mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.5)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("headless_ci scene prepares");
    renderer
        .render_active(&scene)
        .expect("headless_ci scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "headless_ci example must render at least one nonblack pixel"
    );

    write_artifact("headless_ci", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}
