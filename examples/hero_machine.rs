//! Hero: the connector-snap assembly as product photography.
//!
//! Same model and the same documented helpers as
//! `docs/guides/easy-scene-setup.md` — `bounds_world`, `add_studio_lighting`,
//! `add_grid_floor`, `frame_bounds`, `AutoExposureConfig::product_studio`.
//!
//! What changes is presentation, not plumbing:
//!   * the subject fills the frame instead of floating in it,
//!   * the camera drops to a heroic elevation rather than looking down,
//!   * the floor is matte, because `ScreenSpaceReflectionConfig` mirrors the
//!     frame about a fixed image-height fraction rather than reflecting the
//!     floor, and a glossy floor blows out into a grazing-angle highlight,
//!   * and ambient occlusion, bloom and depth of field are switched on.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use scena::{
    Assets, Background, Camera, Color, DepthOfFieldConfig, EnvironmentPreset, FramingOptions,
    GridFloorOptions, PerspectiveCamera, PostBloomConfig, ReconstructionFilter, Renderer, Scene,
    ScreenSpaceAmbientOcclusionConfig,
};

const WIDTH: u32 = 1800;
const HEIGHT: u32 = 1150;
const SUPERSAMPLE: u32 = 2;
const ASSEMBLY: &str = "demo/samples/connector-snap/connector_snap_assembly.glb";

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/hero".to_string()),
    );
    std::fs::create_dir_all(&out)?;
    render(&out.join("machine-hero.png"))?;
    eprintln!("wrote {}", out.join("machine-hero.png").display());
    Ok(())
}

fn render(path: &Path) -> Result<(), Box<dyn Error>> {
    let assets = Assets::new();
    let model = pollster::block_on(assets.load_scene(ASSEMBLY))?;
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&model)?;
    let bounds = import
        .bounds_world(&scene)
        .ok_or("connector assembly has no bounds")?;

    scene.add_studio_lighting()?;

    // The documented floor helper. Matte on purpose: at this shallow camera
    // angle a glossy floor throws a displaced screen-space highlight that reads
    // as a second, offset copy of the machine.
    scene.add_grid_floor(
        &assets,
        GridFloorOptions::new()
            .under_bounds(bounds)
            .color(Color::from_srgb_u8(48, 51, 59))
            .line_color(Color::from_srgb_u8(46, 49, 57))
            .roughness(0.95)
            .padding(0.55),
    )?;

    let camera = scene.add_perspective_camera_default_for(bounds, (WIDTH, HEIGHT))?;
    scene.set_camera(camera, Camera::Perspective(PerspectiveCamera::portrait()))?;
    let framing = scene.frame_bounds(
        camera,
        bounds,
        FramingOptions::new()
            // Low and to the side: the angle a product photographer picks so
            // the form reads long, instead of small seen from above.
            .azimuth_elevation(-34.0, 12.0)
            .fill(0.88)
            .margin_px(36.0)
            .viewport(WIDTH, HEIGHT),
    )?;
    scene.set_active_camera(camera)?;

    let mut renderer = Renderer::headless_gpu(WIDTH, HEIGHT)?;
    renderer.set_supersample_factor(SUPERSAMPLE)?;
    renderer.set_reconstruction_filter(ReconstructionFilter::Tent);
    renderer.set_environment(environment);
    renderer.set_background(Background::DarkStudio);
    // Fixed exposure, deliberately. `AutoExposureConfig::product_studio()`
    // meters this frame roughly 4 EV under: the subject is a small bright
    // object in a large dark studio, which is exactly the case an average-based
    // meter gets wrong. A hero image should not depend on that.
    renderer.set_exposure_ev(4.0);

    // The post chain the previous hero never turned on.
    // No screen-space reflections. `ScreenSpaceReflectionConfig` mirrors the
    // frame about a horizontal line at `horizon_fraction` of image height,
    // across the full frame width -- it never consults the floor, so it cannot
    // place a reflection on it. At supersample 2 it resolves into a hard,
    // obviously displaced second copy of the subject.
    renderer.set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::subtle()));
    renderer.set_bloom(Some(PostBloomConfig::subtle()));
    renderer.set_depth_of_field(Some(DepthOfFieldConfig::new(framing.distance, 3.6, 5)));
    renderer.prepare_with_assets(&mut scene, &assets)?;
    renderer.render_active(&scene)?;

    let stats = renderer.stats();
    eprintln!(
        "shadow_maps={} ao_passes={} bloom_passes={} dof_passes={} draw_calls={}",
        stats.shadow_maps,
        stats.ambient_occlusion_passes,
        stats.bloom_passes,
        stats.depth_of_field_passes,
        stats.draw_calls
    );

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
