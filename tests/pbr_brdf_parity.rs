#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use scena::{
    AntiAliasing, Assets, Background, CameraKey, Color, DirectionalLight, GeometryDesc,
    GeometryTopology, GeometryVertex, MaterialDesc, PerspectiveCamera, Scene, Transform, Vec3,
};
use support::parity::{
    OwnedRgbaFrame, ParitySweep, PixelRegion, compare_frames_in_region,
    render_scene_cpu_gpu_pair_with_renderer, require_cpu_gpu_parity_adapter_or_skip,
};

const WIDTH: u32 = 112;
const HEIGHT: u32 = 84;
const SCHEMA: &str = "scena.core_pbr_brdf_parity_sweep.v1";
const REGION: PixelRegion = PixelRegion {
    x: 34,
    y: 24,
    width: 44,
    height: 34,
};

#[derive(Debug, Clone, Copy)]
struct PbrCase {
    name: &'static str,
    base_color: Color,
    metallic: f32,
    roughness: f32,
}

#[test]
fn core_pbr_brdf_matches_cpu_and_gpu_across_metallic_roughness_sweep() {
    if !require_cpu_gpu_parity_adapter_or_skip(
        "core_pbr_brdf_matches_cpu_and_gpu_across_metallic_roughness_sweep",
    ) {
        return;
    }

    let artifacts = artifact_dir();
    let mut sweep = ParitySweep::new(SCHEMA);
    let mut cpu_frames = Vec::new();

    for case in pbr_cases() {
        let pair = render_scene_cpu_gpu_pair_with_renderer(
            case.name,
            WIDTH,
            HEIGHT,
            AntiAliasing::None,
            |renderer| renderer.set_background(Background::DarkStudio),
            move |scene, assets| build_direct_pbr_scene(scene, assets, case),
        );
        write_ppm(
            &artifacts.join(format!("{}-cpu.ppm", case.name)),
            WIDTH,
            HEIGHT,
            &pair.cpu.rgba8,
        );
        write_ppm(
            &artifacts.join(format!("{}-gpu.ppm", case.name)),
            WIDTH,
            HEIGHT,
            &pair.gpu.rgba8,
        );

        let comparison = sweep.compare_region(
            format!("{}-cpu-vs-gpu", case.name),
            pair.cpu.borrowed(),
            pair.gpu.borrowed(),
            REGION,
        );
        assert!(
            comparison.rmse <= 0.075,
            "{} CPU/GPU direct-PBR RMSE too high: {:.5}",
            case.name,
            comparison.rmse
        );
        assert!(
            comparison.channel_delta.mean_channel_delta <= 14.0,
            "{} CPU/GPU direct-PBR mean channel delta too high: {:.5}",
            case.name,
            comparison.channel_delta.mean_channel_delta
        );
        assert!(
            comparison.left_structure.luminance_range > 0.006
                && comparison.right_structure.luminance_range > 0.006,
            "{} must retain measurable material shading structure: CPU {:.5}, GPU {:.5}",
            case.name,
            comparison.left_structure.luminance_range,
            comparison.right_structure.luminance_range
        );
        cpu_frames.push((case.name, pair.cpu));
    }

    assert_material_response_is_not_inert(&cpu_frames);
    sweep.write_json(&artifacts.join("pbr-brdf-parity.json"), &[]);
}

fn pbr_cases() -> [PbrCase; 5] {
    [
        PbrCase {
            name: "dielectric-glossy",
            base_color: Color::from_srgb_u8(191, 78, 52),
            metallic: 0.0,
            roughness: 0.12,
        },
        PbrCase {
            name: "dielectric-rough",
            base_color: Color::from_srgb_u8(191, 78, 52),
            metallic: 0.0,
            roughness: 0.82,
        },
        PbrCase {
            name: "metal-glossy",
            base_color: Color::from_srgb_u8(210, 214, 220),
            metallic: 1.0,
            roughness: 0.08,
        },
        PbrCase {
            name: "metal-mid",
            base_color: Color::from_srgb_u8(210, 214, 220),
            metallic: 1.0,
            roughness: 0.42,
        },
        PbrCase {
            name: "mixed-mid",
            base_color: Color::from_srgb_u8(170, 196, 230),
            metallic: 0.45,
            roughness: 0.32,
        },
    ]
}

fn build_direct_pbr_scene(scene: &mut Scene, assets: &Assets, case: PbrCase) -> CameraKey {
    let geometry = assets.create_geometry(subdivided_front_plane(1.08, 0.78, 32, 24));
    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(case.base_color, case.metallic, case.roughness)
            .with_double_sided(false),
    );
    scene
        .mesh(geometry, material)
        .add()
        .expect("PBR sweep mesh inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(4_500.0))
        .transform(Transform::default().rotate_x_deg(-18.0).rotate_y_deg(24.0))
        .add()
        .expect("PBR sweep directional light inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.15)),
        )
        .expect("PBR sweep camera inserts");
    scene
        .set_active_camera(camera)
        .expect("PBR sweep camera is active");
    camera
}

fn subdivided_front_plane(width: f32, height: f32, columns: u32, rows: u32) -> GeometryDesc {
    let columns = columns.max(1);
    let rows = rows.max(1);
    let mut vertices = Vec::with_capacity(((columns + 1) * (rows + 1)) as usize);
    let mut indices = Vec::with_capacity((columns * rows * 6) as usize);
    for row in 0..=rows {
        let y = (row as f32 / rows as f32 - 0.5) * height;
        for column in 0..=columns {
            let x = (column as f32 / columns as f32 - 0.5) * width;
            vertices.push(GeometryVertex {
                position: Vec3::new(x, y, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            });
        }
    }
    let stride = columns + 1;
    for row in 0..rows {
        for column in 0..columns {
            let a = row * stride + column;
            let b = a + 1;
            let d = (row + 1) * stride + column;
            let c = d + 1;
            indices.extend_from_slice(&[a, b, c, a, c, d]);
        }
    }
    GeometryDesc::try_new(GeometryTopology::Triangles, vertices, indices)
        .expect("subdivided PBR parity plane is valid")
}

fn assert_material_response_is_not_inert(frames: &[(&'static str, OwnedRgbaFrame)]) {
    let dielectric_delta = compare_named_frames(frames, "dielectric-glossy", "dielectric-rough");
    assert!(
        dielectric_delta.rmse > 0.001,
        "roughness must visibly change dielectric direct-PBR output: rmse {:.5}",
        dielectric_delta.rmse
    );

    let metal_delta = compare_named_frames(frames, "metal-glossy", "metal-mid");
    assert!(
        metal_delta.rmse > 0.018,
        "roughness must visibly change metallic direct-PBR output: rmse {:.5}",
        metal_delta.rmse
    );

    let metallic_delta = compare_named_frames(frames, "dielectric-rough", "metal-mid");
    assert!(
        metallic_delta.rmse > 0.05,
        "metallic must visibly change direct-PBR output: rmse {:.5}",
        metallic_delta.rmse
    );
}

fn compare_named_frames(
    frames: &[(&'static str, OwnedRgbaFrame)],
    left: &'static str,
    right: &'static str,
) -> support::parity::ParityComparison {
    let left = named_frame(frames, left);
    let right = named_frame(frames, right);
    compare_frames_in_region(left.borrowed(), right.borrowed(), REGION)
}

fn named_frame<'a>(
    frames: &'a [(&'static str, OwnedRgbaFrame)],
    name: &'static str,
) -> &'a OwnedRgbaFrame {
    frames
        .iter()
        .find_map(|(frame_name, frame)| (*frame_name == name).then_some(frame))
        .unwrap_or_else(|| panic!("{name} frame must exist"))
}

fn artifact_dir() -> PathBuf {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/pbr-brdf-parity");
    fs::create_dir_all(&path).expect("artifact dir exists");
    path
}

fn write_ppm(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    let mut file = File::create(path).expect("create artifact PPM");
    writeln!(file, "P6\n{width} {height}\n255").expect("write PPM header");
    for pixel in rgba.chunks_exact(4) {
        file.write_all(&pixel[..3]).expect("write PPM pixel");
    }
}
