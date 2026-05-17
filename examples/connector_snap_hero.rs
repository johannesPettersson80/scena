use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer, Scene, Transform, Vec3,
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;
const FRAMES: u32 = 80;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("target/gate-artifacts/connector-snap-hero/frames");
    fs::create_dir_all(out_dir)?;

    let assets = Assets::new();
    let drive_part = pollster::block_on(assets.load_scene("tests/assets/gltf/drive_unit.glb"))?;
    let load_part = pollster::block_on(assets.load_scene("tests/assets/gltf/load_unit.glb"))?;
    let start = Transform::at(Vec3::new(-1.48, 0.11, 0.0));
    let final_transform = solved_mate_transform(&assets, &drive_part, &load_part, start)?;

    for frame in 0..FRAMES {
        let mut scene = Scene::new();
        let load = scene.instantiate(&load_part)?;
        let drive = scene.instantiate(&drive_part)?;
        let snap_t = snap_progress(frame);

        if snap_t >= 1.0 {
            scene.set_transform(drive.roots()[0], start)?;
            scene.mate(&drive, "shaft", &load, "hub")?;
        } else {
            scene.set_transform(
                drive.roots()[0],
                lerp_transform(start, final_transform, snap_t),
            )?;
        }

        if contact_strobe(frame) {
            add_contact_strobe(&assets, &mut scene)?;
        }

        let camera = add_camera(&mut scene, camera_plan(frame))?;
        let mut renderer = Renderer::headless(WIDTH, HEIGHT)?;
        renderer.set_environment(assets.default_environment());
        renderer.set_background_color(Color::from_linear_rgb(0.014, 0.017, 0.024));
        renderer.set_exposure_ev(0.5);
        renderer.prepare_with_assets(&mut scene, &assets)?;
        renderer.render(&scene, camera)?;

        write_png(
            renderer.frame_rgba8(),
            WIDTH,
            HEIGHT,
            &out_dir.join(format!("frame_{frame:03}.png")),
        )?;
    }

    println!(
        "wrote {FRAMES} connector-snap hero frames under {}",
        out_dir.display()
    );
    Ok(())
}

fn solved_mate_transform(
    _assets: &Assets,
    drive_part: &scena::SceneAsset,
    load_part: &scena::SceneAsset,
    start: Transform,
) -> Result<Transform, Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    let load = scene.instantiate(load_part)?;
    let drive = scene.instantiate(drive_part)?;
    scene.set_transform(drive.roots()[0], start)?;
    scene.mate(&drive, "shaft", &load, "hub")?;
    Ok(scene
        .world_transform(drive.roots()[0])
        .ok_or("missing drive root")?)
}

fn snap_progress(frame: u32) -> f32 {
    if frame < 40 {
        0.0
    } else if frame >= 65 {
        1.0
    } else {
        smoothstep((frame - 40) as f32 / 25.0)
    }
}

fn contact_strobe(frame: u32) -> bool {
    (63..=66).contains(&frame)
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_transform(start: Transform, end: Transform, amount: f32) -> Transform {
    Transform {
        translation: start.translation.lerp(end.translation, amount),
        rotation: start.rotation.slerp(end.rotation, amount),
        scale: start.scale.lerp(end.scale, amount),
    }
}

fn add_contact_strobe(
    assets: &Assets,
    scene: &mut Scene,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.018, 0.34, 0.34));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 184, 90)));
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::new(0.045, 0.18, 0.0)))
        .add()?;
    Ok(())
}

fn camera_plan(frame: u32) -> (Vec3, Vec3) {
    match frame {
        // 0.0-1.0s: wide establishing — both parts on neutral ground
        0..=9 => (Vec3::new(-0.20, 0.78, 2.05), Vec3::new(-0.65, 0.04, 0.0)),
        // 1.0-2.5s: dolly in to the drive shaft (at start position, world x ≈ -0.92)
        10..=24 => (Vec3::new(-0.65, 0.22, 0.50), Vec3::new(-0.92, 0.07, 0.0)),
        // 2.5-4.0s: cut to the flywheel hub (load hub bearing flange at world x ≈ 0.05)
        25..=39 => (Vec3::new(0.30, 0.22, 0.45), Vec3::new(0.06, 0.07, 0.0)),
        // 4.0-6.5s: cut wide — drive translates along the inferred mate axis
        40..=64 => (Vec3::new(-0.48, 0.56, 1.35), Vec3::new(-0.12, 0.05, 0.0)),
        // 6.5-8.0s: final hold for snippet overlay
        _ => (Vec3::new(-0.40, 0.50, 1.22), Vec3::new(-0.12, 0.04, 0.0)),
    }
}

fn add_camera(
    scene: &mut Scene,
    (position, target): (Vec3, Vec3),
) -> Result<scena::CameraKey, Box<dyn std::error::Error>> {
    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::default().with_aspect(WIDTH as f32 / HEIGHT as f32),
        Transform::at(position),
    )?;
    scene.look_at_point(camera, target)?;
    Ok(camera)
}

fn write_png(
    rgba8: &[u8],
    width: u32,
    height: u32,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba8)?;
    Ok(())
}
