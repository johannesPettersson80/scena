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
/// a plain metallic-roughness sphere may show in this scene. What it measures
/// is the base-layer image-based lighting: the CPU renderer weights it with
/// the analytic split-sum fit and single scattering, the shader with the baked
/// table and multiple scattering. That difference grows with the reflectance
/// at normal incidence, so the metal sits far above the dielectric. Mesa
/// lavapipe measures 0.67 / 2.12 / 2.89 (red / green / blue) for the dielectric
/// and 15.12 / 15.71 / 16.66 for the metal.
const PLAIN_DIELECTRIC_CEILING: f64 = 3.5;
const PLAIN_METAL_CEILING: f64 = 18.0;
/// A layered sphere may differ by at most this much more than the plain sphere
/// it is built on. Both renderers evaluate the layers with the same formulas,
/// so a layer adds nothing systematic. Worst channel on lavapipe, before the
/// CPU renderer evaluated layers the way the shader does and after:
///
/// | sphere                  | before | after | plain base |
/// |-------------------------|-------:|------:|-----------:|
/// | clearcoat               |   23.5 |   2.3 |        2.9 |
/// | sheen                   |   71.2 |   2.3 |        2.9 |
/// | anisotropy              |    6.9 |   2.9 |        2.9 |
/// | iridescence             |  159.9 |   2.5 |        2.9 |
/// | clearcoat over sheen    |   58.2 |   2.1 |        2.9 |
/// | anisotropy on metal     |   17.8 |  16.5 |       16.7 |
/// | iridescence on metal    |   69.0 |  16.4 |       16.7 |
/// | index of refraction 2.0 |   37.3 |   7.3 |  see below |
const LAYER_MARGIN_OVER_BASELINE: f64 = 0.5;
/// A layer must move the image by at least this much on both renderers, so
/// agreement cannot come from both of them ignoring it.
const MIN_LAYER_EFFECT: f64 = 1.0;

#[derive(Clone, Copy, PartialEq)]
enum Base {
    Dielectric,
    Metal,
}

impl Base {
    const fn name(self) -> &'static str {
        match self {
            Self::Dielectric => "plain-dielectric",
            Self::Metal => "plain-metal",
        }
    }

    const fn ceiling(self) -> f64 {
        match self {
            Self::Dielectric => PLAIN_DIELECTRIC_CEILING,
            Self::Metal => PLAIN_METAL_CEILING,
        }
    }

    fn material(self) -> MaterialDesc {
        match self {
            Self::Dielectric => {
                MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(170, 60, 44), 0.0, 0.4)
            }
            Self::Metal => {
                MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(200, 204, 210), 1.0, 0.38)
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Bound {
    /// A layer on top of a base: the plain base's difference plus the margin.
    LayerOver,
    /// A change of the base reflectance itself. An index of refraction of 2.0
    /// raises a dielectric's reflectance from 0.04 to 0.111, and the
    /// base-layer difference grows with it, so the sphere sits between the
    /// plain dielectric and the plain metal (reflectance about 0.6) and must
    /// stay below the metal.
    BelowPlainMetal,
}

struct LayerCase {
    name: &'static str,
    base: Base,
    material: MaterialDesc,
    bound: Bound,
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
    let anisotropy = |material: MaterialDesc| material.with_anisotropy_strength_factor(0.8);
    let iridescence = |material: MaterialDesc| {
        material
            .with_iridescence_factor(1.0)
            .with_iridescence_ior(1.3)
            .with_iridescence_thickness_range_nm(100.0, 400.0)
    };
    let layer = |name, base: Base, material| LayerCase {
        name,
        base,
        material,
        bound: Bound::LayerOver,
    };
    vec![
        layer(
            "clearcoat",
            Base::Dielectric,
            clearcoat(Base::Dielectric.material()),
        ),
        layer(
            "sheen",
            Base::Dielectric,
            sheen(Base::Dielectric.material()),
        ),
        layer(
            "anisotropy",
            Base::Dielectric,
            anisotropy(Base::Dielectric.material()),
        ),
        layer(
            "iridescence",
            Base::Dielectric,
            iridescence(Base::Dielectric.material()),
        ),
        layer(
            "clearcoat-sheen",
            Base::Dielectric,
            sheen(clearcoat(Base::Dielectric.material())),
        ),
        layer(
            "anisotropy-metal",
            Base::Metal,
            anisotropy(Base::Metal.material()),
        ),
        layer(
            "iridescence-metal",
            Base::Metal,
            iridescence(Base::Metal.material()),
        ),
        LayerCase {
            name: "index-of-refraction",
            base: Base::Dielectric,
            material: Base::Dielectric.material().with_ior(2.0),
            bound: Bound::BelowPlainMetal,
        },
    ]
}

struct PlainFrames {
    base: Base,
    cpu: Vec<u8>,
    gpu: Vec<u8>,
    delta: MaskedDelta,
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

    let plain = [Base::Dielectric, Base::Metal].map(|base| {
        let cpu_frame = cpu.render(base.material());
        let gpu_frame = gpu.render(base.material());
        write_png(
            &artifacts.join(format!("{}-cpu.png", base.name())),
            &cpu_frame,
        );
        write_png(
            &artifacts.join(format!("{}-gpu.png", base.name())),
            &gpu_frame,
        );
        let delta = masked_delta(&cpu_frame, &gpu_frame);
        report(base.name(), &delta);
        PlainFrames {
            base,
            cpu: cpu_frame,
            gpu: gpu_frame,
            delta,
        }
    });
    for frames in &plain {
        records.push(Record::new(frames.base.name(), &frames.delta, None));
        assert!(
            frames.delta.worst_channel() <= frames.base.ceiling(),
            "the {} sphere differs between CPU and GPU by {:?} per channel, above the {} \
             ceiling the layer tolerances are derived from",
            frames.base.name(),
            frames.delta.mean_abs,
            frames.base.ceiling(),
        );
        assertions += 1;
    }
    let plain_of = |base: Base| {
        plain
            .iter()
            .find(|frames| frames.base == base)
            .expect("both plain bases are rendered")
    };

    for case in layer_cases() {
        let base = plain_of(case.base);
        let tolerance = match case.bound {
            Bound::LayerOver => base.delta.worst_channel() + LAYER_MARGIN_OVER_BASELINE,
            Bound::BelowPlainMetal => plain_of(Base::Metal).delta.worst_channel(),
        };
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

        let cpu_effect = masked_delta(&cpu_frame, &base.cpu).mean;
        let gpu_effect = masked_delta(&gpu_frame, &base.gpu).mean;
        assert!(
            cpu_effect >= MIN_LAYER_EFFECT && gpu_effect >= MIN_LAYER_EFFECT,
            "{} must change the rendered sphere on both renderers; CPU moved {cpu_effect:.3} and \
             GPU moved {gpu_effect:.3} 8-bit steps from the {} sphere",
            case.name,
            case.base.name(),
        );
        assertions += 1;
        assert!(
            delta.worst_channel() <= tolerance,
            "{} differs between CPU and GPU by {:?} per channel, above {tolerance:.3}; the {} \
             sphere differs by {:.3} in the same run",
            case.name,
            delta.mean_abs,
            case.base.name(),
            base.delta.worst_channel(),
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
        let material = assets.create_material(Base::Dielectric.material());
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
        "  \"plain_dielectric_ceiling\": {PLAIN_DIELECTRIC_CEILING}, \"plain_metal_ceiling\": {PLAIN_METAL_CEILING},"
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
