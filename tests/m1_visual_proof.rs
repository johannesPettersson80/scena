#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    Assets, Color, GeometryDesc, GeometryTopology, MaterialDesc, PerspectiveCamera, Primitive,
    Renderer, Scene, Transform, Vec3, Vertex,
};

const M1_HEADLESS_FIXTURE_METADATA: &str = include_str!("visual/fixtures/m1-headless-core.toml");
const M1_HEADLESS_REFERENCE_METADATA: &str =
    include_str!("visual/references/m1-headless-core.toml");

#[test]
fn m1_headless_visual_artifacts_cover_core_material_paths() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    for fixture in visual_fixtures() {
        assert!(
            M1_HEADLESS_FIXTURE_METADATA.contains(&format!("name = \"{}\"", fixture.name)),
            "fixture metadata must list {}",
            fixture.name
        );
        let frame = (fixture.render)();
        (fixture.validate)(&frame, fixture.width, fixture.height);
        assert!(
            frame
                .chunks_exact(4)
                .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
            "{} should render visible nonblack pixels",
            fixture.name
        );
        write_ppm_artifact(
            &artifact_dir,
            fixture.name,
            fixture.width,
            fixture.height,
            &frame,
        );
    }
}

#[test]
fn m1_headless_reference_tolerances_match_current_fixtures() {
    let references = reference_specs();
    let mut mismatches = Vec::new();

    for fixture in visual_fixtures() {
        let reference = references
            .iter()
            .find(|reference| reference.name == fixture.name)
            .unwrap_or_else(|| panic!("missing reference metadata for {}", fixture.name));
        let frame = (fixture.render)();
        let center = pixel_at(&frame, fixture.width, fixture.width / 2, fixture.height / 2);
        let corner = pixel_at(&frame, fixture.width, 0, 0);
        let nonblack_pixels = nonblack_pixel_count(&frame);
        let rgba_hash = rgba_fnv1a64(&frame);

        if !rgba_within_tolerance(center, reference.center_rgba, reference.max_abs_diff)
            || !rgba_within_tolerance(corner, reference.corner_rgba, reference.max_abs_diff)
            || nonblack_pixels != reference.nonblack_pixels
            || rgba_hash != reference.rgba_hash
        {
            mismatches.push(format!(
                "{}: center={:?} corner={:?} nonblack_pixels={} rgba_hash=\"{}\"",
                fixture.name, center, corner, nonblack_pixels, rgba_hash
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "visual reference mismatches:\n{}",
        mismatches.join("\n")
    );
}

struct VisualFixture {
    name: &'static str,
    width: u32,
    height: u32,
    render: fn() -> Vec<u8>,
    validate: fn(&[u8], u32, u32),
}

#[derive(Debug, Clone)]
struct ReferenceSpec {
    name: String,
    max_abs_diff: u8,
    center_rgba: [u8; 4],
    corner_rgba: [u8; 4],
    nonblack_pixels: usize,
    rgba_hash: String,
}

fn visual_fixtures() -> [VisualFixture; 7] {
    [
        VisualFixture {
            name: "primitive-fullscreen",
            width: 16,
            height: 16,
            render: render_primitive_fullscreen,
            validate: validate_nonblack,
        },
        VisualFixture {
            name: "unlit-asset-mesh",
            width: 16,
            height: 16,
            render: render_unlit_asset_mesh,
            validate: validate_nonblack,
        },
        VisualFixture {
            name: "pbr-asset-mesh",
            width: 16,
            height: 16,
            render: render_pbr_asset_mesh,
            validate: validate_nonblack,
        },
        VisualFixture {
            name: "transparent-blend",
            width: 16,
            height: 16,
            render: render_transparent_blend,
            validate: validate_nonblack,
        },
        VisualFixture {
            name: "line-material",
            width: 16,
            height: 16,
            render: render_line_material,
            validate: validate_nonblack,
        },
        VisualFixture {
            name: "wire-edge-materials",
            width: 16,
            height: 16,
            render: render_wire_edge_materials,
            validate: validate_nonblack,
        },
        VisualFixture {
            name: "default-cube",
            width: 16,
            height: 16,
            render: render_default_cube_with_default_environment,
            validate: validate_default_cube_luminance_and_silhouette,
        },
    ]
}

fn render_primitive_fullscreen() -> Vec<u8> {
    let (mut scene, camera) = scene_with_camera();
    scene
        .add_renderable(
            scene.root(),
            vec![fullscreen_triangle(Color::from_linear_rgb(0.2, 0.6, 1.0))],
            Transform::default(),
        )
        .expect("primitive renderable inserts");
    render_scene(scene, camera)
}

fn render_unlit_asset_mesh() -> Vec<u8> {
    render_asset_mesh(MaterialDesc::unlit(Color::from_linear_rgb(1.0, 0.1, 0.05)))
}

fn render_pbr_asset_mesh() -> Vec<u8> {
    render_asset_mesh(MaterialDesc::pbr_metallic_roughness(
        Color::from_linear_rgb(0.72, 0.74, 0.76),
        0.0,
        0.8,
    ))
}

fn render_transparent_blend() -> Vec<u8> {
    let (mut scene, camera) = scene_with_camera();
    scene
        .add_renderable(
            scene.root(),
            vec![
                fullscreen_triangle(Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0)),
                fullscreen_triangle(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)),
            ],
            Transform::default(),
        )
        .expect("transparent renderable inserts");
    render_scene(scene, camera)
}

fn render_line_material() -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(-0.85, 0.0, 0.0),
        Vec3::new(0.85, 0.0, 0.0),
    ));
    let material = assets.create_material(MaterialDesc::line(Color::WHITE, 1.0));
    render_asset_mesh_handles(&assets, geometry, material)
}

fn render_wire_edge_materials() -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(flat_square_geometry());
    let wire = assets.create_material(MaterialDesc::wireframe(
        Color::from_linear_rgb(0.2, 0.7, 1.0),
        1.0,
    ));
    let edge = assets.create_material(MaterialDesc::edge(
        Color::from_linear_rgb(1.0, 0.9, 0.1),
        1.0,
    ));
    let (mut scene, camera) = scene_with_camera();
    scene.mesh(geometry, wire).add().expect("wire mesh inserts");
    scene.mesh(geometry, edge).add().expect("edge mesh inserts");

    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("technical material meshes prepare");
    renderer
        .render(&scene, camera)
        .expect("technical material meshes render");
    renderer.frame_rgba8().to_vec()
}

fn render_default_cube_with_default_environment() -> Vec<u8> {
    let assets = Assets::new();
    let environment = assets.default_environment();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.2, 1.2, 0.1));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("default cube inserts");
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("default cube prepares with default environment");
    renderer
        .render(&scene, camera)
        .expect("default cube renders with visible environment");
    renderer.frame_rgba8().to_vec()
}

fn render_asset_mesh(material: MaterialDesc) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material = assets.create_material(material);
    render_asset_mesh_handles(&assets, geometry, material)
}

fn render_asset_mesh_handles(
    assets: &Assets,
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
) -> Vec<u8> {
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("asset mesh inserts");
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("asset mesh prepares");
    renderer.render(&scene, camera).expect("asset mesh renders");
    renderer.frame_rgba8().to_vec()
}

fn render_scene(mut scene: Scene, camera: scena::CameraKey) -> Vec<u8> {
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

fn scene_with_camera() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    (scene, camera)
}

fn fullscreen_triangle(color: Color) -> Primitive {
    Primitive::triangle([
        Vertex {
            position: Vec3::new(-2.0, -2.0, 0.0),
            color,
        },
        Vertex {
            position: Vec3::new(4.0, -2.0, 0.0),
            color,
        },
        Vertex {
            position: Vec3::new(-2.0, 4.0, 0.0),
            color,
        },
    ])
}

fn fullscreen_triangle_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-2.0, -2.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(4.0, -2.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-2.0, 4.0, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2],
    )
    .expect("fullscreen test geometry is valid")
}

fn flat_square_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.75, -0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.75, -0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.75, 0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-0.75, 0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("flat square test geometry is valid")
}

fn validate_nonblack(_frame: &[u8], _width: u32, _height: u32) {}

fn validate_default_cube_luminance_and_silhouette(frame: &[u8], width: u32, height: u32) {
    assert_eq!(
        pixel_at(frame, width, width / 2, height / 2),
        [206, 206, 206, 255]
    );
    assert_eq!(pixel_at(frame, width, 0, 0), [0, 0, 0, 255]);
    assert_eq!(
        pixel_at(frame, width, width - 1, height - 1),
        [0, 0, 0, 255]
    );
}

fn reference_specs() -> Vec<ReferenceSpec> {
    let mut references = Vec::new();
    let mut current: Option<ReferenceSpec> = None;

    for line in M1_HEADLESS_REFERENCE_METADATA.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[reference]]" {
            if let Some(reference) = current.take() {
                references.push(reference);
            }
            current = Some(ReferenceSpec {
                name: String::new(),
                max_abs_diff: 0,
                center_rgba: [0; 4],
                corner_rgba: [0; 4],
                nonblack_pixels: 0,
                rgba_hash: String::new(),
            });
            continue;
        }

        let Some(reference) = current.as_mut() else {
            continue;
        };
        if let Some(value) = line.strip_prefix("name = ") {
            reference.name = parse_quoted(value);
        } else if let Some(value) = line.strip_prefix("max_abs_diff = ") {
            reference.max_abs_diff = value.parse().expect("max_abs_diff is a u8");
        } else if let Some(value) = line.strip_prefix("center_rgba = ") {
            reference.center_rgba = parse_rgba(value);
        } else if let Some(value) = line.strip_prefix("corner_rgba = ") {
            reference.corner_rgba = parse_rgba(value);
        } else if let Some(value) = line.strip_prefix("nonblack_pixels = ") {
            reference.nonblack_pixels = value.parse().expect("nonblack_pixels is a usize");
        } else if let Some(value) = line.strip_prefix("rgba_hash = ") {
            reference.rgba_hash = parse_quoted(value);
        }
    }

    if let Some(reference) = current {
        references.push(reference);
    }
    references
}

fn parse_quoted(value: &str) -> String {
    value
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("quoted string value")
        .to_owned()
}

fn parse_rgba(value: &str) -> [u8; 4] {
    let value = value
        .trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .expect("RGBA array");
    let channels: Vec<u8> = value
        .split(',')
        .map(|channel| channel.trim().parse().expect("RGBA channel is u8"))
        .collect();
    channels
        .try_into()
        .expect("RGBA reference contains four channels")
}

fn rgba_within_tolerance(actual: [u8; 4], expected: [u8; 4], max_abs_diff: u8) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| actual.abs_diff(expected) <= max_abs_diff)
}

fn nonblack_pixel_count(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn rgba_fnv1a64(frame: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in frame {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn pixel_at(frame: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    frame[offset..offset + 4]
        .try_into()
        .expect("pixel slice has four channels")
}

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    assert_eq!(rgba.len(), width as usize * height as usize * 4);
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact can be written");
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\nname = \"{name}\"\nformat = \"ppm\"\nencoding = \"srgb8\"\nwidth = {width}\nheight = {height}\n"
        ),
    )
    .expect("artifact metadata can be written");
}

fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m1-visual")
}
