#![cfg(not(target_arch = "wasm32"))]

#[allow(dead_code)]
mod support;

use std::fs::{self, File};
use std::io::{Cursor, Write as _};
use std::path::{Path, PathBuf};

use base64::Engine as _;
use scena::{
    AntiAliasing, Assets, Background, CameraKey, Color, GeometryDesc, GeometryTopology,
    GeometryVertex, MaterialDesc, PerspectiveCamera, Scene, TextureColorSpace, Transform, Vec3,
};
use support::parity::{
    OwnedRgbaFrame, ParitySweep, PixelRegion, render_scene_cpu_gpu_pair_with_renderer,
    require_cpu_gpu_parity_adapter_or_skip,
};

const WIDTH: u32 = 192;
const HEIGHT: u32 = 128;
const LEFT_INTERIOR: PixelRegion = PixelRegion {
    x: 52,
    y: 51,
    width: 32,
    height: 28,
};
const RIGHT_INTERIOR: PixelRegion = PixelRegion {
    x: 108,
    y: 51,
    width: 32,
    height: 28,
};

#[test]
fn pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu() {
    if !require_cpu_gpu_parity_adapter_or_skip(
        "pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu",
    ) {
        return;
    }

    let pair = render_scene_cpu_gpu_pair_with_renderer(
        "pf08-perspective-texture-panels",
        WIDTH,
        HEIGHT,
        AntiAliasing::None,
        |renderer| renderer.set_background(Background::Black),
        build_scene,
    );

    let mut sweep = ParitySweep::new("scena.pf08.texture_bake_parity.v1");
    let left = sweep.compare_region(
        "left-perspective-panel-cpu-vs-gpu",
        pair.cpu.borrowed(),
        pair.gpu.borrowed(),
        LEFT_INTERIOR,
    );
    let right = sweep.compare_region(
        "right-perspective-panel-cpu-vs-gpu",
        pair.cpu.borrowed(),
        pair.gpu.borrowed(),
        RIGHT_INTERIOR,
    );
    for (name, comparison) in [("left", left), ("right", right)] {
        assert!(
            comparison.rmse <= 0.08,
            "{name} depth-skewed textured panel must retain perspective-correct CPU/GPU parity; rmse={:.5}",
            comparison.rmse
        );
        assert!(
            comparison.channel_delta.mean_channel_delta <= 14.0,
            "{name} adaptive CPU bake must stay close to fragment-sampled GPU output; mean_channel_delta={:.5}",
            comparison.channel_delta.mean_channel_delta
        );
        assert!(
            comparison.left_structure.luminance_range > 0.08
                && comparison.right_structure.luminance_range > 0.08,
            "{name} panel must preserve the authored texture gradient on both backends: cpu={:.5}, gpu={:.5}",
            comparison.left_structure.luminance_range,
            comparison.right_structure.luminance_range
        );
    }

    for frame in [&pair.cpu, &pair.gpu] {
        assert_material_identity(frame, LEFT_INTERIOR, 0, 2, "left red material");
        assert_material_identity(frame, RIGHT_INTERIOR, 2, 0, "right blue material");
        assert_shared_triangle_diagonal_has_no_gap(frame, 54..=82, |x| 142 - x, "left");
        assert_shared_triangle_diagonal_has_no_gap(frame, 110..=138, |x| 190 - x, "right");
    }

    let artifacts = artifact_dir();
    write_ppm(&artifacts.join("cpu.ppm"), &pair.cpu);
    write_ppm(&artifacts.join("gpu.ppm"), &pair.gpu);
    sweep.write_json(
        &artifacts.join("pf08-texture-bake-parity.json"),
        &[
            ("cpu_backend", "\"headless\"".to_owned()),
            ("gpu_backend", "\"headless_gpu\"".to_owned()),
            (
                "acceptance",
                "[\"shared-triangle-seam\",\"perspective-interpolation\",\"material-identity\",\"cpu-gpu-comparison\"]".to_owned(),
            ),
        ],
    );
}

fn build_scene(scene: &mut Scene, assets: &Assets) -> CameraKey {
    let left_texture = load_texture(
        assets,
        "left",
        gradient_texture(|u, v| {
            [
                (80 + u * 150 / 255) as u8,
                (42 + v * 92 / 255) as u8,
                28,
                255,
            ]
        }),
    );
    let right_texture = load_texture(
        assets,
        "right",
        gradient_texture(|u, v| {
            [
                28,
                (42 + u * 74 / 255) as u8,
                (80 + v * 150 / 255) as u8,
                255,
            ]
        }),
    );
    let left_material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE)
            .with_base_color_texture(left_texture)
            .with_double_sided(true),
    );
    let right_material = assets.create_material(
        MaterialDesc::unlit(Color::WHITE)
            .with_base_color_texture(right_texture)
            .with_double_sided(true),
    );
    let left_geometry = assets.create_geometry(perspective_quad([
        Vec3::new(-1.15, -0.72, 0.55),
        Vec3::new(-0.15, -0.56, -0.55),
        Vec3::new(-0.15, 0.56, -0.55),
        Vec3::new(-1.15, 0.72, 0.55),
    ]));
    let right_geometry = assets.create_geometry(perspective_quad([
        Vec3::new(0.15, -0.56, -0.55),
        Vec3::new(1.15, -0.72, 0.55),
        Vec3::new(1.15, 0.72, 0.55),
        Vec3::new(0.15, 0.56, -0.55),
    ]));
    scene
        .mesh(left_geometry, left_material)
        .add()
        .expect("left perspective panel inserts");
    scene
        .mesh(right_geometry, right_material)
        .add()
        .expect("right perspective panel inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("PF08 camera inserts");
    scene
        .set_active_camera(camera)
        .expect("PF08 camera activates");
    camera
}

fn perspective_quad(positions: [Vec3; 4]) -> GeometryDesc {
    GeometryDesc::try_new_with_vertex_colors_and_tex_coords(
        GeometryTopology::Triangles,
        positions
            .into_iter()
            .map(|position| GeometryVertex {
                position,
                normal: Vec3::Z,
            })
            .collect(),
        vec![0, 1, 2, 0, 2, 3],
        vec![Color::WHITE; 4],
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    )
    .expect("perspective textured quad validates")
}

fn gradient_texture(mut pixel: impl FnMut(u16, u16) -> [u8; 4]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(8 * 8 * 4);
    for y in 0..8_u16 {
        for x in 0..8_u16 {
            rgba.extend_from_slice(&pixel(x * 255 / 7, y * 255 / 7));
        }
    }
    png_rgba8(8, 8, &rgba)
}

fn load_texture(assets: &Assets, name: &str, png: Vec<u8>) -> scena::TextureHandle {
    let uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    );
    pollster::block_on(assets.load_texture(uri, TextureColorSpace::Srgb))
        .unwrap_or_else(|error| panic!("{name} PF08 texture loads: {error:?}"))
}

fn png_rgba8(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("PF08 PNG header writes");
        writer
            .write_image_data(rgba)
            .expect("PF08 PNG pixels write");
    }
    bytes
}

fn assert_material_identity(
    frame: &OwnedRgbaFrame,
    region: PixelRegion,
    dominant: usize,
    secondary: usize,
    label: &str,
) {
    let average = average_rgb(frame, region);
    assert!(
        average[dominant] > average[secondary] + 35.0,
        "{label} must retain its material/texture slot on {}: average={average:?}",
        frame.name
    );
}

fn average_rgb(frame: &OwnedRgbaFrame, region: PixelRegion) -> [f32; 3] {
    let mut total = [0_u64; 3];
    let mut count = 0_u64;
    for y in region.y..region.end_y() {
        for x in region.x..region.end_x() {
            let offset = ((y * frame.width + x) * 4) as usize;
            for (channel, channel_total) in total.iter_mut().enumerate() {
                *channel_total += u64::from(frame.rgba8[offset + channel]);
            }
            count += 1;
        }
    }
    total.map(|channel| channel as f32 / count as f32)
}

fn assert_shared_triangle_diagonal_has_no_gap(
    frame: &OwnedRgbaFrame,
    x_range: std::ops::RangeInclusive<u32>,
    expected_y: impl Fn(u32) -> u32,
    label: &str,
) {
    for x in x_range.step_by(4) {
        let y = expected_y(x);
        let brightest = (-1_i32..=1)
            .map(|dy| pixel(frame, x, y.saturating_add_signed(dy)))
            .map(|pixel| pixel[0].max(pixel[1]).max(pixel[2]))
            .max()
            .expect("diagonal sample exists");
        assert!(
            brightest >= 28,
            "{label} shared-triangle diagonal must not expose the black background on {} at ({x},{y})",
            frame.name
        );
    }
}

fn pixel(frame: &OwnedRgbaFrame, x: u32, y: u32) -> [u8; 4] {
    let offset = ((y * frame.width + x) * 4) as usize;
    frame.rgba8[offset..offset + 4]
        .try_into()
        .expect("pixel has four channels")
}

fn artifact_dir() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/gate-artifacts/pf08-texture-bake-parity");
    fs::create_dir_all(&path).expect("PF08 artifact directory creates");
    path
}

fn write_ppm(path: &Path, frame: &OwnedRgbaFrame) {
    let mut file = File::create(path).expect("PF08 PPM creates");
    writeln!(file, "P6\n{} {}\n255", frame.width, frame.height).expect("PF08 PPM header writes");
    for pixel in frame.rgba8.chunks_exact(4) {
        file.write_all(&pixel[..3]).expect("PF08 PPM pixel writes");
    }
}
