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
    Aabb, Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene,
    SourceCoordinateSystem, SourceUnits, Transform, Vec3,
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
fn examples_visual_camera_framing_renders_framed_part_to_ppm() {
    // Mirror examples/camera_framing.rs: a single oriented part, framed via Aabb.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.2, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 160, 240)));

    let mut scene = Scene::new();
    let inspected_part = scene
        .mesh(geometry, material)
        .add()
        .expect("framed mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    let bounds = Aabb::new(Vec3::new(-0.6, -0.2, -0.2), Vec3::new(0.6, 0.2, 0.2));
    scene.frame(camera, bounds).expect("frame succeeds");
    scene
        .look_at(camera, inspected_part)
        .expect("look_at succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("camera_framing scene prepares");
    renderer
        .render_active(&scene)
        .expect("camera_framing scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "camera_framing example must render at least one nonblack pixel"
    );

    write_artifact("camera_framing", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_layers_visibility_renders_active_layer_to_ppm() {
    // Mirror examples/layers_visibility.rs: helper-on-top + hidden layer + tag-based
    // selection, rendered through the camera layer mask.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
    let visible_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 170, 255)));
    let helper_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(255, 230, 80)));

    let mut scene = Scene::new();
    let machine = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(-0.25, 0.0, 0.0)))
        .add()
        .expect("machine mesh inserts");
    let helper = scene
        .mesh(geometry, helper_material)
        .transform(Transform::at(Vec3::new(0.25, 0.0, 0.0)).scale_by(0.5))
        .add()
        .expect("helper mesh inserts");
    let hidden = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(0.0, 0.4, 0.0)))
        .add()
        .expect("hidden mesh inserts");
    scene.add_tag(machine, "operational").expect("tag inserts");
    scene
        .set_layer_mask(machine, 0b0001)
        .expect("machine layer set");
    scene
        .set_layer_mask(helper, 0b0001)
        .expect("helper layer set");
    scene
        .set_layer_mask(hidden, 0b0010)
        .expect("hidden layer set");
    scene
        .set_visible(hidden, false)
        .expect("hidden visibility set");
    scene
        .set_render_group(helper, 10)
        .expect("helper render group set");
    scene
        .set_helper_on_top(helper, true)
        .expect("helper-on-top set");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .set_camera_layer_mask(camera, 0b0001)
        .expect("camera layer mask set");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("layers_visibility scene prepares");
    renderer
        .render_active(&scene)
        .expect("layers_visibility scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "layers_visibility example must render at least one nonblack pixel"
    );

    write_artifact("layers_visibility", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_coordinate_units_renders_converted_position_to_ppm() {
    // Mirror examples/coordinate_units.rs: a CAD-authored Z-up millimeter position
    // converted to scena Y-up meters, then framed by a default camera.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.12, 0.12, 0.12));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 230, 90)));

    let cad_position_mm = Vec3::new(250.0, 0.0, 100.0);
    let meters_per_unit = SourceUnits::Millimeters.meters_per_unit();
    let y_up_position = SourceCoordinateSystem::ZUpRightHanded.convert_position(cad_position_mm);
    let render_position = Vec3::new(
        y_up_position.x * meters_per_unit,
        y_up_position.y * meters_per_unit,
        y_up_position.z * meters_per_unit,
    );

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(render_position))
        .add()
        .expect("converted-position mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .look_at_point(camera, render_position)
        .expect("look_at_point succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("coordinate_units scene prepares");
    renderer
        .render_active(&scene)
        .expect("coordinate_units scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "coordinate_units example must render at least one nonblack pixel"
    );

    write_artifact("coordinate_units", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_static_batching_renders_repeated_boxes_to_ppm() {
    // Mirror examples/static_batching.rs: 12 transforms baked through
    // create_static_batch_with_report and rendered as a single mesh batch.
    let assets = Assets::new();
    let source = GeometryDesc::box_xyz(0.12, 0.12, 0.12);
    let transforms = (0..12).map(|index| {
        Transform::at(Vec3::new(
            (index % 6) as f32 * 0.18 - 0.45,
            (index / 6) as f32 * 0.18 - 0.09,
            0.0,
        ))
    });
    let (batch, _report) = assets.create_static_batch_with_report(&source, transforms);
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 200, 60)));
    let mut scene = Scene::new();
    scene
        .mesh(batch, material)
        .add()
        .expect("static-batch mesh inserts");
    scene.add_default_camera().expect("default camera inserts");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("static_batching scene prepares");
    renderer
        .render_active(&scene)
        .expect("static_batching scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "static_batching example must render at least one nonblack pixel"
    );

    write_artifact("static_batching", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
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
