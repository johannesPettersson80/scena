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

    // A sculpture, not a test chart: interlocking rings with real silhouette,
    // a glass core, and a machined base. Still only stock primitives.
    let big_ring = assets.create_geometry(GeometryDesc::torus(1.62, 0.115, 288, 72));
    let mid_ring = assets.create_geometry(GeometryDesc::torus(1.18, 0.085, 256, 64));
    let core = assets.create_geometry(GeometryDesc::sphere(0.72, 224, 144));
    let base = assets.create_geometry(GeometryDesc::cylinder_with_bevel(1.05, 0.16, 160, 0.035));
    let pin = assets.create_geometry(GeometryDesc::cylinder_with_bevel(0.055, 3.1, 96, 0.02));

    let chrome = assets.create_material(MaterialDesc::chrome().with_roughness_factor(0.015));
    let gold = assets.create_material(MaterialDesc::metal(Color::from_srgb_u8(214, 168, 88)));
    let glass = assets.create_material(MaterialDesc::clear_glass(Color::from_srgb_u8(
        206, 236, 250,
    )));
    let steel = assets.create_material(MaterialDesc::brushed_steel());
    let accent = assets.create_material(MaterialDesc::clearcoat_plastic(Color::from_srgb_u8(
        186, 32, 36,
    )));

    // Outer ring: near-vertical, tipped toward camera.
    scene
        .mesh(big_ring, chrome)
        .transform(
            Transform::at(Vec3::new(0.0, 0.62, 0.0))
                .rotate_x_deg(74.0)
                .rotate_z_deg(-14.0),
        )
        .add()?;

    // Inner ring: crossing the outer one at roughly a right angle.
    scene
        .mesh(mid_ring, gold)
        .transform(
            Transform::at(Vec3::new(0.0, 0.62, 0.0))
                .rotate_x_deg(74.0)
                .rotate_y_deg(88.0)
                .rotate_z_deg(10.0),
        )
        .add()?;

    // Glass core suspended at the crossing point.
    scene
        .mesh(core, glass)
        .transform(Transform::at(Vec3::new(0.0, 0.62, 0.0)))
        .add()?;

    // Axle through the whole assembly.
    scene
        .mesh(pin, steel)
        .transform(Transform::at(Vec3::new(0.0, 0.62, 0.0)).rotate_z_deg(90.0))
        .add()?;

    // Machined base plate.
    scene
        .mesh(base, steel)
        .transform(Transform::at(Vec3::new(0.0, -0.92, 0.0)))
        .add()?;

    // Small red accent, off to the side, catching the key light.
    let bead = assets.create_geometry(GeometryDesc::sphere(0.2, 128, 96));
    scene
        .mesh(bead, accent)
        .transform(Transform::at(Vec3::new(1.95, -0.80, 1.1)))
        .add()?;

    scene.add_studio_lighting()?;

    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard().with_fov_degrees(31.0),
        Transform::at(Vec3::new(3.05, 0.95, 6.35)),
    )?;
    scene.set_active_camera(camera)?;
    scene.look_at_point(camera, Vec3::new(0.0, 0.34, 0.0))?;

    let mut renderer = Renderer::headless(WIDTH, HEIGHT)?;
    renderer.set_supersample_factor(SUPERSAMPLE)?;
    renderer.set_reconstruction_filter(ReconstructionFilter::Tent);
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_tonemapper(Tonemapper::PbrNeutral);
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
