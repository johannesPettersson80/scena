#![cfg(not(target_arch = "wasm32"))]

//! CPU/GPU parity for the layered glTF surface model.
//!
//! `surface_lighting.wgsl` and `src/render/prepare/pbr_contract/surface.rs`
//! evaluate one `Surface`: extension lobes are layers of the base BRDF, not
//! terms added on top of it. Each layer is rendered on a sphere under one
//! directional light plus the default environment by both renderers, and must
//! agree as closely as the plain metallic-roughness sphere it is built on,
//! measured in the same run.

#[allow(dead_code)]
mod support;

use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use scena::{
    AntiAliasing, Assets, Background, CameraKey, Color, DirectionalLight, GeometryDesc,
    MaterialDesc, NodeKey, PerspectiveCamera, Renderer, Scene, Transform, Vec3,
};
use support::parity::{
    RenderBackend, record_cpu_gpu_parity_pass, renderer_for_backend,
    require_cpu_gpu_parity_adapter_or_skip,
};

const TEST_NAME: &str =
    "layered_materials_match_cpu_and_gpu_as_closely_as_plain_metallic_roughness";
const SCHEMA: &str = "scena.layered_material_parity.v1";
const SIZE: u32 = 192;
const SPHERE_RADIUS: f32 = 0.5;
const CAMERA_DISTANCE: f32 = 1.6;
/// Share of the projected sphere radius that is measured. The outermost ring
/// is left out because the two rasterizers cover silhouette pixels differently,
/// which is a geometry-edge property the antialiasing proofs already own.
const MEASURED_RADIUS_FRACTION: f32 = 0.92;

/// Mean absolute CPU/GPU difference per colour channel, in 8-bit steps, that
/// the plain metallic-roughness sphere may show in this scene. Mesa lavapipe
/// measures 0.67 / 2.12 / 2.89 for red / green / blue. The blue channel carries
/// the specular reflection of the environment, which the CPU renderer still
/// evaluates with the analytic split-sum fit where the shader reads the baked
/// table; that is the remaining difference between the two base layers.
const PLAIN_BASELINE_CEILING: f64 = 3.5;
/// A layered sphere may differ by at most this much more than the plain one.
/// Both renderers evaluate the layers with the same formulas, so a layer adds
/// nothing systematic: lavapipe measures 2.31 (clearcoat), 2.31 (sheen) and
/// 2.07 (clearcoat over sheen) for the worst channel, below the plain sphere's
/// 2.89. Before the CPU renderer evaluated layers the way the shader does, the
/// same measurement gave 23.5, 71.2 and 58.2.
const LAYER_MARGIN_OVER_BASELINE: f64 = 0.5;
/// A layer must move the image by at least this much on both renderers, so
/// agreement cannot come from both of them ignoring it.
const MIN_LAYER_EFFECT: f64 = 1.0;

fn plain_material() -> MaterialDesc {
    MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(170, 60, 44), 0.0, 0.4)
}

struct LayerCase {
    name: &'static str,
    material: MaterialDesc,
}

fn layer_cases() -> Vec<LayerCase> {
    let clearcoat = |material: MaterialDesc| {
        material
            .with_clearcoat_factor(1.0)
            .with_clearcoat_roughness_factor(0.3)
    };
    let sheen = |material: MaterialDesc| {
        material
            .with_sheen_color_factor(Color::from_linear_rgb(0.9, 0.75, 0.6))
            .with_sheen_roughness_factor(0.45)
    };
    vec![
        LayerCase {
            name: "clearcoat",
            material: clearcoat(plain_material()),
        },
        LayerCase {
            name: "sheen",
            material: sheen(plain_material()),
        },
        LayerCase {
            name: "clearcoat-sheen",
            material: sheen(clearcoat(plain_material())),
        },
    ]
}

#[test]
fn layered_materials_match_cpu_and_gpu_as_closely_as_plain_metallic_roughness() {
    if !require_cpu_gpu_parity_adapter_or_skip(TEST_NAME) {
        return;
    }
    let artifacts = artifact_dir();
    let mut cpu = SphereRig::new(RenderBackend::Cpu);
    let mut gpu = SphereRig::new(RenderBackend::Gpu);
    let mut records = Vec::new();
    let mut assertions = 0_u64;

    let plain_cpu = cpu.render(plain_material());
    let plain_gpu = gpu.render(plain_material());
    write_png(&artifacts.join("plain-cpu.png"), &plain_cpu);
    write_png(&artifacts.join("plain-gpu.png"), &plain_gpu);
    let plain_delta = masked_delta(&plain_cpu, &plain_gpu);
    report("plain", &plain_delta);
    records.push(Record::new("plain", &plain_delta, None));
    assert!(
        plain_delta.worst_channel() <= PLAIN_BASELINE_CEILING,
        "the plain metallic-roughness sphere differs between CPU and GPU by {:?} per channel, \
         above the {PLAIN_BASELINE_CEILING} ceiling the layer tolerance is derived from",
        plain_delta.mean_abs,
    );
    assertions += 1;
    let tolerance = plain_delta.worst_channel() + LAYER_MARGIN_OVER_BASELINE;

    for case in layer_cases() {
        let cpu_frame = cpu.render(case.material.clone());
        let gpu_frame = gpu.render(case.material);
        write_png(
            &artifacts.join(format!("{}-cpu.png", case.name)),
            &cpu_frame,
        );
        write_png(
            &artifacts.join(format!("{}-gpu.png", case.name)),
            &gpu_frame,
        );
        let delta = masked_delta(&cpu_frame, &gpu_frame);
        report(case.name, &delta);
        records.push(Record::new(case.name, &delta, Some(tolerance)));

        let cpu_effect = masked_delta(&cpu_frame, &plain_cpu).mean;
        let gpu_effect = masked_delta(&gpu_frame, &plain_gpu).mean;
        assert!(
            cpu_effect >= MIN_LAYER_EFFECT && gpu_effect >= MIN_LAYER_EFFECT,
            "{} must change the rendered sphere on both renderers; CPU moved {cpu_effect:.3} and \
             GPU moved {gpu_effect:.3} 8-bit steps from the plain sphere",
            case.name,
        );
        assertions += 1;
        assert!(
            delta.worst_channel() <= tolerance,
            "{} differs between CPU and GPU by {:?} per channel, above {tolerance:.3}: the plain \
             sphere differs by {:.3} in the same run and a layer may add at most \
             {LAYER_MARGIN_OVER_BASELINE}",
            case.name,
            delta.mean_abs,
            plain_delta.worst_channel(),
        );
        assertions += 1;
    }

    write_json(&artifacts.join("layered-material-parity.json"), &records);
    record_cpu_gpu_parity_pass(
        TEST_NAME,
        &gpu.renderer
            .gpu_adapter_report()
            .expect("layered parity GPU adapter is recorded"),
        assertions,
    );
}

/// One sphere, one directional light, the default environment and one
/// renderer. The renderer is kept across materials so the environment is
/// prefiltered once per backend rather than once per case.
struct SphereRig {
    assets: Assets,
    scene: Scene,
    sphere: NodeKey,
    camera: CameraKey,
    renderer: Renderer,
}

impl SphereRig {
    fn new(backend: RenderBackend) -> Self {
        let assets = Assets::new();
        let mut scene = Scene::new();
        let geometry = assets.create_geometry(GeometryDesc::sphere(SPHERE_RADIUS, 192, 128));
        let material = assets.create_material(plain_material());
        let sphere = scene
            .mesh(geometry, material)
            .add()
            .expect("layered parity sphere inserts");
        scene
            .directional_light(DirectionalLight::default().with_illuminance_lux(10_000.0))
            .transform(Transform::default().rotate_x_deg(-32.0).rotate_y_deg(28.0))
            .add()
            .expect("layered parity directional light inserts");
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default(),
                Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE)),
            )
            .expect("layered parity camera inserts");
        scene
            .set_active_camera(camera)
            .expect("layered parity camera is active");
        let mut renderer = renderer_for_backend(backend, SIZE, SIZE, AntiAliasing::None);
        renderer.set_background(Background::DarkStudio);
        renderer.set_environment(assets.default_environment());
        Self {
            assets,
            scene,
            sphere,
            camera,
            renderer,
        }
    }

    fn render(&mut self, material: MaterialDesc) -> Vec<u8> {
        let material = self.assets.create_material(material);
        self.scene
            .set_mesh_material(self.sphere, material)
            .expect("layered parity sphere takes the case material");
        self.renderer
            .prepare_with_assets(&mut self.scene, &self.assets)
            .expect("layered parity scene prepares");
        self.renderer
            .render(&self.scene, self.camera)
            .expect("layered parity scene renders");
        self.renderer.frame_rgba8().to_vec()
    }
}

#[derive(Debug, Clone, Copy)]
struct MaskedDelta {
    /// Mean absolute difference of each colour channel, in 8-bit steps.
    mean_abs: [f64; 3],
    mean: f64,
    max: u8,
    pixels: usize,
}

impl MaskedDelta {
    fn worst_channel(&self) -> f64 {
        self.mean_abs.into_iter().fold(0.0, f64::max)
    }
}

/// Compares the pixels inside the measured disc of the projected sphere.
fn masked_delta(left: &[u8], right: &[u8]) -> MaskedDelta {
    assert_eq!(left.len(), right.len());
    assert_eq!(left.len(), (SIZE * SIZE * 4) as usize);
    let radius = measured_radius_pixels();
    let centre = SIZE as f32 * 0.5;
    let mut totals = [0_u64; 3];
    let mut max = 0_u8;
    let mut pixels = 0_usize;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 + 0.5 - centre;
            let dy = y as f32 + 0.5 - centre;
            if dx.hypot(dy) > radius {
                continue;
            }
            let offset = ((y * SIZE + x) * 4) as usize;
            for (channel, total) in totals.iter_mut().enumerate() {
                let delta = left[offset + channel].abs_diff(right[offset + channel]);
                *total += u64::from(delta);
                max = max.max(delta);
            }
            pixels += 1;
        }
    }
    assert!(pixels > 5_000, "the measured disc must cover the sphere");
    let mean_abs = totals.map(|total| total as f64 / pixels as f64);
    MaskedDelta {
        mean_abs,
        mean: mean_abs.iter().sum::<f64>() / 3.0,
        max,
        pixels,
    }
}

/// Radius of the measured disc: the sphere's projected silhouette under the
/// default 60 degree vertical field of view, pulled in by the edge margin.
fn measured_radius_pixels() -> f32 {
    let silhouette = (SPHERE_RADIUS / CAMERA_DISTANCE).asin().tan();
    let half_frame = 30.0_f32.to_radians().tan();
    silhouette / half_frame * SIZE as f32 * 0.5 * MEASURED_RADIUS_FRACTION
}

struct Record {
    name: String,
    delta: MaskedDelta,
    tolerance: Option<f64>,
}

impl Record {
    fn new(name: &str, delta: &MaskedDelta, tolerance: Option<f64>) -> Self {
        Self {
            name: name.to_string(),
            delta: *delta,
            tolerance,
        }
    }
}

fn report(name: &str, delta: &MaskedDelta) {
    eprintln!(
        "[layered-parity] {name} cpu-vs-gpu: mean_abs_rgb=[{:.3}, {:.3}, {:.3}] mean={:.3} \
         max={} pixels={}",
        delta.mean_abs[0],
        delta.mean_abs[1],
        delta.mean_abs[2],
        delta.mean,
        delta.max,
        delta.pixels,
    );
}

fn write_json(path: &Path, records: &[Record]) {
    let mut json = String::new();
    writeln!(&mut json, "{{").expect("write to string");
    writeln!(&mut json, "  \"schema\": \"{SCHEMA}\",").expect("write to string");
    writeln!(
        &mut json,
        "  \"plain_baseline_ceiling\": {PLAIN_BASELINE_CEILING},"
    )
    .expect("write to string");
    writeln!(
        &mut json,
        "  \"layer_margin_over_baseline\": {LAYER_MARGIN_OVER_BASELINE},"
    )
    .expect("write to string");
    writeln!(&mut json, "  \"records\": [").expect("write to string");
    for (index, record) in records.iter().enumerate() {
        let comma = if index + 1 == records.len() { "" } else { "," };
        let tolerance = record
            .tolerance
            .map_or_else(|| "null".to_string(), |value| format!("{value:.5}"));
        writeln!(
            &mut json,
            "    {{ \"name\": \"{}\", \"mean_abs_rgb\": [{:.5}, {:.5}, {:.5}], \"mean\": {:.5}, \
             \"max\": {}, \"pixels\": {}, \"tolerance\": {tolerance} }}{comma}",
            record.name,
            record.delta.mean_abs[0],
            record.delta.mean_abs[1],
            record.delta.mean_abs[2],
            record.delta.mean,
            record.delta.max,
            record.delta.pixels,
        )
        .expect("write to string");
    }
    writeln!(&mut json, "  ]").expect("write to string");
    writeln!(&mut json, "}}").expect("write to string");
    fs::write(path, json).expect("layered parity artifact writes");
}

fn artifact_dir() -> PathBuf {
    let path = PathBuf::from("target/gate-artifacts/layered-material-parity");
    fs::create_dir_all(&path).expect("artifact dir exists");
    path
}

fn write_png(path: &Path, rgba: &[u8]) {
    let file = File::create(path).expect("create artifact PNG");
    let mut encoder = png::Encoder::new(BufWriter::new(file), SIZE, SIZE);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .expect("write PNG header")
        .write_image_data(rgba)
        .expect("write PNG pixels");
}
