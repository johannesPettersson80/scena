#[allow(dead_code)]
mod support;

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use scena::{
    AnimationChannel, AnimationClip, AnimationInterpolation, AnimationOutput, AnimationTarget,
    AntiAliasing, AssetPath, Assets, Color, GeometryDesc, MaterialDesc, Scene, Transform, Vec3,
};
use support::parity::{
    ParitySweep, PixelRegion, RenderBackend, RgbaFrame, renderer_for_backend,
    require_cpu_gpu_parity_adapter_or_skip,
};

const WIDTH: u32 = 96;
const HEIGHT: u32 = 72;
const SCHEMA: &str = "scena.dynamic_transform_parity_sweep.v1";

#[derive(Debug)]
struct MotionCapture {
    before: Vec<u8>,
    after: Vec<u8>,
    before_centroid: (f32, f32),
    after_centroid: (f32, f32),
}

#[derive(Debug)]
struct MovementRecord {
    case_name: &'static str,
    cpu_delta_x: f32,
    gpu_delta_x: f32,
    cpu_delta_y: f32,
    gpu_delta_y: f32,
}

#[test]
fn dynamic_transform_motion_matches_cpu_and_gpu_for_authored_animation_and_imports() {
    if !require_cpu_gpu_parity_adapter_or_skip(
        "dynamic_transform_motion_matches_cpu_and_gpu_for_authored_animation_and_imports",
    ) {
        return;
    }

    let artifacts = artifact_dir();
    let region = PixelRegion {
        x: 0,
        y: 0,
        width: WIDTH,
        height: HEIGHT,
    };
    let mut sweep = ParitySweep::new(SCHEMA);
    let mut movements = Vec::new();

    compare_case(
        "authored-set-transform",
        render_authored_set_transform,
        &artifacts,
        region,
        &mut sweep,
        &mut movements,
    );
    compare_case(
        "authored-animation-seek",
        render_authored_animation_seek,
        &artifacts,
        region,
        &mut sweep,
        &mut movements,
    );
    compare_case(
        "imported-gltf-set-transform",
        render_imported_set_transform,
        &artifacts,
        region,
        &mut sweep,
        &mut movements,
    );

    let movement_json = movement_records_json(&movements);
    sweep.write_json(
        &artifacts.join("dynamic-transform-parity.json"),
        &[("movement_records", movement_json)],
    );
}

fn compare_case(
    case_name: &'static str,
    render: fn(RenderBackend, &Path, &'static str) -> MotionCapture,
    artifacts: &Path,
    region: PixelRegion,
    sweep: &mut ParitySweep,
    movements: &mut Vec<MovementRecord>,
) {
    let cpu = render(RenderBackend::Cpu, artifacts, case_name);
    let gpu = render(RenderBackend::Gpu, artifacts, case_name);

    let cpu_delta_x = cpu.after_centroid.0 - cpu.before_centroid.0;
    let gpu_delta_x = gpu.after_centroid.0 - gpu.before_centroid.0;
    let cpu_delta_y = cpu.after_centroid.1 - cpu.before_centroid.1;
    let gpu_delta_y = gpu.after_centroid.1 - gpu.before_centroid.1;
    assert!(
        cpu_delta_x > 10.0,
        "{case_name} CPU path must move rendered object pixels: before={:?}, after={:?}",
        cpu.before_centroid,
        cpu.after_centroid
    );
    assert!(
        gpu_delta_x > 10.0,
        "{case_name} GPU path must move rendered object pixels, not only update scene state: before={:?}, after={:?}",
        gpu.before_centroid,
        gpu.after_centroid
    );
    assert!(
        (cpu_delta_x - gpu_delta_x).abs() <= 8.0,
        "{case_name} CPU/GPU movement delta must agree: cpu_delta_x={cpu_delta_x:.2}, gpu_delta_x={gpu_delta_x:.2}"
    );
    assert!(
        (cpu_delta_y - gpu_delta_y).abs() <= 4.0,
        "{case_name} CPU/GPU vertical movement must remain stable: cpu_delta_y={cpu_delta_y:.2}, gpu_delta_y={gpu_delta_y:.2}"
    );

    let before = sweep.compare_region(
        format!("{case_name}-before-cpu-vs-gpu"),
        RgbaFrame::new("cpu-before", &cpu.before, WIDTH, HEIGHT),
        RgbaFrame::new("gpu-before", &gpu.before, WIDTH, HEIGHT),
        region,
    );
    assert!(
        before.rmse <= 0.14,
        "{case_name} CPU/GPU initial-frame RMSE too high: {:.5}",
        before.rmse
    );
    assert!(
        before.channel_delta.mean_channel_delta <= 22.0,
        "{case_name} CPU/GPU initial-frame mean channel delta too high: {:.5}",
        before.channel_delta.mean_channel_delta
    );

    let after = sweep.compare_region(
        format!("{case_name}-after-cpu-vs-gpu"),
        RgbaFrame::new("cpu-after", &cpu.after, WIDTH, HEIGHT),
        RgbaFrame::new("gpu-after", &gpu.after, WIDTH, HEIGHT),
        region,
    );
    assert!(
        after.rmse <= 0.14,
        "{case_name} CPU/GPU moved-frame RMSE too high: {:.5}",
        after.rmse
    );
    assert!(
        after.channel_delta.mean_channel_delta <= 22.0,
        "{case_name} CPU/GPU moved-frame mean channel delta too high: {:.5}",
        after.channel_delta.mean_channel_delta
    );

    movements.push(MovementRecord {
        case_name,
        cpu_delta_x,
        gpu_delta_x,
        cpu_delta_y,
        gpu_delta_y,
    });
}

fn render_authored_set_transform(
    backend: RenderBackend,
    artifacts: &Path,
    case_name: &'static str,
) -> MotionCapture {
    render_motion(backend, artifacts, case_name, |scene, assets| {
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.34, 0.34, 0.34));
        let material =
            assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 190, 240)));
        let moving = scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::new(-0.45, 0.0, 0.0)))
            .add()
            .expect("authored moving mesh inserts");
        Box::new(move |scene| {
            scene
                .set_transform(moving, Transform::at(Vec3::new(0.45, 0.0, 0.0)))
                .expect("authored moving mesh transform updates");
        })
    })
}

fn render_authored_animation_seek(
    backend: RenderBackend,
    artifacts: &Path,
    case_name: &'static str,
) -> MotionCapture {
    render_motion(backend, artifacts, case_name, |scene, assets| {
        let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.34, 0.34, 0.34));
        let material =
            assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 170, 70)));
        let moving = scene
            .mesh(geometry, material)
            .add()
            .expect("authored animated mesh inserts");
        let clip = AnimationClip::authored(
            Some("MoveX".to_owned()),
            vec![AnimationChannel::new(
                moving,
                AnimationTarget::Translation,
                vec![0.0, 1.0],
                AnimationOutput::Vec3(vec![Vec3::new(-0.45, 0.0, 0.0), Vec3::new(0.45, 0.0, 0.0)]),
                AnimationInterpolation::Linear,
            )],
            1.0,
        )
        .expect("authored translation clip is valid");
        let mixer = scene
            .create_authored_animation_mixer(clip)
            .expect("authored mixer creates");
        scene
            .seek_animation(mixer, 0.0)
            .expect("initial animation pose applies");
        Box::new(move |scene| {
            scene
                .seek_animation(mixer, 1.0)
                .expect("moved animation pose applies");
        })
    })
}

fn render_imported_set_transform(
    backend: RenderBackend,
    artifacts: &Path,
    case_name: &'static str,
) -> MotionCapture {
    render_motion(backend, artifacts, case_name, |scene, assets| {
        let scene_asset = pollster::block_on(assets.load_scene(AssetPath::from(
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        )))
        .expect("movement glTF fixture loads");
        let import = scene
            .instantiate(&scene_asset)
            .expect("movement glTF fixture instantiates");
        let moving = import.roots()[0];
        scene
            .set_transform(moving, Transform::at(Vec3::new(-0.45, 0.0, 0.0)))
            .expect("import root starts left");
        Box::new(move |scene| {
            scene
                .set_transform(moving, Transform::at(Vec3::new(0.45, 0.0, 0.0)))
                .expect("import root moves right");
        })
    })
}

fn render_motion(
    backend: RenderBackend,
    artifacts: &Path,
    case_name: &'static str,
    build: impl FnOnce(&mut Scene, &Assets) -> Box<dyn FnOnce(&mut Scene)>,
) -> MotionCapture {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let mutate = build(&mut scene, &assets);
    let mut renderer = renderer_for_backend(backend, WIDTH, HEIGHT, AntiAliasing::None);

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial render succeeds");
    let before = renderer.frame_rgba8().to_vec();
    write_png(
        &artifacts.join(format!("{case_name}-{}-before.png", backend.name())),
        &before,
    );
    let before_centroid =
        visible_centroid(&before).expect("initial render must contain visible object pixels");

    mutate(&mut scene);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("moved prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("moved render succeeds");
    let after = renderer.frame_rgba8().to_vec();
    write_png(
        &artifacts.join(format!("{case_name}-{}-after.png", backend.name())),
        &after,
    );
    let after_centroid =
        visible_centroid(&after).expect("moved render must contain visible object pixels");

    MotionCapture {
        before,
        after,
        before_centroid,
        after_centroid,
    }
}

fn visible_centroid(frame: &[u8]) -> Option<(f32, f32)> {
    let mut weighted_x = 0.0_f32;
    let mut weighted_y = 0.0_f32;
    let mut count = 0_u32;
    for (index, pixel) in frame.chunks_exact(4).enumerate() {
        if pixel[3] > 0 && pixel[..3].iter().any(|channel| *channel > 24) {
            weighted_x += (index as u32 % WIDTH) as f32;
            weighted_y += (index as u32 / WIDTH) as f32;
            count = count.saturating_add(1);
        }
    }
    (count > 0).then_some((weighted_x / count as f32, weighted_y / count as f32))
}

fn artifact_dir() -> PathBuf {
    let dir = PathBuf::from("target/gate-artifacts/dynamic-transform-parity");
    fs::create_dir_all(&dir).expect("artifact dir creates");
    dir
}

fn write_png(path: &Path, rgba8: &[u8]) {
    let file = File::create(path).expect("create artifact PNG");
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, WIDTH, HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header writes");
    writer.write_image_data(rgba8).expect("PNG payload writes");
}

fn movement_records_json(records: &[MovementRecord]) -> String {
    let mut json = String::from("[");
    for (index, record) in records.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&format!(
            "{{ \"case\": \"{}\", \"cpu_delta_x\": {:.3}, \"gpu_delta_x\": {:.3}, \"cpu_delta_y\": {:.3}, \"gpu_delta_y\": {:.3} }}",
            record.case_name,
            record.cpu_delta_x,
            record.gpu_delta_x,
            record.cpu_delta_y,
            record.gpu_delta_y
        ));
    }
    json.push(']');
    json
}
