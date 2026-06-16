//! Phase 5A — visual proof artifacts for the public examples that aren't already
//! covered by the M7 / M8 / M3a / M3b visual suites. The contract is
//! "every public example whose output is meant to be visible on screen has a
//! 256x256 headless PPM proof under `target/gate-artifacts/examples-visual/`",
//! per state-of-art-threejs-replacement-plan.md line 1396.
//!
//! Native-window, browser-canvas, and inspection-only examples are deliberately
//! skipped: they describe runtime patterns (event loop, attached canvas,
//! diagnostic introspection) that don't have a single deterministic frame to
//! capture.
//!
//! Gated to non-wasm32 targets: every test in this file renders through the
//! CPU headless renderer and writes PPM/PNG artifacts via `std::fs::write`,
//! which is not meaningful in the browser test runner. The wasm32 release
//! lane still compiles the rest of the crate against `m6_browser_renderer_parity`.
#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::PathBuf;
use std::{cell::RefCell, rc::Rc};

use scena::material_showcase::{
    MaterialShowcasePreset, glass_background_target_bars, material_preset_showcase,
};
use scena::{
    Aabb, AnimationPlaybackState, Assets, AutoExposureConfig, Background, Color, ConnectOptions,
    ConnectionAlignment, ConnectionError, ConnectorFrame, CursorPosition, DirectionalLight,
    EnvironmentPreset, GeometryDesc, InteractiveGltfViewer, LabelDesc, MaterialDesc,
    OrbitControlAction, OrbitControls, PerspectiveCamera, PlatformSurface, PointLight,
    PointerEvent, Profile, Quat, Renderer, RendererOptions, Scene, SourceCoordinateSystem,
    SourceUnits, TouchEvent, Transform, Vec3, Viewport, headless_gltf_viewer,
    interactive_gltf_viewer,
};

const ARTIFACT_WIDTH: u32 = 256;
const ARTIFACT_HEIGHT: u32 = 256;

fn artifact_dir() -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/examples-visual");
    fs::create_dir_all(&dir).expect("examples-visual artifact directory");
    dir
}

/// Minimum unique RGB triplets required for an example-visual harness-smoke
/// artifact to be considered evidence of a real render rather than a flat
/// constant-color block. Two strictly distinct triplets prove the renderer
/// produced more than a single-color fill; production-claim proofs (which
/// these artifacts are NOT — `production_claim = false` in the TOML) carry
/// stricter spatial-variation floors documented in
/// docs/specs/visual-quality-contract.md. Surfaces the
/// visual-quality-validator F3 floor.
const MIN_UNIQUE_PIXELS: usize = 2;

fn count_nonblack_pixels(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn count_unique_rgb_triplets(rgba: &[u8]) -> usize {
    let mut triplets: std::collections::BTreeSet<[u8; 3]> = std::collections::BTreeSet::new();
    for pixel in rgba.chunks_exact(4) {
        triplets.insert([pixel[0], pixel[1], pixel[2]]);
    }
    triplets.len()
}

#[derive(Debug, Clone, Copy)]
struct PixelRect {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl PixelRect {
    fn width(self) -> u32 {
        self.max_x - self.min_x + 1
    }

    fn height(self) -> u32 {
        self.max_y - self.min_y + 1
    }

    fn center_x(self) -> f32 {
        (self.min_x + self.max_x) as f32 * 0.5
    }

    fn center_y(self) -> f32 {
        (self.min_y + self.max_y) as f32 * 0.5
    }
}

fn nonblack_pixel_rect(rgba: &[u8], width: u32, height: u32) -> Option<PixelRect> {
    let mut rect: Option<PixelRect> = None;
    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) as usize) * 4;
            let pixel = &rgba[index..index + 4];
            if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
                continue;
            }
            rect = Some(match rect {
                Some(rect) => PixelRect {
                    min_x: rect.min_x.min(x),
                    min_y: rect.min_y.min(y),
                    max_x: rect.max_x.max(x),
                    max_y: rect.max_y.max(y),
                },
                None => PixelRect {
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                },
            });
        }
    }
    rect
}

fn write_artifact(name: &str, width: u32, height: u32, rgba: &[u8]) {
    let dir = artifact_dir();
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact can be written");
    let nonblack_pixels = count_nonblack_pixels(rgba);
    let unique_pixels = count_unique_rgb_triplets(rgba);
    assert!(
        unique_pixels >= MIN_UNIQUE_PIXELS,
        "example-visual harness-smoke artifact `{name}` has {unique_pixels} unique \
         RGB triplets, below the MIN_UNIQUE_PIXELS={MIN_UNIQUE_PIXELS} floor; the \
         capture is too uniform to count as evidence the renderer produced a real \
         frame (visual-quality-validator F3)",
    );
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\n\
             name = \"{name}\"\n\
             example_source = \"examples/{name}.rs\"\n\
             format = \"ppm\"\n\
             encoding = \"srgb8\"\n\
             width = {width}\n\
             height = {height}\n\
             nonblack_pixels = {nonblack_pixels}\n\
             unique_pixels = {unique_pixels}\n\
             min_unique_pixels = {MIN_UNIQUE_PIXELS}\n\
             tolerance = \"nonblack-smoke\"\n\
             proof_class = \"example-visual-harness-smoke\"\n\
             production_claim = false\n"
        ),
    )
    .expect("artifact metadata can be written");
}

fn write_frame_bounds_artifact(
    name: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
    outcome: scena::FramingOutcome,
    pixel_rect: PixelRect,
) {
    let dir = artifact_dir();
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact can be written");
    fs::write(
        dir.join(format!("{name}.json")),
        serde_json::json!({
            "proof_class": "frame-bounds-rendered-output",
            "rendered_image": format!("{}.ppm", name),
            "viewport": { "width": width, "height": height, "aspect": width as f32 / height as f32 },
            "target_fill": outcome.fill,
            "margin_px": outcome.margin_px,
            "computed_distance": outcome.distance,
            "projected_rect": {
                "min_x": outcome.projected_rect.min_x,
                "min_y": outcome.projected_rect.min_y,
                "max_x": outcome.projected_rect.max_x,
                "max_y": outcome.projected_rect.max_y,
                "fill": outcome.projected_rect.fill_fraction(width, height),
            },
            "nonblack_pixel_rect": {
                "min_x": pixel_rect.min_x,
                "min_y": pixel_rect.min_y,
                "max_x": pixel_rect.max_x,
                "max_y": pixel_rect.max_y,
                "width": pixel_rect.width(),
                "height": pixel_rect.height(),
            },
            "assertions": {
                "not_tiny": true,
                "not_clipped": true,
                "centered": true,
            }
        })
        .to_string(),
    )
    .expect("frame_bounds metadata can be written");
}

fn write_docs_image_artifact(name: &str, source: &str, width: u32, height: u32, rgba: &[u8]) {
    let dir = artifact_dir();
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("docs-image PPM can be written");
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\n\
             name = \"{name}\"\n\
             source = \"{source}\"\n\
             format = \"ppm\"\n\
             encoding = \"srgb8\"\n\
             width = {width}\n\
             height = {height}\n\
             proof_class = \"docs-image\"\n\
             generated = true\n\
             screenshot = false\n"
        ),
    )
    .expect("docs-image metadata can be written");
}

fn write_reference_docs_image_artifact(
    name: &str,
    source: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) {
    let dir = artifact_dir();
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm)
        .expect("reference docs-image PPM can be written");
    let nonblack_pixels = count_nonblack_pixels(rgba);
    let unique_pixels = count_unique_rgb_triplets(rgba);
    assert!(
        nonblack_pixels > 0 && unique_pixels >= MIN_UNIQUE_PIXELS,
        "reference docs-image `{name}` must contain visible rendered pixels"
    );
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\n\
             name = \"{name}\"\n\
             source = \"{source}\"\n\
             format = \"ppm\"\n\
             encoding = \"srgb8\"\n\
             width = {width}\n\
             height = {height}\n\
             nonblack_pixels = {nonblack_pixels}\n\
             unique_pixels = {unique_pixels}\n\
             proof_class = \"reference-image+docs-image\"\n\
             generated = true\n\
             screenshot = false\n"
        ),
    )
    .expect("reference docs-image metadata can be written");
}

fn write_auto_exposure_reference_docs_image_artifact(
    name: &str,
    source: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
    exposure_evs: &[f32],
) {
    let dir = artifact_dir();
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm)
        .expect("auto-exposure reference docs-image PPM can be written");
    let exposure_evs = exposure_evs
        .iter()
        .map(|value| format!("{value:.4}"))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\n\
             name = \"{name}\"\n\
             source = \"{source}\"\n\
             format = \"ppm\"\n\
             encoding = \"srgb8\"\n\
             width = {width}\n\
             height = {height}\n\
             exposure_evs = [{exposure_evs}]\n\
             proof_class = \"reference-image+docs-image\"\n\
             generated = true\n\
             screenshot = false\n"
        ),
    )
    .expect("auto-exposure reference docs-image metadata can be written");
}

fn write_animated_docs_image_artifact(
    name: &str,
    source: &str,
    frame_width: u32,
    frame_height: u32,
    frames: &[Vec<u8>],
) {
    let dir = artifact_dir();
    let contact_width = frame_width * frames.len() as u32;
    let contact_height = frame_height;
    let mut contact = vec![0_u8; (contact_width * contact_height * 4) as usize];
    for (index, frame) in frames.iter().enumerate() {
        let mut ppm = format!("P6\n{frame_width} {frame_height}\n255\n").into_bytes();
        for pixel in frame.chunks_exact(4) {
            ppm.extend_from_slice(&pixel[..3]);
        }
        fs::write(dir.join(format!("{name}-{index:02}.ppm")), ppm)
            .expect("animated-proof frame PPM can be written");
        for y in 0..frame_height {
            let dst_start = ((y * contact_width + index as u32 * frame_width) * 4) as usize;
            let src_start = (y * frame_width * 4) as usize;
            let byte_count = (frame_width * 4) as usize;
            contact[dst_start..dst_start + byte_count]
                .copy_from_slice(&frame[src_start..src_start + byte_count]);
        }
    }
    let mut contact_ppm = format!("P6\n{contact_width} {contact_height}\n255\n").into_bytes();
    for pixel in contact.chunks_exact(4) {
        contact_ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), contact_ppm)
        .expect("animated-proof contact-sheet PPM can be written");
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\n\
             name = \"{name}\"\n\
             source = \"{source}\"\n\
             format = \"ppm-sequence\"\n\
             encoding = \"srgb8\"\n\
             frame_width = {frame_width}\n\
             frame_height = {frame_height}\n\
             contact_sheet = \"{name}.ppm\"\n\
             contact_width = {contact_width}\n\
             contact_height = {contact_height}\n\
             frame_count = {}\n\
             proof_class = \"animated-proof+docs-image\"\n\
             generated = true\n\
             screenshot = false\n",
            frames.len()
        ),
    )
    .expect("animated-proof metadata can be written");
}

fn srgb8_from_linear(color: Color) -> [u8; 4] {
    [
        linear_channel_to_srgb8(color.r),
        linear_channel_to_srgb8(color.g),
        linear_channel_to_srgb8(color.b),
        linear_channel_to_srgb8(color.a),
    ]
}

fn linear_channel_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

#[test]
fn round_a_named_color_swatch_docs_image() {
    let width = 320;
    let height = 96;
    let palette = [
        Color::GRAY,
        Color::BLUE,
        Color::from_hex("#f5f7fb").expect("studio swatch hex parses"),
        Color::from_kelvin(3200.0),
        Color::from_kelvin(6500.0),
    ];
    let mut rgba = vec![255_u8; (width * height * 4) as usize];
    let swatch_width = width / palette.len() as u32;
    for (index, color) in palette.iter().enumerate() {
        let srgb = srgb8_from_linear(*color);
        let min_x = index as u32 * swatch_width;
        let max_x = if index + 1 == palette.len() {
            width
        } else {
            (index as u32 + 1) * swatch_width
        };
        for y in 0..height {
            for x in min_x..max_x {
                let offset = ((y * width + x) * 4) as usize;
                rgba[offset..offset + 4].copy_from_slice(&srgb);
            }
        }
    }
    assert!(
        count_unique_rgb_triplets(&rgba) >= palette.len(),
        "round-a color swatch must show each named/derived color as a distinct docs-image proof"
    );
    write_docs_image_artifact(
        "round-a-named-color-swatch-docs-image",
        "docs/guides/easy-scene-setup.md",
        width,
        height,
        &rgba,
    );
}

#[test]
fn round_a_lens_preset_comparison_docs_image() {
    let tile_width = 128;
    let tile_height = 128;
    let presets = [
        PerspectiveCamera::wide_angle(),
        PerspectiveCamera::standard(),
        PerspectiveCamera::portrait(),
        PerspectiveCamera::telephoto(),
    ];
    let mut rgba = vec![0_u8; (tile_width * tile_height * presets.len() as u32 * 4) as usize];
    for (index, camera) in presets.iter().enumerate() {
        let tile = render_lens_preset_tile(*camera, tile_width, tile_height);
        for y in 0..tile_height {
            let dst_start =
                ((y * tile_width * presets.len() as u32 + index as u32 * tile_width) * 4) as usize;
            let src_start = (y * tile_width * 4) as usize;
            let byte_count = (tile_width * 4) as usize;
            rgba[dst_start..dst_start + byte_count]
                .copy_from_slice(&tile[src_start..src_start + byte_count]);
        }
    }
    assert!(
        count_nonblack_pixels(&rgba) > 0,
        "round-a lens comparison docs-image must render the shared subject"
    );
    write_docs_image_artifact(
        "round-a-lens-preset-comparison-docs-image",
        "docs/guides/easy-scene-setup.md",
        tile_width * presets.len() as u32,
        tile_height,
        &rgba,
    );
}

fn render_lens_preset_tile(camera: PerspectiveCamera, width: u32, height: u32) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::BLUE));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("lens subject inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            camera,
            Transform::at(Vec3::new(0.0, 0.0, 5.0)),
        )
        .expect("lens preset camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("lens docs-image scene prepares");
    renderer
        .render_active(&scene)
        .expect("lens docs-image scene renders");
    renderer.frame_rgba8().to_vec()
}

#[test]
fn round_b_light_preset_reference_docs_image() {
    let tile_width = 112;
    let tile_height = 112;
    let tiles = [
        render_directional_light_preset_tile(DirectionalLight::sun(), tile_width, tile_height),
        render_directional_light_preset_tile(
            DirectionalLight::key_light(),
            tile_width,
            tile_height,
        ),
        render_directional_light_preset_tile(
            DirectionalLight::fill_light(),
            tile_width,
            tile_height,
        ),
        render_directional_light_preset_tile(
            DirectionalLight::rim_light(),
            tile_width,
            tile_height,
        ),
        render_point_light_preset_tile(PointLight::softbox(), tile_width, tile_height),
        render_point_light_preset_tile(PointLight::bulb_warm(), tile_width, tile_height),
        render_point_light_preset_tile(PointLight::bulb_cool(), tile_width, tile_height),
    ];
    let width = tile_width * tiles.len() as u32;
    let height = tile_height;
    let mut rgba = vec![0_u8; (width * height * 4) as usize];
    for (index, tile) in tiles.iter().enumerate() {
        assert!(
            count_nonblack_pixels(tile) > 0,
            "light preset tile {index} must render a visible lit subject"
        );
        for y in 0..tile_height {
            let dst_start = ((y * width + index as u32 * tile_width) * 4) as usize;
            let src_start = (y * tile_width * 4) as usize;
            let byte_count = (tile_width * 4) as usize;
            rgba[dst_start..dst_start + byte_count]
                .copy_from_slice(&tile[src_start..src_start + byte_count]);
        }
    }
    write_reference_docs_image_artifact(
        "round-b-light-preset-reference-docs-image",
        "docs/guides/easy-scene-setup.md",
        width,
        height,
        &rgba,
    );
}

fn render_directional_light_preset_tile(
    light: DirectionalLight,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let assets = Assets::new();
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::NeutralStudio))
            .expect("neutral studio environment loads for light preset docs image");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 0.35));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::LIGHT_GRAY,
        0.0,
        0.7,
    ));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("light preset subject inserts");
    scene
        .directional_light(light)
        .add()
        .expect("directional preset inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("directional light docs-image scene prepares");
    renderer
        .render_active(&scene)
        .expect("directional light docs-image scene renders");
    renderer.frame_rgba8().to_vec()
}

fn render_point_light_preset_tile(light: PointLight, width: u32, height: u32) -> Vec<u8> {
    let assets = Assets::new();
    let environment =
        pollster::block_on(assets.load_environment_preset(EnvironmentPreset::NeutralStudio))
            .expect("neutral studio environment loads for light preset docs image");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 0.35));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::LIGHT_GRAY,
        0.0,
        0.7,
    ));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("light preset subject inserts");
    scene
        .point_light(light)
        .transform(Transform::at(Vec3::new(0.0, 0.0, 2.0)))
        .add()
        .expect("point preset inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("point light docs-image scene prepares");
    renderer
        .render_active(&scene)
        .expect("point light docs-image scene renders");
    renderer.frame_rgba8().to_vec()
}

#[test]
fn round_b_material_preset_reference_docs_image() {
    let tile_width = 128;
    let tile_height = 128;
    let tiles = material_preset_showcase()
        .iter()
        .copied()
        .map(|preset| render_material_preset_tile(preset, tile_width, tile_height))
        .collect::<Vec<_>>();
    let width = tile_width * tiles.len() as u32;
    let height = tile_height;
    let mut rgba = vec![0_u8; (width * height * 4) as usize];
    for (index, tile) in tiles.iter().enumerate() {
        assert!(
            count_nonblack_pixels(tile) > 0,
            "material preset tile {index} must render a visible subject"
        );
        for y in 0..tile_height {
            let dst_start = ((y * width + index as u32 * tile_width) * 4) as usize;
            let src_start = (y * tile_width * 4) as usize;
            let byte_count = (tile_width * 4) as usize;
            rgba[dst_start..dst_start + byte_count]
                .copy_from_slice(&tile[src_start..src_start + byte_count]);
        }
    }
    write_reference_docs_image_artifact(
        "round-b-material-preset-reference-docs-image",
        "docs/guides/easy-scene-setup.md",
        width,
        height,
        &rgba,
    );
}

#[test]
fn round_e_material_reference_docs_image_metrics() {
    assert_round_e_reference_docs_image_metrics();
}

fn assert_round_e_reference_docs_image_metrics() {
    let thresholds = material_identity_thresholds();
    let chrome = load_round_e_reference_image("chrome");
    let chrome_luminance = luminance_percentiles(&foreground_luminance_values(&chrome));
    assert!(
        chrome_luminance.p99 >= thresholds.chrome_bright_reflection_luminance_p99_min,
        "Round E chrome reference image p99 luminance {:.3} is below the material-identity floor {:.3}",
        chrome_luminance.p99,
        thresholds.chrome_bright_reflection_luminance_p99_min
    );
    assert!(
        chrome_luminance.p05 <= thresholds.chrome_dark_reflection_luminance_p05_max,
        "Round E chrome reference image p05 luminance {:.3} is above the dark-reflection ceiling {:.3}",
        chrome_luminance.p05,
        thresholds.chrome_dark_reflection_luminance_p05_max
    );
    assert!(
        chrome_luminance.p99 / chrome_luminance.p10.max(1.0)
            >= thresholds.chrome_specular_dynamic_range_min,
        "Round E chrome reference image has insufficient dark/bright reflection dynamic range"
    );

    let brushed = load_round_e_reference_image("brushed_steel");
    let brushed_aspect = highlight_aspect_ratio(&brushed);
    assert!(
        brushed_aspect >= thresholds.brushed_steel_reference_aspect_ratio_min,
        "Round E brushed-steel reference image aspect ratio {brushed_aspect:.3} is below the reference-image floor {:.3}",
        thresholds.brushed_steel_reference_aspect_ratio_min
    );

    let plastic = load_round_e_reference_image("plastic");
    let clearcoat = load_round_e_reference_image("clearcoat_plastic");
    let plastic_p99 = luminance_percentiles(&foreground_luminance_values(&plastic)).p99;
    let clearcoat_p99 = luminance_percentiles(&foreground_luminance_values(&clearcoat)).p99;
    let clearcoat_lobe_delta = ((clearcoat_p99 - plastic_p99) / 255.0).max(0.0);
    assert!(
        clearcoat_lobe_delta >= thresholds.clearcoat_lobe_delta_min,
        "Round E clearcoat reference image lobe delta {clearcoat_lobe_delta:.3} is below {:.3}",
        thresholds.clearcoat_lobe_delta_min
    );

    for (preset, minimum) in [
        ("satin", thresholds.source_backed_variance_min),
        ("leather", thresholds.source_backed_variance_min),
        ("rubber", thresholds.source_backed_variance_min),
    ] {
        let image = load_round_e_reference_image(preset);
        let stats = luminance_stats(&foreground_luminance_values(&image));
        let variance = stats.stddev / 255.0;
        assert!(
            variance >= minimum,
            "Round E {preset} reference image luminance variance {variance:.3} is below {minimum:.3}; \
             source-backed materials must not collapse to a flat fill"
        );
    }

    let clear_glass = load_round_e_reference_image("clear_glass");
    let frosted_glass = load_round_e_reference_image("frosted_glass");
    let target = dark_target_offset(&clear_glass);
    assert!(
        target.count >= thresholds.glass_dark_target_pixel_count_min
            && target.offset_px >= thresholds.glass_refraction_offset_min,
        "Round E clear-glass reference target count {} / offset {:.3} does not prove a visible background target",
        target.count,
        target.offset_px
    );
    let clear_edge = sobel_edge_energy(&clear_glass);
    let frosted_edge = sobel_edge_energy(&frosted_glass);
    assert!(
        clear_edge > 0.0 && frosted_edge > 0.0,
        "Round E glass reference images must contain a measurable background target; got clear edge {clear_edge:.4}, frosted edge {frosted_edge:.4}"
    );

    for (left, right) in [
        ("metal", "rough_metal"),
        ("metal", "chrome"),
        ("chrome", "plastic"),
        ("clearcoat_plastic", "plastic"),
        ("clear_glass", "frosted_glass"),
        ("rubber", "plastic"),
    ] {
        let left_image = load_round_e_reference_image(left);
        let right_image = load_round_e_reference_image(right);
        let delta = foreground_mean_rgb_distance(&left_image, &right_image);
        assert!(
            delta >= thresholds.neighbor_rgb_distance_min,
            "Round E reference neighbor pair {left}/{right} has RGB distance {delta:.3}, below {:.3}",
            thresholds.neighbor_rgb_distance_min
        );
    }
}

fn render_material_preset_tile(preset: MaterialShowcasePreset, width: u32, height: u32) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(preset.geometry_desc());
    let material = match preset.id {
        "satin" => pollster::block_on(assets.material_presets().satin())
            .expect("source-backed satin preset loads for docs image"),
        "leather" => pollster::block_on(assets.material_presets().leather())
            .expect("source-backed leather preset loads for docs image"),
        "rubber" => pollster::block_on(assets.material_presets().rubber())
            .expect("source-backed rubber preset loads for docs image"),
        _ => assets.create_material(preset.material_desc().with_double_sided(true)),
    };
    let environment = pollster::block_on(assets.load_environment_preset(EnvironmentPreset::Studio))
        .expect("studio environment loads for material preset docs image");

    let mut scene = Scene::new();
    if preset.geometry.uses_background_target() {
        let target = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
        for bar in glass_background_target_bars() {
            scene
                .mesh(
                    target,
                    assets.create_material(MaterialDesc::matte(bar.color)),
                )
                .transform(Transform {
                    translation: Vec3::new(0.0, 0.0, -0.14) + bar.offset,
                    rotation: Quat::IDENTITY,
                    scale: bar.scale,
                })
                .add()
                .expect("material preset background target inserts");
        }
    }
    let mut transform = preset.transform();
    transform.translation = Vec3::ZERO;
    scene
        .mesh(geometry, material)
        .transform(transform)
        .add()
        .expect("material preset subject inserts");
    scene
        .directional_light(DirectionalLight::key_light().with_shadows(false))
        .add()
        .expect("material preset light inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("material preset docs-image scene prepares");
    renderer
        .render_active(&scene)
        .expect("material preset docs-image scene renders");
    renderer.frame_rgba8().to_vec()
}

#[derive(Debug, Clone)]
struct RgbaProofImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct MaterialIdentityThresholds {
    chrome_specular_dynamic_range_min: f32,
    chrome_dark_reflection_luminance_p05_max: f32,
    chrome_bright_reflection_luminance_p99_min: f32,
    brushed_steel_reference_aspect_ratio_min: f32,
    clearcoat_lobe_delta_min: f32,
    source_backed_variance_min: f32,
    glass_dark_target_pixel_count_min: usize,
    glass_refraction_offset_min: f32,
    neighbor_rgb_distance_min: f32,
}

#[derive(Debug, Clone, Copy)]
struct LuminancePercentiles {
    p05: f32,
    p10: f32,
    p99: f32,
}

#[derive(Debug, Clone, Copy)]
struct LuminanceStats {
    stddev: f32,
}

#[derive(Debug, Clone, Copy)]
struct DarkTargetMetrics {
    count: usize,
    offset_px: f32,
}

fn material_identity_thresholds() -> MaterialIdentityThresholds {
    MaterialIdentityThresholds {
        chrome_specular_dynamic_range_min: 2.0,
        chrome_dark_reflection_luminance_p05_max: 90.0,
        chrome_bright_reflection_luminance_p99_min: 230.0,
        // The model-viewer reference is a cropped oracle image, not the live
        // WebGL2 proof crop. Keep this close to the committed reference output
        // while still rejecting a round blob or flat plate.
        brushed_steel_reference_aspect_ratio_min: 1.8,
        clearcoat_lobe_delta_min: 0.05,
        source_backed_variance_min: 0.02,
        glass_dark_target_pixel_count_min: 64,
        glass_refraction_offset_min: 4.0,
        neighbor_rgb_distance_min: 6.0,
    }
}

fn load_round_e_reference_image(preset: &str) -> RgbaProofImage {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/visual/references/round_e")
        .join(format!("{preset}.png"));
    let image = image::ImageReader::open(&path)
        .unwrap_or_else(|err| {
            panic!(
                "Round E reference image {} is readable: {err}",
                path.display()
            )
        })
        .decode()
        .unwrap_or_else(|err| panic!("Round E reference image {} decodes: {err}", path.display()))
        .to_rgba8();
    let (width, height) = image.dimensions();
    assert!(
        width >= 128 && height >= 128,
        "Round E reference image {preset} must be a crop large enough for material metrics, got {width}x{height}"
    );
    RgbaProofImage {
        width,
        height,
        rgba: image.into_raw(),
    }
}

fn foreground_luminance_values(image: &RgbaProofImage) -> Vec<f32> {
    foreground_pixel_offsets(image)
        .into_iter()
        .map(|offset| {
            luminance_rgb(
                image.rgba[offset],
                image.rgba[offset + 1],
                image.rgba[offset + 2],
            )
        })
        .collect()
}

fn foreground_pixel_offsets(image: &RgbaProofImage) -> Vec<usize> {
    let background = estimate_background_rgb(image);
    let mut offsets = Vec::new();
    for y in 0..image.height {
        for x in 0..image.width {
            let offset = ((y * image.width + x) as usize) * 4;
            if image.rgba[offset + 3] <= 8 {
                continue;
            }
            let distance = rgb_distance(
                [
                    image.rgba[offset] as f32,
                    image.rgba[offset + 1] as f32,
                    image.rgba[offset + 2] as f32,
                ],
                background,
            );
            if distance > 7.0 {
                offsets.push(offset);
            }
        }
    }
    if offsets.len() >= 64 {
        return offsets;
    }
    (0..image.rgba.len()).step_by(4).collect()
}

fn luminance_percentiles(values: &[f32]) -> LuminancePercentiles {
    assert!(
        !values.is_empty(),
        "Round E material metric needs at least one luminance sample"
    );
    let mut values = values.to_vec();
    values.sort_by(|left, right| left.total_cmp(right));
    LuminancePercentiles {
        p05: percentile_sorted(&values, 0.05),
        p10: percentile_sorted(&values, 0.10),
        p99: percentile_sorted(&values, 0.99),
    }
}

fn luminance_stats(values: &[f32]) -> LuminanceStats {
    assert!(
        !values.is_empty(),
        "Round E material metric needs at least one luminance sample"
    );
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let variance = values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f32>()
        / values.len() as f32;
    LuminanceStats {
        stddev: variance.sqrt(),
    }
}

fn highlight_aspect_ratio(image: &RgbaProofImage) -> f32 {
    let offsets = foreground_pixel_offsets(image);
    if offsets.len() < 16 {
        return 0.0;
    }
    let mut luminance = offsets
        .iter()
        .map(|offset| {
            luminance_rgb(
                image.rgba[*offset],
                image.rgba[*offset + 1],
                image.rgba[*offset + 2],
            )
        })
        .collect::<Vec<_>>();
    luminance.sort_by(|left, right| left.total_cmp(right));
    let cutoff = percentile_sorted(&luminance, 0.95);
    let mut total_weight = 0.0;
    let mut mean_x = 0.0;
    let mut mean_y = 0.0;
    let mut highlights = Vec::new();
    for offset in offsets {
        let pixel_index = offset / 4;
        let x = (pixel_index as u32 % image.width) as f32;
        let y = (pixel_index as u32 / image.width) as f32;
        let lum = luminance_rgb(
            image.rgba[offset],
            image.rgba[offset + 1],
            image.rgba[offset + 2],
        );
        if lum < cutoff {
            continue;
        }
        let weight = lum.max(1.0);
        total_weight += weight;
        mean_x += x * weight;
        mean_y += y * weight;
        highlights.push((x, y, weight));
    }
    if highlights.len() < 8 || total_weight <= 0.0 {
        return 0.0;
    }
    mean_x /= total_weight;
    mean_y /= total_weight;
    let mut xx = 0.0;
    let mut yy = 0.0;
    let mut xy = 0.0;
    for (x, y, weight) in highlights {
        let dx = x - mean_x;
        let dy = y - mean_y;
        xx += weight * dx * dx;
        yy += weight * dy * dy;
        xy += weight * dx * dy;
    }
    xx /= total_weight;
    yy /= total_weight;
    xy /= total_weight;
    let trace = xx + yy;
    let determinant = xx * yy - xy * xy;
    let discriminant = ((trace * trace) * 0.25 - determinant).max(0.0).sqrt();
    let major = trace * 0.5 + discriminant;
    let minor = (trace * 0.5 - discriminant).max(1e-6);
    (major / minor).sqrt()
}

fn dark_target_offset(image: &RgbaProofImage) -> DarkTargetMetrics {
    let background = estimate_background_rgb(image);
    let background_luminance = luminance_rgb_f32(background);
    let mut count = 0;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for y in 0..image.height {
        for x in 0..image.width {
            let offset = ((y * image.width + x) as usize) * 4;
            let luminance = luminance_rgb(
                image.rgba[offset],
                image.rgba[offset + 1],
                image.rgba[offset + 2],
            );
            if luminance >= background_luminance - 18.0 {
                continue;
            }
            count += 1;
            sum_x += x as f32;
            sum_y += y as f32;
        }
    }
    if count == 0 {
        return DarkTargetMetrics {
            count,
            offset_px: 0.0,
        };
    }
    let center_x = sum_x / count as f32;
    let center_y = sum_y / count as f32;
    DarkTargetMetrics {
        count,
        offset_px: ((center_x - image.width as f32 * 0.5).powi(2)
            + (center_y - image.height as f32 * 0.5).powi(2))
        .sqrt(),
    }
}

fn sobel_edge_energy(image: &RgbaProofImage) -> f32 {
    let width = image.width as usize;
    let height = image.height as usize;
    if width < 3 || height < 3 {
        return 0.0;
    }
    let gray = image
        .rgba
        .chunks_exact(4)
        .map(|pixel| luminance_rgb(pixel[0], pixel[1], pixel[2]))
        .collect::<Vec<_>>();
    let mut sum = 0.0;
    let mut count = 0;
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let sample = |x: usize, y: usize| gray[y * width + x];
            let gx = -sample(x - 1, y - 1) + sample(x + 1, y - 1) - 2.0 * sample(x - 1, y)
                + 2.0 * sample(x + 1, y)
                - sample(x - 1, y + 1)
                + sample(x + 1, y + 1);
            let gy = -sample(x - 1, y - 1) - 2.0 * sample(x, y - 1) - sample(x + 1, y - 1)
                + sample(x - 1, y + 1)
                + 2.0 * sample(x, y + 1)
                + sample(x + 1, y + 1);
            sum += (gx * gx + gy * gy).sqrt() / 255.0;
            count += 1;
        }
    }
    sum / count as f32
}

fn foreground_mean_rgb_distance(left: &RgbaProofImage, right: &RgbaProofImage) -> f32 {
    rgb_distance(foreground_mean_rgb(left), foreground_mean_rgb(right))
}

fn foreground_mean_rgb(image: &RgbaProofImage) -> [f32; 3] {
    let offsets = foreground_pixel_offsets(image);
    let mut sum = [0.0, 0.0, 0.0];
    for offset in &offsets {
        sum[0] += image.rgba[*offset] as f32;
        sum[1] += image.rgba[*offset + 1] as f32;
        sum[2] += image.rgba[*offset + 2] as f32;
    }
    let divisor = offsets.len().max(1) as f32;
    [sum[0] / divisor, sum[1] / divisor, sum[2] / divisor]
}

fn estimate_background_rgb(image: &RgbaProofImage) -> [f32; 3] {
    let corner_size = 8_u32.min((image.width.min(image.height) / 4).max(1));
    let mut sum = [0.0, 0.0, 0.0];
    let mut count = 0.0;
    for (x0, y0) in [
        (0, 0),
        (image.width - corner_size, 0),
        (0, image.height - corner_size),
        (image.width - corner_size, image.height - corner_size),
    ] {
        for y in y0..y0 + corner_size {
            for x in x0..x0 + corner_size {
                let offset = ((y * image.width + x) as usize) * 4;
                sum[0] += image.rgba[offset] as f32;
                sum[1] += image.rgba[offset + 1] as f32;
                sum[2] += image.rgba[offset + 2] as f32;
                count += 1.0;
            }
        }
    }
    [sum[0] / count, sum[1] / count, sum[2] / count]
}

fn percentile_sorted(values: &[f32], percentile: f32) -> f32 {
    values[((values.len() - 1) as f32 * percentile).floor() as usize]
}

fn luminance_rgb(red: u8, green: u8, blue: u8) -> f32 {
    luminance_rgb_f32([red as f32, green as f32, blue as f32])
}

fn luminance_rgb_f32(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

fn rgb_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

#[test]
fn round_b_background_preset_reference_docs_image() {
    let tile_width = 96;
    let tile_height = 96;
    let presets = [
        Background::Studio,
        Background::DarkStudio,
        Background::NeutralGray,
        Background::White,
        Background::Black,
        Background::Sky,
        Background::Transparent,
        Background::Custom(Color::MAGENTA),
    ];
    let width = tile_width * presets.len() as u32;
    let height = tile_height;
    let mut rgba = vec![0_u8; (width * height * 4) as usize];
    for (index, background) in presets.iter().enumerate() {
        let tile = render_background_preset_tile(*background, tile_width, tile_height);
        let expected = srgb8_from_linear(background.color());
        assert_eq!(
            &tile[0..4],
            &expected,
            "background preset tile {index} must clear its corner to the named color"
        );
        assert!(
            count_unique_rgb_triplets(&tile) >= MIN_UNIQUE_PIXELS,
            "background preset tile {index} must include both background and subject pixels"
        );
        for y in 0..tile_height {
            let dst_start = ((y * width + index as u32 * tile_width) * 4) as usize;
            let src_start = (y * tile_width * 4) as usize;
            let byte_count = (tile_width * 4) as usize;
            rgba[dst_start..dst_start + byte_count]
                .copy_from_slice(&tile[src_start..src_start + byte_count]);
        }
    }
    write_reference_docs_image_artifact(
        "round-b-background-preset-reference-docs-image",
        "docs/guides/easy-scene-setup.md",
        width,
        height,
        &rgba,
    );
}

fn render_background_preset_tile(background: Background, width: u32, height: u32) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.8, 0.25));
    let material = assets.create_material(MaterialDesc::unlit(Color::ORANGE));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("background preset subject inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_background(background);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("background preset docs-image scene prepares");
    renderer
        .render_active(&scene)
        .expect("background preset docs-image scene renders");
    renderer.frame_rgba8().to_vec()
}

#[test]
fn round_b_orbit_control_preset_animated_docs_image() {
    let frame_width = 96;
    let frame_height = 96;
    let mut frames = Vec::new();
    for controls in [
        OrbitControls::new(Vec3::ZERO, 2.0).presentation(),
        OrbitControls::new(Vec3::ZERO, 2.0).turntable(6.0),
    ] {
        let mut controls = controls;
        let first = render_orbit_control_motion_frame(controls, frame_width, frame_height);
        frames.push(first.clone());
        for _ in 0..3 {
            assert_eq!(controls.advance(0.5), OrbitControlAction::Orbit);
            let frame = render_orbit_control_motion_frame(controls, frame_width, frame_height);
            assert_ne!(
                frame, first,
                "orbit preset animated proof must show a changed frame after advancing"
            );
            frames.push(frame);
        }
    }
    write_animated_docs_image_artifact(
        "round-b-orbit-control-preset-animated-docs-image",
        "docs/guides/easy-scene-setup.md",
        frame_width,
        frame_height,
        &frames,
    );
}

fn render_orbit_control_motion_frame(controls: OrbitControls, width: u32, height: u32) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 0.4, 0.3));
    let material = assets.create_material(MaterialDesc::unlit(Color::CYAN));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("orbit proof subject inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    controls
        .apply_to_scene(&mut scene, camera)
        .expect("orbit controls apply for animated proof");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_background(Background::DarkStudio);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("orbit animated-proof scene prepares");
    renderer
        .render_active(&scene)
        .expect("orbit animated-proof scene renders");
    renderer.frame_rgba8().to_vec()
}

#[test]
fn round_d_orbit_zoom_limit_animated_docs_image() {
    let frame_width = 96;
    let frame_height = 96;
    let mut controls = OrbitControls::new(Vec3::ZERO, 4.0).zoom_limits_bounds_relative(0.5, 2.0);

    let start = render_orbit_zoom_limit_frame(controls, frame_width, frame_height);
    let start_rect =
        nonblack_pixel_rect(&start, frame_width, frame_height).expect("start frame is visible");

    for _ in 0..16 {
        assert_eq!(
            controls.handle_pointer(PointerEvent::wheel(0.0, 0.0, -10.0)),
            OrbitControlAction::Zoom
        );
    }
    assert_close(controls.distance(), 2.0);
    let min = render_orbit_zoom_limit_frame(controls, frame_width, frame_height);
    let min_rect =
        nonblack_pixel_rect(&min, frame_width, frame_height).expect("min frame is visible");

    for _ in 0..16 {
        assert_eq!(
            controls.handle_pointer(PointerEvent::wheel(0.0, 0.0, -10.0)),
            OrbitControlAction::Zoom
        );
    }
    assert_close(controls.distance(), 2.0);
    let min_clamped = render_orbit_zoom_limit_frame(controls, frame_width, frame_height);
    assert_eq!(
        min, min_clamped,
        "orbit zoom-limit proof must show repeated zoom-in clamped at the minimum distance"
    );

    for _ in 0..16 {
        assert_eq!(
            controls.handle_touch(TouchEvent::pinch(0.0, 0.0, 10.0)),
            OrbitControlAction::Zoom
        );
    }
    assert_close(controls.distance(), 8.0);
    let max = render_orbit_zoom_limit_frame(controls, frame_width, frame_height);
    let max_rect =
        nonblack_pixel_rect(&max, frame_width, frame_height).expect("max frame is visible");

    assert!(
        min_rect.width() > start_rect.width(),
        "minimum zoom distance should render the subject larger: start={start_rect:?} min={min_rect:?}"
    );
    assert!(
        max_rect.width() < start_rect.width(),
        "maximum zoom distance should render the subject smaller: start={start_rect:?} max={max_rect:?}"
    );

    write_animated_docs_image_artifact(
        "round-d-orbit-zoom-limit-animated-docs-image",
        "docs/guides/easy-scene-setup.md",
        frame_width,
        frame_height,
        &[start, min, min_clamped, max],
    );
}

fn render_orbit_zoom_limit_frame(controls: OrbitControls, width: u32, height: u32) -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 0.5, 0.3));
    let material = assets.create_material(MaterialDesc::unlit(Color::CYAN));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("orbit zoom-limit subject inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    controls
        .apply_to_scene(&mut scene, camera)
        .expect("orbit zoom-limit controls apply");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_background(Background::Black);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("orbit zoom-limit scene prepares");
    renderer
        .render_active(&scene)
        .expect("orbit zoom-limit scene renders");
    renderer.frame_rgba8().to_vec()
}

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 1.0e-4,
        "expected {left} to be close to {right}"
    );
}

#[test]
fn round_c_auto_exposure_preset_reference_docs_image() {
    let tile_width = 96;
    let tile_height = 96;
    let presets = [
        AutoExposureConfig::product_studio(),
        AutoExposureConfig::indoor(),
        AutoExposureConfig::outdoor(),
        AutoExposureConfig::mixed(),
    ];
    let width = tile_width * presets.len() as u32;
    let height = tile_height;
    let mut rgba = vec![0_u8; (width * height * 4) as usize];
    let mut exposure_evs = Vec::new();
    for (index, config) in presets.iter().enumerate() {
        let (tile, exposure_ev) =
            render_auto_exposure_preset_tile(*config, tile_width, tile_height);
        assert!(
            count_nonblack_pixels(&tile) > 0,
            "auto-exposure preset tile {index} must render a visible subject"
        );
        exposure_evs.push(exposure_ev);
        for y in 0..tile_height {
            let dst_start = ((y * width + index as u32 * tile_width) * 4) as usize;
            let src_start = (y * tile_width * 4) as usize;
            let byte_count = (tile_width * 4) as usize;
            rgba[dst_start..dst_start + byte_count]
                .copy_from_slice(&tile[src_start..src_start + byte_count]);
        }
    }
    assert!(
        exposure_evs[1] > exposure_evs[2],
        "indoor preset should lift the matched dim scene more than outdoor: {exposure_evs:?}"
    );
    write_auto_exposure_reference_docs_image_artifact(
        "round-c-auto-exposure-preset-reference-docs-image",
        "docs/guides/easy-scene-setup.md",
        width,
        height,
        &rgba,
        &exposure_evs,
    );
}

fn render_auto_exposure_preset_tile(
    config: AutoExposureConfig,
    width: u32,
    height: u32,
) -> (Vec<u8>, f32) {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.9, 0.9, 0.2));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(
        0.22, 0.22, 0.22,
    )));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("auto-exposure preset subject inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_background(Background::DarkStudio);
    renderer.set_auto_exposure(config);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("auto-exposure preset docs-image scene prepares");
    renderer
        .render_active(&scene)
        .expect("auto-exposure preset docs-image scene renders");
    let exposure_ev = renderer
        .last_auto_exposure()
        .expect("auto exposure result is recorded")
        .exposure_ev();
    (renderer.frame_rgba8().to_vec(), exposure_ev)
}

#[test]
fn round_c_animation_playback_reference_animated_docs_image() {
    let frame_width = 96;
    let frame_height = 96;
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/animated_connector_scene.gltf"))
            .expect("animated fixture loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("animated fixture instantiates");
    let animated = import
        .node("AnimatedMount")
        .expect("animated node resolves");
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.35, 0.35, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::CYAN));
    scene
        .mesh(geometry, material)
        .parent(animated)
        .add()
        .expect("animated proof subject inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.45, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mixer = scene
        .play_animation_by_name(&import, "MoveMount")
        .expect("clip starts by name");
    let mut frames = Vec::new();
    frames.push(render_animation_playback_frame(
        &mut scene,
        &assets,
        camera,
        frame_width,
        frame_height,
    ));
    for delta_seconds in [0.25, 0.25, 0.25] {
        scene
            .update_animation(mixer, delta_seconds)
            .expect("animation advances");
        frames.push(render_animation_playback_frame(
            &mut scene,
            &assets,
            camera,
            frame_width,
            frame_height,
        ));
    }
    assert_ne!(
        frames.first(),
        frames.last(),
        "animation playback proof must show a changed rendered frame"
    );
    write_animated_docs_image_artifact(
        "round-c-animation-playback-reference-animated-docs-image",
        "docs/guides/easy-scene-setup.md",
        frame_width,
        frame_height,
        &frames,
    );
}

fn render_animation_playback_frame(
    scene: &mut Scene,
    assets: &Assets,
    camera: scena::CameraKey,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_background(Background::DarkStudio);
    renderer
        .prepare_with_assets(scene, assets)
        .expect("animation playback scene prepares");
    renderer
        .render(scene, camera)
        .expect("animation playback scene renders");
    renderer.frame_rgba8().to_vec()
}

#[test]
fn examples_visual_primitive_shapes_renders_box_to_ppm() {
    // Mirror examples/primitive_shapes.rs at the same headless renderer scale.
    let assets = Assets::new();
    let cube = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(90, 148, 255),
        0.0,
        0.55,
    ));

    let mut scene = Scene::new();
    scene.mesh(cube, material).add().expect("box mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("primitive_shapes scene prepares");
    renderer
        .render_active(&scene)
        .expect("primitive_shapes scene renders");

    let frame = renderer.frame_rgba8();
    assert_eq!(
        frame.len(),
        (ARTIFACT_WIDTH as usize) * (ARTIFACT_HEIGHT as usize) * 4
    );
    assert!(
        count_nonblack_pixels(frame) > 0,
        "primitive_shapes example must render at least one nonblack pixel"
    );

    write_artifact("primitive_shapes", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_beginner_diagnostics_renders_recovery_scene_to_ppm() {
    // examples/beginner_diagnostics.rs walks the user through diagnostic
    // recovery; the visible scene after recovery is a single colored quad.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.5, 1.5, 0.05));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(220, 80, 80)));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("recovery mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("beginner diagnostics recovery scene prepares");
    renderer
        .render_active(&scene)
        .expect("beginner diagnostics scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "beginner diagnostics recovery scene must render at least one nonblack pixel"
    );

    write_artifact(
        "beginner_diagnostics",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_camera_framing_renders_framed_part_to_ppm() {
    // Mirror examples/camera_framing.rs: a single oriented part, framed via Aabb.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.2, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 160, 240)));

    let mut scene = Scene::new();
    let inspected_part = scene
        .mesh(geometry, material)
        .add()
        .expect("framed mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    let bounds = Aabb::new(Vec3::new(-0.6, -0.2, -0.2), Vec3::new(0.6, 0.2, 0.2));
    scene.frame(camera, bounds).expect("frame succeeds");
    scene
        .look_at(camera, inspected_part)
        .expect("look_at succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("camera_framing scene prepares");
    renderer
        .render_active(&scene)
        .expect("camera_framing scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "camera_framing example must render at least one nonblack pixel"
    );

    write_artifact("camera_framing", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn frame_bounds_rendered_output_proves_fill_center_and_unclipped_object() {
    let width = 320;
    let height = 180;
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.8, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 160, 240)));

    let mut scene = Scene::new();
    scene.mesh(geometry, material).add().expect("mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default().with_aspect(width as f32 / height as f32),
            Transform::default(),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");
    let bounds = Aabb::new(
        Vec3::new(-0.9, -0.225, -0.175),
        Vec3::new(0.9, 0.225, 0.175),
    );
    let outcome = scene
        .frame_bounds(
            camera,
            bounds,
            scena::FramingOptions::new()
                .front()
                .fill(0.70)
                .margin_px(0.0)
                .viewport(width, height),
        )
        .expect("frame_bounds succeeds");

    assert!(outcome.projected_rect.min_x >= 0.0, "{outcome:?}");
    assert!(outcome.projected_rect.min_y >= 0.0, "{outcome:?}");
    assert!(outcome.projected_rect.max_x <= width as f32, "{outcome:?}");
    assert!(outcome.projected_rect.max_y <= height as f32, "{outcome:?}");
    assert!(
        (0.66..=0.72).contains(&outcome.projected_rect.fill_fraction(width, height)),
        "{outcome:?}"
    );

    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("frame_bounds proof prepares");
    renderer
        .render_active(&scene)
        .expect("frame_bounds proof renders");
    let frame = renderer.frame_rgba8();
    let pixel_rect = nonblack_pixel_rect(frame, width, height).expect("visible rendered object");
    assert!(pixel_rect.width() > width / 3, "{pixel_rect:?}");
    assert!(pixel_rect.height() > height / 10, "{pixel_rect:?}");
    assert!(
        pixel_rect.min_x > 0 && pixel_rect.max_x < width - 1,
        "{pixel_rect:?}"
    );
    assert!(
        pixel_rect.min_y > 0 && pixel_rect.max_y < height - 1,
        "{pixel_rect:?}"
    );
    assert!(
        (pixel_rect.center_x() - width as f32 * 0.5).abs() < width as f32 * 0.08,
        "{pixel_rect:?}"
    );
    assert!(
        (pixel_rect.center_y() - height as f32 * 0.5).abs() < height as f32 * 0.10,
        "{pixel_rect:?}"
    );

    write_frame_bounds_artifact(
        "camera_framing_frame_bounds",
        width,
        height,
        frame,
        outcome,
        pixel_rect,
    );
}

#[test]
fn examples_visual_layers_visibility_renders_active_layer_to_ppm() {
    // Mirror examples/layers_visibility.rs: helper-on-top + hidden layer + tag-based
    // selection, rendered through the camera layer mask.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
    let visible_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 170, 255)));
    let helper_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(255, 230, 80)));

    let mut scene = Scene::new();
    let machine = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(-0.25, 0.0, 0.0)))
        .add()
        .expect("machine mesh inserts");
    let helper = scene
        .mesh(geometry, helper_material)
        .transform(Transform::at(Vec3::new(0.25, 0.0, 0.0)).scale_by(0.5))
        .add()
        .expect("helper mesh inserts");
    let hidden = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(0.0, 0.4, 0.0)))
        .add()
        .expect("hidden mesh inserts");
    scene.add_tag(machine, "operational").expect("tag inserts");
    scene
        .set_layer_mask(machine, 0b0001)
        .expect("machine layer set");
    scene
        .set_layer_mask(helper, 0b0001)
        .expect("helper layer set");
    scene
        .set_layer_mask(hidden, 0b0010)
        .expect("hidden layer set");
    scene
        .set_visible(hidden, false)
        .expect("hidden visibility set");
    scene
        .set_render_group(helper, 10)
        .expect("helper render group set");
    scene
        .set_helper_on_top(helper, true)
        .expect("helper-on-top set");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .set_camera_layer_mask(camera, 0b0001)
        .expect("camera layer mask set");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("layers_visibility scene prepares");
    renderer
        .render_active(&scene)
        .expect("layers_visibility scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "layers_visibility example must render at least one nonblack pixel"
    );

    write_artifact("layers_visibility", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_coordinate_units_renders_converted_position_to_ppm() {
    // Mirror examples/coordinate_units.rs: a CAD-authored Z-up millimeter position
    // converted to scena Y-up meters, then framed by a default camera.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.12, 0.12, 0.12));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 230, 90)));

    let cad_position_mm = Vec3::new(250.0, 0.0, 100.0);
    let meters_per_unit = SourceUnits::Millimeters.meters_per_unit();
    let y_up_position = SourceCoordinateSystem::ZUpRightHanded.convert_position(cad_position_mm);
    let render_position = Vec3::new(
        y_up_position.x * meters_per_unit,
        y_up_position.y * meters_per_unit,
        y_up_position.z * meters_per_unit,
    );

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(render_position))
        .add()
        .expect("converted-position mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .look_at_point(camera, render_position)
        .expect("look_at_point succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("coordinate_units scene prepares");
    renderer
        .render_active(&scene)
        .expect("coordinate_units scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "coordinate_units example must render at least one nonblack pixel"
    );

    write_artifact("coordinate_units", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_imported_cad_drawing_matches_original_dimensions_to_ppm() {
    // CAD-export-style glTF fixture: original drawing is a 120 mm x 60 mm
    // rectangular plate, exported as meters in glTF. This proves the model
    // viewer path imports the drawing, preserves dimensions, frames it, and
    // writes a visual artifact for human comparison.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/cad_plate_drawing_scene.gltf"))
            .expect("CAD drawing glTF loads");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("CAD drawing instantiates");
    let bounds = import
        .bounds_world(&scene)
        .expect("CAD drawing import has bounds");
    let width_m = bounds.max.x - bounds.min.x;
    let height_m = bounds.max.y - bounds.min.y;
    assert_approx_eq(width_m, 0.120, 0.000_5, "CAD drawing width");
    assert_approx_eq(height_m, 0.060, 0.000_5, "CAD drawing height");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene.frame(camera, bounds).expect("CAD drawing frames");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("CAD drawing scene prepares");
    renderer
        .render_active(&scene)
        .expect("CAD drawing scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "CAD drawing import must render at least one nonblack pixel"
    );

    write_artifact(
        "cad_plate_drawing_import",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

fn assert_approx_eq(actual: f32, expected: f32, tolerance: f32, label: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label} expected {expected} +/- {tolerance}, got {actual}"
    );
}

#[test]
fn examples_visual_static_batching_renders_repeated_boxes_to_ppm() {
    // Mirror examples/static_batching.rs: 12 transforms baked through
    // create_static_batch_with_report and rendered as a single mesh batch.
    let assets = Assets::new();
    let source = GeometryDesc::box_xyz(0.12, 0.12, 0.12);
    let transforms = (0..12).map(|index| {
        Transform::at(Vec3::new(
            (index % 6) as f32 * 0.18 - 0.45,
            (index / 6) as f32 * 0.18 - 0.09,
            0.0,
        ))
    });
    let (batch, _report) = assets.create_static_batch_with_report(&source, transforms);
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 200, 60)));
    let mut scene = Scene::new();
    scene
        .mesh(batch, material)
        .add()
        .expect("static-batch mesh inserts");
    scene.add_default_camera().expect("default camera inserts");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("static_batching scene prepares");
    renderer
        .render_active(&scene)
        .expect("static_batching scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "static_batching example must render at least one nonblack pixel"
    );

    write_artifact("static_batching", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_instancing_renders_instance_set_to_ppm() {
    // Mirror examples/instancing.rs: an instance set with reserve + push_instance,
    // 10 boxes laid out along the x axis.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 220, 160)));

    let mut scene = Scene::new();
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    scene
        .reserve_instances(set, 16)
        .expect("instance reserve succeeds");
    for index in 0..10 {
        scene
            .push_instance(
                set,
                Transform {
                    translation: Vec3::new(index as f32 * 0.24 - 1.0, 0.0, 0.0),
                    ..Transform::default()
                },
            )
            .expect("push_instance succeeds");
    }
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("instancing scene prepares");
    renderer
        .render_active(&scene)
        .expect("instancing scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "instancing example must render at least one nonblack pixel"
    );

    write_artifact("instancing", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_labels_helpers_renders_axes_bounds_anchor_label_to_ppm() {
    // Mirror examples/labels_helpers.rs: axes + bounding box + anchor marker + a
    // single bitmap label, all rendered through the line material.
    let assets = Assets::new();
    let axes = assets.create_geometry(GeometryDesc::axes(1.0));
    let bounds = assets.create_geometry(GeometryDesc::bounding_box(Aabb::new(
        Vec3::new(-0.5, -0.5, -0.5),
        Vec3::new(0.5, 0.5, 0.5),
    )));
    let anchor = assets.create_geometry(GeometryDesc::anchor_marker(0.15));
    let material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(200, 220, 255), 1.0));

    let mut scene = Scene::new();
    scene.mesh(axes, material).add().expect("axes mesh inserts");
    scene
        .mesh(bounds, material)
        .add()
        .expect("bounds mesh inserts");
    scene
        .mesh(anchor, material)
        .add()
        .expect("anchor mesh inserts");
    let label = LabelDesc::bitmap("origin")
        .with_color(Color::from_srgb_u8(255, 255, 255))
        .with_size(14.0);
    scene
        .add_label(
            scene.root(),
            label,
            Transform {
                translation: Vec3::new(0.0, 0.15, 0.0),
                ..Transform::default()
            },
        )
        .expect("label inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("labels_helpers scene prepares");
    renderer
        .render_active(&scene)
        .expect("labels_helpers scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "labels_helpers example must render at least one nonblack pixel"
    );

    write_artifact("labels_helpers", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_picking_selection_hover_renders_pick_state_to_ppm() {
    // Mirror examples/picking_selection_hover.rs: a single mesh, framed via
    // frame_all_with_assets, then pick_and_select_with_assets at the viewport
    // center. The artifact proves picking updates the interaction state while
    // the typed CursorPosition + Viewport API keeps the rendered scene visible.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.7, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(64, 160, 255)));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("picked mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("picking_selection_hover scene prepares");
    let viewport =
        Viewport::new(ARTIFACT_WIDTH, ARTIFACT_HEIGHT, 1.0).expect("static viewport is valid");
    scene
        .pick_and_select_with_assets(
            camera,
            CursorPosition::physical(ARTIFACT_WIDTH as f32 / 2.0, ARTIFACT_HEIGHT as f32 / 2.0),
            viewport,
            &assets,
        )
        .expect("pick_and_select succeeds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("picking_selection_hover scene re-prepares after selection mutation");
    renderer
        .render_active(&scene)
        .expect("picking_selection_hover scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "picking_selection_hover example must render at least one nonblack pixel"
    );

    write_artifact(
        "picking_selection_hover",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn round_d_viewer_pointer_callback_animated_docs_image() {
    let frame_width = 96;
    let frame_height = 64;
    let mut viewer = interactive_gltf_viewer(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        PlatformSurface::native_window(frame_width, frame_height),
    )
    .build()
    .expect("interactive viewer builds");

    let (hit_x, hit_y) = viewer_hit_coordinate(&viewer, frame_width, frame_height);
    let click_events: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let hover_events: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    viewer.on_click({
        let click_events = Rc::clone(&click_events);
        move |result| click_events.borrow_mut().push(callback_state(result))
    });
    viewer.on_hover({
        let hover_events = Rc::clone(&hover_events);
        move |result| hover_events.borrow_mut().push(callback_state(result))
    });

    let idle_scene = render_interactive_viewer_scene_frame(&mut viewer, frame_width, frame_height);
    assert_interactive_viewer_frame_visible(&idle_scene, "idle");
    let idle = overlay_pointer_callback_state(
        idle_scene,
        frame_width,
        frame_height,
        PointerCallbackVisualState::Idle,
    );

    viewer
        .hover_at(hit_x, hit_y)
        .expect("hover callback pick runs");
    let hover_scene = render_interactive_viewer_scene_frame(&mut viewer, frame_width, frame_height);
    assert_interactive_viewer_frame_visible(&hover_scene, "hover");
    let hover = overlay_pointer_callback_state(
        hover_scene,
        frame_width,
        frame_height,
        PointerCallbackVisualState::HoverHit,
    );

    viewer
        .click_at(hit_x, hit_y)
        .expect("click callback pick runs");
    let click_scene = render_interactive_viewer_scene_frame(&mut viewer, frame_width, frame_height);
    assert_interactive_viewer_frame_visible(&click_scene, "click");
    let click = overlay_pointer_callback_state(
        click_scene,
        frame_width,
        frame_height,
        PointerCallbackVisualState::ClickHit,
    );

    viewer
        .hover_at(10_000.0, 10_000.0)
        .expect("hover miss callback pick runs");
    let miss_scene = render_interactive_viewer_scene_frame(&mut viewer, frame_width, frame_height);
    assert_interactive_viewer_frame_visible(&miss_scene, "miss");
    let miss = overlay_pointer_callback_state(
        miss_scene,
        frame_width,
        frame_height,
        PointerCallbackVisualState::HoverMiss,
    );

    assert_eq!(&*click_events.borrow(), &["hit"]);
    assert_eq!(&*hover_events.borrow(), &["hit", "miss"]);
    assert_frames_differ(
        &idle,
        &hover,
        "viewer hover callback proof should show the hover-hit state",
    );
    assert_frames_differ(
        &hover,
        &click,
        "viewer click callback proof should show the click-hit state",
    );
    assert_frames_differ(
        &click,
        &miss,
        "viewer hover miss callback proof should show the miss state",
    );

    write_animated_docs_image_artifact(
        "round-d-viewer-pointer-callback-animated-docs-image",
        "docs/guides/easy-scene-setup.md",
        frame_width,
        frame_height,
        &[idle, hover, click, miss],
    );
}

#[test]
fn viewer_material_variant_reference_docs_image() {
    let tile_width = 96;
    let tile_height = 96;
    let variants = [None, Some("midnight"), Some("noon")];
    let expected_dominant_channels = [0, 2, 1];
    let mut tiles = Vec::new();

    for (variant, expected_channel) in variants
        .into_iter()
        .zip(expected_dominant_channels.into_iter())
    {
        let frame = render_material_variant_tile(variant, tile_width, tile_height);
        assert_eq!(
            dominant_nonblack_channel(&frame),
            expected_channel,
            "material variant {variant:?} should render its expected color family",
        );
        tiles.push(frame);
    }

    let width = tile_width * tiles.len() as u32;
    let height = tile_height;
    let mut contact = vec![0_u8; (width * height * 4) as usize];
    for (index, tile) in tiles.iter().enumerate() {
        for y in 0..tile_height {
            let dst_start = ((y * width + index as u32 * tile_width) * 4) as usize;
            let src_start = (y * tile_width * 4) as usize;
            let byte_count = (tile_width * 4) as usize;
            contact[dst_start..dst_start + byte_count]
                .copy_from_slice(&tile[src_start..src_start + byte_count]);
        }
    }

    assert_frames_differ(
        &tiles[0],
        &tiles[1],
        "default and midnight material variants must differ visually",
    );
    assert_frames_differ(
        &tiles[1],
        &tiles[2],
        "midnight and noon material variants must differ visually",
    );
    write_reference_docs_image_artifact(
        "viewer-material-variant-reference-docs-image",
        "docs/guides/easy-scene-setup.md",
        width,
        height,
        &contact,
    );
}

fn render_material_variant_tile(variant: Option<&str>, width: u32, height: u32) -> Vec<u8> {
    let mut viewer = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/material_variants_scene.gltf")
            .size(width, height)
            .build(),
    )
    .expect("material variants viewer builds");
    if let Some(name) = variant {
        viewer
            .set_active_material_variant(Some(name))
            .expect("material variant applies");
    }
    viewer
        .render_next_frame()
        .expect("material variant frame renders");
    viewer.snapshot_rgba8().to_vec()
}

fn dominant_nonblack_channel(rgba: &[u8]) -> usize {
    let mut sums = [0_u64; 3];
    let mut visible = 0_u64;
    for pixel in rgba.chunks_exact(4) {
        if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
            continue;
        }
        sums[0] += u64::from(pixel[0]);
        sums[1] += u64::from(pixel[1]);
        sums[2] += u64::from(pixel[2]);
        visible += 1;
    }
    assert!(visible > 0, "frame must contain visible material pixels");
    sums.iter()
        .enumerate()
        .max_by_key(|(_, sum)| *sum)
        .map(|(index, _)| index)
        .expect("RGB channel sums are non-empty")
}

fn callback_state(
    result: std::result::Result<Option<scena::Hit>, scena::LookupError>,
) -> &'static str {
    match result {
        Ok(Some(_)) => "hit",
        Ok(None) => "miss",
        Err(_) => "error",
    }
}

fn viewer_hit_coordinate(viewer: &InteractiveGltfViewer, width: u32, height: u32) -> (f32, f32) {
    for y in (4..height).step_by(4) {
        for x in (4..width).step_by(4) {
            if viewer
                .pick_at(x as f32, y as f32)
                .expect("viewer hit search pick runs")
                .is_some()
            {
                return (x as f32, y as f32);
            }
        }
    }
    panic!("interactive viewer fixture should expose at least one pickable pixel");
}

#[derive(Debug, Clone, Copy)]
enum PointerCallbackVisualState {
    Idle,
    HoverHit,
    ClickHit,
    HoverMiss,
}

fn assert_interactive_viewer_frame_visible(frame: &[u8], label: &str) {
    assert!(
        count_nonblack_pixels(frame) > 0,
        "viewer pointer callback {label} frame must render at least one nonblack scene pixel"
    );
}

fn overlay_pointer_callback_state(
    mut frame: Vec<u8>,
    width: u32,
    height: u32,
    state: PointerCallbackVisualState,
) -> Vec<u8> {
    let colors = match state {
        PointerCallbackVisualState::Idle => [[72, 72, 72], [72, 72, 72], [72, 72, 72]],
        PointerCallbackVisualState::HoverHit => [[255, 210, 64], [72, 72, 72], [72, 72, 72]],
        PointerCallbackVisualState::ClickHit => [[255, 210, 64], [255, 64, 180], [72, 72, 72]],
        PointerCallbackVisualState::HoverMiss => [[72, 72, 72], [255, 64, 180], [120, 120, 120]],
    };
    let marker_size = 5_u32;
    let marker_gap = 2_u32;
    let top = height.saturating_sub(marker_size + marker_gap);
    for (index, color) in colors.into_iter().enumerate() {
        let left = marker_gap + index as u32 * (marker_size + marker_gap);
        paint_marker(&mut frame, width, left, top, marker_size, color);
    }
    frame
}

fn paint_marker(frame: &mut [u8], width: u32, left: u32, top: u32, size: u32, color: [u8; 3]) {
    for y in top..top + size {
        for x in left..left + size {
            let index = ((y * width + x) * 4) as usize;
            frame[index] = color[0];
            frame[index + 1] = color[1];
            frame[index + 2] = color[2];
            frame[index + 3] = 255;
        }
    }
}

fn assert_frames_differ(before: &[u8], after: &[u8], message: &str) {
    let changed_pixels = before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .filter(|(left, right)| left != right)
        .count();
    assert!(changed_pixels > 0, "{message}; changed_pixels=0");
}

fn render_interactive_viewer_scene_frame(
    viewer: &mut InteractiveGltfViewer,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let assets = viewer.assets().clone();
    let camera = viewer.camera();
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer
        .prepare_with_assets(viewer.scene_mut(), &assets)
        .expect("interactive viewer callback scene prepares");
    renderer
        .render(viewer.scene(), camera)
        .expect("interactive viewer callback scene renders");
    renderer.frame_rgba8().to_vec()
}

#[test]
fn examples_visual_animation_renders_morph_clip_at_frame_to_ppm() {
    // Mirror examples/animation.rs: load the Khronos AnimatedMorphCube glTF, create
    // and play the "Square" mixer, advance one 60Hz frame, then render. Proves the
    // animation mixer + glTF morph-target path produces visible pixels end-to-end.
    let assets = Assets::new();
    let scene_asset = pollster::block_on(
        assets.load_scene("tests/assets/gltf/khronos/MorphCube/AnimatedMorphCube.gltf"),
    )
    .expect("morph cube fixture loads");

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("morph cube instantiates");
    let mixer = scene
        .create_animation_mixer(&import, "Square")
        .expect("Square mixer creates");
    scene
        .play_animation(mixer)
        .expect("play animation succeeds");
    scene
        .update_animation(mixer, 1.0 / 60.0)
        .expect("update_animation succeeds");

    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("animation scene prepares");
    renderer
        .render_active(&scene)
        .expect("animation scene renders");

    let state = scene.animation_mixer(mixer).expect("mixer query").state();
    assert_eq!(
        state,
        AnimationPlaybackState::Playing,
        "animation example must record the mixer as playing after update"
    );

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "animation example must render at least one nonblack pixel"
    );

    write_artifact("animation", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_glb_model_viewer_renders_imported_mesh_to_ppm() {
    // Mirror examples/glb_model_viewer.rs: a single first_render_gltf_headless call
    // against the mesh+material+vertex-color sample fixture. Proves the high-level
    // first-render + glTF mesh import + framing path produces visible pixels.
    let first = pollster::block_on(scena::first_render_gltf_headless(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
    ))
    .expect("first_render_gltf_headless succeeds");

    let frame = first.renderer().frame_rgba8();
    assert_eq!(
        frame.len(),
        (ARTIFACT_WIDTH as usize) * (ARTIFACT_HEIGHT as usize) * 4
    );
    assert!(
        count_nonblack_pixels(frame) > 0,
        "glb_model_viewer example must render at least one nonblack pixel"
    );

    write_artifact("glb_model_viewer", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_industrial_static_scene_renders_to_ppm() {
    // Mirror examples/industrial_static_scene.rs: a floor grid plus three machine
    // bodies with pipe connectors, framed via frame_all_with_assets and rendered
    // through the Industrial render profile.
    let assets = Assets::new();
    let floor = assets.create_geometry(GeometryDesc::grid(3.0, 12));
    let body = assets.create_geometry(GeometryDesc::box_xyz(0.36, 0.2, 0.18));
    let pipe = assets.create_geometry(GeometryDesc::box_xyz(0.08, 0.08, 0.7));
    let floor_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(90, 110, 130), 1.0));
    let body_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(55, 150, 220)));
    let pipe_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(205, 210, 220)));

    let mut scene = Scene::new();
    scene
        .mesh(floor, floor_material)
        .transform(Transform::at(Vec3::new(0.0, -0.35, 0.0)))
        .add()
        .expect("floor mesh inserts");
    for x in [-0.45_f32, 0.0, 0.45] {
        scene
            .mesh(body, body_material)
            .transform(Transform::at(Vec3::new(x, 0.0, 0.0)))
            .add()
            .expect("body mesh inserts");
        scene
            .mesh(pipe, pipe_material)
            .transform(Transform::at(Vec3::new(x, -0.18, 0.0)))
            .add()
            .expect("pipe mesh inserts");
    }
    scene
        .add_label(
            scene.root(),
            LabelDesc::bitmap("Line A"),
            Transform::at(Vec3::new(0.0, 0.34, 0.0)),
        )
        .expect("label inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let options = RendererOptions::default().with_profile(Profile::Industrial);
    let mut renderer = Renderer::headless_with_options(ARTIFACT_WIDTH, ARTIFACT_HEIGHT, options)
        .expect("headless renderer builds with industrial options");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("industrial_static_scene prepares");
    renderer
        .render_active(&scene)
        .expect("industrial_static_scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "industrial_static_scene example must render at least one nonblack pixel"
    );

    write_artifact(
        "industrial_static_scene",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_connect_objects_renders_assembled_pair_to_ppm() {
    // Mirror examples/connect_objects.rs: typed connector handles join two empties
    // by named connectors, then we frame the assembly via frame_all_with_assets so
    // the rendered output proves the connector solve placed both nodes inside the
    // viewport.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let motor_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(220, 110, 70)));
    let pump_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 180, 220)));

    let mut scene = Scene::new();
    let motor = scene
        .mesh(geometry, motor_material)
        .transform(Transform::IDENTITY)
        .add()
        .expect("motor mesh inserts");
    let pump = scene
        .mesh(geometry, pump_material)
        .transform(Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .add()
        .expect("pump mesh inserts");
    let motor_shaft = scene
        .add_connector(
            ConnectorFrame::new(motor, Transform::at(Vec3::new(0.5, 0.0, 0.0))).named("shaft"),
        )
        .expect("motor connector inserts");
    let pump_drive = scene
        .add_connector(
            ConnectorFrame::new(pump, Transform::at(Vec3::new(-0.25, 0.0, 0.0))).named("drive"),
        )
        .expect("pump connector inserts");
    scene
        .connect_by_key(motor_shaft, pump_drive, ConnectOptions::default())
        .expect("connect_by_key succeeds");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("connect_objects scene prepares");
    renderer
        .render_active(&scene)
        .expect("connect_objects scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "connect_objects example must render at least one nonblack pixel"
    );

    write_artifact("connect_objects", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_imported_anchor_connection_renders_to_ppm() {
    // Mirror examples/imported_anchor_connection.rs: two imports of the anchor-debug
    // glTF scene connected by their named inspection anchors via
    // ConnectorFrame::from_import_anchor + connect_by_key, then framed via
    // frame_import on the target.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor debug scene loads");

    let mut scene = Scene::new();
    let source = scene
        .instantiate(&scene_asset)
        .expect("source instantiates");
    let target = scene
        .instantiate(&scene_asset)
        .expect("target instantiates");
    scene
        .set_transform(target.roots()[0], Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("target transform succeeds");

    let source_anchor = scene
        .add_connector(
            ConnectorFrame::from_import_anchor(source.anchor("inspection").expect("source anchor"))
                .with_kind("mount"),
        )
        .expect("source connector inserts");
    let target_anchor = scene
        .add_connector(
            ConnectorFrame::from_import_anchor(target.anchor("inspection").expect("target anchor"))
                .with_kind("mount"),
        )
        .expect("target connector inserts");
    scene
        .connect_by_key(source_anchor, target_anchor, ConnectOptions::default())
        .expect("connect_by_key succeeds");

    // Add a small visible marker so the scene has nonblack pixels — the anchor-only
    // imports themselves carry no mesh content and the upstream example similarly
    // doesn't render visible geometry.
    let marker = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let marker_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 200, 255)));
    scene
        .mesh(marker, marker_material)
        .add()
        .expect("anchor visualisation marker inserts");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");
    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("imported_anchor_connection scene prepares");
    renderer
        .render_active(&scene)
        .expect("imported_anchor_connection scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "imported_anchor_connection example must render at least one nonblack pixel"
    );

    write_artifact(
        "imported_anchor_connection",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_anchor_alignment_renders_anchor_marker_to_ppm() {
    // Mirror examples/anchor_alignment.rs: load the anchor-debug glTF, snap an
    // anchor-marker mesh to its named "inspection" anchor, then frame the imported
    // anchor and a small visible marker via frame_all_with_assets so the PPM
    // proves the snap_anchor + ConnectorFrame::from_import_anchor path lands the
    // marker at the right position. The upstream example uses frame_import which
    // panics on the anchor-only fixture; we use frame_all_with_assets instead.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor debug scene loads");
    let marker_geometry = assets.create_geometry(GeometryDesc::anchor_marker(0.2));
    let marker_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(255, 220, 70), 1.0));

    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).expect("scene instantiates");
    let marker = scene
        .mesh(marker_geometry, marker_material)
        .add()
        .expect("marker mesh inserts");
    scene
        .snap_anchor(
            marker,
            import.anchor("inspection").expect("inspection anchor"),
        )
        .expect("snap_anchor succeeds");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("anchor_alignment scene prepares");
    renderer
        .render_active(&scene)
        .expect("anchor_alignment scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "anchor_alignment example must render at least one nonblack pixel"
    );

    write_artifact("anchor_alignment", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_industrial_connector_assembly_renders_to_ppm() {
    // Mirror examples/industrial_connector_assembly.rs: three imports of the
    // connector-debug glTF chained pump → base and sensor → pump via named
    // "mount" connectors with ConnectionAlignment::ForwardToBack and a small
    // mate offset, plus a visible marker so the otherwise-anchor-only fixture
    // produces nonblack pixels under frame_all_with_assets.
    let assets = Assets::new();
    let part_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_debug_scene.gltf"))
            .expect("connector debug scene loads");

    let mut scene = Scene::new();
    let base = scene.instantiate(&part_asset).expect("base instantiates");
    let pump = scene.instantiate(&part_asset).expect("pump instantiates");
    let sensor = scene.instantiate(&part_asset).expect("sensor instantiates");

    scene
        .set_transform(base.roots()[0], Transform::at(Vec3::new(0.0, 0.0, 0.0)))
        .expect("base transform succeeds");
    scene
        .set_transform(pump.roots()[0], Transform::at(Vec3::new(0.01, 0.0, 0.0)))
        .expect("pump transform succeeds");
    scene
        .set_transform(sensor.roots()[0], Transform::at(Vec3::new(2.0, 0.0, 0.0)))
        .expect("sensor transform succeeds");
    scene
        .lock_node_for_connections(base.roots()[0])
        .expect("lock base succeeds");

    let base_mount =
        ConnectorFrame::from_import_connector(base.connector("mount").expect("base mount"));
    let pump_mount =
        ConnectorFrame::from_import_connector(pump.connector("mount").expect("pump mount"));
    let sensor_mount =
        ConnectorFrame::from_import_connector(sensor.connector("mount").expect("sensor mount"));
    let options = ConnectOptions::default().with_alignment(ConnectionAlignment::ForwardToBack);
    scene
        .connect(pump_mount.clone(), base_mount, options)
        .expect("pump-base connect succeeds");
    scene
        .connect(
            sensor_mount,
            pump_mount,
            options.with_mate_offset(Transform::at(Vec3::new(0.4, 0.0, 0.0))),
        )
        .expect("sensor-pump connect succeeds");

    // Anchor-only fixture has no mesh content; add a visible marker so frame_all
    // produces nonblack pixels.
    let marker = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let marker_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(220, 180, 70)));
    scene
        .mesh(marker, marker_material)
        .add()
        .expect("assembly marker inserts");

    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("industrial_connector_assembly scene prepares");
    renderer
        .render_active(&scene)
        .expect("industrial_connector_assembly scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "industrial_connector_assembly example must render at least one nonblack pixel"
    );

    write_artifact(
        "industrial_connector_assembly",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_coordinate_connector_repair_renders_repaired_assembly_to_ppm() {
    // Mirror examples/coordinate_connector_repair.rs: import the connector_zup
    // fixture with the WRONG handedness (YUpLeftHanded), assert the connect call
    // fails closed with ConnectionError::HandednessMismatch, then re-import with
    // the correct ZUpRightHanded coordinate system and connect successfully. Plus
    // a visible marker so the otherwise-anchor-only fixture renders nonblack.
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_zup_scene.gltf"))
            .expect("connector_zup fixture loads");

    let mut scene = Scene::new();
    let source = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("source empty inserts");

    let wrong_import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::YUpLeftHanded),
        )
        .expect("wrong-handedness instantiates");
    let error = scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_connector(
                wrong_import.connector("z-up-mount").expect("z-up-mount"),
            ),
            ConnectOptions::default(),
        )
        .expect_err("left-handed import must be repaired before connecting");
    match error {
        ConnectionError::HandednessMismatch { .. } => {}
        other => panic!("expected HandednessMismatch error, got {other:?}"),
    }

    let repaired_import = scene
        .instantiate_with(
            &scene_asset,
            scena::ImportOptions::gltf_default()
                .with_source_coordinate_system(SourceCoordinateSystem::ZUpRightHanded),
        )
        .expect("repaired instantiates");
    scene
        .connect(
            ConnectorFrame::new(source, Transform::IDENTITY).named("source"),
            ConnectorFrame::from_import_connector(
                repaired_import.connector("z-up-mount").expect("z-up-mount"),
            ),
            ConnectOptions::default(),
        )
        .expect("repaired connect succeeds");

    let marker = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let marker_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(110, 220, 160)));
    scene
        .mesh(marker, marker_material)
        .add()
        .expect("repair marker inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");
    scene
        .frame_all_with_assets(camera, &assets)
        .expect("frame_all succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("coordinate_connector_repair scene prepares");
    renderer
        .render_active(&scene)
        .expect("coordinate_connector_repair scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "coordinate_connector_repair example must render at least one nonblack pixel"
    );

    write_artifact(
        "coordinate_connector_repair",
        ARTIFACT_WIDTH,
        ARTIFACT_HEIGHT,
        frame,
    );
}

#[test]
fn examples_visual_orbit_controls_renders_oriented_box_to_ppm() {
    // Mirror examples/orbit_controls.rs: a single box, default camera, then run
    // the orbit-controls pointer + touch + wheel sequence and apply_to_scene
    // before rendering. Proves the platform-neutral controls drive a real camera
    // transform that survives the prepare/render lifecycle.
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.45, 0.35));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(90, 180, 220)));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("orbit-controlled mesh inserts");
    let camera = scene.add_default_camera().expect("default camera inserts");

    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0);
    controls.handle_pointer(PointerEvent::primary_pressed(160.0, 120.0));
    controls.handle_pointer(PointerEvent::moved(168.0, 116.0, 8.0, -4.0));
    controls.handle_touch(TouchEvent::pinch(168.0, 116.0, -0.1));
    controls.handle_pointer(PointerEvent::wheel(168.0, 116.0, -0.25));
    controls
        .apply_to_scene(&mut scene, camera)
        .expect("apply_to_scene succeeds");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("orbit_controls scene prepares");
    renderer
        .render_active(&scene)
        .expect("orbit_controls scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "orbit_controls example must render at least one nonblack pixel"
    );

    write_artifact("orbit_controls", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}

#[test]
fn examples_visual_headless_ci_renders_default_scene_to_ppm() {
    // examples/headless_ci.rs exercises the deterministic headless rendering
    // path; the proof is "the deterministic frame produces non-black pixels".
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.8, 0.8, 0.8));
    let material = assets.create_material(MaterialDesc::pbr_metallic_roughness(
        Color::from_srgb_u8(120, 200, 140),
        0.1,
        0.6,
    ));

    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("headless ci mesh inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.5)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let mut renderer =
        Renderer::headless(ARTIFACT_WIDTH, ARTIFACT_HEIGHT).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("headless_ci scene prepares");
    renderer
        .render_active(&scene)
        .expect("headless_ci scene renders");

    let frame = renderer.frame_rgba8();
    assert!(
        count_nonblack_pixels(frame) > 0,
        "headless_ci example must render at least one nonblack pixel"
    );

    write_artifact("headless_ci", ARTIFACT_WIDTH, ARTIFACT_HEIGHT, frame);
}
