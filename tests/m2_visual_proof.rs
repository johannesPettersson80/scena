#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    Assets, ClippingPlane, ClippingPlaneSet, Color, DirectionalLight, GeometryDesc,
    GeometryTopology, MaterialDesc, PerspectiveCamera, Primitive, Renderer, Scene, Transform, Vec3,
    Vertex,
};

const M2_HEADLESS_FIXTURE_METADATA: &str = include_str!("visual/fixtures/m2-headless-core.toml");
const M2_HEADLESS_REFERENCE_METADATA: &str =
    include_str!("visual/references/m2-headless-core.toml");
const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;

#[test]
fn m2_headless_visual_artifacts_cover_lighting_depth_and_clipping() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    for fixture in visual_fixtures() {
        assert!(
            M2_HEADLESS_FIXTURE_METADATA.contains(&format!("name = \"{}\"", fixture.name)),
            "fixture metadata must list {}",
            fixture.name
        );
        let proof = (fixture.render)();
        (fixture.validate)(&proof);
        assert!(
            proof
                .frame
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
            &proof.frame,
        );
    }
}

#[test]
fn m2_headless_reference_tolerances_match_current_fixtures() {
    let references = reference_specs();
    let mut mismatches = Vec::new();

    for fixture in visual_fixtures() {
        let reference = references
            .iter()
            .find(|reference| reference.name == fixture.name)
            .unwrap_or_else(|| panic!("missing reference metadata for {}", fixture.name));
        let proof = (fixture.render)();
        let center = pixel_at(
            &proof.frame,
            fixture.width,
            fixture.width / 2,
            fixture.height / 2,
        );
        let left_mid = pixel_at(&proof.frame, fixture.width, 3, fixture.height / 2);
        let right_mid = pixel_at(
            &proof.frame,
            fixture.width,
            fixture.width - 4,
            fixture.height / 2,
        );
        let nonblack_pixels = nonblack_pixel_count(&proof.frame);
        let rgba_hash = rgba_fnv1a64(&proof.frame);

        if !rgba_within_tolerance(center, reference.center_rgba, reference.max_abs_diff)
            || !rgba_within_tolerance(left_mid, reference.left_mid_rgba, reference.max_abs_diff)
            || !rgba_within_tolerance(right_mid, reference.right_mid_rgba, reference.max_abs_diff)
            || nonblack_pixels != reference.nonblack_pixels
            || rgba_hash != reference.rgba_hash
        {
            mismatches.push(format!(
                "{}: center={:?} left_mid={:?} right_mid={:?} nonblack_pixels={} rgba_hash=\"{}\"",
                fixture.name, center, left_mid, right_mid, nonblack_pixels, rgba_hash
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
    render: fn() -> VisualProof,
    validate: fn(&VisualProof),
}

struct VisualProof {
    frame: Vec<u8>,
    stats: scena::RendererStats,
}

#[derive(Debug, Clone)]
struct ReferenceSpec {
    name: String,
    max_abs_diff: u8,
    center_rgba: [u8; 4],
    left_mid_rgba: [u8; 4],
    right_mid_rgba: [u8; 4],
    nonblack_pixels: usize,
    rgba_hash: String,
}

fn visual_fixtures() -> [VisualFixture; 5] {
    [
        VisualFixture {
            name: "direct-lights-pbr",
            width: 16,
            height: 16,
            render: render_direct_lights_pbr,
            validate: validate_direct_lights,
        },
        VisualFixture {
            name: "shadowed-directional-light",
            width: 16,
            height: 16,
            render: render_shadowed_directional_light,
            validate: validate_shadowed_directional_light,
        },
        VisualFixture {
            name: "ibl-environment",
            width: 16,
            height: 16,
            render: render_ibl_environment,
            validate: validate_ibl_environment,
        },
        VisualFixture {
            name: "fxaa-edge",
            width: 16,
            height: 16,
            render: render_fxaa_edge,
            validate: validate_fxaa_edge,
        },
        VisualFixture {
            name: "clipping-half-space",
            width: 16,
            height: 16,
            render: render_clipping_half_space,
            validate: validate_clipping_half_space,
        },
    ]
}

fn render_direct_lights_pbr() -> VisualProof {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let (mut scene, _camera) = scene_with_camera();
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))
                .with_illuminance_lux(10_000.0),
        )
        .add()
        .expect("red directional light inserts");
    scene.mesh(geometry, material).add().expect("mesh inserts");
    render_scene_with_assets(scene, &assets)
}

fn render_shadowed_directional_light() -> VisualProof {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let (mut scene, _camera) = scene_with_camera();
    scene
        .directional_light(DirectionalLight::default().with_shadows(true))
        .add()
        .expect("shadowed directional light inserts");
    scene.mesh(geometry, material).add().expect("mesh inserts");
    render_scene_with_assets(scene, &assets)
}

fn render_ibl_environment() -> VisualProof {
    let assets = Assets::new();
    let environment =
        pollster::block_on(assets.load_environment("tests/assets/environment/studio_1024x512.hdr"))
            .expect("equirectangular HDR environment loads");
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let (mut scene, _camera) = scene_with_camera();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("IBL scene prepares");
    renderer
        .render_active(&scene)
        .expect("IBL scene renders through active camera");
    VisualProof {
        frame: renderer.frame_rgba8().to_vec(),
        stats: renderer.stats(),
    }
}

fn render_fxaa_edge() -> VisualProof {
    let (mut scene, _camera) = scene_with_camera();
    scene
        .add_renderable(
            scene.root(),
            vec![
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-1.0, -1.0, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.0, -1.0, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.0, 1.0, 0.0),
                        color: Color::WHITE,
                    },
                ]),
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-1.0, -1.0, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.0, 1.0, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(-1.0, 1.0, 0.0),
                        color: Color::WHITE,
                    },
                ]),
            ],
            Transform::default(),
        )
        .expect("FXAA fixture primitives insert");
    render_scene(scene)
}

fn render_clipping_half_space() -> VisualProof {
    let (mut scene, _camera) = scene_with_camera();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-1.0, -1.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(3.0, -1.0, 0.0),
                    color: Color::WHITE,
                },
                Vertex {
                    position: Vec3::new(-1.0, 3.0, 0.0),
                    color: Color::WHITE,
                },
            ])],
            Transform::default(),
        )
        .expect("clipping fixture primitive inserts");
    let plane = scene.add_clipping_plane(ClippingPlane::new(Vec3::new(1.0, 0.0, 0.0), 0.0));
    scene
        .set_clipping_planes(ClippingPlaneSet::new().with_plane(plane))
        .expect("clipping plane activates");
    render_scene(scene)
}

fn render_scene_with_assets(mut scene: Scene, assets: &Assets) -> VisualProof {
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("scene prepares with assets");
    renderer
        .render_active(&scene)
        .expect("scene renders through active camera");
    VisualProof {
        frame: renderer.frame_rgba8().to_vec(),
        stats: renderer.stats(),
    }
}

fn render_scene(mut scene: Scene) -> VisualProof {
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render_active(&scene)
        .expect("scene renders through active camera");
    VisualProof {
        frame: renderer.frame_rgba8().to_vec(),
        stats: renderer.stats(),
    }
}

fn scene_with_camera() -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    (scene, camera)
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

fn validate_direct_lights(proof: &VisualProof) {
    let pixel = pixel_at(&proof.frame, 16, 8, 8);
    assert!(
        pixel[0] > 100 && pixel[1] <= 1 && pixel[2] <= 2 && pixel[3] == 255,
        "direct-light fixture should stay red-dominant after PBR preview shading, got {pixel:?}",
    );
    assert_eq!(
        proof.stats.depth_prepass_passes, 0,
        "single-primitive visual fixtures should use the trivial-scene depth-prepass skip path"
    );
}

fn validate_shadowed_directional_light(proof: &VisualProof) {
    assert_eq!(proof.stats.shadow_maps, 1);
    assert_eq!(proof.stats.directional_shadow_map_resolution, Some(2048));
    assert_eq!(proof.stats.directional_shadow_pcf_kernel, Some(3));
}

fn validate_ibl_environment(proof: &VisualProof) {
    assert_eq!(proof.stats.environment_cubemaps, 1);
    assert_eq!(proof.stats.environment_prefilter_passes, 1);
    assert_eq!(proof.stats.environment_brdf_luts, 1);
}

fn validate_fxaa_edge(proof: &VisualProof) {
    assert_eq!(proof.stats.fxaa_passes, 1);
    assert_eq!(pixel_at(&proof.frame, 16, 12, 8), [0, 0, 0, 255]);
    assert!(pixel_at(&proof.frame, 16, 8, 8)[0] > 0);
}

fn validate_clipping_half_space(proof: &VisualProof) {
    assert_eq!(pixel_at(&proof.frame, 16, 3, 8), [0, 0, 0, 255]);
    assert_eq!(pixel_at(&proof.frame, 16, 12, 8), [206, 206, 206, 255]);
}

fn reference_specs() -> Vec<ReferenceSpec> {
    let mut references = Vec::new();
    let mut current: Option<ReferenceSpec> = None;

    for line in M2_HEADLESS_REFERENCE_METADATA.lines() {
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
                left_mid_rgba: [0; 4],
                right_mid_rgba: [0; 4],
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
        } else if let Some(value) = line.strip_prefix("left_mid_rgba = ") {
            reference.left_mid_rgba = parse_rgba(value);
        } else if let Some(value) = line.strip_prefix("right_mid_rgba = ") {
            reference.right_mid_rgba = parse_rgba(value);
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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m2-visual")
}
