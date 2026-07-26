//! Hero scene for the public demo page.
//!
//! Everything here is built from scena's own primitives and named presets —
//! no imported art. The point the demo makes is that a scene worth looking at
//! can be *composed*, not sourced.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use scena::{
    Assets, Background, Color, EnvironmentPreset, GeometryDesc, MaterialDesc, PerspectiveCamera,
    ReconstructionFilter, Renderer, Scene, ScreenSpaceReflectionConfig, Tonemapper, Transform,
    Vec3,
};

const WIDTH: u32 = 1600;
const HEIGHT: u32 = 1000;
const SUPERSAMPLE: u32 = 2;

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/hero".to_string()),
    );
    std::fs::create_dir_all(&out)?;
    render_hero(&out.join("hero.png"))?;
    eprintln!("wrote {}", out.join("hero.png").display());
    render_swatches(&out)?;
    Ok(())
}

fn render_hero(path: &Path) -> Result<(), Box<dyn Error>> {
    let assets = Assets::new();

    let floor = assets.create_geometry(GeometryDesc::plane(200.0, 200.0));
    let backdrop = assets.create_geometry(GeometryDesc::plane(400.0, 160.0));

    // Glossy dielectric floor: dark, but smooth enough that every sphere
    // above it gets a reflection and a contact point.
    let floor_mat = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(26, 27, 32),
        0.0,
        0.10,
    ));
    let backdrop_mat = assets.create_material(MaterialDesc::matte(Color::from_srgb_u8(30, 32, 40)));

    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    scene
        .mesh(floor, floor_mat)
        .transform(Transform::at(Vec3::new(0.0, -1.0, 0.0)))
        .add()?;
    scene
        .mesh(backdrop, backdrop_mat)
        .transform(Transform::at(Vec3::new(0.0, 30.0, -97.0)).rotate_x_deg(90.0))
        .add()?;

    // Every sphere is a named preset. The row *is* the pitch: no numbers.
    let lineup: [(MaterialDesc, f32, f32); 6] = [
        (
            MaterialDesc::chrome().with_roughness_factor(0.02),
            -3.30,
            1.00,
        ),
        (
            MaterialDesc::clear_glass(Color::from_srgb_u8(214, 238, 248)),
            -1.35,
            0.86,
        ),
        (
            MaterialDesc::clearcoat_plastic(Color::from_srgb_u8(178, 30, 34)),
            0.52,
            0.94,
        ),
        (MaterialDesc::brushed_steel(), 2.28, 0.80),
        (
            MaterialDesc::metal(Color::from_srgb_u8(212, 168, 92)),
            3.86,
            0.72,
        ),
        (
            MaterialDesc::matte(Color::from_srgb_u8(226, 226, 230)),
            5.22,
            0.62,
        ),
    ];

    for (index, (material, x, radius)) in lineup.into_iter().enumerate() {
        let geometry = assets.create_geometry(GeometryDesc::sphere(radius, 192, 128));
        let handle = assets.create_material(material);
        // Gentle arc: the row curves away so it reads as depth, not a chart.
        // Smooth arc: centre spheres forward, ends receding.
        let t = (index as f32 - 2.5) / 2.5;
        let z = -1.35 * t * t;
        scene
            .mesh(geometry, handle)
            .transform(Transform::at(Vec3::new(x, radius - 1.0, z)))
            .add()?;
    }

    scene.add_studio_lighting()?;

    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard().with_fov_degrees(32.0),
        Transform::at(Vec3::new(0.95, 1.15, 12.6)),
    )?;
    scene.set_active_camera(camera)?;
    scene.look_at_point(camera, Vec3::new(0.95, -0.16, -0.4))?;

    let mut renderer = Renderer::headless(WIDTH, HEIGHT)?;
    renderer.set_supersample_factor(SUPERSAMPLE)?;
    renderer.set_reconstruction_filter(ReconstructionFilter::Tent);
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_tonemapper(Tonemapper::PbrNeutral);
    // The floor is the second subject: let the lineup reflect into it.
    renderer.set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::studio_floor()));
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render(&scene, camera)?;

    write_png(renderer.frame_rgba8(), WIDTH, HEIGHT, path)?;
    Ok(())
}

/// One 512x512 close-up per named material, for the preset grid on the page.
fn render_swatches(out: &Path) -> Result<(), Box<dyn Error>> {
    const SIZE: u32 = 512;
    let swatches: [(&str, MaterialDesc); 6] = [
        ("chrome", MaterialDesc::chrome().with_roughness_factor(0.02)),
        (
            "clear-glass",
            MaterialDesc::clear_glass(Color::from_srgb_u8(214, 238, 248)),
        ),
        (
            "clearcoat",
            MaterialDesc::clearcoat_plastic(Color::from_srgb_u8(178, 30, 34)),
        ),
        ("brushed-steel", MaterialDesc::brushed_steel()),
        (
            "metal",
            MaterialDesc::metal(Color::from_srgb_u8(212, 168, 92)),
        ),
        (
            "matte",
            MaterialDesc::matte(Color::from_srgb_u8(226, 226, 230)),
        ),
    ];

    for (name, material) in swatches {
        let assets = Assets::new();
        let floor = assets.create_geometry(GeometryDesc::plane(200.0, 200.0));
        let backdrop = assets.create_geometry(GeometryDesc::plane(400.0, 160.0));
        let ball = assets.create_geometry(GeometryDesc::sphere(1.0, 192, 128));
        let floor_mat = assets.create_material(MaterialDesc::pbr_metallic_roughness(
            Color::from_srgb_u8(26, 27, 32),
            0.0,
            0.10,
        ));
        let backdrop_mat =
            assets.create_material(MaterialDesc::matte(Color::from_srgb_u8(30, 32, 40)));
        let handle = assets.create_material(material);
        let environment =
            pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

        let mut scene = Scene::new();
        scene
            .mesh(floor, floor_mat)
            .transform(Transform::at(Vec3::new(0.0, -1.0, 0.0)))
            .add()?;
        scene
            .mesh(backdrop, backdrop_mat)
            .transform(Transform::at(Vec3::new(0.0, 30.0, -97.0)).rotate_x_deg(90.0))
            .add()?;
        scene.mesh(ball, handle).add()?;
        scene.add_studio_lighting()?;

        let camera = scene.add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard().with_fov_degrees(30.0),
            Transform::at(Vec3::new(0.0, 0.55, 6.2)),
        )?;
        scene.set_active_camera(camera)?;
        scene.look_at_point(camera, Vec3::new(0.0, -0.16, 0.0))?;

        let mut renderer = Renderer::headless(SIZE, SIZE)?;
        renderer.set_supersample_factor(SUPERSAMPLE)?;
        renderer.set_reconstruction_filter(ReconstructionFilter::Tent);
        renderer.set_environment(environment);
        renderer.set_background(Background::DarkStudio);
        renderer.set_tonemapper(Tonemapper::PbrNeutral);
        renderer.set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::studio_floor()));
        renderer.prepare_with_assets(&mut scene, &assets)?;
        renderer.render(&scene, camera)?;

        let path = out.join(format!("mat-{name}.png"));
        write_png(renderer.frame_rgba8(), SIZE, SIZE, &path)?;
        eprintln!("wrote {}", path.display());
    }
    Ok(())
}

fn write_png(rgba: &[u8], width: u32, height: u32, path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}
