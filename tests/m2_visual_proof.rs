#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use base64::Engine;
use scena::{
    AntiAliasing, Assets, ClippingPlane, ClippingPlaneSet, Color, DirectionalLight, GeometryDesc,
    GeometryTopology, MaterialDesc, OrderIndependentTransparencyConfig, PerspectiveCamera,
    PostBloomConfig, Primitive, Renderer, Scene, ScreenSpaceAmbientOcclusionConfig, Transform,
    Vec3, Vertex,
};
use serde::Serialize;

const M2_HEADLESS_FIXTURE_METADATA: &str = include_str!("visual/fixtures/m2-headless-core.toml");
const M2_HEADLESS_REFERENCE_METADATA: &str =
    include_str!("visual/references/m2-headless-core.toml");
const M2_HEADLESS_REFERENCE_FRAMES: &str =
    include_str!("visual/references/m2-headless-core-frames.toml");
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
        if let Some(pair) = &proof.effect_pair {
            let failures = effect_pair_failures(pair);
            assert!(
                failures.is_empty(),
                "{} effect-footprint proof failed: {}",
                fixture.name,
                failures.join("; ")
            );
        }
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
            proof.effect_pair.is_some(),
        );
    }
}

#[test]
fn m2_headless_local_structure_references_match_current_fixtures() {
    assert_eq!(
        fixture_reference_mode(),
        reference_mode(),
        "fixture and reference metadata modes must match before evaluation"
    );
    assert_eq!(
        reference_mode(),
        "local-structure-v2",
        "reference metadata mode must match the local SSIM/edge/foreground evaluator"
    );
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("M2 structural artifact directory creates");
    let mut mismatches = Vec::new();

    for fixture in visual_fixtures() {
        let proof = (fixture.render)();
        let (reference_width, reference_height, reference) = reference_frame(fixture.name);
        assert_eq!(
            (reference_width, reference_height),
            (fixture.width, fixture.height),
            "{} committed reference dimensions",
            fixture.name
        );
        let report =
            structural_reference_report(&proof.frame, &reference, fixture.width, fixture.height);
        write_structural_reference_artifacts(
            &artifact_dir,
            fixture.name,
            &proof.frame,
            &reference,
            fixture.width,
            fixture.height,
            &report,
        );
        if !report.passed {
            mismatches.push(format!(
                "{}: mean_ssim={:.4} worst_ssim={:.4} edge_iou={:.4} \
                 foreground_iou={:.4} worst_region={:?}; candidate_quadrants={:?} \
                 reference_quadrants={:?}",
                fixture.name,
                report.mean_window_ssim,
                report.worst_window_ssim,
                report.edge_iou,
                report.foreground_iou,
                report.worst_region,
                report.debug_candidate_quadrants,
                report.debug_reference_quadrants,
            ));
        }
    }

    assert!(
        mismatches.is_empty(),
        "visual reference mismatches:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn q03_quadrant_debug_rows_notice_coarse_corruption() {
    let fixture = visual_fixtures()
        .into_iter()
        .find(|fixture| fixture.name == "direct-lights-pbr")
        .expect("direct-light fixture exists");
    let reference = reference_specs()
        .into_iter()
        .find(|reference| reference.name == fixture.name)
        .expect("direct-light reference exists");
    let mut frame = (fixture.render)().frame;
    for y in 0..4 {
        for x in fixture.width / 2..fixture.width / 2 + 4 {
            let offset = ((y * fixture.width + x) * 4) as usize;
            if frame[offset..offset + 3] != [0, 0, 0] {
                frame[offset..offset + 4].copy_from_slice(&[0, 0, 164, 255]);
            }
        }
    }

    assert!(
        !quadrant_debug_rows_match(&frame, fixture.width, fixture.height, &reference, 4),
        "diagnostic quadrant rows should still notice a coarse color corruption"
    );
}

#[test]
fn q03_structure_oracle_rejects_quadrant_mean_preserving_mutations() {
    let fixture = visual_fixtures()
        .into_iter()
        .find(|fixture| fixture.name == "shadowed-directional-light")
        .expect("shadow fixture exists");
    let reference = (fixture.render)().frame;
    let expected_quadrants = quadrant_metrics(&reference, fixture.width, fixture.height);
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("Q03 mutation artifact directory creates");
    for (name, candidate) in [
        (
            "moved-structure",
            rotate_each_quadrant_180(&reference, fixture.width, fixture.height),
        ),
        (
            "structure-collapsed-to-stripes",
            sort_each_quadrant_by_luminance(&reference, fixture.width, fixture.height),
        ),
    ] {
        assert_eq!(
            quadrant_metrics(&candidate, fixture.width, fixture.height),
            expected_quadrants,
            "{name} must preserve the legacy quadrant means and occupancy exactly"
        );
        let report =
            structural_reference_report(&candidate, &reference, fixture.width, fixture.height);
        write_structural_reference_artifacts(
            &artifact_dir,
            &format!("q03-{name}"),
            &candidate,
            &reference,
            fixture.width,
            fixture.height,
            &report,
        );
        assert!(
            !report.passed,
            "{name} must fail the local structure oracle despite preserving broad means: {report:#?}"
        );
    }
}

#[test]
fn q05_effect_footprint_masks_reject_erased_effect_regions() {
    let mut paired_effects = 0;
    for fixture in visual_fixtures() {
        let proof = (fixture.render)();
        let Some(pair) = proof.effect_pair else {
            continue;
        };
        paired_effects += 1;
        assert!(
            effect_pair_failures(&pair).is_empty(),
            "{} starts as a valid paired proof",
            pair.name
        );

        let mut corrupted = pair.clone();
        for y in pair.mask.y_min..pair.mask.y_max {
            for x in pair.mask.x_min..pair.mask.x_max {
                let offset = ((y * pair.width + x) * 4) as usize;
                corrupted.on[offset..offset + 4].copy_from_slice(&pair.off[offset..offset + 4]);
            }
        }
        assert!(
            !effect_pair_failures(&corrupted).is_empty(),
            "{} must reject an on-frame whose declared effect mask was copied from off",
            pair.name
        );
    }
    assert_eq!(
        paired_effects, 8,
        "direct light, receiver shadow, IBL, AA, bloom, SSAO, OIT, and clipping all require paired proofs"
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
    effect_pair: Option<EffectPair>,
}

#[derive(Debug, Clone)]
struct EffectPair {
    name: &'static str,
    off: Vec<u8>,
    on: Vec<u8>,
    width: u32,
    height: u32,
    mask: PixelMask,
    min_changed_pixels: usize,
    min_mean_abs_rgb_delta: f32,
    luma_direction: Option<(LumaDirection, f32)>,
}

#[derive(Debug, Clone, Copy)]
struct PixelMask {
    x_min: u32,
    y_min: u32,
    x_max: u32,
    y_max: u32,
}

impl PixelMask {
    const fn new(x_min: u32, y_min: u32, x_max: u32, y_max: u32) -> Self {
        Self {
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    const fn full(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }
}

#[derive(Debug, Clone, Copy)]
enum LumaDirection {
    Lighten,
    Darken,
}

#[derive(Debug, Clone)]
struct ReferenceSpec {
    name: String,
    max_abs_diff: u8,
    quadrant_mean_rgba: [[u8; 4]; 4],
    quadrant_nonblack: [usize; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct QuadrantMetrics {
    mean_rgba: [[u8; 4]; 4],
    nonblack: [usize; 4],
}

const STRUCTURE_WINDOW_SIZE: u32 = 8;
const STRUCTURE_WINDOW_STRIDE: u32 = 4;
const STRUCTURE_MEAN_SSIM_MIN: f32 = 0.97;
const STRUCTURE_WORST_WINDOW_SSIM_MIN: f32 = 0.70;
const STRUCTURE_EDGE_IOU_MIN: f32 = 0.85;
const STRUCTURE_FOREGROUND_IOU_MIN: f32 = 0.95;

#[derive(Debug, Clone, Serialize)]
struct StructuralReferenceReport {
    passed: bool,
    mean_window_ssim: f32,
    worst_window_ssim: f32,
    edge_iou: f32,
    foreground_iou: f32,
    worst_region: [u32; 4],
    debug_candidate_quadrants: QuadrantMetrics,
    debug_reference_quadrants: QuadrantMetrics,
}

fn visual_fixtures() -> [VisualFixture; 9] {
    [
        VisualFixture {
            name: "direct-lights-pbr",
            width: 32,
            height: 16,
            render: render_direct_lights_pbr,
            validate: validate_direct_lights,
        },
        VisualFixture {
            name: "shadowed-directional-light",
            width: 160,
            height: 80,
            render: render_shadowed_directional_light,
            validate: validate_shadowed_directional_light,
        },
        VisualFixture {
            name: "ibl-environment",
            width: 32,
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
            name: "anti-aliasing-on-off",
            width: 32,
            height: 16,
            render: render_anti_aliasing_on_off,
            validate: validate_anti_aliasing_on_off,
        },
        VisualFixture {
            name: "bloom-on-off",
            width: 32,
            height: 16,
            render: render_bloom_on_off,
            validate: validate_bloom_on_off,
        },
        VisualFixture {
            name: "ssao-contact-on-off",
            width: 32,
            height: 16,
            render: render_ssao_contact_on_off,
            validate: validate_ssao_contact_on_off,
        },
        VisualFixture {
            name: "oit-overlap-order-invariance",
            width: 32,
            height: 16,
            render: render_oit_overlap_order_invariance,
            validate: validate_oit_overlap_order_invariance,
        },
        VisualFixture {
            name: "clipping-half-space",
            width: 32,
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
    let (mut off_scene, _camera) = scene_with_camera();
    off_scene
        .mesh(geometry, material)
        .add()
        .expect("unlit comparison mesh inserts");
    let (off, _) = render_scene_with_assets_frame(off_scene, &assets, 16, 16);

    let (mut on_scene, _camera) = scene_with_camera();
    on_scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))
                .with_illuminance_lux(12_000.0),
        )
        .add()
        .expect("red directional light inserts");
    on_scene
        .mesh(geometry, material)
        .add()
        .expect("lit comparison mesh inserts");
    let (on, stats) = render_scene_with_assets_frame(on_scene, &assets, 16, 16);

    paired_visual_proof(
        EffectPair {
            name: "direct-light",
            off,
            on,
            width: 16,
            height: 16,
            mask: PixelMask::full(16, 16),
            min_changed_pixels: 100,
            min_mean_abs_rgb_delta: 20.0,
            luma_direction: None,
        },
        stats,
    )
}

fn render_shadowed_directional_light() -> VisualProof {
    let (off, _) = render_shadow_receiver_frame(false);
    let (on, stats) = render_shadow_receiver_frame(true);
    paired_visual_proof(
        EffectPair {
            name: "receiver-shadow",
            off,
            on,
            width: 80,
            height: 80,
            mask: PixelMask::new(30, 30, 50, 50),
            min_changed_pixels: 20,
            min_mean_abs_rgb_delta: 2.0,
            luma_direction: Some((LumaDirection::Darken, 4.0)),
        },
        stats,
    )
}

fn render_shadow_receiver_frame(with_caster: bool) -> (Vec<u8>, scena::RendererStats) {
    let assets = Assets::new();
    let receiver = assets.create_geometry(shadow_receiver_geometry());
    let caster = assets.create_geometry(shadow_caster_geometry());
    let receiver_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let caster_material = assets.create_material(MaterialDesc::unlit(Color::BLACK));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("shadow camera inserts");
    scene
        .set_active_camera(camera)
        .expect("shadow camera becomes active");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_illuminance_lux(10_000.0)
                .with_shadows(true),
        )
        .transform(Transform::IDENTITY.rotate_y_deg(30.0))
        .add()
        .expect("shadowed directional light inserts");
    scene
        .mesh(receiver, receiver_material)
        .add()
        .expect("shadow receiver inserts");
    if with_caster {
        scene
            .mesh(caster, caster_material)
            .transform(Transform::at(Vec3::new(0.29, 0.0, 0.50)))
            .add()
            .expect("shadow caster inserts");
    }
    render_scene_with_assets_frame(scene, &assets, 80, 80)
}

fn render_ibl_environment() -> VisualProof {
    let assets = Assets::new();
    let environment = pollster::block_on(
        assets.load_environment("tests/assets/environment/polyhaven/studio_small_08_2k.hdr"),
    )
    .expect("equirectangular HDR environment loads");
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let (mut off_scene, _camera) = scene_with_camera();
    off_scene
        .mesh(geometry, material)
        .add()
        .expect("IBL-off mesh inserts");
    let (off, _) = render_scene_with_assets_frame(off_scene, &assets, 16, 16);

    let (mut on_scene, _camera) = scene_with_camera();
    on_scene
        .mesh(geometry, material)
        .add()
        .expect("IBL-on mesh inserts");
    let mut on_renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    on_renderer.set_environment(environment);
    on_renderer
        .prepare_with_assets(&mut on_scene, &assets)
        .expect("IBL scene prepares");
    on_renderer
        .render_active(&on_scene)
        .expect("IBL scene renders through active camera");
    paired_visual_proof(
        EffectPair {
            name: "ibl-material-response",
            off,
            on: on_renderer.frame_rgba8().to_vec(),
            width: 16,
            height: 16,
            mask: PixelMask::full(16, 16),
            min_changed_pixels: 100,
            min_mean_abs_rgb_delta: 5.0,
            luma_direction: None,
        },
        on_renderer.stats(),
    )
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
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds");
    renderer.set_anti_aliasing(AntiAliasing::Fxaa);
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render_active(&scene)
        .expect("scene renders through active camera");
    VisualProof {
        frame: renderer.frame_rgba8().to_vec(),
        stats: renderer.stats(),
        effect_pair: None,
    }
}

fn render_anti_aliasing_on_off() -> VisualProof {
    let mut off_scene = fxaa_edge_scene();
    let mut off_renderer = Renderer::headless(16, 16).expect("AA-off renderer builds");
    off_renderer.set_anti_aliasing(AntiAliasing::None);
    off_renderer
        .prepare(&mut off_scene)
        .expect("AA-off scene prepares");
    off_renderer
        .render_active(&off_scene)
        .expect("AA-off scene renders");

    let mut on_scene = fxaa_edge_scene();
    let mut on_renderer = Renderer::headless(16, 16).expect("AA-on renderer builds");
    on_renderer.set_anti_aliasing(AntiAliasing::Fxaa);
    on_renderer
        .prepare(&mut on_scene)
        .expect("AA-on scene prepares");
    on_renderer
        .render_active(&on_scene)
        .expect("AA-on scene renders");

    paired_visual_proof(
        EffectPair {
            name: "anti-aliasing",
            off: off_renderer.frame_rgba8().to_vec(),
            on: on_renderer.frame_rgba8().to_vec(),
            width: 16,
            height: 16,
            mask: PixelMask::new(6, 0, 10, 16),
            min_changed_pixels: 8,
            min_mean_abs_rgb_delta: 1.0,
            luma_direction: None,
        },
        on_renderer.stats(),
    )
}

fn fxaa_edge_scene() -> Scene {
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
    scene
}

fn render_bloom_on_off() -> VisualProof {
    let mut off_scene = bloom_highlight_scene();
    let mut off_renderer = Renderer::headless(16, 16).expect("bloom-off renderer builds");
    off_renderer
        .prepare(&mut off_scene)
        .expect("bloom-off scene prepares");
    off_renderer
        .render_active(&off_scene)
        .expect("bloom-off scene renders");

    let mut on_scene = bloom_highlight_scene();
    let mut on_renderer = Renderer::headless(16, 16).expect("bloom-on renderer builds");
    on_renderer.set_bloom(Some(PostBloomConfig::subtle()));
    on_renderer
        .prepare(&mut on_scene)
        .expect("bloom-on scene prepares");
    on_renderer
        .render_active(&on_scene)
        .expect("bloom-on scene renders");

    paired_visual_proof(
        EffectPair {
            name: "bloom",
            off: off_renderer.frame_rgba8().to_vec(),
            on: on_renderer.frame_rgba8().to_vec(),
            width: 16,
            height: 16,
            mask: PixelMask::new(3, 3, 13, 13),
            min_changed_pixels: 12,
            min_mean_abs_rgb_delta: 0.5,
            luma_direction: Some((LumaDirection::Lighten, 0.5)),
        },
        on_renderer.stats(),
    )
}

fn bloom_highlight_scene() -> Scene {
    let (mut scene, _camera) = scene_with_camera();
    scene
        .add_renderable(
            scene.root(),
            vec![
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.2, -0.2, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.2, -0.2, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.2, 0.2, 0.0),
                        color: Color::WHITE,
                    },
                ]),
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.2, -0.2, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.2, 0.2, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(-0.2, 0.2, 0.0),
                        color: Color::WHITE,
                    },
                ]),
            ],
            Transform::default(),
        )
        .expect("bloom highlight primitives insert");
    scene
}

fn render_ssao_contact_on_off() -> VisualProof {
    let mut off_scene = depth_contact_scene();
    let mut off_renderer = Renderer::headless(16, 16).expect("SSAO-off renderer builds");
    off_renderer
        .prepare(&mut off_scene)
        .expect("SSAO-off scene prepares");
    off_renderer
        .render_active(&off_scene)
        .expect("SSAO-off scene renders");

    let mut on_scene = depth_contact_scene();
    let mut on_renderer = Renderer::headless(16, 16).expect("SSAO-on renderer builds");
    on_renderer.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::new(
        4, 0.8, 0.0,
    )));
    on_renderer
        .prepare(&mut on_scene)
        .expect("SSAO-on scene prepares");
    on_renderer
        .render_active(&on_scene)
        .expect("SSAO-on scene renders");

    paired_visual_proof(
        EffectPair {
            name: "ssao",
            off: off_renderer.frame_rgba8().to_vec(),
            on: on_renderer.frame_rgba8().to_vec(),
            width: 16,
            height: 16,
            mask: PixelMask::new(1, 2, 15, 14),
            min_changed_pixels: 20,
            min_mean_abs_rgb_delta: 1.0,
            luma_direction: Some((LumaDirection::Darken, 0.5)),
        },
        on_renderer.stats(),
    )
}

fn depth_contact_scene() -> Scene {
    let (mut scene, _camera) = scene_with_camera();
    scene
        .add_renderable(
            scene.root(),
            vec![
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.75, -0.55, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.75, -0.55, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.75, 0.35, 0.0),
                        color: Color::WHITE,
                    },
                ]),
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.75, -0.55, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(0.75, 0.35, 0.0),
                        color: Color::WHITE,
                    },
                    Vertex {
                        position: Vec3::new(-0.75, 0.35, 0.0),
                        color: Color::WHITE,
                    },
                ]),
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.14, -0.18, 0.16),
                        color: Color::from_linear_rgb(0.72, 0.72, 0.72),
                    },
                    Vertex {
                        position: Vec3::new(0.14, -0.18, 0.16),
                        color: Color::from_linear_rgb(0.72, 0.72, 0.72),
                    },
                    Vertex {
                        position: Vec3::new(0.14, 0.18, 0.16),
                        color: Color::from_linear_rgb(0.72, 0.72, 0.72),
                    },
                ]),
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.14, -0.18, 0.16),
                        color: Color::from_linear_rgb(0.72, 0.72, 0.72),
                    },
                    Vertex {
                        position: Vec3::new(0.14, 0.18, 0.16),
                        color: Color::from_linear_rgb(0.72, 0.72, 0.72),
                    },
                    Vertex {
                        position: Vec3::new(-0.14, 0.18, 0.16),
                        color: Color::from_linear_rgb(0.72, 0.72, 0.72),
                    },
                ]),
            ],
            Transform::default(),
        )
        .expect("SSAO contact primitives insert");
    scene
}

fn render_clipping_half_space() -> VisualProof {
    let (off, _) = render_scene_frame(clipping_scene(false), 16, 16);
    let (on, stats) = render_scene_frame(clipping_scene(true), 16, 16);
    paired_visual_proof(
        EffectPair {
            name: "clipping",
            off,
            on,
            width: 16,
            height: 16,
            mask: PixelMask::new(0, 0, 8, 16),
            min_changed_pixels: 50,
            min_mean_abs_rgb_delta: 20.0,
            luma_direction: Some((LumaDirection::Darken, 20.0)),
        },
        stats,
    )
}

fn clipping_scene(with_plane: bool) -> Scene {
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
    if with_plane {
        let plane = scene.add_clipping_plane(ClippingPlane::new(Vec3::new(1.0, 0.0, 0.0), 0.0));
        scene
            .set_clipping_planes(ClippingPlaneSet::new().with_plane(plane))
            .expect("clipping plane activates");
    }
    scene
}

fn render_oit_overlap_order_invariance() -> VisualProof {
    let (off, off_stats) = render_oit_overlap_scene(true, false);
    let (on, stats) = render_oit_overlap_scene(true, true);
    let (on_reversed, reversed_stats) = render_oit_overlap_scene(false, true);
    assert_eq!(off_stats.order_independent_transparency_passes, 0);
    assert_eq!(stats.order_independent_transparency_passes, 1);
    assert_eq!(reversed_stats.order_independent_transparency_passes, 1);
    assert_eq!(
        on, on_reversed,
        "weighted OIT output must be insertion-order invariant across the complete frame"
    );
    paired_visual_proof(
        EffectPair {
            name: "order-independent-transparency",
            off,
            on,
            width: 16,
            height: 16,
            mask: PixelMask::new(2, 2, 14, 14),
            min_changed_pixels: 20,
            min_mean_abs_rgb_delta: 1.0,
            luma_direction: None,
        },
        stats,
    )
}

fn render_oit_overlap_scene(red_first: bool, oit: bool) -> (Vec<u8>, scena::RendererStats) {
    let mut scene = overlapping_transparency_scene(red_first);
    let mut renderer = Renderer::headless(16, 16).expect("OIT renderer builds");
    renderer.set_anti_aliasing(AntiAliasing::None);
    if oit {
        renderer.set_order_independent_transparency(Some(
            OrderIndependentTransparencyConfig::weighted_blended(),
        ));
    }
    renderer.prepare(&mut scene).expect("OIT scene prepares");
    renderer
        .render_active(&scene)
        .expect("OIT scene renders through active camera");
    (renderer.frame_rgba8().to_vec(), renderer.stats())
}

fn render_scene_with_assets_frame(
    mut scene: Scene,
    assets: &Assets,
    width: u32,
    height: u32,
) -> (Vec<u8>, scena::RendererStats) {
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("scene prepares with assets");
    renderer
        .render_active(&scene)
        .expect("scene renders through active camera");
    (renderer.frame_rgba8().to_vec(), renderer.stats())
}

fn render_scene_frame(
    mut scene: Scene,
    width: u32,
    height: u32,
) -> (Vec<u8>, scena::RendererStats) {
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer
        .render_active(&scene)
        .expect("scene renders through active camera");
    (renderer.frame_rgba8().to_vec(), renderer.stats())
}

fn paired_visual_proof(pair: EffectPair, stats: scena::RendererStats) -> VisualProof {
    let expected_len = pair.width as usize * pair.height as usize * 4;
    assert_eq!(
        pair.off.len(),
        expected_len,
        "off frame has declared dimensions"
    );
    assert_eq!(
        pair.on.len(),
        expected_len,
        "on frame has declared dimensions"
    );
    let mut frame = vec![0_u8; expected_len * 2];
    blit_frame(
        &pair.off,
        pair.width,
        pair.height,
        &mut frame,
        pair.width * 2,
        0,
        0,
    );
    blit_frame(
        &pair.on,
        pair.width,
        pair.height,
        &mut frame,
        pair.width * 2,
        pair.width,
        0,
    );
    VisualProof {
        frame,
        stats,
        effect_pair: Some(pair),
    }
}

fn effect_pair_failures(pair: &EffectPair) -> Vec<String> {
    let mut failures = Vec::new();
    let expected_len = pair.width as usize * pair.height as usize * 4;
    if pair.off.len() != expected_len || pair.on.len() != expected_len {
        failures.push(format!(
            "dimension mismatch off={} on={} expected={expected_len}",
            pair.off.len(),
            pair.on.len()
        ));
        return failures;
    }
    if pair.mask.x_min >= pair.mask.x_max
        || pair.mask.y_min >= pair.mask.y_max
        || pair.mask.x_max > pair.width
        || pair.mask.y_max > pair.height
    {
        failures.push(format!("invalid spatial mask {:?}", pair.mask));
        return failures;
    }

    let mut changed_pixels = 0_usize;
    let mut absolute_rgb_delta = 0_u64;
    let mut signed_luma_delta = 0_f32;
    let mut samples = 0_usize;
    for y in pair.mask.y_min..pair.mask.y_max {
        for x in pair.mask.x_min..pair.mask.x_max {
            let offset = ((y * pair.width + x) * 4) as usize;
            let off = &pair.off[offset..offset + 4];
            let on = &pair.on[offset..offset + 4];
            let deltas = [
                off[0].abs_diff(on[0]),
                off[1].abs_diff(on[1]),
                off[2].abs_diff(on[2]),
            ];
            if deltas.into_iter().max().unwrap_or(0) > 2 {
                changed_pixels += 1;
            }
            absolute_rgb_delta += deltas.into_iter().map(u64::from).sum::<u64>();
            let off_luma = 0.2126 * f32::from(off[0])
                + 0.7152 * f32::from(off[1])
                + 0.0722 * f32::from(off[2]);
            let on_luma =
                0.2126 * f32::from(on[0]) + 0.7152 * f32::from(on[1]) + 0.0722 * f32::from(on[2]);
            signed_luma_delta += on_luma - off_luma;
            samples += 1;
        }
    }
    let mean_abs_rgb_delta = absolute_rgb_delta as f32 / (samples.max(1) * 3) as f32;
    let mean_luma_delta = signed_luma_delta / samples.max(1) as f32;
    if changed_pixels < pair.min_changed_pixels {
        failures.push(format!(
            "changed_pixels={changed_pixels} below {} inside {:?}",
            pair.min_changed_pixels, pair.mask
        ));
    }
    if mean_abs_rgb_delta < pair.min_mean_abs_rgb_delta {
        failures.push(format!(
            "mean_abs_rgb_delta={mean_abs_rgb_delta:.3} below {:.3}",
            pair.min_mean_abs_rgb_delta
        ));
    }
    if let Some((direction, minimum)) = pair.luma_direction {
        let directional_delta = match direction {
            LumaDirection::Lighten => mean_luma_delta,
            LumaDirection::Darken => -mean_luma_delta,
        };
        if directional_delta < minimum {
            failures.push(format!(
                "{direction:?} mean_luma_delta={mean_luma_delta:.3} below directional minimum {minimum:.3}"
            ));
        }
    }
    failures
}

fn overlapping_transparency_scene(red_first: bool) -> Scene {
    let (mut scene, _camera) = scene_with_camera();
    let red = Primitive::triangle([
        Vertex {
            position: Vec3::new(-0.72, -0.72, 0.08),
            color: Color::from_linear_rgba(1.0, 0.0, 0.0, 0.45),
        },
        Vertex {
            position: Vec3::new(0.72, -0.72, 0.08),
            color: Color::from_linear_rgba(1.0, 0.0, 0.0, 0.45),
        },
        Vertex {
            position: Vec3::new(0.0, 0.72, 0.08),
            color: Color::from_linear_rgba(1.0, 0.0, 0.0, 0.45),
        },
    ]);
    let green = Primitive::triangle([
        Vertex {
            position: Vec3::new(-0.72, -0.72, -0.08),
            color: Color::from_linear_rgba(0.0, 1.0, 0.0, 0.45),
        },
        Vertex {
            position: Vec3::new(0.72, -0.72, -0.08),
            color: Color::from_linear_rgba(0.0, 1.0, 0.0, 0.45),
        },
        Vertex {
            position: Vec3::new(0.0, 0.72, -0.08),
            color: Color::from_linear_rgba(0.0, 1.0, 0.0, 0.45),
        },
    ]);
    let primitives = if red_first {
        vec![red, green]
    } else {
        vec![green, red]
    };
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("transparent overlap inserts");
    scene
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

fn shadow_receiver_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.15, -0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.15, -0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.15, 0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-0.15, 0.18, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("shadow receiver geometry is valid")
}

fn shadow_caster_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.23, -0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.23, -0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.23, 0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-0.23, 0.24, 0.0),
                normal: Vec3::new(0.0, 0.0, -1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("shadow caster geometry is valid")
}

fn validate_direct_lights(proof: &VisualProof) {
    let pixel = pixel_at(&proof.frame, 32, 24, 8);
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
    let pair = proof.effect_pair.as_ref().expect("shadow has paired proof");
    let lit_center = pixel_at(&pair.off, 80, 40, 40);
    let shadowed_center = pixel_at(&pair.on, 80, 40, 40);
    assert!(
        shadowed_center[0] + 30 < lit_center[0]
            && shadowed_center[1] + 30 < lit_center[1]
            && shadowed_center[2] + 30 < lit_center[2],
        "shadowed receiver center must be visibly darker; lit={lit_center:?} shadowed={shadowed_center:?}"
    );
}

fn validate_ibl_environment(proof: &VisualProof) {
    assert_eq!(proof.stats.environment_cubemaps, 1);
    assert_eq!(proof.stats.environment_prefilter_passes, 1);
    assert_eq!(proof.stats.environment_brdf_luts, 1);
    let pair = proof.effect_pair.as_ref().expect("IBL has paired proof");
    let off = pixel_at(&pair.off, 16, 8, 8);
    let on = pixel_at(&pair.on, 16, 8, 8);
    assert!(
        on[0].abs_diff(off[0]) > 8 || on[1].abs_diff(off[1]) > 8 || on[2].abs_diff(off[2]) > 8,
        "IBL must visibly change the PBR material response; off={off:?} on={on:?}"
    );
}

fn validate_fxaa_edge(proof: &VisualProof) {
    assert_eq!(proof.stats.fxaa_passes, 1);
    assert_eq!(pixel_at(&proof.frame, 16, 12, 8), [0, 0, 0, 255]);
    assert!(pixel_at(&proof.frame, 16, 8, 8)[0] > 0);
}

fn validate_anti_aliasing_on_off(proof: &VisualProof) {
    assert_eq!(proof.stats.fxaa_passes, 1);
    let aliased_dark_edge = pixel_at(&proof.frame, 32, 8, 8);
    let smoothed_dark_edge = pixel_at(&proof.frame, 32, 24, 8);
    assert_eq!(aliased_dark_edge, [0, 0, 0, 255]);
    assert!(
        smoothed_dark_edge[0] > 20,
        "right-side FXAA proof should smooth the dark side of the hard edge; \
         off={aliased_dark_edge:?} on={smoothed_dark_edge:?}",
    );
}

fn validate_bloom_on_off(proof: &VisualProof) {
    assert_eq!(proof.stats.bloom_passes, 1);
    let off_halo = pixel_at(&proof.frame, 32, 11, 8);
    let on_halo = pixel_at(&proof.frame, 32, 27, 8);
    assert!(
        on_halo[0] > off_halo[0] + 2
            && on_halo[1] > off_halo[1] + 2
            && on_halo[2] > off_halo[2] + 2,
        "right-side bloom proof must show a halo compared with the left-side off render; \
         off={off_halo:?} on={on_halo:?}",
    );
}

fn validate_ssao_contact_on_off(proof: &VisualProof) {
    assert_eq!(proof.stats.ambient_occlusion_passes, 1);
    let mut strongest_drop = 0_i16;
    let mut strongest_sample = ([0_u8; 4], [0_u8; 4], 0_u32, 0_u32);
    for y in 0..16 {
        for x in 0..16 {
            let off_contact = pixel_at(&proof.frame, 32, x, y);
            let on_contact = pixel_at(&proof.frame, 32, x + 16, y);
            let off_luma =
                (i16::from(off_contact[0]) + i16::from(off_contact[1]) + i16::from(off_contact[2]))
                    / 3;
            let on_luma =
                (i16::from(on_contact[0]) + i16::from(on_contact[1]) + i16::from(on_contact[2]))
                    / 3;
            let drop = off_luma - on_luma;
            if drop > strongest_drop {
                strongest_drop = drop;
                strongest_sample = (off_contact, on_contact, x, y);
            }
        }
    }
    assert!(
        strongest_drop > 12,
        "right-side SSAO proof must darken the depth contact compared with the off render; \
         strongest_drop={strongest_drop} sample={strongest_sample:?}",
    );
}

fn validate_oit_overlap_order_invariance(proof: &VisualProof) {
    assert_eq!(proof.stats.order_independent_transparency_passes, 1);
    let off = pixel_at(&proof.frame, 32, 8, 8);
    let on = pixel_at(&proof.frame, 32, 24, 8);
    assert_ne!(off, on, "weighted OIT on/off overlap pixels must differ");
    assert!(
        on[0] > 40 && on[1] > 40 && on[2] < 5,
        "resolved overlap should visibly contain the red and green transparent surfaces; got {on:?}",
    );
}

fn validate_clipping_half_space(proof: &VisualProof) {
    assert_eq!(pixel_at(&proof.frame, 32, 3, 8), [240, 240, 240, 255]);
    assert_eq!(pixel_at(&proof.frame, 32, 19, 8), [0, 0, 0, 255]);
    assert_eq!(pixel_at(&proof.frame, 32, 28, 8), [240, 240, 240, 255]);
}

fn reference_frame(name: &str) -> (u32, u32, Vec<u8>) {
    let marker = format!("[[frame]]\nname = \"{name}\"");
    let start = M2_HEADLESS_REFERENCE_FRAMES
        .find(&marker)
        .unwrap_or_else(|| panic!("missing committed structural reference frame {name}"));
    let block = &M2_HEADLESS_REFERENCE_FRAMES[start..];
    let block = block
        .find("\n[[frame]]")
        .map(|next| &block[..next])
        .unwrap_or(block);
    let encoded = block
        .lines()
        .find_map(|line| line.trim().strip_prefix("png_base64 = "))
        .map(parse_quoted)
        .unwrap_or_else(|| panic!("reference frame {name} must provide png_base64"));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap_or_else(|error| panic!("reference frame {name} base64 decodes: {error}"));
    decode_png_rgba8(name, &bytes)
}

fn decode_png_rgba8(name: &str, bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder
        .read_info()
        .unwrap_or_else(|error| panic!("reference frame {name} PNG header decodes: {error}"));
    let mut decoded = vec![
        0;
        reader
            .output_buffer_size()
            .expect("expanded PNG output buffer size is known")
    ];
    let info = reader
        .next_frame(&mut decoded)
        .unwrap_or_else(|error| panic!("reference frame {name} PNG pixels decode: {error}"));
    let decoded = &decoded[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => decoded.to_vec(),
        png::ColorType::Rgb => decoded
            .chunks_exact(3)
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::Grayscale => decoded
            .iter()
            .flat_map(|&value| [value, value, value, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => decoded
            .chunks_exact(2)
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        png::ColorType::Indexed => {
            panic!("reference frame {name} remained indexed after PNG expansion")
        }
    };
    assert_eq!(
        rgba.len(),
        info.width as usize * info.height as usize * 4,
        "reference frame {name} RGBA8 byte count"
    );
    (info.width, info.height, rgba)
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
                quadrant_mean_rgba: [[0; 4]; 4],
                quadrant_nonblack: [0; 4],
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
        } else if let Some(value) = line.strip_prefix("top_left_mean_rgba = ") {
            reference.quadrant_mean_rgba[0] = parse_rgba(value);
        } else if let Some(value) = line.strip_prefix("top_right_mean_rgba = ") {
            reference.quadrant_mean_rgba[1] = parse_rgba(value);
        } else if let Some(value) = line.strip_prefix("bottom_left_mean_rgba = ") {
            reference.quadrant_mean_rgba[2] = parse_rgba(value);
        } else if let Some(value) = line.strip_prefix("bottom_right_mean_rgba = ") {
            reference.quadrant_mean_rgba[3] = parse_rgba(value);
        } else if let Some(value) = line.strip_prefix("quadrant_nonblack = ") {
            reference.quadrant_nonblack = parse_usize4(value);
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

fn parse_usize4(value: &str) -> [usize; 4] {
    parse_array4(value, "quadrant nonblack", |value| {
        value.parse().expect("quadrant nonblack value is usize")
    })
}

fn parse_array4<T>(value: &str, context: &str, parse: impl Fn(&str) -> T) -> [T; 4] {
    let value = value
        .trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or_else(|| panic!("{context} array"));
    value
        .split(',')
        .map(|item| parse(item.trim()))
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_else(|_| panic!("{context} contains four values"))
}

fn rgba_within_tolerance(actual: [u8; 4], expected: [u8; 4], max_abs_diff: u8) -> bool {
    actual
        .into_iter()
        .zip(expected)
        .all(|(actual, expected)| actual.abs_diff(expected) <= max_abs_diff)
}

fn quadrant_debug_rows_match(
    frame: &[u8],
    width: u32,
    height: u32,
    reference: &ReferenceSpec,
    nonblack_tolerance: usize,
) -> bool {
    let actual = quadrant_metrics(frame, width, height);
    actual
        .mean_rgba
        .into_iter()
        .zip(reference.quadrant_mean_rgba)
        .all(|(actual, expected)| rgba_within_tolerance(actual, expected, reference.max_abs_diff))
        && actual
            .nonblack
            .into_iter()
            .zip(reference.quadrant_nonblack)
            .all(|(actual, expected)| actual.abs_diff(expected) <= nonblack_tolerance)
}

fn quadrant_metrics(frame: &[u8], width: u32, height: u32) -> QuadrantMetrics {
    assert_eq!(frame.len(), width as usize * height as usize * 4);
    let mut sums = [[0_u64; 4]; 4];
    let mut counts = [0_u64; 4];
    let mut nonblack = [0_usize; 4];
    for y in 0..height {
        for x in 0..width {
            let quadrant = usize::from(y >= height / 2) * 2 + usize::from(x >= width / 2);
            let pixel = pixel_at(frame, width, x, y);
            counts[quadrant] += 1;
            for channel in 0..4 {
                sums[quadrant][channel] += u64::from(pixel[channel]);
            }
            if pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0 {
                nonblack[quadrant] += 1;
            }
        }
    }
    let mut mean_rgba = [[0_u8; 4]; 4];
    for quadrant in 0..4 {
        for channel in 0..4 {
            mean_rgba[quadrant][channel] =
                ((sums[quadrant][channel] + counts[quadrant] / 2) / counts[quadrant]) as u8;
        }
    }
    QuadrantMetrics {
        mean_rgba,
        nonblack,
    }
}

fn structural_reference_report(
    candidate: &[u8],
    reference: &[u8],
    width: u32,
    height: u32,
) -> StructuralReferenceReport {
    assert_eq!(candidate.len(), reference.len());
    assert_eq!(candidate.len(), width as usize * height as usize * 4);
    let candidate_luma = blurred_luminance(candidate, width, height);
    let reference_luma = blurred_luminance(reference, width, height);
    let window_size = STRUCTURE_WINDOW_SIZE.min(width).min(height).max(1);
    let x_positions = window_positions(width, window_size, STRUCTURE_WINDOW_STRIDE);
    let y_positions = window_positions(height, window_size, STRUCTURE_WINDOW_STRIDE);
    let mut ssim_sum = 0.0_f64;
    let mut window_count = 0_u32;
    let mut worst_window_ssim = f64::INFINITY;
    let mut worst_region = [0, 0, window_size, window_size];
    for &y in &y_positions {
        for &x in &x_positions {
            let ssim = window_ssim(&candidate_luma, &reference_luma, width, x, y, window_size);
            ssim_sum += ssim;
            window_count += 1;
            if ssim < worst_window_ssim {
                worst_window_ssim = ssim;
                worst_region = [x, y, window_size, window_size];
            }
        }
    }
    let mean_window_ssim = (ssim_sum / f64::from(window_count.max(1))) as f32;
    let worst_window_ssim = worst_window_ssim as f32;
    let candidate_edges = dilate_mask(
        &sobel_edge_mask(&candidate_luma, width, height, 18.0),
        width,
        height,
        1,
    );
    let reference_edges = dilate_mask(
        &sobel_edge_mask(&reference_luma, width, height, 18.0),
        width,
        height,
        1,
    );
    let edge_iou = mask_iou(&candidate_edges, &reference_edges);
    let candidate_foreground =
        dilate_mask(&foreground_mask(candidate, width, height), width, height, 1);
    let reference_foreground =
        dilate_mask(&foreground_mask(reference, width, height), width, height, 1);
    let foreground_iou = mask_iou(&candidate_foreground, &reference_foreground);
    StructuralReferenceReport {
        passed: mean_window_ssim >= STRUCTURE_MEAN_SSIM_MIN
            && worst_window_ssim >= STRUCTURE_WORST_WINDOW_SSIM_MIN
            && edge_iou >= STRUCTURE_EDGE_IOU_MIN
            && foreground_iou >= STRUCTURE_FOREGROUND_IOU_MIN,
        mean_window_ssim,
        worst_window_ssim,
        edge_iou,
        foreground_iou,
        worst_region,
        debug_candidate_quadrants: quadrant_metrics(candidate, width, height),
        debug_reference_quadrants: quadrant_metrics(reference, width, height),
    }
}

fn window_positions(length: u32, window: u32, stride: u32) -> Vec<u32> {
    if length <= window {
        return vec![0];
    }
    let mut positions = (0..=length - window)
        .step_by(stride.max(1) as usize)
        .collect::<Vec<_>>();
    let final_position = length - window;
    if positions.last().copied() != Some(final_position) {
        positions.push(final_position);
    }
    positions
}

fn blurred_luminance(frame: &[u8], width: u32, height: u32) -> Vec<f64> {
    let mut luminance = vec![0.0; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let pixel = pixel_at(frame, width, x, y);
            luminance[(y * width + x) as usize] = 0.2126 * f64::from(pixel[0])
                + 0.7152 * f64::from(pixel[1])
                + 0.0722 * f64::from(pixel[2]);
        }
    }
    let mut blurred = vec![0.0; luminance.len()];
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0;
            let mut count = 0.0;
            for offset_y in -1_i32..=1 {
                for offset_x in -1_i32..=1 {
                    let sample_x = (x as i32 + offset_x).clamp(0, width as i32 - 1) as u32;
                    let sample_y = (y as i32 + offset_y).clamp(0, height as i32 - 1) as u32;
                    sum += luminance[(sample_y * width + sample_x) as usize];
                    count += 1.0;
                }
            }
            blurred[(y * width + x) as usize] = sum / count;
        }
    }
    blurred
}

fn window_ssim(
    candidate: &[f64],
    reference: &[f64],
    width: u32,
    x_min: u32,
    y_min: u32,
    window: u32,
) -> f64 {
    let count = f64::from(window * window);
    let mut candidate_mean = 0.0;
    let mut reference_mean = 0.0;
    for y in y_min..y_min + window {
        for x in x_min..x_min + window {
            let index = (y * width + x) as usize;
            candidate_mean += candidate[index];
            reference_mean += reference[index];
        }
    }
    candidate_mean /= count;
    reference_mean /= count;
    let mut candidate_variance = 0.0;
    let mut reference_variance = 0.0;
    let mut covariance = 0.0;
    for y in y_min..y_min + window {
        for x in x_min..x_min + window {
            let index = (y * width + x) as usize;
            let candidate_delta = candidate[index] - candidate_mean;
            let reference_delta = reference[index] - reference_mean;
            candidate_variance += candidate_delta * candidate_delta;
            reference_variance += reference_delta * reference_delta;
            covariance += candidate_delta * reference_delta;
        }
    }
    let denominator = (count - 1.0).max(1.0);
    candidate_variance /= denominator;
    reference_variance /= denominator;
    covariance /= denominator;
    let c1 = (0.01_f64 * 255.0).powi(2);
    let c2 = (0.03_f64 * 255.0).powi(2);
    ((2.0 * candidate_mean * reference_mean + c1) * (2.0 * covariance + c2))
        / ((candidate_mean.powi(2) + reference_mean.powi(2) + c1)
            * (candidate_variance + reference_variance + c2))
}

fn sobel_edge_mask(luminance: &[f64], width: u32, height: u32, threshold: f64) -> Vec<bool> {
    let mut edges = vec![false; luminance.len()];
    if width < 3 || height < 3 {
        return edges;
    }
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let sample = |offset_x: i32, offset_y: i32| {
                luminance
                    [((y as i32 + offset_y) as u32 * width + (x as i32 + offset_x) as u32) as usize]
            };
            let gradient_x = -sample(-1, -1) + sample(1, -1) - 2.0 * sample(-1, 0)
                + 2.0 * sample(1, 0)
                - sample(-1, 1)
                + sample(1, 1);
            let gradient_y = -sample(-1, -1) - 2.0 * sample(0, -1) - sample(1, -1)
                + sample(-1, 1)
                + 2.0 * sample(0, 1)
                + sample(1, 1);
            edges[(y * width + x) as usize] = gradient_x.hypot(gradient_y) >= threshold;
        }
    }
    edges
}

fn foreground_mask(frame: &[u8], width: u32, height: u32) -> Vec<bool> {
    (0..width * height)
        .map(|index| {
            let offset = index as usize * 4;
            frame[offset] > 3 || frame[offset + 1] > 3 || frame[offset + 2] > 3
        })
        .collect()
}

fn dilate_mask(mask: &[bool], width: u32, height: u32, radius: i32) -> Vec<bool> {
    let mut dilated = vec![false; mask.len()];
    for y in 0..height {
        for x in 0..width {
            if !mask[(y * width + x) as usize] {
                continue;
            }
            for offset_y in -radius..=radius {
                for offset_x in -radius..=radius {
                    let target_x = x as i32 + offset_x;
                    let target_y = y as i32 + offset_y;
                    if target_x >= 0
                        && target_y >= 0
                        && target_x < width as i32
                        && target_y < height as i32
                    {
                        dilated[(target_y as u32 * width + target_x as u32) as usize] = true;
                    }
                }
            }
        }
    }
    dilated
}

fn mask_iou(left: &[bool], right: &[bool]) -> f32 {
    let mut intersection = 0_u32;
    let mut union = 0_u32;
    for (&left, &right) in left.iter().zip(right) {
        intersection += u32::from(left && right);
        union += u32::from(left || right);
    }
    if union == 0 {
        1.0
    } else {
        intersection as f32 / union as f32
    }
}

fn rotate_each_quadrant_180(frame: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rotated = frame.to_vec();
    for (x_min, x_max) in [(0, width / 2), (width / 2, width)] {
        for (y_min, y_max) in [(0, height / 2), (height / 2, height)] {
            for y in y_min..y_max {
                for x in x_min..x_max {
                    let source_x = x_max - 1 - (x - x_min);
                    let source_y = y_max - 1 - (y - y_min);
                    let source = ((source_y * width + source_x) * 4) as usize;
                    let target = ((y * width + x) * 4) as usize;
                    rotated[target..target + 4].copy_from_slice(&frame[source..source + 4]);
                }
            }
        }
    }
    rotated
}

fn sort_each_quadrant_by_luminance(frame: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut sorted_frame = frame.to_vec();
    for (x_min, x_max) in [(0, width / 2), (width / 2, width)] {
        for (y_min, y_max) in [(0, height / 2), (height / 2, height)] {
            let mut pixels = Vec::new();
            for y in y_min..y_max {
                for x in x_min..x_max {
                    pixels.push(pixel_at(frame, width, x, y));
                }
            }
            pixels.sort_by_key(|pixel| {
                (
                    u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2]),
                    pixel[0],
                    pixel[1],
                    pixel[2],
                    pixel[3],
                )
            });
            for (index, pixel) in pixels.into_iter().enumerate() {
                let local_width = (x_max - x_min) as usize;
                let x = x_min + (index % local_width) as u32;
                let y = y_min + (index / local_width) as u32;
                let target = ((y * width + x) * 4) as usize;
                sorted_frame[target..target + 4].copy_from_slice(&pixel);
            }
        }
    }
    sorted_frame
}

#[allow(clippy::too_many_arguments)]
fn write_structural_reference_artifacts(
    dir: &Path,
    name: &str,
    candidate: &[u8],
    reference: &[u8],
    width: u32,
    height: u32,
    report: &StructuralReferenceReport,
) {
    let mut heatmap = format!("P6\n{width} {height}\n255\n").into_bytes();
    for (candidate, reference) in candidate.chunks_exact(4).zip(reference.chunks_exact(4)) {
        let delta = candidate[..3]
            .iter()
            .zip(&reference[..3])
            .map(|(&candidate, &reference)| candidate.abs_diff(reference))
            .max()
            .unwrap_or(0);
        heatmap.extend_from_slice(&[delta.saturating_mul(4), delta, 0]);
    }
    fs::write(dir.join(format!("{name}-reference-diff.ppm")), heatmap)
        .expect("M2 structural heatmap writes");
    let artifact = serde_json::json!({
        "proof_class": "q03-m2-local-structure-reference",
        "reference_mode": "local-structure-v2",
        "fixture": name,
        "resolution": [width, height],
        "encoding": "srgb8",
        "thresholds": {
            "window_size": STRUCTURE_WINDOW_SIZE,
            "window_stride": STRUCTURE_WINDOW_STRIDE,
            "mean_window_ssim_min": STRUCTURE_MEAN_SSIM_MIN,
            "worst_window_ssim_min": STRUCTURE_WORST_WINDOW_SSIM_MIN,
            "edge_iou_min": STRUCTURE_EDGE_IOU_MIN,
            "foreground_iou_min": STRUCTURE_FOREGROUND_IOU_MIN,
        },
        "quadrant_metrics_role": "debug-only",
        "report": report,
        "heatmap": format!("target/gate-artifacts/m2-visual/{name}-reference-diff.ppm"),
    });
    fs::write(
        dir.join(format!("{name}-reference-metrics.json")),
        serde_json::to_vec_pretty(&artifact).expect("M2 structural report serializes"),
    )
    .expect("M2 structural report writes");
}

fn reference_mode() -> String {
    declared_reference_mode(M2_HEADLESS_REFERENCE_METADATA)
}

fn fixture_reference_mode() -> String {
    declared_reference_mode(M2_HEADLESS_FIXTURE_METADATA)
}

fn declared_reference_mode(metadata: &str) -> String {
    metadata
        .lines()
        .find_map(|line| line.trim().strip_prefix("reference_mode = "))
        .map(parse_quoted)
        .expect("reference metadata declares reference_mode")
}

fn blit_frame(
    source: &[u8],
    source_width: u32,
    source_height: u32,
    target: &mut [u8],
    target_width: u32,
    target_x: u32,
    target_y: u32,
) {
    for y in 0..source_height {
        let source_start = (y * source_width * 4) as usize;
        let source_end = source_start + (source_width * 4) as usize;
        let target_start = (((target_y + y) * target_width + target_x) * 4) as usize;
        let target_end = target_start + (source_width * 4) as usize;
        target[target_start..target_end].copy_from_slice(&source[source_start..source_end]);
    }
}

fn pixel_at(frame: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    frame[offset..offset + 4]
        .try_into()
        .expect("pixel slice has four channels")
}

fn write_ppm_artifact(
    dir: &Path,
    name: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
    paired_effect: bool,
) {
    assert_eq!(rgba.len(), width as usize * height as usize * 4);
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact can be written");
    let nonblack_pixels = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count();
    let mut triplets: std::collections::BTreeSet<[u8; 3]> = std::collections::BTreeSet::new();
    for pixel in rgba.chunks_exact(4) {
        triplets.insert([pixel[0], pixel[1], pixel[2]]);
    }
    let unique_pixels = triplets.len();
    let proof_class = if paired_effect {
        "paired-effect-footprint"
    } else {
        "harness-smoke"
    };
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\n\
             name = \"{name}\"\n\
             format = \"ppm\"\n\
             encoding = \"srgb8\"\n\
             width = {width}\n\
             height = {height}\n\
             nonblack_pixels = {nonblack_pixels}\n\
             unique_pixels = {unique_pixels}\n\
             tolerance = \"local-structure-v2-windowed-ssim-edge-foreground\"\n\
             proof_class = \"{proof_class}\"\n\
             production_claim = false\n\
             fixture_suite = \"m2-headless-core\"\n\
             fixture_source = \"tests/visual/fixtures/m2-headless-core.toml\"\n"
        ),
    )
    .expect("artifact metadata can be written");
}

fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m2-visual")
}
