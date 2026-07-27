//! Hero: a camera lens, composed entirely from scena primitives.
//!
//! Nothing here is imported art. Barrel sections are bevelled cylinders, rings
//! are tori, and every optical element is a sphere squashed along the optical
//! axis — which is what a convex element actually is.
//!
//! Three states share one model: assembled, exploded along the axis, and
//! sectioned.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use scena::{
    Assets, Background, Color, DepthOfFieldConfig, EnvironmentPreset, GeometryDesc, MaterialDesc,
    PerspectiveCamera, PostBloomConfig, ReconstructionFilter, Renderer, Scene,
    ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig, Tonemapper, Transform, Vec3,
};

const WIDTH: u32 = 1800;
const HEIGHT: u32 = 1150;
const SUPERSAMPLE: u32 = 2;

/// One part of the lens. `z` is its resting position along the optical axis;
/// `burst` is how far it travels when the assembly is exploded.
struct Part {
    geometry: GeometryDesc,
    material: MaterialDesc,
    z: f32,
    burst: f32,
    scale: Vec3,
}

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/hero".to_string()),
    );
    std::fs::create_dir_all(&out)?;
    render(&out.join("lens-hero.png"), 0.0)?;
    eprintln!("wrote {}", out.join("lens-hero.png").display());
    Ok(())
}

fn lens_parts() -> Vec<Part> {
    let glass = MaterialDesc::clear_glass(Color::from_srgb_u8(206, 238, 250));
    let barrel = MaterialDesc::metal(Color::from_srgb_u8(96, 100, 110));
    let bright = MaterialDesc::brushed_steel();
    let grip = MaterialDesc::rubber();
    let gold = MaterialDesc::metal(Color::from_srgb_u8(216, 172, 92));
    let baffle = MaterialDesc::matte(Color::from_srgb_u8(14, 14, 17));

    // Front of the lens is +z, mount is -z.
    vec![
        // --- mount end -------------------------------------------------
        Part {
            geometry: GeometryDesc::cylinder_with_bevel(0.86, 0.09, 192, 0.02),
            material: gold.clone(),
            z: -1.62,
            burst: -2.30,
            scale: Vec3::ONE,
        },
        Part {
            geometry: GeometryDesc::cylinder_with_bevel(0.92, 0.13, 192, 0.03),
            material: bright.clone(),
            z: -1.48,
            burst: -1.70,
            scale: Vec3::ONE,
        },
        // rear optical group
        Part {
            geometry: GeometryDesc::sphere(0.72, 224, 144),
            material: glass.clone(),
            z: -1.16,
            burst: -1.15,
            scale: Vec3::new(1.0, 1.0, 0.22),
        },
        Part {
            geometry: GeometryDesc::torus(0.90, 0.045, 224, 48),
            material: baffle.clone(),
            z: -1.00,
            burst: -0.80,
            scale: Vec3::ONE,
        },
        // --- rear barrel -----------------------------------------------
        Part {
            geometry: GeometryDesc::cylinder_with_bevel(0.95, 0.52, 192, 0.03),
            material: barrel.clone(),
            z: -0.66,
            burst: -0.45,
            scale: Vec3::ONE,
        },
        // --- aperture ring ---------------------------------------------
        Part {
            geometry: GeometryDesc::torus(0.99, 0.075, 224, 48),
            material: bright.clone(),
            z: -0.30,
            burst: -0.12,
            scale: Vec3::ONE,
        },
        // --- middle optical group --------------------------------------
        Part {
            geometry: GeometryDesc::sphere(0.80, 224, 144),
            material: glass.clone(),
            z: -0.02,
            burst: 0.22,
            scale: Vec3::new(1.0, 1.0, 0.26),
        },
        Part {
            geometry: GeometryDesc::sphere(0.78, 224, 144),
            material: glass.clone(),
            z: 0.24,
            burst: 0.62,
            scale: Vec3::new(1.0, 1.0, 0.18),
        },
        // --- focus grip -------------------------------------------------
        Part {
            geometry: GeometryDesc::cylinder_with_bevel(1.03, 0.62, 192, 0.02),
            material: grip,
            z: 0.62,
            burst: 1.05,
            scale: Vec3::ONE,
        },
        // --- front barrel + element --------------------------------------
        Part {
            geometry: GeometryDesc::cylinder_with_bevel(1.00, 0.30, 192, 0.03),
            material: barrel,
            z: 1.06,
            burst: 1.62,
            scale: Vec3::ONE,
        },
        Part {
            geometry: GeometryDesc::sphere(0.94, 256, 160),
            material: glass,
            z: 1.30,
            burst: 2.20,
            scale: Vec3::new(1.0, 1.0, 0.30),
        },
        Part {
            geometry: GeometryDesc::torus(1.02, 0.055, 256, 48),
            material: bright,
            z: 1.44,
            burst: 2.60,
            scale: Vec3::ONE,
        },
    ]
}

fn render(path: &Path, explode: f32) -> Result<(), Box<dyn Error>> {
    let assets = Assets::new();

    let floor = assets.create_geometry(GeometryDesc::plane(300.0, 300.0));
    let backdrop = assets.create_geometry(GeometryDesc::plane(500.0, 200.0));
    let floor_mat = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(23, 24, 29),
        0.0,
        0.09,
    ));
    let backdrop_mat = assets.create_material(MaterialDesc::matte(Color::from_srgb_u8(28, 30, 37)));
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    scene
        .mesh(floor, floor_mat)
        .transform(Transform::at(Vec3::new(0.0, -1.35, 0.0)))
        .add()?;
    scene
        .mesh(backdrop, backdrop_mat)
        .transform(Transform::at(Vec3::new(0.0, 40.0, -130.0)).rotate_x_deg(90.0))
        .add()?;

    // Cylinders and tori stand on Y; the lens lies along Z, so the whole
    // assembly is tipped a quarter turn and then posed for the camera.
    for part in lens_parts() {
        let geometry = assets.create_geometry(part.geometry);
        let material = assets.create_material(part.material);
        let z = part.z + explode * part.burst;
        scene
            .mesh(geometry, material)
            .transform(
                Transform::at(Vec3::new(0.0, 0.0, z))
                    .rotate_x_deg(90.0)
                    .with_scale(part.scale),
            )
            .add()?;
    }

    scene.add_studio_lighting()?;

    let camera = scene.add_perspective_camera(
        scene.root(),
        PerspectiveCamera::standard().with_fov_degrees(30.0),
        Transform::at(Vec3::new(4.15, 1.42, 5.05)),
    )?;
    scene.set_active_camera(camera)?;
    scene.look_at_point(camera, Vec3::new(0.0, -0.05, 0.05))?;

    let mut renderer = Renderer::headless(WIDTH, HEIGHT)?;
    renderer.set_supersample_factor(SUPERSAMPLE)?;
    renderer.set_reconstruction_filter(ReconstructionFilter::Tent);
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    renderer.set_tonemapper(Tonemapper::PbrNeutral);
    // The post chain is what separates a render from a photograph.
    renderer.set_screen_space_reflections(Some(ScreenSpaceReflectionConfig::studio_floor()));
    renderer.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::subtle()));
    renderer.set_bloom(Some(PostBloomConfig::subtle()));
    renderer.set_depth_of_field(Some(DepthOfFieldConfig::new(6.6, 3.2, 6)));
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render(&scene, camera)?;

    write_png(renderer.frame_rgba8(), WIDTH, HEIGHT, path)
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
