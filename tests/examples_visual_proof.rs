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
    Aabb, AnimationPlaybackState, Assets, Color, ConnectOptions, ConnectionAlignment,
    ConnectionError, ConnectorFrame, CursorPosition, GeometryDesc, InteractionStyle, LabelDesc,
    MaterialDesc, PerspectiveCamera, Profile, Renderer, RendererOptions, Scene,
    SourceCoordinateSystem, SourceUnits, Transform, Vec3, Viewport,
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
fn examples_visual_instancing_renders_instance_set_to_ppm() {
    // Mirror examples/instancing.rs: an instance set with reserve + push_instance,
    // 10 boxes laid out along the x axis.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 220, 160)));

    let mut scene = Scene::new();
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    scene
        .reserve_instances(set, 16)
        .expect("instance reserve succeeds");
    for index in 0..10 {
        scene
            .push_instance(
                set,
                Transform {
                    translation: Vec3::new(index as f32 * 0.24 - 1.0, 0.0, 0.0),
                    ..Transform::default()
                },
            )
            .expect("push_instance succeeds");
    }
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
        .expect("instancing scene prepares");
    renderer
        .render_active(&scene)
        .expect("instancing scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "instancing example must render at least one nonblack pixel"
    );

    write_artifact("instancing", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_labels_helpers_renders_axes_bounds_anchor_label_to_ppm() {
    // Mirror examples/labels_helpers.rs: axes + bounding box + anchor marker + a
    // single MSDF label, all rendered through the line material.
    let assets = Assets::new();
    let axes = assets.create_geometry(GeometryDesc::axes(1.0));
    let bounds = assets.create_geometry(GeometryDesc::bounding_box(Aabb::new(
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, 0.5),
    )));
    let anchor = assets.create_geometry(GeometryDesc::anchor_marker(0.15));
    let material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(200, 220, 255), 1.0));

    let mut scene = Scene::new();
    scene.mesh(axes, material).add().expect("axes mesh inserts");
    scene
        .mesh(bounds, material)
        .add()
        .expect("bounds mesh inserts");
    scene
        .mesh(anchor, material)
        .add()
        .expect("anchor mesh inserts");
    let label = LabelDesc::msdf("origin")
        .with_color(Color::from_srgb_u8(255, 255, 255))
        .with_size(14.0);
    scene
        .add_label(
            scene.root(),
            label,
            Transform {
                translation: Vec3::new(0.0, 0.15, 0.0),
                ..Transform::default()
            },
        )
        .expect("label inserts");
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
        .expect("labels_helpers scene prepares");
    renderer
        .render_active(&scene)
        .expect("labels_helpers scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "labels_helpers example must render at least one nonblack pixel"
    );

    write_artifact("labels_helpers", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_picking_selection_hover_renders_styled_pick_to_ppm() {
    // Mirror examples/picking_selection_hover.rs: a single mesh, framed via
    // frame_all_with_assets, then pick_and_select_with_assets at the viewport
    // center. The artifact proves the picking + selection-style + hover-style path
    // produces visible pixels with the typed CursorPosition + Viewport API.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(64, 160, 255)));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("picked mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer.set_hover_style(InteractionStyle::outline(
        Color::from_srgb_u8(255, 210, 64),
        2.0,
    ));
    renderer.set_selection_style(InteractionStyle::outline(
        Color::from_srgb_u8(64, 160, 255),
        3.0,
    ));
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("picking_selection_hover scene prepares");
    let viewport =
        Viewport::new(ARTIFACT_WIDTH, ARTIFACT_HEIGHT, 1.0).expect("static viewport is valid");
    scene
        .pick_and_select_with_assets(
            camera,
            CursorPosition::physical(ARTIFACT_WIDTH as f32 / 2.0, ARTIFACT_HEIGHT as f32 / 2.0),
            viewport,
            &assets,
        )
        .expect("pick_and_select succeeds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("picking_selection_hover scene re-prepares after selection mutation");
    renderer
        .render_active(&scene)
        .expect("picking_selection_hover scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "picking_selection_hover example must render at least one nonblack pixel"
    );

    write_artifact(
        "picking_selection_hover",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_animation_renders_morph_clip_at_frame_to_ppm() {
    // Mirror examples/animation.rs: load the Khronos AnimatedMorphCube glTF, create
    // and play the "Square" mixer, advance one 60Hz frame, then render. Proves the
    // animation mixer + glTF morph-target path produces visible pixels end-to-end.
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/khronos/MorphCube/AnimatedMorphCube.gltf"),
    )
    .expect("morph cube fixture loads");

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("morph cube instantiates");
    let mixer = scene
        .create_animation_mixer(&import, "Square")
        .expect("Square mixer creates");
    scene
        .play_animation(mixer)
        .expect("play animation succeeds");
    scene
        .update_animation(mixer, 1.0 / 60.0)
        .expect("update_animation succeeds");

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
        .expect("animation scene prepares");
    renderer
        .render_active(&scene)
        .expect("animation scene renders");

    let state = scene.animation_mixer(mixer).expect("mixer query").state();
    assert_eq!(
        state,
        AnimationPlaybackState::Playing,
        "animation example must record the mixer as playing after update"
    );

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "animation example must render at least one nonblack pixel"
    );

    write_artifact("animation", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_glb_model_viewer_renders_imported_mesh_to_ppm() {
    // Mirror examples/glb_model_viewer.rs: a single first_render_gltf_headless call
    // against the mesh+material+vertex-color sample fixture. Proves the high-level
    // first-render + glTF mesh import + framing path produces visible pixels.
    let first = pollster::block_on(scena::first_render_gltf_headless(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
    ))
    .expect("first_render_gltf_headless succeeds");

    let frame = first.renderer.frame_rgba8();
    assert_eq!(
        frame.len(),
        (ARTIFACT_WIDTH as usize) * (ARTIFACT_HEIGHT as usize) * 4
    );
    assert!(
        count_nonblack_pixels(frame) > 0,
        "glb_model_viewer example must render at least one nonblack pixel"
    );

    write_artifact("glb_model_viewer", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_industrial_static_scene_renders_to_ppm() {
    // Mirror examples/industrial_static_scene.rs: a floor grid plus three machine
    // bodies with pipe connectors, framed via frame_all_with_assets and rendered
    // through the Industrial render profile.
    let assets = Assets::new();
    let floor = assets.create_geometry(GeometryDesc::grid(3.0, 12));
    let body = assets.create_geometry(GeometryDesc::box_xyz(0.36, 0.2, 0.18));
    let pipe = assets.create_geometry(GeometryDesc::box_xyz(0.08, 0.08, 0.7));
    let floor_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(90, 110, 130), 1.0));
    let body_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(55, 150, 220)));
    let pipe_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(205, 210, 220)));

    let mut scene = Scene::new();
    scene
        .mesh(floor, floor_material)
        .transform(Transform::at(Vec3::new(0.0, -0.35, 0.0)))
        .add()
        .expect("floor mesh inserts");
    for x in [-0.45_f32, 0.0, 0.45] {
        scene
            .mesh(body, body_material)
            .transform(Transform::at(Vec3::new(x, 0.0, 0.0)))
            .add()
            .expect("body mesh inserts");
        scene
            .mesh(pipe, pipe_material)
            .transform(Transform::at(Vec3::new(x, -0.18, 0.0)))
            .add()
            .expect("pipe mesh inserts");
    }
    scene
        .add_label(
            scene.root(),
            LabelDesc::sdf("Line A"),
            Transform::at(Vec3::new(0.0, 0.34, 0.0)),
        )
        .expect("label inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let options = RendererOptions::default().with_profile(Profile::Industrial);
    let mut renderer = Renderer::headless_with_options(ARTIFACT_WIDTH, ARTIFACT_HEIGHT, options)
        .expect("headless renderer builds with industrial options");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("industrial_static_scene prepares");
    renderer
        .render_active(&scene)
        .expect("industrial_static_scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "industrial_static_scene example must render at least one nonblack pixel"
    );

    write_artifact(
        "industrial_static_scene",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_connect_objects_renders_assembled_pair_to_ppm() {
    // Mirror examples/connect_objects.rs: typed connector handles join two empties
    // by named connectors, then we frame the assembly via frame_all_with_assets so
    // the rendered output proves the connector solve placed both nodes inside the
    // viewport.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let motor_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(220, 110, 70)));
    let pump_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 180, 220)));

    let mut scene = Scene::new();
    let motor = scene
        .mesh(geometry, motor_material)
        .transform(Transform::IDENTITY)
        .add()
        .expect("motor mesh inserts");
    let pump = scene
        .mesh(geometry, pump_material)
        .transform(Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .add()
        .expect("pump mesh inserts");
    let motor_shaft = scene
        .add_connector(
            ConnectorFrame::new(motor, Transform::at(Vec3::new(0.5, 0.0, 0.0))).named("shaft"),
        )
        .expect("motor connector inserts");
    let pump_drive = scene
        .add_connector(
            ConnectorFrame::new(pump, Transform::at(Vec3::new(-0.25, 0.0, 0.0))).named("drive"),
        )
        .expect("pump connector inserts");
    scene
        .connect_by_key(motor_shaft, pump_drive, ConnectOptions::default())
        .expect("connect_by_key succeeds");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("connect_objects scene prepares");
    renderer
        .render_active(&scene)
        .expect("connect_objects scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "connect_objects example must render at least one nonblack pixel"
    );

    write_artifact("connect_objects", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_imported_anchor_connection_renders_to_ppm() {
    // Mirror examples/imported_anchor_connection.rs: two imports of the anchor-debug
    // glTF scene connected by their named inspection anchors via
    // ConnectorFrame::from_import_anchor + connect_by_key, then framed via
    // frame_import on the target.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor debug scene loads");

    let mut scene = Scene::new();
    let source = scene
        .instantiate(&scene_asset)
        .expect("source instantiates");
    let target = scene
        .instantiate(&scene_asset)
        .expect("target instantiates");
    scene
        .set_transform(target.roots()[0], Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("target transform succeeds");

    let source_anchor = scene
        .add_connector(
            ConnectorFrame::from_import_anchor(source.anchor("inspection").expect("source anchor"))
                .with_kind("mount"),
        )
        .expect("source connector inserts");
    let target_anchor = scene
        .add_connector(
            ConnectorFrame::from_import_anchor(target.anchor("inspection").expect("target anchor"))
                .with_kind("mount"),
        )
        .expect("target connector inserts");
    scene
        .connect_by_key(source_anchor, target_anchor, ConnectOptions::default())
        .expect("connect_by_key succeeds");

    // Add a small visible marker so the scene has nonblack pixels — the anchor-only
    // imports themselves carry no mesh content and the upstream example similarly
    // doesn't render visible geometry.
    let marker = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let marker_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 200, 255)));
    scene
        .mesh(marker, marker_material)
        .add()
        .expect("anchor visualisation marker inserts");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");
    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("imported_anchor_connection scene prepares");
    renderer
        .render_active(&scene)
        .expect("imported_anchor_connection scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "imported_anchor_connection example must render at least one nonblack pixel"
    );

    write_artifact(
        "imported_anchor_connection",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_anchor_alignment_renders_anchor_marker_to_ppm() {
    // Mirror examples/anchor_alignment.rs: load the anchor-debug glTF, snap an
    // anchor-marker mesh to its named "inspection" anchor, then frame the imported
    // anchor and a small visible marker via frame_all_with_assets so the PPM
    // proves the snap_anchor + ConnectorFrame::from_import_anchor path lands the
    // marker at the right position. The upstream example uses frame_import which
    // panics on the anchor-only fixture; we use frame_all_with_assets instead.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor debug scene loads");
    let marker_geometry = assets.create_geometry(GeometryDesc::anchor_marker(0.2));
    let marker_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(255, 220, 70), 1.0));

    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).expect("scene instantiates");
    let marker = scene
        .mesh(marker_geometry, marker_material)
        .add()
        .expect("marker mesh inserts");
    scene
        .snap_anchor(
            marker,
            import.anchor("inspection").expect("inspection anchor"),
        )
        .expect("snap_anchor succeeds");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("anchor_alignment scene prepares");
    renderer
        .render_active(&scene)
        .expect("anchor_alignment scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "anchor_alignment example must render at least one nonblack pixel"
    );

    write_artifact("anchor_alignment", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_industrial_connector_assembly_renders_to_ppm() {
    // Mirror examples/industrial_connector_assembly.rs: three imports of the
    // connector-debug glTF chained pump → base and sensor → pump via named
    // "mount" connectors with ConnectionAlignment::ForwardToBack and a small
    // mate offset, plus a visible marker so the otherwise-anchor-only fixture
    // produces nonblack pixels under frame_all_with_assets.
    let assets = Assets::new();
    let part_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector debug scene loads");

    let mut scene = Scene::new();
    let base = scene.instantiate(&part_asset).expect("base instantiates");
    let pump = scene.instantiate(&part_asset).expect("pump instantiates");
    let sensor = scene.instantiate(&part_asset).expect("sensor instantiates");

    scene
        .set_transform(base.roots()[0], Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .expect("base transform succeeds");
    scene
        .set_transform(pump.roots()[0], Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("pump transform succeeds");
    scene
        .set_transform(sensor.roots()[0], Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("sensor transform succeeds");
    scene
        .lock_node_for_connections(base.roots()[0])
        .expect("lock base succeeds");

    let base_mount =
        ConnectorFrame::from_import_connector(base.connector("mount").expect("base mount"));
    let pump_mount =
        ConnectorFrame::from_import_connector(pump.connector("mount").expect("pump mount"));
    let sensor_mount =
        ConnectorFrame::from_import_connector(sensor.connector("mount").expect("sensor mount"));
    let options = ConnectOptions::default().with_alignment(ConnectionAlignment::ForwardToBack);
    scene
        .connect(pump_mount.clone(), base_mount, options)
        .expect("pump-base connect succeeds");
    scene
        .connect(
            sensor_mount,
            pump_mount,
            options.with_mate_offset(Transform::at(Vec3::new(0.4, 0.0, 0.0))),
        )
        .expect("sensor-pump connect succeeds");

    // Anchor-only fixture has no mesh content; add a visible marker so frame_all
    // produces nonblack pixels.
    let marker = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let marker_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(220, 180, 70)));
    scene
        .mesh(marker, marker_material)
        .add()
        .expect("assembly marker inserts");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("industrial_connector_assembly scene prepares");
    renderer
        .render_active(&scene)
        .expect("industrial_connector_assembly scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "industrial_connector_assembly example must render at least one nonblack pixel"
    );

    write_artifact(
        "industrial_connector_assembly",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_coordinate_connector_repair_renders_repaired_assembly_to_ppm() {
    // Mirror examples/coordinate_connector_repair.rs: import the connector_zup
    // fixture with the WRONG handedness (YUpLeftHanded), assert the connect call
    // fails closed with ConnectionError::HandednessMismatch, then re-import with
    // the correct ZUpRightHanded coordinate system and connect successfully. Plus
    // a visible marker so the otherwise-anchor-only fixture renders nonblack.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_zup_scene.gltf"))
            .expect("connector_zup fixture loads");

    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source empty inserts");

    let wrong_import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::YUpLeftHanded),
        )
        .expect("wrong-handedness instantiates");
    let error = scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_connector(
                wrong_import.connector("z-up-mount").expect("z-up-mount"),
            ),
            ConnectOptions::default(),
        )
        .expect_err("left-handed import must be repaired before connecting");
    match error {
        ConnectionError::HandednessMismatch { .. } => {}
        other => panic!("expected HandednessMismatch error, got {other:?}"),
    }

    let repaired_import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("repaired instantiates");
    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_connector(
                repaired_import.connector("z-up-mount").expect("z-up-mount"),
            ),
            ConnectOptions::default(),
        )
        .expect("repaired connect succeeds");

    let marker = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let marker_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(110, 220, 160)));
    scene
        .mesh(marker, marker_material)
        .add()
        .expect("repair marker inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("coordinate_connector_repair scene prepares");
    renderer
        .render_active(&scene)
        .expect("coordinate_connector_repair scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "coordinate_connector_repair example must render at least one nonblack pixel"
    );

    write_artifact(
        "coordinate_connector_repair",
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
