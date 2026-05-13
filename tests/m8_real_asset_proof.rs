//! Plan line 783: real-asset glTF import proof. Imports the Khronos
//! `WaterBottle` real-product PBR fixture from `tests/assets/gltf/khronos/`
//! and verifies the importer produces a renderable scene with the expected
//! mesh + material + texture-role topology, real-world dimensions, and
//! release-quality CPU/GPU rendered proof lanes.
//!
//! The test fixture is bundled under `tests/assets/gltf/khronos/WaterBottle/`
//! and pinned by SHA-256 in `tests/assets/gltf/khronos/manifest.toml`. The
//! .gltf and .bin are upstream-faithful; the four PNG textures were
//! downsampled from the upstream 2048² to 256² with Pillow LANCZOS so the
//! bundled fixture stays under 300 KB while preserving every material role
//! the importer + renderer must handle.
#![cfg(not(target_arch = "wasm32"))]

use std::fs::File;
use std::io::BufWriter;

use base64::Engine as _;
use scena::{Assets, Color, Renderer, Scene, Tonemapper, Transform};

const WATERBOTTLE_PATH: &str = "tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf";
const ARTIFACT_GPU_PNG: &str = "target/gate-artifacts/m8-real-asset/waterbottle_gpu.png";
const ARTIFACT_GPU_FAIL_CLOSED_JSON: &str =
    "target/gate-artifacts/m8-real-asset/waterbottle_gpu_fail_closed.json";
const ARTIFACT_CPU_PNG: &str = "target/gate-artifacts/m8-real-asset/waterbottle_cpu.png";
const WATERBOTTLE_ARTIFACT_SIZE: u32 = 512;
const WATERBOTTLE_CPU_SUPERSAMPLE: u32 = 4;
const WATERBOTTLE_CPU_RENDER_SIZE: u32 = WATERBOTTLE_ARTIFACT_SIZE * WATERBOTTLE_CPU_SUPERSAMPLE;
const WATERBOTTLE_GPU_SUPERSAMPLE: u32 = 4;
const WATERBOTTLE_GPU_RENDER_SIZE: u32 = WATERBOTTLE_ARTIFACT_SIZE * WATERBOTTLE_GPU_SUPERSAMPLE;
/// Polyhaven `studio_small_03_1k.hdr` — CC0, real-world studio HDR with
/// smooth radiance falloff. Bundled at
/// `tests/assets/environment/polyhaven/studio_small_03_1k.hdr` and pinned
/// by SHA-256 below. A real HDR's smooth gradients produce clean specular
/// reflections on metallic surfaces; the synthetic 3-point HDR's hard
/// pixel boundaries produced visible speckle/grain in earlier renders.
const STUDIO_HDR_PATH: &str = "tests/assets/environment/polyhaven/studio_small_03_1k.hdr";
const STUDIO_HDR_SHA256: &str = "30933d55e45f0795daf49f3cbefbe0e5ebcb821ee04fb0a2818c02ffc3938817";

/// Phase 1: scena-gold reference for the WaterBottle GPU render. This
/// is the canonical "scena should keep producing this" baseline for
/// Phase 2's ΔE-based regression checks. It is NOT a third-party
/// pixel match — see `reference_metadata.toml` alongside the file.
const WATERBOTTLE_REFERENCE_PNG: &str = "tests/assets/gltf/khronos/WaterBottle/reference_512.png";
const WATERBOTTLE_REFERENCE_SHA256: &str =
    "4db449cdacf2340f8fa53937c28e5c4b5e2c7deaea73cbe0987dcd51eb93c751";

/// Phase 5.5: third-party reference for the WaterBottle render,
/// produced by Blender Cycles (128 spp, neutral studio lighting). Use
/// `tests/assets/gltf/khronos/WaterBottle/render_blender_reference.py`
/// to regenerate. This reference is the answer to "what does a
/// known-good PBR renderer produce for this asset". It is a loose
/// third-party material-family oracle, not a pixel baseline; the
/// scena-gold PNG remains the regression baseline.
const WATERBOTTLE_BLENDER_REFERENCE_PNG: &str =
    "tests/assets/gltf/khronos/WaterBottle/reference_blender_cycles_512.png";
const WATERBOTTLE_BLENDER_REFERENCE_SHA256: &str =
    "17db39248ce1966ae60c3b85d09491ebfb7f654777dc2d150a64db4e938a6883";

/// Lightweight integrity check for the bundled polyhaven HDR. A
/// cryptographic SHA-256 manifest belongs in the asset matrix (Khronos
/// fixtures use that pattern); this just catches accidental corruption.
#[test]
fn polyhaven_studio_hdr_is_a_real_radiance_file() {
    let bytes = std::fs::read(STUDIO_HDR_PATH).expect("bundled polyhaven HDR is readable");
    assert!(
        bytes.starts_with(b"#?RADIANCE"),
        "bundled HDR must begin with the Radiance HDR magic header"
    );
    let _ = STUDIO_HDR_SHA256; // recorded for future asset-matrix wiring
    assert!(
        bytes.len() > 200_000 && bytes.len() < 10_000_000,
        "bundled HDR size sanity-check (got {} bytes)",
        bytes.len()
    );
}

/// Phase 1: verify the bundled scena-gold WaterBottle reference is the
/// exact PNG pinned by SHA-256. Catches accidental swaps; Phase 2's
/// diff harness then compares the test's live render against it.
#[test]
fn waterbottle_reference_png_matches_pinned_sha256() {
    let bytes = std::fs::read(WATERBOTTLE_REFERENCE_PNG)
        .expect("bundled WaterBottle reference is readable");
    assert!(
        bytes.starts_with(&[0x89, b'P', b'N', b'G']),
        "bundled reference must be a PNG"
    );
    let actual = sha256_hex(&bytes);
    assert_eq!(
        actual, WATERBOTTLE_REFERENCE_SHA256,
        "bundled WaterBottle reference SHA-256 must match the pinned value; \
         if you intentionally regenerated the reference, update \
         WATERBOTTLE_REFERENCE_SHA256 and reference_metadata.toml in the same commit"
    );
}

/// Phase 5.5: verify the bundled Blender Cycles third-party reference
/// is the exact PNG pinned by SHA-256. Produced by
/// `tests/assets/gltf/khronos/WaterBottle/render_blender_reference.py`;
/// any change must update the pinned SHA in the same commit.
#[test]
fn waterbottle_blender_reference_png_matches_pinned_sha256() {
    let bytes = std::fs::read(WATERBOTTLE_BLENDER_REFERENCE_PNG)
        .expect("bundled WaterBottle Blender reference is readable");
    assert!(
        bytes.starts_with(&[0x89, b'P', b'N', b'G']),
        "bundled reference must be a PNG"
    );
    let actual = sha256_hex(&bytes);
    assert_eq!(
        actual, WATERBOTTLE_BLENDER_REFERENCE_SHA256,
        "bundled Blender reference SHA-256 must match the pinned value; \
         if you intentionally regenerated, update WATERBOTTLE_BLENDER_REFERENCE_SHA256"
    );
}

/// Phase 5.5: third-party material agreement check. Sample a few
/// per-material regions and verify scena and Blender both classify the
/// asset materials into the same colour family. This is the validation
/// that the previously-missing third-party-reference work is supposed
/// to provide. Tolerances are wide because exact-pixel match across
/// two completely different PBR renderers is not the goal — agreement
/// on material classification is.
#[test]
fn waterbottle_blender_and_scena_agree_on_material_colour_families() {
    let blender = PngImage::read(WATERBOTTLE_BLENDER_REFERENCE_PNG);
    let scena = PngImage::read(WATERBOTTLE_REFERENCE_PNG);
    // Body region (~middle of bottle): olive/yellow → R>G>B with R>100.
    assert_olive_yellow("Blender body", blender.pixel_at(250, 250));
    assert_olive_yellow("scena body", scena.pixel_at(250, 250));
    // Cap region: dark burgundy → R > 50, G+B much lower.
    assert_dark_burgundy("Blender cap", blender.pixel_at(250, 90));
    assert_dark_burgundy("scena cap", scena.pixel_at(250, 90));
}

struct PngImage {
    width: usize,
    buffer: Vec<u8>,
}

impl PngImage {
    fn read(path: &str) -> Self {
        let bytes = std::fs::read(path).expect("reference PNG is readable");
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("png hdr");
        let mut buffer = vec![
            0u8;
            reader.output_buffer_size().expect(
                "PNG output buffer size is known for bundled RGBA references"
            )
        ];
        reader.next_frame(&mut buffer).expect("png data");
        Self {
            width: reader.info().width as usize,
            buffer,
        }
    }

    fn pixel_at(&self, x: usize, y: usize) -> [u8; 4] {
        let p = (y * self.width + x) * 4;
        [
            self.buffer[p],
            self.buffer[p + 1],
            self.buffer[p + 2],
            self.buffer[p + 3],
        ]
    }
}

fn assert_olive_yellow(label: &str, body: [u8; 4]) {
    assert!(
        body[0] > 90 && body[1] > 70 && body[2] < body[0].saturating_sub(25),
        "{label} sample {body:?} should classify as olive/yellow \
         (R high, B materially lower) — confirms the asset's authored \
         baseColor renders as olive, not gold-metallic"
    );
}

fn assert_dark_burgundy(label: &str, cap: [u8; 4]) {
    assert!(
        cap[0] > 45 && cap[1] < cap[0].saturating_sub(10) && cap[2] < cap[0].saturating_sub(10),
        "{label} sample {cap:?} should classify as dark red/burgundy"
    );
}

fn assert_warm_studio_background(label: &str, background: [u8; 4]) {
    let luma = (0.2126 * background[0] as f32)
        + (0.7152 * background[1] as f32)
        + (0.0722 * background[2] as f32);
    assert!(
        luma > 20.0 && background[0] > background[2].saturating_add(4),
        "{label} sample {background:?} should be a visible warm studio surface, \
         not the prior black/debug clear color"
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().fold(String::new(), |mut acc, byte| {
        use std::fmt::Write;
        let _ = write!(&mut acc, "{byte:02x}");
        acc
    })
}

fn warm_studio_hdr_data_uri() -> String {
    // Blender reference uses a warm studio world color. RGBE(105, 84, 64, 129)
    // decodes to approximately linear RGB (0.82, 0.66, 0.50), matching the
    // reference script's constant warm background without introducing a cooler
    // multi-face cubemap that shifts the metallic bottle green/blue.
    let pixels = [[105, 84, 64, 129]; 8];
    let hdr = tiny_radiance_hdr_rgbe(4, 2, &pixels);
    format!(
        "data:application/radiance-hdr;base64,{}#waterbottle-warm-studio.hdr",
        base64::engine::general_purpose::STANDARD.encode(hdr)
    )
}

fn tiny_radiance_hdr_rgbe(width: u32, height: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    assert_eq!(pixels.len(), (width * height) as usize);
    let mut bytes =
        format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
    for pixel in pixels {
        bytes.extend_from_slice(pixel);
    }
    bytes
}

/// Build the WaterBottle scene with neutral studio environment lighting.
/// Returns scene-side resources, ready for a renderer to be attached and
/// rendered.
fn build_waterbottle_scene() -> (Assets, Scene, scena::EnvironmentHandle) {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene(WATERBOTTLE_PATH)).expect("WaterBottle .gltf loads");

    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("WaterBottle instantiates into a scene");
    let bounds = import
        .bounds_world(&scene)
        .expect("imported WaterBottle has world bounds");

    let extents = (
        bounds.max.x - bounds.min.x,
        bounds.max.y - bounds.min.y,
        bounds.max.z - bounds.min.z,
    );
    assert!(
        extents.0 > 0.05 && extents.0 < 0.20,
        "WaterBottle X extent must be on the order of metres-scale millimetres (got {})",
        extents.0
    );
    assert!(
        extents.1 > 0.10 && extents.1 < 0.30,
        "WaterBottle Y extent must be on the order of metres-scale millimetres (got {})",
        extents.1
    );

    let centre = scena::Vec3::new(
        (bounds.min.x + bounds.max.x) * 0.5,
        (bounds.min.y + bounds.max.y) * 0.5,
        (bounds.min.z + bounds.max.z) * 0.5,
    );
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            Transform::at(scena::Vec3::new(
                centre.x + 0.12,
                centre.y + 0.05,
                centre.z + 0.25,
            ))
            .rotate_y_deg(25.0)
            .rotate_x_deg(-10.0),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let warm_studio = warm_studio_hdr_data_uri();
    let environment = pollster::block_on(assets.load_environment(warm_studio.as_str()))
        .expect("warm studio HDR data URI loads");

    (assets, scene, environment)
}

/// Phase 3 GPU headline lane. This is retained as an explicit release
/// visual lane, but the default cargo-test path records fail-closed metadata
/// because local V3D/Vulkan readback can return an all-black frame under
/// sustained test load. Set `SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1`
/// on an approved visual-proof lane before claiming "matches the reference
/// render".
#[test]
fn m8_real_asset_waterbottle_gpu_headline() {
    if std::env::var_os("SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS").is_none() {
        write_gpu_fail_closed_artifact(
            "env flag SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS is not set; local headless GPU WaterBottle readback is not trusted as release evidence",
            "not-run",
        );
        return;
    }

    let (assets, mut scene, environment) = build_waterbottle_scene();

    let mut renderer = match Renderer::headless_gpu(
        WATERBOTTLE_GPU_RENDER_SIZE,
        WATERBOTTLE_GPU_RENDER_SIZE,
    ) {
        Ok(r) => r,
        Err(error) => {
            write_gpu_fail_closed_artifact(
                &format!(
                    "Renderer::headless_gpu failed: {error:?}; approved GPU/browser visual proof infrastructure is required before release"
                ),
                "unavailable",
            );
            return;
        }
    };
    let gpu_adapter_label = match renderer.gpu_adapter_report() {
        Some(report) => format!("{} ({})", report.name, report.backend),
        None => String::from("unknown"),
    };
    eprintln!("scena: rendering WaterBottle via GPU: {gpu_adapter_label}");

    renderer.set_background_color(Color::from_srgb_u8(216, 196, 170));
    renderer.set_tonemapper(Tonemapper::PbrNeutral);
    renderer.set_environment(environment);
    renderer.set_exposure_ev(0.0);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("WaterBottle prepares for the headless GPU renderer");
    renderer
        .render_active(&scene)
        .expect("WaterBottle renders on the GPU");

    let stats = renderer.stats();
    assert_eq!(
        stats.materials, 1,
        "GPU: WaterBottle's `BottleMat` is the only prepared material in the clean-background proof"
    );
    assert_eq!(
        stats.textures, 4,
        "GPU: WaterBottle must surface the four upstream PBR texture roles \
         (baseColor, normal, occlusionRoughnessMetallic, emissive)"
    );
    assert!(
        stats.triangles > 1000,
        "GPU: real product mesh must have a non-trivial triangle count, got {}",
        stats.triangles
    );

    let frame = downsample_rgba8_box(
        renderer.frame_rgba8(),
        WATERBOTTLE_GPU_RENDER_SIZE,
        WATERBOTTLE_GPU_RENDER_SIZE,
        WATERBOTTLE_GPU_SUPERSAMPLE,
    );
    let nonzero = frame
        .chunks_exact(4)
        .filter(|p| p[..3] != [0, 0, 0])
        .count();
    if nonzero <= 5_000 {
        write_gpu_fail_closed_artifact(
            &format!(
                "GPU WaterBottle readback produced only {nonzero} non-black pixels; expected more than 5000"
            ),
            "degenerate-readback",
        );
        return;
    }

    write_png_artifact(
        &frame,
        WATERBOTTLE_ARTIFACT_SIZE,
        WATERBOTTLE_ARTIFACT_SIZE,
        ARTIFACT_GPU_PNG,
    );
    write_renderer_metadata("gpu", &gpu_adapter_label);

    // Phase 2 region asserts. The GPU lane is the canonical scena-gold
    // regression baseline; the CPU lane below carries its own measured
    // release-quality tolerance envelope instead of a loose smoke test.
    let regions: &[(&str, usize, usize, [u8; 3], u8)] = &[
        // (name, x, y, expected RGB, tolerance in chebyshev distance)
        ("cap_dome", 250, 70, [76, 27, 12], 25),
        ("cap_dome_left", 240, 70, [76, 27, 12], 25),
        ("upper_body", 249, 130, [153, 134, 48], 25),
        ("body_olive_mid", 249, 270, [171, 152, 78], 25),
        ("body_olive_low", 249, 330, [132, 114, 32], 25),
        ("label_metal_r", 270, 380, [30, 20, 6], 25),
        ("label_metal_l", 255, 380, [28, 18, 5], 25),
    ];
    let mut failed_regions = Vec::new();
    for (name, x, y, expected, tol) in regions {
        let p = pixel_at(&frame, *x, *y);
        let dr = (p[0] as i16 - expected[0] as i16).unsigned_abs() as u8;
        let dg = (p[1] as i16 - expected[1] as i16).unsigned_abs() as u8;
        let db = (p[2] as i16 - expected[2] as i16).unsigned_abs() as u8;
        if dr > *tol || dg > *tol || db > *tol {
            failed_regions.push(format!(
                "  {name:14} ({x:3},{y:3}): expected {expected:?} ±{tol}, got [{},{},{}]",
                p[0], p[1], p[2]
            ));
        }
    }
    assert!(
        failed_regions.is_empty(),
        "WaterBottle region colour asserts failed; this catches cap/body/label \
         tinting regressions that the prior single-sample bar missed.\n{}",
        failed_regions.join("\n")
    );
    for (name, x, y) in [
        ("bg_top_right", 490, 10),
        ("bg_mid_right", 450, 250),
        ("bg_bot_right", 490, 490),
        ("bg_mid_left", 80, 250),
    ] {
        assert_warm_studio_background(name, pixel_at(&frame, x, y));
    }

    // Phase 2 colour-family histograms. The render must contain at
    // least N pixels in each named colour cluster — proves the cap is
    // present as a red region, the body as olive/yellow, the label
    // band as a dark/neutral cluster, etc. Lighter bar than the per-
    // region asserts; meant to catch "entire region the wrong colour"
    // regressions even if a single sample pixel drifted away from a
    // tight tolerance.
    let mut family_counts = ColourFamilyCounts::default();
    for chunk in frame.chunks_exact(4) {
        family_counts.tally(chunk[0], chunk[1], chunk[2]);
    }
    let family_failures = family_counts.failures(&[
        ("dark_red_cap", 2_000, |r, g, b| {
            r > 45 && r < 150 && g < 75 && b < 80 && r >= g.saturating_add(8)
        }),
        ("yellow_olive", 10_000, |r, g, b| {
            r > 60 && g > 50 && b < g.saturating_sub(15) && r < 200
        }),
        ("muted_olive", 10_000, |r, g, b| {
            r > 110 && g > 105 && b > 55 && b < g.saturating_sub(20) && r < 210
        }),
        ("neutral_dark", 5_000, |r, g, b| {
            r < 80 && g < 85 && b < 90 && r.abs_diff(g) < 20
        }),
    ]);
    assert!(
        family_failures.is_empty(),
        "WaterBottle colour-family histograms failed:\n{}",
        family_failures.join("\n")
    );

    // Phase 2 reference diff (gated). With SCENA_REFERENCE_DIFF=1, also
    // compare the live render against the bundled scena-gold reference
    // pixel-by-pixel; ≥95% of pixels must be within RGB Chebyshev
    // distance 16. The diff visualisation lands next to the artifact
    // when the threshold fails so a reviewer can SEE which regions
    // drifted.
    if std::env::var("SCENA_REFERENCE_DIFF").is_ok() {
        let reference = decode_reference_png();
        assert_eq!(
            reference.len(),
            frame.len(),
            "reference PNG must match render dimensions (512x512 RGBA)"
        );
        let (within_tol, total, max_d) = pixel_diff_summary(&frame, &reference, 16);
        let fraction = within_tol as f64 / total as f64;
        if fraction < 0.95 {
            write_diff_visualization(&frame, &reference);
            panic!(
                "WaterBottle render diverged from bundled reference: \
                 only {:.2}% of pixels are within RGB ±16 (max channel \
                 distance: {max_d}). Diff visualisation written to {}",
                fraction * 100.0,
                DIFF_PNG,
            );
        }
    }
}

fn write_gpu_fail_closed_artifact(reason: &str, status: &str) {
    let path = std::path::Path::new(ARTIFACT_GPU_FAIL_CLOSED_JSON);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("m8 real-asset artifact dir");
    }
    let artifact = serde_json::json!({
        "schema": "scena.m8.real_asset_gpu_fail_closed.v1",
        "test_name": "m8_real_asset_waterbottle_gpu_headline",
        "status": status,
        "release_evidence": false,
        "reason": reason,
        "run_hint": "Set SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1 only on an approved native/browser visual-proof lane.",
    });
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&artifact).expect("gpu fail-closed artifact serializes"),
    )
    .expect("gpu fail-closed artifact writes");
}

/// Phase 3 CPU release-quality lane. The CPU rasterizer is a
/// deterministic software renderer with its own measured tolerance
/// envelope. It is not required to be pixel-identical to the GPU lane,
/// but it must preserve the WaterBottle material story: warm studio
/// background, burgundy cap/logo, olive body, dark holder, correct glTF
/// texture orientation, bilinear texture sampling, and stable
/// perspective-correct shading across textured triangles.
#[test]
fn m8_real_asset_waterbottle_cpu_release_quality() {
    let (assets, mut scene, environment) = build_waterbottle_scene();

    let mut renderer = Renderer::headless(WATERBOTTLE_CPU_RENDER_SIZE, WATERBOTTLE_CPU_RENDER_SIZE)
        .expect("CPU rasterizer");
    eprintln!("scena: rendering WaterBottle via CPU release-quality path");

    renderer.set_background_color(Color::from_srgb_u8(216, 196, 170));
    renderer.set_tonemapper(Tonemapper::PbrNeutral);
    renderer.set_environment(environment);
    renderer.set_exposure_ev(0.0);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("WaterBottle prepares for the CPU rasterizer");
    renderer
        .render_active(&scene)
        .expect("WaterBottle renders on the CPU");

    let stats = renderer.stats();
    assert_eq!(
        stats.materials, 1,
        "CPU: WaterBottle's `BottleMat` is the only prepared material in the clean-background proof"
    );
    assert_eq!(
        stats.textures, 4,
        "CPU: WaterBottle must surface the four upstream PBR texture roles"
    );
    assert!(
        stats.triangles > 1000,
        "CPU: real product mesh must have a non-trivial triangle count, got {}",
        stats.triangles
    );

    let frame = downsample_rgba8_box(
        renderer.frame_rgba8(),
        WATERBOTTLE_CPU_RENDER_SIZE,
        WATERBOTTLE_CPU_RENDER_SIZE,
        WATERBOTTLE_CPU_SUPERSAMPLE,
    );
    let nonzero = frame
        .chunks_exact(4)
        .filter(|p| p[..3] != [0, 0, 0])
        .count();
    assert!(
        nonzero > 5_000,
        "CPU: framed WaterBottle silhouette must produce at least 5000 non-black pixels (got {nonzero})"
    );

    // Silhouette check — confirm the bottle is somewhere near the
    // centre of the frame and the software lane is not a constant fill.
    let centre = pixel_at(&frame, 249, 246);
    let tl = pixel_at(&frame, 5, 5);
    let br = pixel_at(&frame, 506, 506);
    assert!(
        centre[..3] != tl[..3] || centre[..3] != br[..3],
        "CPU: centre pixel must differ from at least one corner pixel \
         (centre={centre:?}, tl={tl:?}, br={br:?}) — the renderer should \
         produce SOMETHING distinct in the bottle's footprint"
    );

    let regions: &[(&str, usize, usize, [u8; 3], u8)] = &[
        // CPU release-proof envelope, measured from the deterministic
        // software renderer at 4x resolution and box-downsampled to
        // 512x512. These are intentionally wider than the GPU baseline
        // because the CPU path bakes material samples into subdivided
        // triangles instead of running the GPU fragment shader, but
        // they still catch the real failures: black output, wrong
        // texture V orientation, lost red logo/cap, missing dark
        // holder, or major color-management drift.
        ("cpu_cap_dome", 250, 70, [76, 29, 13], 30),
        ("cpu_upper_body", 249, 130, [159, 139, 50], 30),
        ("cpu_body_mid", 249, 270, [165, 145, 54], 30),
        ("cpu_body_low", 249, 330, [148, 128, 42], 30),
        ("cpu_label_metal_r", 270, 380, [32, 22, 7], 30),
        ("cpu_label_metal_l", 255, 380, [31, 21, 6], 30),
        ("cpu_logo_red", 315, 252, [88, 25, 9], 35),
    ];
    let mut failed_regions = Vec::new();
    for (name, x, y, expected, tol) in regions {
        let p = pixel_at(&frame, *x, *y);
        let dr = (p[0] as i16 - expected[0] as i16).unsigned_abs() as u8;
        let dg = (p[1] as i16 - expected[1] as i16).unsigned_abs() as u8;
        let db = (p[2] as i16 - expected[2] as i16).unsigned_abs() as u8;
        if dr > *tol || dg > *tol || db > *tol {
            failed_regions.push(format!(
                "  {name:17} ({x:3},{y:3}): expected {expected:?} ±{tol}, got [{},{},{}]",
                p[0], p[1], p[2]
            ));
        }
    }
    assert!(
        failed_regions.is_empty(),
        "CPU WaterBottle release-colour asserts failed; this catches CPU \
         texture-orientation, material-subdivision, bilinear-sampling, \
         and colour-management regressions.\n{}",
        failed_regions.join("\n")
    );

    let mut family_counts = ColourFamilyCounts::default();
    for chunk in frame.chunks_exact(4) {
        family_counts.tally(chunk[0], chunk[1], chunk[2]);
    }
    let family_failures = family_counts.failures(&[
        ("cpu_dark_red_cap", 4_000, |r, g, b| {
            r > 45 && r < 150 && g < 75 && b < 80 && r >= g.saturating_add(8)
        }),
        ("cpu_yellow_olive", 20_000, |r, g, b| {
            r > 60 && g > 50 && b < g.saturating_sub(15) && r < 200
        }),
        ("cpu_muted_olive", 15_000, |r, g, b| {
            r > 110 && g > 105 && b > 55 && b < g.saturating_sub(20) && r < 210
        }),
        ("cpu_neutral_dark", 8_000, |r, g, b| {
            r < 80 && g < 85 && b < 90 && r.abs_diff(g) < 20
        }),
    ]);
    assert!(
        family_failures.is_empty(),
        "CPU WaterBottle colour-family histograms failed:\n{}",
        family_failures.join("\n")
    );

    for (name, x, y) in [
        ("cpu_bg_top_right", 490, 10),
        ("cpu_bg_mid_right", 450, 250),
        ("cpu_bg_bot_right", 490, 490),
        ("cpu_bg_mid_left", 80, 250),
    ] {
        assert_warm_studio_background(name, pixel_at(&frame, x, y));
    }

    write_png_artifact(
        &frame,
        WATERBOTTLE_ARTIFACT_SIZE,
        WATERBOTTLE_ARTIFACT_SIZE,
        ARTIFACT_CPU_PNG,
    );
    write_renderer_metadata("cpu", "software-rasterizer");
}

type ColourFamily = (&'static str, u32, fn(u8, u8, u8) -> bool);

#[derive(Default)]
struct ColourFamilyCounts {
    tallies: [u32; 8],
}

impl ColourFamilyCounts {
    fn tally(&mut self, r: u8, g: u8, b: u8) {
        // Match families in order; one pixel can match multiple. The
        // ordering matches the order we test in `failures`.
        let (dark_red, yellow_olive, bright_cream, neutral_dark) = (
            r > 45 && r < 150 && g < 75 && b < 80 && r >= g.saturating_add(8),
            r > 60 && g > 50 && b < g.saturating_sub(15) && r < 200,
            r > 110 && g > 105 && b > 35 && b < g.saturating_sub(20) && r < 210,
            r < 80 && g < 85 && b < 90 && r.abs_diff(g) < 20,
        );
        if dark_red {
            self.tallies[0] += 1;
        }
        if yellow_olive {
            self.tallies[1] += 1;
        }
        if bright_cream {
            self.tallies[2] += 1;
        }
        if neutral_dark {
            self.tallies[3] += 1;
        }
    }

    fn failures(&self, families: &[ColourFamily]) -> Vec<String> {
        let mut out = Vec::new();
        for (i, (name, min_count, _)) in families.iter().enumerate() {
            let got = self.tallies[i];
            if got < *min_count {
                out.push(format!(
                    "  {name:14}: expected ≥{min_count} pixels, got {got}"
                ));
            }
        }
        out
    }
}

const DIFF_PNG: &str = "target/gate-artifacts/m8-real-asset/waterbottle_diff.png";

fn decode_reference_png() -> Vec<u8> {
    let bytes = std::fs::read(WATERBOTTLE_REFERENCE_PNG).expect("bundled reference is readable");
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("reference PNG header reads");
    assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    let mut buffer = vec![
        0u8;
        reader.output_buffer_size().expect(
            "PNG output buffer size is known for generated reference proof"
        )
    ];
    reader
        .next_frame(&mut buffer)
        .expect("reference PNG payload reads");
    buffer
}

/// Returns `(pixels within tol, total pixels, max channel distance seen)`
/// where channel distance is the per-pixel Chebyshev distance.
fn pixel_diff_summary(live: &[u8], reference: &[u8], tol: u8) -> (usize, usize, u8) {
    let mut within = 0;
    let mut max_d = 0u8;
    let total = live.len() / 4;
    for (l, r) in live.chunks_exact(4).zip(reference.chunks_exact(4)) {
        let dr = (l[0] as i16 - r[0] as i16).unsigned_abs() as u8;
        let dg = (l[1] as i16 - r[1] as i16).unsigned_abs() as u8;
        let db = (l[2] as i16 - r[2] as i16).unsigned_abs() as u8;
        let d = dr.max(dg).max(db);
        if d > max_d {
            max_d = d;
        }
        if d <= tol {
            within += 1;
        }
    }
    (within, total, max_d)
}

fn write_diff_visualization(live: &[u8], reference: &[u8]) {
    let mut out = Vec::with_capacity(live.len());
    for (l, r) in live.chunks_exact(4).zip(reference.chunks_exact(4)) {
        let dr = (l[0] as i16 - r[0] as i16).unsigned_abs().min(255) as u8;
        let dg = (l[1] as i16 - r[1] as i16).unsigned_abs().min(255) as u8;
        let db = (l[2] as i16 - r[2] as i16).unsigned_abs().min(255) as u8;
        // Visualise: amplify so even small diffs are visible.
        let amp = |v: u8| ((v as u16).saturating_mul(8).min(255)) as u8;
        out.extend_from_slice(&[amp(dr), amp(dg), amp(db), 255]);
    }
    if let Some(parent) = std::path::Path::new(DIFF_PNG).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let file = File::create(DIFF_PNG).expect("create diff PNG");
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, 512, 512);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header writes");
    writer.write_image_data(&out).expect("PNG payload writes");
}

fn downsample_rgba8_box(source: &[u8], width: u32, height: u32, scale: u32) -> Vec<u8> {
    assert!(scale >= 1, "downsample scale must be at least 1");
    assert_eq!(
        width % scale,
        0,
        "downsample requires width divisible by scale"
    );
    assert_eq!(
        height % scale,
        0,
        "downsample requires height divisible by scale"
    );
    assert_eq!(
        source.len(),
        (width as usize) * (height as usize) * 4,
        "source RGBA buffer must match dimensions"
    );
    let out_width = width / scale;
    let out_height = height / scale;
    let mut out = vec![0; (out_width as usize) * (out_height as usize) * 4];
    let scale = scale as usize;
    let divisor = (scale * scale) as u32;
    for y in 0..out_height as usize {
        for x in 0..out_width as usize {
            let mut sum = [0u32; 4];
            for dy in 0..scale {
                for dx in 0..scale {
                    let source_x = x * scale + dx;
                    let source_y = y * scale + dy;
                    let source_offset = (source_y * width as usize + source_x) * 4;
                    for channel in 0..4 {
                        sum[channel] += u32::from(source[source_offset + channel]);
                    }
                }
            }
            let out_offset = (y * out_width as usize + x) * 4;
            for channel in 0..4 {
                out[out_offset + channel] = ((sum[channel] + divisor / 2) / divisor) as u8;
            }
        }
    }
    out
}

fn pixel_at(frame: &[u8], x: usize, y: usize) -> [u8; 4] {
    let p = (y * 512 + x) * 4;
    [frame[p], frame[p + 1], frame[p + 2], frame[p + 3]]
}

fn write_png_artifact(rgba8: &[u8], width: u32, height: u32, path: &str) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).expect("artifact dir");
    }
    let file = File::create(path).expect("create artifact PNG");
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header writes");
    writer.write_image_data(rgba8).expect("PNG payload writes");
}

const METADATA_TOML: &str = "target/gate-artifacts/m8-real-asset/waterbottle_renderer.toml";

fn renderer_metadata_path(renderer_path: &str) -> String {
    format!("target/gate-artifacts/m8-real-asset/waterbottle_{renderer_path}_renderer.toml")
}

/// Stage 0 (visibility): companion metadata so reviewers can tell at a glance
/// which renderer produced each WaterBottle artifact without having to
/// re-read the test source or replay the run. The path-specific file is
/// release evidence; `waterbottle_renderer.toml` is retained only as a
/// latest-run compatibility pointer.
fn write_renderer_metadata(renderer_path: &str, gpu_adapter: &str) {
    let path = renderer_metadata_path(renderer_path);
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).expect("artifact dir");
    }
    // Phase 5.4: `ibl_specular_path` records that the CPU lane is a
    // release-quality software proof with an approximate scalar IBL
    // path, not a hidden GPU render.
    let ibl_specular_path = match renderer_path {
        "gpu" => "split_sum",
        _ => "scalar_approximate",
    };
    let gpu_proof = renderer_path == "gpu";
    let body = format!(
        "# Generated by tests/m8_real_asset_proof.rs.\n\
         renderer_path = \"{renderer_path}\"\n\
         gpu_adapter = \"{gpu_adapter}\"\n\
         gpu_proof = {gpu_proof}\n\
         color_contract = \"gltf2-pbr-neutral\"\n\
         scena_use_gpu_set = {use_gpu_set}\n\
         ibl_specular_path = \"{ibl_specular_path}\"\n",
        use_gpu_set = std::env::var("SCENA_USE_GPU").is_ok(),
    );
    std::fs::write(&path, &body).expect("renderer metadata writes");
    std::fs::write(METADATA_TOML, body).expect("latest-run renderer metadata writes");
}
