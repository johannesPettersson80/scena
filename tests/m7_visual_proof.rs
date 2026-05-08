#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    Aabb, Assets, Color, ConnectOptions, ConnectorFrame, GeometryDesc, LabelDesc, MaterialDesc,
    OrbitControls, Renderer, Scene, SourceCoordinateSystem, SourceUnits, Transform, Vec3,
};

#[test]
fn m7_headless_visual_artifacts_cover_ergonomics_workflows() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");
    let connector_before = render_connector_connection(false);
    let connector_after = render_connector_connection(true);

    assert_ne!(
        connector_before.rgba, connector_after.rgba,
        "connector before/after proof must show a rendered placement change"
    );

    for artifact in [
        render_first_render(),
        render_first_glb(),
        render_camera_frame(),
        render_picking_selection(),
        render_helpers(),
        render_labels(),
        render_controls(),
        render_layers_helper_on_top(),
        render_static_batching(),
        render_anchor_alignment(),
        connector_before,
        connector_after,
        render_coordinate_units(),
        render_industrial_static_scene(),
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

fn render_first_render() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.4, 0.3));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.65, 1.0)));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("camera frames mesh");
    render_scene_with_assets("m7-first-render", scene, camera, &assets)
}

fn render_first_glb() -> VisualArtifact {
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
    )
    .expect("fixture glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("fixture instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_import(camera, &import)
        .expect("camera frames import");
    render_scene_with_assets("m7-first-glb", scene, camera, &assets)
}

fn render_camera_frame() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.25, 0.25));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.1, 0.7, 1.0)));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let bounds = Aabb::new(
        Vec3::new(-0.25, -0.125, -0.125),
        Vec3::new(0.25, 0.125, 0.125),
    );
    scene.frame(camera, bounds).expect("camera frames bounds");
    render_scene_with_assets("m7-camera-frame", scene, camera, &assets)
}

fn render_picking_selection() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.4, 0.3));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.65, 1.0)));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("camera frames mesh");
    let hit = scene
        .pick_and_select_with_assets(
            camera,
            scena::CursorPosition::physical(24.0, 24.0),
            scena::Viewport::new(48, 48, 1.0).expect("viewport validates"),
            &assets,
        )
        .expect("pick succeeds")
        .expect("pick hits");
    assert_eq!(scene.interaction().primary_selection(), Some(hit.target()));
    render_scene_with_assets("m7-picking-selection", scene, camera, &assets)
}

fn render_helpers() -> VisualArtifact {
    let assets = Assets::new();
    let line = assets.create_material(MaterialDesc::line(Color::WHITE, 1.0));
    let bounds = Aabb::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));
    let mut scene = Scene::new();
    for helper in [
        GeometryDesc::axes(0.5),
        GeometryDesc::grid(0.5, 4),
        GeometryDesc::bounding_box(bounds),
        GeometryDesc::camera_frustum(0.1, 2.0, 1.0, 60.0),
        GeometryDesc::light_helper(0.2),
        GeometryDesc::origin_marker(0.2),
        GeometryDesc::pivot_marker(0.2),
        GeometryDesc::anchor_marker(0.2),
    ] {
        let geometry = assets.create_geometry(helper);
        scene.mesh(geometry, line).add().expect("helper inserts");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    render_scene_with_assets("m7-helpers", scene, camera, &assets)
}

fn render_labels() -> VisualArtifact {
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::msdf("M7").with_color(Color::from_linear_rgb(0.1, 1.0, 0.3)),
            Transform::default(),
        )
        .expect("label inserts");
    render_scene("m7-labels", scene, camera)
}

fn render_controls() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.6, 0.4, 0.3));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.65, 1.0)));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    OrbitControls::new(Vec3::ZERO, 2.5)
        .with_damping(0.2)
        .focus(Vec3::ZERO, 2.0)
        .apply_to_scene(&mut scene, camera)
        .expect("controls apply");
    render_scene_with_assets("m7-controls", scene, camera, &assets)
}

fn render_layers_helper_on_top() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.45, 0.3, 0.25));
    let visible_material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.8, 1.0)));
    let hidden_material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.2, 0.1)));
    let mut scene = Scene::new();
    let visible = scene
        .mesh(geometry, visible_material)
        .add()
        .expect("visible mesh inserts");
    let hidden = scene
        .mesh(geometry, hidden_material)
        .transform(Transform::at(Vec3::new(0.3, 0.0, 0.0)))
        .add()
        .expect("hidden mesh inserts");
    scene.set_layer_mask(hidden, 0b0010).expect("layer mask");
    scene.set_helper_on_top(visible, true).expect("helper flag");
    let camera = scene.add_default_camera().expect("camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("camera frames meshes");
    scene
        .set_camera_layer_mask(camera, 0b0001)
        .expect("camera layer mask");
    render_scene_with_assets("m7-layers-helper-on-top", scene, camera, &assets)
}

fn render_static_batching() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_static_batch(
        &GeometryDesc::box_xyz(0.12, 0.12, 0.12),
        [
            Transform::at(Vec3::new(-0.25, 0.0, 0.0)),
            Transform::at(Vec3::new(0.0, 0.0, 0.0)),
            Transform::at(Vec3::new(0.25, 0.0, 0.0)),
        ],
    );
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.9, 0.7, 0.1)));
    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("batch inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    render_scene_with_assets("m7-static-batching", scene, camera, &assets)
}

fn render_anchor_alignment() -> VisualArtifact {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor fixture loads");
    let marker_geometry = assets.create_geometry(GeometryDesc::anchor_marker(0.25));
    let marker_material = assets.create_material(MaterialDesc::line(
        Color::from_linear_rgb(1.0, 0.9, 0.1),
        1.0,
    ));
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("anchor fixture instantiates");
    let marker = scene
        .mesh(marker_geometry, marker_material)
        .add()
        .expect("anchor marker inserts");
    scene
        .snap_anchor(marker, import.anchor("inspection").expect("anchor exists"))
        .expect("marker snaps to anchor");
    let camera = scene.add_default_camera().expect("camera inserts");
    render_scene_with_assets("m7-anchor-alignment", scene, camera, &assets)
}

fn render_connector_connection(connected: bool) -> VisualArtifact {
    let assets = Assets::new();
    let source_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.28, 0.2, 0.2));
    let target_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.28, 0.2, 0.2));
    let source_material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.2, 0.1)));
    let target_material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.1, 0.45, 1.0)));
    let mut scene = Scene::new();
    let source = scene
        .mesh(source_geometry, source_material)
        .transform(Transform::at(Vec3::new(-0.65, 0.0, 0.0)))
        .add()
        .expect("source part inserts");
    let target = scene
        .mesh(target_geometry, target_material)
        .transform(Transform::at(Vec3::new(0.45, 0.0, 0.0)))
        .add()
        .expect("target part inserts");
    if connected {
        scene
            .connect(
                ConnectorFrame::new(source, Transform::at(Vec3::new(0.14, 0.0, 0.0)))
                    .named("source-face"),
                ConnectorFrame::new(target, Transform::at(Vec3::new(-0.14, 0.0, 0.0)))
                    .named("target-face"),
                ConnectOptions::default(),
            )
            .expect("connector placement solves before visual proof");
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    let name = if connected {
        "m7-connector-after"
    } else {
        "m7-connector-before"
    };
    render_scene_with_assets(name, scene, camera, &assets)
}

fn render_coordinate_units() -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.6, 1.0, 0.2)));
    let mut scene = Scene::new();
    let converted_mm =
        SourceCoordinateSystem::ZUpRightHanded.convert_position(Vec3::new(250.0, 0.0, 0.0));
    let meters_per_unit = SourceUnits::Millimeters.meters_per_unit();
    let converted = Vec3::new(
        converted_mm.x * meters_per_unit,
        converted_mm.y * meters_per_unit,
        converted_mm.z * meters_per_unit,
    );
    scene
        .mesh(geometry, material)
        .transform(Transform::at(converted))
        .add()
        .expect("converted mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    render_scene_with_assets("m7-coordinate-units", scene, camera, &assets)
}

fn render_industrial_static_scene() -> VisualArtifact {
    let assets = Assets::new();
    let body = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.18, 0.18));
    let pipe = assets.create_geometry(GeometryDesc::box_xyz(0.08, 0.08, 0.6));
    let body_material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.2, 0.6, 1.0)));
    let pipe_material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.8, 0.8, 0.85)));
    let mut scene = Scene::new();
    for x in [-0.35, 0.0, 0.35] {
        scene
            .mesh(body, body_material)
            .transform(Transform::at(Vec3::new(x, 0.0, 0.0)))
            .add()
            .expect("body inserts");
        scene
            .mesh(pipe, pipe_material)
            .transform(Transform::at(Vec3::new(x, -0.22, 0.0)))
            .add()
            .expect("pipe inserts");
    }
    scene
        .add_label(
            scene.root(),
            LabelDesc::sdf("Line A"),
            Transform::at(Vec3::new(0.0, 0.35, 0.0)),
        )
        .expect("label inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    render_scene_with_assets("m7-industrial-static-scene", scene, camera, &assets)
}

fn render_scene(name: &'static str, mut scene: Scene, camera: scena::CameraKey) -> VisualArtifact {
    let mut renderer = Renderer::headless(48, 48).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    VisualArtifact {
        name,
        width: 48,
        height: 48,
        rgba: renderer.frame_rgba8().to_vec(),
    }
}

fn render_scene_with_assets<F>(
    name: &'static str,
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets<F>,
) -> VisualArtifact {
    let mut renderer = Renderer::headless(48, 48).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("asset scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    VisualArtifact {
        name,
        width: 48,
        height: 48,
        rgba: renderer.frame_rgba8().to_vec(),
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
            "[artifact]\nname = \"{name}\"\nformat = \"ppm\"\nencoding = \"srgb8\"\nwidth = {width}\nheight = {height}\ntolerance = \"nonblack-smoke\"\n"
        ),
    )
    .expect("artifact metadata can be written");
}

fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m7-visual")
}

struct VisualArtifact {
    name: &'static str,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
