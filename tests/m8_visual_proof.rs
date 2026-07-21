#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::fs;
use std::future::{Ready, ready};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use base64::Engine;
use scena::{
    AlphaMode, AssetError, AssetFetcher, AssetPath, Assets, Color, DirectionalLight,
    EnvironmentHandle, GeometryDesc, GeometryTopology, MaterialDesc, PerspectiveCamera, Renderer,
    Scene, TextureColorSpace, Transform, Vec3,
};
use serde::Serialize;

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;

fn ndc_fixture_camera_transform() -> Transform {
    Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES))
}

#[test]
fn m8_khr_material_visual_oracle_rejects_disabled_and_wrong_direction_mutations() {
    let reports = evaluate_khr_material_feature_mutations();
    let names = reports.iter().map(|report| report.name).collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "clearcoat",
            "sheen",
            "anisotropy-light-left",
            "anisotropy-light-right",
            "iridescence",
            "dispersion",
            "transmission-volume",
        ],
        "every covered KHR material feature and both directional anisotropy views must be evaluated"
    );
    for report in reports {
        assert!(
            report.positive_passed,
            "{} enabled feature must exceed its declared visible-effect floor: {report:#?}",
            report.name
        );
        assert!(
            report.disabled_rejected,
            "{} disabled control must fail the same evaluator: {report:#?}",
            report.name
        );
        assert!(
            report.wrong_direction_rejected,
            "{} inverted-effect mutation must fail the same evaluator: {report:#?}",
            report.name
        );
        assert!(
            report.one_lsb_noise_passed,
            "{} harmless one-LSB noise must not fail visible acceptance: {report:#?}",
            report.name
        );
        assert!(
            report.two_lsb_effect_nudge_rejected,
            "{} two-LSB control nudge must not create a visible feature pass: {report:#?}",
            report.name
        );
    }
}

#[test]
fn m8_headless_visual_artifacts_cover_material_texture_environment_paths() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    let artifacts = [
        render_unlit_textured_asset(),
        render_metallic_roughness_asset(),
        render_normal_mapped_asset(),
        render_emissive_asset(),
        render_alpha_mask(),
        render_base_color_alpha(),
        render_texture_slots(),
        render_environment_color_management(),
        render_clearcoat_material_feature(),
        render_sheen_material_feature(),
        render_anisotropy_material_feature(),
        render_iridescence_material_feature(),
        render_dispersion_material_feature(),
        render_transmission_volume_material_feature(),
    ];
    let expected_artifacts = [
        "m8-unlit-textured-asset",
        "m8-metallic-roughness-asset",
        "m8-normal-mapped-asset",
        "m8-emissive-asset",
        "m8-alpha-mask",
        "m8-alpha-blend",
        "m8-texture-slots",
        "m8-environment-color-management",
        "m8-clearcoat-material-feature",
        "m8-sheen-material-feature",
        "m8-anisotropy-material-feature",
        "m8-iridescence-material-feature",
        "m8-dispersion-material-feature",
        "m8-transmission-volume-material-feature",
    ];
    for expected in expected_artifacts {
        assert!(
            artifacts.iter().any(|artifact| artifact.name == expected),
            "missing M8 visual material proof artifact {expected}"
        );
    }

    for artifact in artifacts {
        assert!(
            nonblack_pixel_count(&artifact.rgba) > 0,
            "{} should render visible nonblack pixels",
            artifact.name
        );
        assert_eq!(
            (artifact.width, artifact.height),
            (256, 256),
            "{} must be a 256x256 visual proof artifact",
            artifact.name
        );
        if artifact.name == "m8-texture-slots" {
            let center = artifact.center_pixel();
            assert!(
                center[0] > 150 && center[1] < 80 && center[2] < 80,
                "m8-texture-slots must prove decoded texture pixels affect output, got {center:?}"
            );
        }
        write_ppm_artifact(&artifact_dir, &artifact);
        let metadata = fs::read_to_string(artifact_dir.join(format!("{}.toml", artifact.name)))
            .expect("visual artifact metadata is readable");
        for key in [
            "backend =",
            "adapter =",
            "renderer_settings =",
            "source_hash =",
            "tolerance =",
            "color_management =",
        ] {
            assert!(
                metadata.contains(key),
                "{} metadata must include {key}",
                artifact.name
            );
        }
    }
}

#[test]
fn m8_visual_reference_sensitivity_covers_camera_transform_depth_material_texture_and_lighting() {
    assert_visual_change(
        "camera",
        render_sensitivity_box(SensitivityOptions {
            camera_x: 0.0,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
        render_sensitivity_box(SensitivityOptions {
            camera_x: 0.45,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
    );
    assert_visual_change(
        "transform",
        render_sensitivity_box(SensitivityOptions {
            mesh_x: 0.0,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
        render_sensitivity_box(SensitivityOptions {
            mesh_x: 0.35,
            ..SensitivityOptions::unlit(Color::from_srgb_u8(40, 180, 240))
        }),
    );
    assert_visual_change(
        "material",
        render_sensitivity_box(SensitivityOptions::unlit(Color::from_srgb_u8(220, 32, 24))),
        render_sensitivity_box(SensitivityOptions::unlit(Color::from_srgb_u8(24, 190, 72))),
    );
    assert_visual_change(
        "texture",
        render_sensitivity_box(SensitivityOptions {
            texture_pixel: Some([240, 32, 24, 255]),
            ..SensitivityOptions::unlit(Color::WHITE)
        }),
        render_sensitivity_box(SensitivityOptions {
            texture_pixel: Some([24, 72, 240, 255]),
            ..SensitivityOptions::unlit(Color::WHITE)
        }),
    );
    assert_visual_change(
        "lighting",
        render_lighting_sensitivity_frame(Color::from_linear_rgb(1.0, 0.0, 0.0)),
        render_lighting_sensitivity_frame(Color::from_linear_rgb(0.0, 1.0, 0.0)),
    );

    let depth = render_depth_sensitivity_scene();
    let center = pixel_at(&depth, 64, 32, 32);
    assert!(
        center[0] > center[2],
        "depth-sensitive visual fixture must keep the nearer red surface visible when the \
         farther blue surface is submitted later, got {center:?}"
    );
}

fn render_unlit_textured_asset() -> VisualArtifact {
    let assets = Assets::new();
    let base = load_pixel_texture(&assets, [240, 32, 24, 255], TextureColorSpace::Srgb);
    render_material_box(
        "m8-unlit-textured-asset",
        &assets,
        MaterialDesc::unlit(Color::WHITE).with_base_color_texture(base),
        None,
        false,
        "unlit-textured-cpu-headless-256",
    )
}

#[derive(Clone, Copy)]
struct SensitivityOptions {
    camera_x: f32,
    mesh_x: f32,
    material_color: Color,
    texture_pixel: Option<[u8; 4]>,
}

impl SensitivityOptions {
    fn unlit(material_color: Color) -> Self {
        Self {
            camera_x: 0.0,
            mesh_x: 0.0,
            material_color,
            texture_pixel: None,
        }
    }
}

fn render_sensitivity_box(options: SensitivityOptions) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let mut material = MaterialDesc::unlit(options.material_color);
    if let Some(pixel) = options.texture_pixel {
        material = material.with_base_color_texture(load_pixel_texture(
            &assets,
            pixel,
            TextureColorSpace::Srgb,
        ));
    }
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(options.mesh_x, 0.0, 0.0)))
        .add()
        .expect("sensitivity mesh inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let camera_node = scene.camera_node(camera).expect("camera node exists");
    scene
        .set_transform(
            camera_node,
            Transform::at(Vec3::new(options.camera_x, 0.0, 2.0)),
        )
        .expect("camera moves");
    scene
        .look_at_point(camera, Vec3::ZERO)
        .expect("camera looks at origin");
    render_sensitivity_scene(scene, camera, &assets)
}

fn render_lighting_sensitivity_frame(light_color: Color) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(fullscreen_triangle_geometry());
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(light_color)
                .with_illuminance_lux(12_000.0),
        )
        .add()
        .expect("directional light inserts");
    scene.mesh(geometry, material).add().expect("mesh inserts");
    render_sensitivity_scene(scene, camera, &assets)
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

fn render_depth_sensitivity_scene() -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.7, 0.08));
    let red = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(230, 16, 16)));
    let blue = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(16, 48, 230)));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, red)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.08)))
        .add()
        .expect("near red mesh inserts");
    scene
        .mesh(geometry, blue)
        .transform(Transform::at(Vec3::new(0.0, 0.0, -0.08)))
        .add()
        .expect("far blue mesh inserts after near red");
    let camera = scene.add_default_camera().expect("camera inserts");
    render_sensitivity_scene(scene, camera, &assets)
}

fn render_sensitivity_scene<F>(
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets<F>,
) -> Vec<u8> {
    let mut renderer = Renderer::headless(64, 64).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("sensitivity scene prepares");
    renderer
        .render(&scene, camera)
        .expect("sensitivity scene renders");
    renderer.frame_rgba8().to_vec()
}

fn assert_visual_change(label: &str, left: Vec<u8>, right: Vec<u8>) {
    assert_ne!(
        fnv1a64_hex(&left),
        fnv1a64_hex(&right),
        "{label} visual sensitivity must change the rendered frame hash"
    );
}

fn render_metallic_roughness_asset() -> VisualArtifact {
    let assets = Assets::new();
    let metallic_roughness =
        load_pixel_texture(&assets, [0, 32, 255, 255], TextureColorSpace::Linear);
    render_material_box(
        "m8-metallic-roughness-asset",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 1.0, 1.0)
            .with_metallic_roughness_texture(metallic_roughness),
        None,
        true,
        "metallic-roughness-cpu-headless-256",
    )
}

fn render_normal_mapped_asset() -> VisualArtifact {
    let assets = Assets::new();
    let normal = load_pixel_texture(&assets, [128, 128, 255, 255], TextureColorSpace::Linear);
    render_material_box(
        "m8-normal-mapped-asset",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 190, 190), 0.0, 0.75)
            .with_normal_texture(normal),
        None,
        true,
        "normal-mapped-cpu-headless-256",
    )
}

fn render_emissive_asset() -> VisualArtifact {
    let assets = Assets::new();
    let emissive = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Srgb);
    render_material_box(
        "m8-emissive-asset",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(20, 20, 20), 0.0, 0.75)
            .with_emissive(Color::WHITE)
            .with_emissive_texture(emissive),
        None,
        false,
        "emissive-textured-cpu-headless-256",
    )
}

fn render_alpha_mask() -> VisualArtifact {
    let assets = Assets::new();
    render_material_box(
        "m8-alpha-mask",
        &assets,
        MaterialDesc::unlit(Color::from_linear_rgba(0.1, 0.85, 0.2, 0.85))
            .with_alpha_mode(AlphaMode::Mask { cutoff: 0.5 }),
        None,
        false,
        "alpha-mask-cpu-headless-256",
    )
}

fn render_base_color_alpha() -> VisualArtifact {
    let assets = Assets::new();
    render_material_box(
        "m8-alpha-blend",
        &assets,
        MaterialDesc::unlit(Color::from_linear_rgba(0.1, 0.45, 1.0, 0.72))
            .with_alpha_mode(AlphaMode::Blend)
            .with_double_sided(true),
        None,
        false,
        "alpha-blend-cpu-headless-256",
    )
}

fn render_texture_slots() -> VisualArtifact {
    let red_png_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";
    let red_png = base64::engine::general_purpose::STANDARD
        .decode(red_png_base64)
        .expect("fixture PNG base64 is valid");
    let gltf = textured_material_gltf();
    let source_hash = fnv1a64_hex(gltf.as_bytes());
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://m8-visual-textures/scene.gltf"),
            gltf.into_bytes(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/base.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/normal.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/metallic_roughness.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/occlusion.png"),
            red_png.clone(),
        ),
        (
            AssetPath::from("memory://m8-visual-textures/emissive.png"),
            red_png,
        ),
    ]));
    let scene_asset =
        pollster::block_on(assets.load_scene("memory://m8-visual-textures/scene.gltf"))
            .expect("textured visual glTF loads");
    let mut scene = Scene::new();
    scene
        .instantiate(&scene_asset)
        .expect("textured visual glTF instantiates");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut artifact = render_scene_with_assets("m8-texture-slots", scene, camera, &assets, None);
    artifact.proof_class = "decoded-texture-pixels-256";
    artifact.source = "memory://m8-visual-textures/scene.gltf".to_string();
    artifact.source_hash = Some(source_hash);
    assert_eq!(artifact.stats_textures, 5);
    artifact
}

fn render_environment_color_management() -> VisualArtifact {
    let assets = Assets::new();
    let environment = assets.default_environment();
    let artifact = render_material_box(
        "m8-environment-color-management",
        &assets,
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(220, 220, 220), 0.0, 0.55),
        Some(environment),
        true,
        "environment-handle-color-management-cpu-headless-256",
    );
    assert_eq!(artifact.stats_environments, 1);
    artifact
}

const KHR_VISIBLE_CHANNEL_DELTA: u8 = 4;
const KHR_NUMERICAL_RMSE_MAX: f32 = 1.1;
const KHR_EFFECT_ALIGNMENT_MIN: f32 = 0.9;

#[derive(Debug, Clone, Copy, Serialize)]
struct FeatureRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum FeatureDirection {
    LuminanceIncrease,
    RedIncrease,
    SpatialRedistribution,
    BlueMinusRedIncrease,
    ChromaticSpreadIncrease,
    BlueDominantDarkening,
}

#[derive(Debug, Serialize)]
struct FeatureEvaluation {
    passed: bool,
    visible_effect_passed: bool,
    numerical_match_passed: bool,
    effect_rmse: f32,
    changed_pixel_fraction: f32,
    direction_metric: f32,
    reference_rmse: f32,
    effect_alignment: f32,
}

#[derive(Debug, Serialize)]
struct KhrMaterialFeatureReport {
    name: &'static str,
    region: FeatureRegion,
    direction: FeatureDirection,
    min_effect_rmse: f32,
    min_changed_pixel_fraction: f32,
    min_direction_metric: f32,
    positive_passed: bool,
    disabled_rejected: bool,
    wrong_direction_rejected: bool,
    one_lsb_noise_passed: bool,
    two_lsb_effect_nudge_rejected: bool,
    positive: FeatureEvaluation,
    disabled_control: FeatureEvaluation,
    wrong_direction: FeatureEvaluation,
    one_lsb_noise: FeatureEvaluation,
    two_lsb_effect_nudge: FeatureEvaluation,
}

struct KhrMaterialFeatureCase {
    name: &'static str,
    control: Vec<u8>,
    enabled: Vec<u8>,
    region: FeatureRegion,
    direction: FeatureDirection,
    min_effect_rmse: f32,
    min_changed_pixel_fraction: f32,
    min_direction_metric: f32,
}

fn evaluate_khr_material_feature_mutations() -> Vec<KhrMaterialFeatureReport> {
    let reports = render_khr_material_feature_cases()
        .into_iter()
        .map(evaluate_khr_material_feature_case)
        .collect::<Vec<_>>();
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("Q02 artifact directory can be created");
    let artifact = serde_json::json!({
        "proof_class": "q02-khr-material-feature-mutation-proof",
        "evaluator_version": 1,
        "resolution": [256, 256],
        "encoding": "srgb8",
        "visible_acceptance": {
            "channel_delta_min": KHR_VISIBLE_CHANNEL_DELTA,
            "feature_specific_effect_rmse_and_changed_fraction": true,
            "feature_specific_expected_direction": true,
        },
        "numerical_tolerance": {
            "reference_rmse_max": KHR_NUMERICAL_RMSE_MAX,
            "effect_alignment_min": KHR_EFFECT_ALIGNMENT_MIN,
        },
        "mutations": [
            "disabled-control",
            "two-lsb-effect-nudge",
            "inverted-effect-direction",
            "one-lsb-noise"
        ],
        "reports": &reports,
    });
    fs::write(
        artifact_dir.join("khr-material-feature-proof.json"),
        serde_json::to_vec_pretty(&artifact).expect("Q02 artifact serializes"),
    )
    .expect("Q02 artifact writes");
    reports
}

fn evaluate_khr_material_feature_case(case: KhrMaterialFeatureCase) -> KhrMaterialFeatureReport {
    let wrong_direction = invert_feature_effect(&case.control, &case.enabled);
    let one_lsb_noise = add_one_lsb_noise(&case.enabled);
    let two_lsb_effect_nudge = add_two_lsb_effect_nudge(&case.control, &case.enabled);
    let positive = evaluate_feature_candidate(&case, &case.enabled);
    let disabled_control = evaluate_feature_candidate(&case, &case.control);
    let wrong_direction_evaluation = evaluate_feature_candidate(&case, &wrong_direction);
    let one_lsb_noise_evaluation = evaluate_feature_candidate(&case, &one_lsb_noise);
    let two_lsb_effect_nudge_evaluation = evaluate_feature_candidate(&case, &two_lsb_effect_nudge);
    KhrMaterialFeatureReport {
        name: case.name,
        region: case.region,
        direction: case.direction,
        min_effect_rmse: case.min_effect_rmse,
        min_changed_pixel_fraction: case.min_changed_pixel_fraction,
        min_direction_metric: case.min_direction_metric,
        positive_passed: positive.passed,
        disabled_rejected: !disabled_control.passed,
        wrong_direction_rejected: !wrong_direction_evaluation.passed,
        one_lsb_noise_passed: one_lsb_noise_evaluation.passed,
        two_lsb_effect_nudge_rejected: !two_lsb_effect_nudge_evaluation.passed,
        positive,
        disabled_control,
        wrong_direction: wrong_direction_evaluation,
        one_lsb_noise: one_lsb_noise_evaluation,
        two_lsb_effect_nudge: two_lsb_effect_nudge_evaluation,
    }
}

fn evaluate_feature_candidate(
    case: &KhrMaterialFeatureCase,
    candidate: &[u8],
) -> FeatureEvaluation {
    let mut effect_squared = 0.0_f64;
    let mut reference_squared = 0.0_f64;
    let mut reference_norm = 0.0_f64;
    let mut candidate_norm = 0.0_f64;
    let mut alignment_dot = 0.0_f64;
    let mut changed_pixels = 0_u32;
    let mut masked_pixels = 0_u32;
    let mut control_rgb = [0.0_f64; 3];
    let mut candidate_rgb = [0.0_f64; 3];
    for y in case.region.y..case.region.y + case.region.height {
        for x in case.region.x..case.region.x + case.region.width {
            let offset = ((y * 256 + x) * 4) as usize;
            let control = &case.control[offset..offset + 3];
            let reference = &case.enabled[offset..offset + 3];
            if control.iter().chain(reference).copied().max().unwrap_or(0) <= 3 {
                continue;
            }
            masked_pixels += 1;
            let mut max_delta = 0_u8;
            for channel in 0..3 {
                control_rgb[channel] += f64::from(control[channel]);
                candidate_rgb[channel] += f64::from(candidate[offset + channel]);
                let expected_delta = f64::from(reference[channel]) - f64::from(control[channel]);
                let candidate_delta =
                    f64::from(candidate[offset + channel]) - f64::from(control[channel]);
                effect_squared += candidate_delta * candidate_delta;
                let reference_error =
                    f64::from(candidate[offset + channel]) - f64::from(reference[channel]);
                reference_squared += reference_error * reference_error;
                reference_norm += expected_delta * expected_delta;
                candidate_norm += candidate_delta * candidate_delta;
                alignment_dot += expected_delta * candidate_delta;
                max_delta = max_delta.max(candidate[offset + channel].abs_diff(control[channel]));
            }
            if max_delta >= KHR_VISIBLE_CHANNEL_DELTA {
                changed_pixels += 1;
            }
        }
    }
    assert!(
        masked_pixels > 0,
        "{} feature region must cover rendered pixels",
        case.name
    );
    let channel_count = f64::from(masked_pixels) * 3.0;
    let effect_rmse = (effect_squared / channel_count).sqrt() as f32;
    let reference_rmse = (reference_squared / channel_count).sqrt() as f32;
    let effect_alignment = if reference_norm > 0.0 && candidate_norm > 0.0 {
        (alignment_dot / (reference_norm * candidate_norm).sqrt()) as f32
    } else {
        -1.0
    };
    let changed_pixel_fraction = changed_pixels as f32 / masked_pixels as f32;
    let count = f64::from(masked_pixels);
    let control_mean = control_rgb.map(|value| value / count);
    let candidate_mean = candidate_rgb.map(|value| value / count);
    let direction_metric = match case.direction {
        FeatureDirection::LuminanceIncrease => {
            ((candidate_mean.iter().sum::<f64>() - control_mean.iter().sum::<f64>()) / 3.0) as f32
        }
        FeatureDirection::RedIncrease => (candidate_mean[0] - control_mean[0]) as f32,
        FeatureDirection::SpatialRedistribution => effect_rmse,
        FeatureDirection::BlueMinusRedIncrease => {
            ((candidate_mean[2] - candidate_mean[0]) - (control_mean[2] - control_mean[0])) as f32
        }
        FeatureDirection::ChromaticSpreadIncrease => {
            let spread = |rgb: [f64; 3]| {
                rgb.into_iter().fold(f64::NEG_INFINITY, f64::max)
                    - rgb.into_iter().fold(f64::INFINITY, f64::min)
            };
            (spread(candidate_mean) - spread(control_mean)) as f32
        }
        FeatureDirection::BlueDominantDarkening => {
            let blue_dominance = candidate_mean[2] - candidate_mean[0].max(candidate_mean[1]);
            let darkening =
                (control_mean.iter().sum::<f64>() - candidate_mean.iter().sum::<f64>()) / 3.0;
            blue_dominance.min(darkening) as f32
        }
    };
    let visible_effect_passed = effect_rmse >= case.min_effect_rmse
        && changed_pixel_fraction >= case.min_changed_pixel_fraction
        && direction_metric >= case.min_direction_metric;
    let numerical_match_passed =
        reference_rmse <= KHR_NUMERICAL_RMSE_MAX && effect_alignment >= KHR_EFFECT_ALIGNMENT_MIN;
    FeatureEvaluation {
        passed: visible_effect_passed && numerical_match_passed,
        visible_effect_passed,
        numerical_match_passed,
        effect_rmse,
        changed_pixel_fraction,
        direction_metric,
        reference_rmse,
        effect_alignment,
    }
}

fn invert_feature_effect(control: &[u8], enabled: &[u8]) -> Vec<u8> {
    control
        .iter()
        .zip(enabled)
        .enumerate()
        .map(|(index, (&control, &enabled))| {
            if index % 4 == 3 {
                enabled
            } else {
                (i16::from(control) * 2 - i16::from(enabled)).clamp(0, 255) as u8
            }
        })
        .collect()
}

fn add_one_lsb_noise(enabled: &[u8]) -> Vec<u8> {
    enabled
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            if index % 4 == 3 {
                value
            } else if (index / 4 + index % 4) % 2 == 0 {
                value.saturating_add(1)
            } else {
                value.saturating_sub(1)
            }
        })
        .collect()
}

fn add_two_lsb_effect_nudge(control: &[u8], enabled: &[u8]) -> Vec<u8> {
    control
        .iter()
        .zip(enabled)
        .enumerate()
        .map(|(index, (&control, &enabled))| {
            if index % 4 == 3 || enabled == control {
                control
            } else if enabled > control {
                control.saturating_add(2).min(enabled)
            } else {
                control.saturating_sub(2).max(enabled)
            }
        })
        .collect()
}

fn render_khr_material_feature_cases() -> Vec<KhrMaterialFeatureCase> {
    let assets = Assets::new();
    let common_region = FeatureRegion {
        x: 98,
        y: 110,
        width: 76,
        height: 78,
    };
    let render = |name, material, light_yaw_deg| {
        render_material_box_with_light_yaw(
            name,
            &assets,
            material,
            None,
            true,
            light_yaw_deg,
            "q02-khr-feature-source-frame",
        )
        .rgba
    };

    let clearcoat_control =
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62);
    let clearcoat_enabled = clearcoat_control
        .clone()
        .with_clearcoat_factor(0.9)
        .with_clearcoat_roughness_factor(0.12);

    let sheen_off = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Srgb);
    let sheen_on = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Srgb);
    let sheen_base =
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(104, 96, 92), 0.0, 0.72)
            .with_sheen_color_factor(Color::WHITE)
            .with_sheen_roughness_factor(0.35);

    let anisotropy_off = load_pixel_texture(&assets, [255, 128, 0, 255], TextureColorSpace::Linear);
    let anisotropy_on =
        load_pixel_texture(&assets, [255, 128, 255, 255], TextureColorSpace::Linear);
    let anisotropy_base =
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(150, 150, 150), 1.0, 0.42)
            .with_anisotropy_strength_factor(1.0);

    let iridescence_off = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Linear);
    let iridescence_on = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Linear);
    let iridescence_thickness =
        load_pixel_texture(&assets, [0, 255, 0, 255], TextureColorSpace::Linear);
    let iridescence_base =
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(180, 180, 180), 1.0, 0.18)
            .with_iridescence_factor(1.0)
            .with_iridescence_ior(1.45)
            .with_iridescence_thickness_range_nm(120.0, 520.0)
            .with_iridescence_thickness_texture(iridescence_thickness);

    let dispersion_control =
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(165, 165, 165), 0.0, 0.24)
            .with_dispersion_factor(0.0);
    let dispersion_enabled = dispersion_control.clone().with_dispersion_factor(1.0);

    let transmission_off = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Linear);
    let transmission_on = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Linear);
    let transmission_thickness =
        load_pixel_texture(&assets, [0, 255, 0, 255], TextureColorSpace::Linear);
    let transmission_base =
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 205, 230), 0.0, 0.08)
            .with_transmission_factor(1.0)
            .with_ior(1.7)
            .with_thickness_factor(2.0)
            .with_thickness_texture(transmission_thickness)
            .with_attenuation_distance(1.0)
            .with_attenuation_color(Color::from_linear_rgb(0.08, 0.35, 1.0));

    vec![
        KhrMaterialFeatureCase {
            name: "clearcoat",
            control: render("q02-clearcoat-control", clearcoat_control, 0.0),
            enabled: render("q02-clearcoat-enabled", clearcoat_enabled, 0.0),
            region: FeatureRegion {
                x: 104,
                y: 112,
                width: 64,
                height: 70,
            },
            direction: FeatureDirection::LuminanceIncrease,
            min_effect_rmse: 2.0,
            min_changed_pixel_fraction: 0.02,
            min_direction_metric: 0.5,
        },
        KhrMaterialFeatureCase {
            name: "sheen",
            control: render(
                "q02-sheen-control",
                sheen_base.clone().with_sheen_color_texture(sheen_off),
                0.0,
            ),
            enabled: render(
                "q02-sheen-enabled",
                sheen_base.with_sheen_color_texture(sheen_on),
                0.0,
            ),
            region: common_region,
            direction: FeatureDirection::RedIncrease,
            min_effect_rmse: 20.0,
            min_changed_pixel_fraction: 0.25,
            min_direction_metric: 20.0,
        },
        KhrMaterialFeatureCase {
            name: "anisotropy-light-left",
            control: render(
                "q02-anisotropy-left-control",
                anisotropy_base
                    .clone()
                    .with_anisotropy_texture(anisotropy_off),
                -35.0,
            ),
            enabled: render(
                "q02-anisotropy-left-enabled",
                anisotropy_base
                    .clone()
                    .with_anisotropy_texture(anisotropy_on),
                -35.0,
            ),
            region: common_region,
            direction: FeatureDirection::SpatialRedistribution,
            min_effect_rmse: 8.0,
            min_changed_pixel_fraction: 0.20,
            min_direction_metric: 8.0,
        },
        KhrMaterialFeatureCase {
            name: "anisotropy-light-right",
            control: render(
                "q02-anisotropy-right-control",
                anisotropy_base
                    .clone()
                    .with_anisotropy_texture(anisotropy_off),
                35.0,
            ),
            enabled: render(
                "q02-anisotropy-right-enabled",
                anisotropy_base.with_anisotropy_texture(anisotropy_on),
                35.0,
            ),
            region: common_region,
            direction: FeatureDirection::SpatialRedistribution,
            min_effect_rmse: 8.0,
            min_changed_pixel_fraction: 0.20,
            min_direction_metric: 8.0,
        },
        KhrMaterialFeatureCase {
            name: "iridescence",
            control: render(
                "q02-iridescence-control",
                iridescence_base
                    .clone()
                    .with_iridescence_texture(iridescence_off),
                0.0,
            ),
            enabled: render(
                "q02-iridescence-enabled",
                iridescence_base.with_iridescence_texture(iridescence_on),
                0.0,
            ),
            region: common_region,
            direction: FeatureDirection::BlueMinusRedIncrease,
            min_effect_rmse: 6.0,
            min_changed_pixel_fraction: 0.20,
            min_direction_metric: 5.0,
        },
        KhrMaterialFeatureCase {
            name: "dispersion",
            control: render("q02-dispersion-control", dispersion_control, 0.0),
            enabled: render("q02-dispersion-enabled", dispersion_enabled, 0.0),
            region: common_region,
            direction: FeatureDirection::ChromaticSpreadIncrease,
            min_effect_rmse: 10.0,
            min_changed_pixel_fraction: 0.20,
            min_direction_metric: 10.0,
        },
        KhrMaterialFeatureCase {
            name: "transmission-volume",
            control: render(
                "q02-transmission-control",
                transmission_base
                    .clone()
                    .with_transmission_texture(transmission_off),
                0.0,
            ),
            enabled: render(
                "q02-transmission-enabled",
                transmission_base.with_transmission_texture(transmission_on),
                0.0,
            ),
            region: common_region,
            direction: FeatureDirection::BlueDominantDarkening,
            min_effect_rmse: 30.0,
            min_changed_pixel_fraction: 0.50,
            min_direction_metric: 15.0,
        },
    ]
}

fn render_clearcoat_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let matte = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(188, 48, 32), 0.0, 0.62);
    let clearcoat = matte
        .clone()
        .with_clearcoat_factor(0.9)
        .with_clearcoat_roughness_factor(0.12);
    let mut left = render_material_box(
        "m8-clearcoat-left",
        &assets,
        matte,
        None,
        true,
        "clearcoat-before",
    );
    let right = render_material_box(
        "m8-clearcoat-right",
        &assets,
        clearcoat,
        None,
        true,
        "clearcoat-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-clearcoat-material-feature";
    left.proof_class = "clearcoat-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-clearcoat-material-feature:clearcoat-before-after",
    ));
    left
}

fn render_sheen_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let black = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Srgb);
    let red = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Srgb);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(104, 96, 92), 0.0, 0.72)
        .with_sheen_color_factor(Color::WHITE)
        .with_sheen_roughness_factor(0.35);
    let no_sheen = base.clone().with_sheen_color_texture(black);
    let red_sheen = base.with_sheen_color_texture(red);
    let mut left = render_material_box(
        "m8-sheen-left",
        &assets,
        no_sheen,
        None,
        true,
        "sheen-before",
    );
    let right = render_material_box(
        "m8-sheen-right",
        &assets,
        red_sheen,
        None,
        true,
        "sheen-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-sheen-material-feature";
    left.proof_class = "sheen-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-sheen-material-feature:sheen-before-after",
    ));
    left
}

fn render_anisotropy_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let off_texture = load_pixel_texture(&assets, [255, 128, 0, 255], TextureColorSpace::Linear);
    let on_texture = load_pixel_texture(&assets, [255, 128, 255, 255], TextureColorSpace::Linear);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(150, 150, 150), 1.0, 0.42)
        .with_anisotropy_strength_factor(1.0);
    let off = base.clone().with_anisotropy_texture(off_texture);
    let on = base.with_anisotropy_texture(on_texture);
    let mut left = render_material_box(
        "m8-anisotropy-left",
        &assets,
        off,
        None,
        true,
        "anisotropy-before",
    );
    let right = render_material_box(
        "m8-anisotropy-right",
        &assets,
        on,
        None,
        true,
        "anisotropy-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-anisotropy-material-feature";
    left.proof_class = "anisotropy-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-anisotropy-material-feature:anisotropy-before-after",
    ));
    left
}

fn render_iridescence_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let off_texture = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Linear);
    let on_texture = load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Linear);
    let thickness_texture =
        load_pixel_texture(&assets, [0, 255, 0, 255], TextureColorSpace::Linear);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(180, 180, 180), 1.0, 0.18)
        .with_iridescence_factor(1.0)
        .with_iridescence_ior(1.45)
        .with_iridescence_thickness_range_nm(120.0, 520.0)
        .with_iridescence_thickness_texture(thickness_texture);
    let off = base.clone().with_iridescence_texture(off_texture);
    let on = base.with_iridescence_texture(on_texture);
    let mut left = render_material_box(
        "m8-iridescence-left",
        &assets,
        off,
        None,
        true,
        "iridescence-before",
    );
    let right = render_material_box(
        "m8-iridescence-right",
        &assets,
        on,
        None,
        true,
        "iridescence-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-iridescence-material-feature";
    left.proof_class = "iridescence-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-iridescence-material-feature:iridescence-before-after",
    ));
    left
}

fn render_dispersion_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let off = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(165, 165, 165), 0.0, 0.24)
        .with_dispersion_factor(0.0);
    let on = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(165, 165, 165), 0.0, 0.24)
        .with_dispersion_factor(1.0);
    let mut left = render_material_box(
        "m8-dispersion-left",
        &assets,
        off,
        None,
        true,
        "dispersion-before",
    );
    let right = render_material_box(
        "m8-dispersion-right",
        &assets,
        on,
        None,
        true,
        "dispersion-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-dispersion-material-feature";
    left.proof_class = "dispersion-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-dispersion-material-feature:dispersion-before-after",
    ));
    left
}

fn render_transmission_volume_material_feature() -> VisualArtifact {
    let assets = Assets::new();
    let blocked_texture = load_pixel_texture(&assets, [0, 0, 0, 255], TextureColorSpace::Linear);
    let transmission_texture =
        load_pixel_texture(&assets, [255, 0, 0, 255], TextureColorSpace::Linear);
    let thickness_texture =
        load_pixel_texture(&assets, [0, 255, 0, 255], TextureColorSpace::Linear);
    let base = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(190, 205, 230), 0.0, 0.08)
        .with_transmission_factor(1.0)
        .with_ior(1.7)
        .with_thickness_factor(2.0)
        .with_thickness_texture(thickness_texture)
        .with_attenuation_distance(1.0)
        .with_attenuation_color(Color::from_linear_rgb(0.08, 0.35, 1.0));
    let off = base.clone().with_transmission_texture(blocked_texture);
    let on = base.with_transmission_texture(transmission_texture);
    let mut left = render_material_box(
        "m8-transmission-volume-left",
        &assets,
        off,
        None,
        true,
        "transmission-volume-before",
    );
    let right = render_material_box(
        "m8-transmission-volume-right",
        &assets,
        on,
        None,
        true,
        "transmission-volume-after",
    );

    for y in 0..left.height {
        for x in left.width / 2..left.width {
            let src = ((y * right.width + x) * 4) as usize;
            let dst = ((y * left.width + x) * 4) as usize;
            left.rgba[dst..dst + 4].copy_from_slice(&right.rgba[src..src + 4]);
        }
    }
    left.name = "m8-transmission-volume-material-feature";
    left.proof_class = "transmission-volume-before-after-cpu-headless-256";
    left.source_hash = Some(fnv1a64_hex(
        b"generated-rust-scene:m8-transmission-volume-material-feature:transmission-volume-before-after",
    ));
    left
}

fn render_material_box<F>(
    name: &'static str,
    assets: &Assets<F>,
    material: MaterialDesc,
    environment: Option<EnvironmentHandle>,
    add_light: bool,
    proof_class: &'static str,
) -> VisualArtifact {
    render_material_box_with_light_yaw(
        name,
        assets,
        material,
        environment,
        add_light,
        0.0,
        proof_class,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_material_box_with_light_yaw<F>(
    name: &'static str,
    assets: &Assets<F>,
    material: MaterialDesc,
    environment: Option<EnvironmentHandle>,
    add_light: bool,
    light_yaw_deg: f32,
    proof_class: &'static str,
) -> VisualArtifact {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.55, 0.55, 0.55));
    let material = assets.create_material(material);
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    if add_light {
        let builder =
            scene.directional_light(DirectionalLight::key_light().with_illuminance_lux(12_000.0));
        if light_yaw_deg == 0.0 {
            builder.add().expect("light inserts");
        } else {
            builder
                .transform(Transform::IDENTITY.rotate_y_deg(light_yaw_deg))
                .add()
                .expect("rotated light inserts");
        }
    }
    let camera = scene.add_default_camera().expect("camera inserts");
    let environment = environment.or_else(|| add_light.then(|| assets.default_environment()));
    let mut artifact = render_scene_with_assets(name, scene, camera, assets, environment);
    artifact.proof_class = proof_class;
    artifact.source_hash = Some(fnv1a64_hex(
        format!("generated-rust-scene:{name}:{proof_class}").as_bytes(),
    ));
    artifact
}

fn render_scene_with_assets<F>(
    name: &'static str,
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets<F>,
    environment: Option<EnvironmentHandle>,
) -> VisualArtifact {
    let (width, height) = (256, 256);
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    if let Some(environment) = environment {
        renderer.set_environment(environment);
    }
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("asset scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let stats = renderer.stats();
    VisualArtifact {
        name,
        width,
        height,
        rgba: renderer.frame_rgba8().to_vec(),
        stats_textures: stats.textures,
        stats_environments: stats.environments,
        proof_class: "headless-material-smoke",
        source: "generated-rust-scene".to_string(),
        source_hash: None,
    }
}

fn load_pixel_texture<F: AssetFetcher>(
    assets: &Assets<F>,
    pixel: [u8; 4],
    color_space: TextureColorSpace,
) -> scena::TextureHandle {
    let png = png_rgba8(1, 1, &[pixel]);
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    let uri = format!("data:image/png;base64,{encoded}");
    pollster::block_on(assets.load_texture(uri, color_space)).expect("pixel texture loads")
}

fn textured_material_gltf() -> String {
    let mut buffer = Vec::new();
    for value in [-0.65_f32, -0.65, 0.0, 0.65, -0.65, 0.0, 0.0, 0.65, 0.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0.0_f32, 0.0, 1.0, 0.0, 0.5, 1.0] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(buffer);
    format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "extensionsUsed": [
                "KHR_materials_unlit",
                "KHR_texture_transform",
                "KHR_materials_emissive_strength"
            ],
            "extensionsRequired": [
                "KHR_materials_unlit",
                "KHR_texture_transform",
                "KHR_materials_emissive_strength"
            ],
            "images": [
                {{ "uri": "base.png" }},
                {{ "uri": "normal.png" }},
                {{ "uri": "metallic_roughness.png" }},
                {{ "uri": "occlusion.png" }},
                {{ "uri": "emissive.png" }}
            ],
            "textures": [
                {{ "source": 0, "sampler": 0 }},
                {{ "source": 1, "sampler": 0 }},
                {{ "source": 2, "sampler": 0 }},
                {{ "source": 3, "sampler": 0 }},
                {{ "source": 4, "sampler": 0 }}
            ],
            "samplers": [
                {{ "magFilter": 9728, "minFilter": 9728, "wrapS": 10497, "wrapT": 10497 }}
            ],
            "materials": [{{
                "pbrMetallicRoughness": {{
                    "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                    "baseColorTexture": {{
                        "index": 0,
                        "extensions": {{
                            "KHR_texture_transform": {{ "offset": [0.0, 0.0], "scale": [1.0, 1.0] }}
                        }}
                    }},
                    "metallicRoughnessTexture": {{ "index": 2 }},
                    "metallicFactor": 0.2,
                    "roughnessFactor": 0.6
                }},
                "normalTexture": {{ "index": 1 }},
                "occlusionTexture": {{ "index": 3 }},
                "emissiveTexture": {{ "index": 4 }},
                "emissiveFactor": [0.0, 0.0, 0.0],
                "extensions": {{
                    "KHR_materials_unlit": {{}},
                    "KHR_materials_emissive_strength": {{ "emissiveStrength": 1.0 }}
                }},
                "alphaMode": "OPAQUE",
                "doubleSided": true
            }}],
            "meshes": [{{
                "primitives": [{{
                    "attributes": {{ "POSITION": 0, "TEXCOORD_0": 1 }},
                    "indices": 2,
                    "material": 0
                }}]
            }}],
            "nodes": [{{ "name": "TexturedVisualProof", "mesh": 0 }}],
            "buffers": [{{ "byteLength": 66, "uri": "data:application/octet-stream;base64,{encoded}" }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 24 }},
                {{ "buffer": 0, "byteOffset": 60, "byteLength": 6 }}
            ],
            "accessors": [
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [-0.65, -0.65, 0.0], "max": [0.65, 0.65, 0.0] }},
                {{ "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }},
                {{ "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ]
        }}"#
    )
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn pixel_at(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * width + x) * 4) as usize;
    rgba[offset..offset + 4]
        .try_into()
        .expect("pixel slice has four channels")
}

fn write_ppm_artifact(dir: &Path, artifact: &VisualArtifact) {
    let mut ppm = format!("P6\n{} {}\n255\n", artifact.width, artifact.height).into_bytes();
    for pixel in artifact.rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{}.ppm", artifact.name)), ppm)
        .expect("PPM artifact can be written");
    let source_hash = artifact
        .source_hash
        .as_deref()
        .unwrap_or("generated-scene-no-source-bytes");
    fs::write(
        dir.join(format!("{}.toml", artifact.name)),
        format!(
            "[artifact]\nname = \"{}\"\nformat = \"ppm\"\nencoding = \"srgb8\"\nwidth = {}\nheight = {}\nbackend = \"Headless\"\nadapter = \"cpu-headless-no-gpu-adapter\"\nrenderer_settings = \"Renderer::headless {}x{} default render mode\"\nproof_class = \"{}\"\nsource = \"{}\"\nsource_hash = \"{}\"\ntolerance = \"material-visible-output-smoke\"\ncolor_management = \"linear-material-to-aces-srgb-output\"\n",
            artifact.name,
            artifact.width,
            artifact.height,
            artifact.width,
            artifact.height,
            artifact.proof_class,
            artifact.source,
            source_hash,
        ),
    )
    .expect("artifact metadata can be written");
}

fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m8-visual")
}

struct VisualArtifact {
    name: &'static str,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    stats_textures: u64,
    stats_environments: u64,
    proof_class: &'static str,
    source: String,
    source_hash: Option<String>,
}

impl VisualArtifact {
    fn center_pixel(&self) -> [u8; 4] {
        let center = ((self.height / 2) * self.width + (self.width / 2)) as usize * 4;
        [
            self.rgba[center],
            self.rgba[center + 1],
            self.rgba[center + 2],
            self.rgba[center + 3],
        ]
    }
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn png_rgba8(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(pixels.len(), (width * height) as usize);
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PNG header writes");
        let raw = pixels
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .collect::<Vec<_>>();
        writer.write_image_data(&raw).expect("PNG payload writes");
    }
    bytes
}

#[derive(Debug, Clone)]
struct MemoryFetcher {
    files: BTreeMap<AssetPath, Vec<u8>>,
}

impl MemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: files.into_iter().collect(),
        }
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}
