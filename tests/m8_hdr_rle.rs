//! Regression tests for Radiance HDR loading, specifically the RLE-encoded
//! scanline path that scena's hand-rolled decoder did not handle. Most
//! real-world HDRs (polyhaven, HDRI Haven) use RLE compression.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::future::{Ready, ready};

use scena::render::precompute_environment_sidecar;
use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, Background, EnvironmentDesc,
    EnvironmentSidecarProfile, GeometryDesc, MaterialDesc, PerspectiveCamera, Renderer,
    SIDECAR_FILE_SUFFIX, Scene, Transform, Vec3,
};

#[derive(Clone)]
struct MemoryFetcher {
    files: BTreeMap<AssetPath, Vec<u8>>,
}

impl MemoryFetcher {
    fn new(files: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            files: files.into_iter().collect(),
        }
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| AssetError::NotFound {
                    path: path.as_str().to_string(),
                }),
        )
    }
}

/// Builds an RLE-encoded Radiance HDR with one repeated pixel value across
/// an 8-wide × `height`-tall image. The RLE format scanline header is
/// `0x02 0x02 W_hi W_lo`, and each of the four (R, G, B, E) channels then
/// emits a "run" code (>= 128) followed by the byte to repeat.
fn rle_radiance_hdr_uniform(width: u32, height: u32, rgbe: [u8; 4]) -> Vec<u8> {
    assert!(
        width >= 8,
        "RLE scanline encoding only triggers for width >= 8 per the Radiance \
         HDR spec; the decoder falls back to uncompressed for narrower scanlines"
    );
    assert!(
        width <= 127,
        "fixture uses single-byte run counts; keep width small"
    );
    let mut bytes =
        format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
    for _ in 0..height {
        // Scanline header: 02 02 W_hi W_lo (width up to 0x7fff).
        bytes.push(0x02);
        bytes.push(0x02);
        bytes.push((width >> 8) as u8);
        bytes.push((width & 0xff) as u8);
        // Each channel: one run of `width` copies of the channel byte.
        // count > 128 means "run of (count - 128) of next byte".
        for channel in &rgbe {
            bytes.push(0x80 + width as u8);
            bytes.push(*channel);
        }
    }
    bytes
}

fn rle_radiance_hdr_vertical_bars(
    width: u32,
    height: u32,
    dark: [u8; 4],
    bright: [u8; 4],
) -> Vec<u8> {
    assert!(width >= 8);
    assert!(width <= 127);
    let mut bytes =
        format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
    for _ in 0..height {
        bytes.push(0x02);
        bytes.push(0x02);
        bytes.push((width >> 8) as u8);
        bytes.push((width & 0xff) as u8);
        for channel in 0..4 {
            bytes.push(width as u8);
            for x in 0..width {
                let source = if (x / 2).is_multiple_of(2) {
                    bright
                } else {
                    dark
                };
                bytes.push(source[channel]);
            }
        }
    }
    bytes
}

#[test]
fn rle_compressed_radiance_hdr_decodes_into_environment_irradiance() {
    // Hand-encoded 8×1 RLE HDR with every pixel = RGBE(64, 32, 16, 129).
    // Decoded radiance (mantissa * 2^(exponent - 128)) for E=129: each
    // channel byte is multiplied by 2^1 / 256 = 1/128 ≈ value/128, so the
    // expected linear RGB is approximately (0.5, 0.25, 0.125).
    let bytes = rle_radiance_hdr_uniform(8, 1, [64, 32, 16, 129]);
    let path = AssetPath::from("memory://rle-fixture/uniform-rg.hdr");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(path.clone(), bytes)]));

    let environment = pollster::block_on(assets.load_environment(path.as_str()))
        .expect("RLE-compressed Radiance HDR loads through scena's decoder");
    let desc = assets
        .environment(environment)
        .expect("environment present");
    let irradiance = desc
        .preview_irradiance_rgb()
        .expect("RLE HDR decode yields preview irradiance");

    let expected = [64.0 / 128.0, 32.0 / 128.0, 16.0 / 128.0];
    let tolerance = 0.02;
    for (channel, (actual, expected)) in irradiance.iter().zip(expected.iter()).enumerate() {
        assert!(
            (actual - expected).abs() < tolerance,
            "channel {channel}: decoded {actual} differs from expected {expected} by more than {tolerance}",
        );
    }
    assert_eq!(desc.source_dimensions(), Some((8, 1)));
}

#[test]
fn profile_mismatched_sidecar_renders_structured_chrome_not_flat() {
    let path = AssetPath::from("memory://profile-mismatch-striped-studio.hdr");
    let bytes = rle_radiance_hdr_vertical_bars(32, 16, [6, 6, 6, 128], [255, 255, 255, 131]);
    let plain = EnvironmentDesc::from_equirectangular_hdr_bytes(path.as_str(), &bytes)
        .expect("striped HDR decodes")
        .with_cubemap_resolution(16);
    let sidecar =
        precompute_environment_sidecar(&plain, EnvironmentSidecarProfile::InteractiveWebGl2)
            .expect("interactive sidecar bakes")
            .to_bytes();
    let sidecar_path = AssetPath::from(format!("{}{}", path.as_str(), SIDECAR_FILE_SUFFIX));
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (path.clone(), bytes),
        (sidecar_path, sidecar),
    ]));
    let environment = pollster::block_on(assets.load_environment(path.as_str()))
        .expect("environment loads from HDR plus sidecar");

    let geometry = assets.create_geometry(GeometryDesc::sphere(0.95, 40, 20));
    let material = assets.create_material(MaterialDesc::chrome());
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .expect("chrome sphere inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 3.0)),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("active camera sets");

    let width = 96;
    let height = 96;
    let mut renderer = Renderer::headless(width, height).expect("headless renderer builds");
    renderer.set_background(Background::Black);
    renderer.set_environment(environment);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("profile-mismatched sidecar falls back to a fresh native bake");
    assert_eq!(
        renderer.stats().environment_prefilter_passes,
        1,
        "native Reference prepares must report a fresh bake when only a WebGl2 sidecar exists"
    );
    renderer
        .render_active(&scene)
        .expect("chrome sphere renders with IBL");

    let (range, gradient) = crop_luma_range_and_gradient(renderer.frame_rgba8(), width, 30, 30, 36);
    assert!(
        range >= 80.0 && gradient >= 0.8,
        "chrome under a structured HDR must retain visible reflection contrast when the \
         attached sidecar profile is mismatched; old behavior fell back to flat constant \
         specular. observed range={range:.2}, gradient={gradient:.2}"
    );
}

fn crop_luma_range_and_gradient(
    rgba: &[u8],
    width: u32,
    x0: u32,
    y0: u32,
    size: u32,
) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut gradient_sum = 0.0_f32;
    let mut gradient_count = 0_u32;
    for y in y0..y0 + size {
        for x in x0..x0 + size {
            let luma = pixel_luma(rgba, width, x, y);
            min = min.min(luma);
            max = max.max(luma);
            if x + 1 < x0 + size {
                gradient_sum += (luma - pixel_luma(rgba, width, x + 1, y)).abs();
                gradient_count += 1;
            }
            if y + 1 < y0 + size {
                gradient_sum += (luma - pixel_luma(rgba, width, x, y + 1)).abs();
                gradient_count += 1;
            }
        }
    }
    (max - min, gradient_sum / gradient_count.max(1) as f32)
}

fn pixel_luma(rgba: &[u8], width: u32, x: u32, y: u32) -> f32 {
    let offset = ((y * width + x) * 4) as usize;
    let r = rgba[offset] as f32;
    let g = rgba[offset + 1] as f32;
    let b = rgba[offset + 2] as f32;
    r * 0.2126 + g * 0.7152 + b * 0.0722
}

/// Verifies that loading an HDR file populates the cubemap path with
/// real per-pixel face data derived from the equirectangular projection,
/// not just a 6-color face-center summary. Before this work the HDR
/// loader threw away pixels after computing the average irradiance, so
/// the prefiltered specular cubemap downstream had no high-contrast
/// content to reflect.
#[test]
fn hdr_environment_produces_per_pixel_cubemap_radiance() {
    // Synthetic equirectangular HDR: 16×8 image, bright at the centre
    // column (which maps to forward (+Z) direction), dim elsewhere.
    // RGBE encoding: bright = (255, 255, 255, 133) ≈ linear 32x white;
    // dim = (32, 32, 32, 128) ≈ linear 0.125 white.
    let mut pixels = vec![[32_u8, 32, 32, 128]; 16 * 8];
    let centre_column = 8;
    for y in 0..8 {
        pixels[y * 16 + centre_column] = [255, 255, 255, 133];
    }
    let bytes = uncompressed_radiance_hdr(16, 8, &pixels);
    let path = AssetPath::from("memory://rle-fixture/forward-bright.hdr");
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![(path.clone(), bytes)]));
    let environment =
        pollster::block_on(assets.load_environment(path.as_str())).expect("synthetic HDR loads");
    let desc = assets
        .environment(environment)
        .expect("environment present");
    let faces = desc
        .cubemap_faces()
        .expect("HDR environment produces cubemap faces");
    let face_pixels = faces.build_face_pixels_rgba32f();

    // Face index 4 = +Z. Centre pixel should be bright; +X, -X, -Z faces
    // should be dim by comparison.
    let face_res = faces.resolution() as usize;
    let centre = (face_res / 2 * face_res + face_res / 2) * 4;
    let pz_centre = face_pixels[4][centre];
    let nx_centre = face_pixels[1][centre];
    let nz_centre = face_pixels[5][centre];
    eprintln!("+Z centre={pz_centre}, -X centre={nx_centre}, -Z centre={nz_centre}");
    assert!(
        pz_centre > 5.0,
        "+Z face centre should sample the bright forward-direction radiance, got {pz_centre}"
    );
    assert!(
        pz_centre > nx_centre * 4.0 && pz_centre > nz_centre * 4.0,
        "+Z face must be much brighter than side/back faces (\
         pz={pz_centre} nx={nx_centre} nz={nz_centre})"
    );
}

fn uncompressed_radiance_hdr(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(pixels.len(), (width * height) as usize);
    let mut bytes =
        format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
    for pixel in pixels {
        bytes.extend_from_slice(pixel);
    }
    bytes
}
