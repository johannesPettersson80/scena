//! Regression tests for Radiance HDR loading, specifically the RLE-encoded
//! scanline path that scena's hand-rolled decoder did not handle. Most
//! real-world HDRs (polyhaven, HDRI Haven) use RLE compression.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::future::{Ready, ready};

use scena::{AssetError, AssetFetcher, AssetPath, Assets};

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
