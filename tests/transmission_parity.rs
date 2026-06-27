#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use std::fs;

use scena::{
    AlphaMode, AntiAliasing, Assets, Background, CameraKey, Color, GeometryDesc, MaterialDesc,
    OrderIndependentTransparencyConfig, PerspectiveCamera, Scene, Transform, Vec3,
};
use support::parity::{
    ParitySweep, PixelRegion, render_scene_cpu_gpu_pair_with_renderer,
    require_cpu_gpu_parity_adapter_or_skip,
};

const WIDTH: u32 = 128;
const HEIGHT: u32 = 96;
const SCHEMA: &str = "scena.physical_glass_transmission_parity_sweep.v1";
// Measure the pane interior, not the silhouette edge: the sweep is a physical
// transmission parity gate, while CPU/GPU edge coverage is already covered by
// the geometry AA/parity tests.
const GLASS_REGION: PixelRegion = PixelRegion {
    x: 45,
    y: 32,
    width: 34,
    height: 32,
};

#[derive(Debug, Clone, Copy)]
struct TransmissionCase {
    name: &'static str,
    roughness: f32,
    thickness: f32,
    attenuation_distance: f32,
    attenuation_color: Color,
}

#[test]
fn physical_glass_transmission_matches_cpu_and_gpu_across_volume_sweep() {
    if !require_cpu_gpu_parity_adapter_or_skip(
        "physical_glass_transmission_matches_cpu_and_gpu_across_volume_sweep",
    ) {
        return;
    }

    let artifacts = artifact_dir();
    let mut sweep = ParitySweep::new(SCHEMA);

    for case in transmission_cases() {
        let pair = render_scene_cpu_gpu_pair_with_renderer(
            case.name,
            WIDTH,
            HEIGHT,
            AntiAliasing::None,
            |renderer| {
                renderer.set_background(Background::White);
                renderer.set_order_independent_transparency(Some(
                    OrderIndependentTransparencyConfig::weighted_blended(),
                ));
            },
            move |scene, assets| build_transmission_scene(scene, assets, case),
        );
        fs::write(
            artifacts.join(format!("{}-cpu.rgba", case.name)),
            &pair.cpu.rgba8,
        )
        .expect("CPU transmission frame artifact writes");
        fs::write(
            artifacts.join(format!("{}-gpu.rgba", case.name)),
            &pair.gpu.rgba8,
        )
        .expect("GPU transmission frame artifact writes");

        let comparison = sweep.compare_region(
            format!("{}-cpu-vs-gpu", case.name),
            pair.cpu.borrowed(),
            pair.gpu.borrowed(),
            GLASS_REGION,
        );
        assert!(
            comparison.rmse <= 0.075,
            "{} CPU/GPU transmission RMSE too high: {:.5}",
            case.name,
            comparison.rmse
        );
        assert!(
            comparison.channel_delta.mean_channel_delta <= 16.0,
            "{} CPU/GPU transmission mean channel delta too high: {:.5}",
            case.name,
            comparison.channel_delta.mean_channel_delta
        );
        assert!(
            comparison.left_structure.sobel_luminance_energy > 0.010
                && comparison.right_structure.sobel_luminance_energy > 0.010,
            "{} transmitted scene color must retain structured backdrop detail: CPU {:.5}, GPU {:.5}",
            case.name,
            comparison.left_structure.sobel_luminance_energy,
            comparison.right_structure.sobel_luminance_energy
        );
    }

    sweep.write_json(
        &artifacts.join("physical-glass-transmission-parity.json"),
        &[],
    );
}

fn transmission_cases() -> [TransmissionCase; 4] {
    [
        TransmissionCase {
            name: "clear-thin-neutral",
            roughness: 0.04,
            thickness: 0.08,
            attenuation_distance: 3.0,
            attenuation_color: Color::WHITE,
        },
        TransmissionCase {
            name: "clear-thick-blue",
            roughness: 0.08,
            thickness: 0.42,
            attenuation_distance: 0.8,
            attenuation_color: Color::from_linear_rgb(0.35, 0.58, 1.0),
        },
        TransmissionCase {
            name: "frosted-mid-green",
            roughness: 0.46,
            thickness: 0.35,
            attenuation_distance: 0.9,
            attenuation_color: Color::from_linear_rgb(0.42, 1.0, 0.64),
        },
        TransmissionCase {
            name: "frosted-thick-warm",
            roughness: 0.72,
            thickness: 0.62,
            attenuation_distance: 0.65,
            attenuation_color: Color::from_linear_rgb(1.0, 0.66, 0.40),
        },
    ]
}

fn build_transmission_scene(
    scene: &mut Scene,
    assets: &Assets,
    case: TransmissionCase,
) -> CameraKey {
    let stripe_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.30, 1.45, 0.02));
    let stripe_materials = [
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(28, 56, 190))),
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(235, 196, 44))),
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(210, 42, 68))),
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(40, 180, 106))),
    ];
    for (index, material) in stripe_materials.iter().copied().enumerate() {
        let x = -0.45 + index as f32 * 0.30;
        scene
            .mesh(stripe_geometry, material)
            .transform(Transform::at(Vec3::new(x, 0.0, -0.22)))
            .add()
            .expect("transmission backdrop stripe inserts");
    }

    let glass_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.86, 0.86, 0.035));
    let glass_material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(
            Color::from_srgb_u8(210, 225, 255),
            0.0,
            case.roughness,
        )
        .with_alpha_mode(AlphaMode::Blend)
        .with_transmission_factor(1.0)
        .with_ior(1.48)
        .with_thickness_factor(case.thickness)
        .with_attenuation_distance(case.attenuation_distance)
        .with_attenuation_color(case.attenuation_color)
        .with_double_sided(true),
    );
    scene
        .mesh(glass_geometry, glass_material)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .add()
        .expect("transmission glass pane inserts");

    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.15)),
        )
        .expect("transmission camera inserts");
    scene
        .set_active_camera(camera)
        .expect("transmission camera active");
    camera
}

fn artifact_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from("target/gate-artifacts/transmission-parity");
    fs::create_dir_all(&dir).expect("transmission parity artifact dir exists");
    dir
}
