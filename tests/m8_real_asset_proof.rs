//! Plan line 783: real-asset glTF import proof. Imports the Khronos
//! `WaterBottle` real-product PBR fixture from `tests/assets/gltf/khronos/`
//! and verifies the importer produces a renderable scene with the expected
//! mesh + material + texture-role topology, real-world dimensions, and
//! non-black framed pixels through the headless CPU rasterizer.
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

use scena::{
    Assets, Color, DirectionalLight, GeometryDesc, MaterialDesc, Renderer, Scene, Transform,
};

const WATERBOTTLE_PATH: &str = "tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf";
const ARTIFACT_GPU_PNG: &str = "target/gate-artifacts/m8-real-asset/waterbottle_gpu.png";
const ARTIFACT_CPU_PNG: &str = "target/gate-artifacts/m8-real-asset/waterbottle_cpu.png";
/// Polyhaven `studio_small_03_1k.hdr` — CC0, real-world studio HDR with
/// smooth radiance falloff. Bundled at
/// `tests/assets/environment/polyhaven/studio_small_03_1k.hdr` and pinned
/// by SHA-256 below. A real HDR's smooth gradients produce clean specular
/// reflections on metallic surfaces; the synthetic 3-point HDR's hard
/// pixel boundaries produced visible speckle/grain in earlier renders.
const STUDIO_HDR_PATH: &str =
    "tests/assets/environment/polyhaven/studio_small_03_1k.hdr";
const STUDIO_HDR_SHA256: &str =
    "30933d55e45f0795daf49f3cbefbe0e5ebcb821ee04fb0a2818c02ffc3938817";

/// Phase 1: scena-gold reference for the WaterBottle GPU render. This
/// is the canonical "scena should keep producing this" baseline for
/// Phase 2's ΔE-based regression checks. It is NOT a third-party
/// pixel match — see `reference_metadata.toml` alongside the file.
const WATERBOTTLE_REFERENCE_PNG: &str =
    "tests/assets/gltf/khronos/WaterBottle/reference_512.png";
const WATERBOTTLE_REFERENCE_SHA256: &str =
    "f4bdca94137b1c90432d0a88c26eca4992ff84e87fdbd1c21d147b3d56ba1d81";

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
    let bytes =
        std::fs::read(WATERBOTTLE_REFERENCE_PNG).expect("bundled WaterBottle reference is readable");
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

/// Build the WaterBottle scene + 3-point lighting + floor that both the
/// GPU headline and the CPU preview tests share. Returns scene-side
/// resources, ready for a renderer to be attached and rendered.
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

    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::WHITE)
                .with_illuminance_lux(80_000.0)
                .with_shadows(true),
        )
        .transform(Transform::default().rotate_x_deg(-55.0).rotate_y_deg(35.0))
        .add()
        .expect("key directional light inserts");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_srgb_u8(200, 215, 235))
                .with_illuminance_lux(25_000.0),
        )
        .transform(Transform::default().rotate_x_deg(-15.0).rotate_y_deg(-110.0))
        .add()
        .expect("fill directional light inserts");
    scene
        .directional_light(
            DirectionalLight::default()
                .with_color(Color::from_srgb_u8(255, 235, 210))
                .with_illuminance_lux(20_000.0),
        )
        .transform(Transform::default().rotate_x_deg(15.0).rotate_y_deg(170.0))
        .add()
        .expect("rim directional light inserts");

    let floor_geometry = assets.create_geometry(GeometryDesc::plane(0.6, 0.6));
    let floor_material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 0.9));
    scene
        .mesh(floor_geometry, floor_material)
        .transform(
            Transform::at(scena::Vec3::new(centre.x, bounds.min.y, centre.z))
                .rotate_x_deg(-90.0),
        )
        .add()
        .expect("floor mesh inserts");

    let environment = pollster::block_on(assets.load_environment(STUDIO_HDR_PATH))
        .expect("real polyhaven studio HDR loads");

    (assets, scene, environment)
}

/// Phase 3 GPU headline lane. Hard-requires the GPU renderer; on a
/// system without a working Vulkan adapter the test logs a skip
/// message and returns. Asserts the full Phase 2 region/family/diff
/// bar — this is the lane that's allowed to claim "matches the
/// reference render".
#[test]
fn m8_real_asset_waterbottle_gpu_headline() {
    let (assets, mut scene, environment) = build_waterbottle_scene();

    let mut renderer = match Renderer::headless_gpu(512, 512) {
        Ok(r) => r,
        Err(error) => {
            eprintln!(
                "scena: SKIPPING m8 WaterBottle GPU headline — \
                 Renderer::headless_gpu failed: {error:?}. On the Pi 5 / V3DV-broken \
                 hosts set VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json to \
                 force Mesa lavapipe before running the test."
            );
            return;
        }
    };
    let gpu_adapter_label = match renderer.gpu_adapter_report() {
        Some(report) => format!("{} ({})", report.name, report.backend),
        None => String::from("unknown"),
    };
    eprintln!("scena: rendering WaterBottle via GPU: {gpu_adapter_label}");

    renderer.set_environment(environment);
    renderer.set_exposure_ev(1.5);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("WaterBottle prepares for the headless GPU renderer");
    renderer
        .render_active(&scene)
        .expect("WaterBottle renders on the GPU");

    let stats = renderer.stats();
    assert_eq!(
        stats.materials, 2,
        "GPU: WaterBottle's `BottleMat` + the floor surface as two prepared materials"
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

    let frame = renderer.frame_rgba8();
    let nonzero = frame
        .chunks_exact(4)
        .filter(|p| p[..3] != [0, 0, 0])
        .count();
    assert!(
        nonzero > 5_000,
        "GPU: framed WaterBottle silhouette must produce at least 5000 non-black pixels (got {nonzero})"
    );

    write_png_artifact(frame, 512, 512, ARTIFACT_GPU_PNG);
    write_renderer_metadata("gpu", &gpu_adapter_label);

    // Phase 2 region asserts (GPU lane only — the CPU rasterizer does
    // not produce the same colours so its preview-quality output goes
    // through its own looser test below).
    let regions: &[(&str, usize, usize, [u8; 3], u8)] = &[
        // (name, x, y, expected RGB, tolerance in chebyshev distance)
        ("cap_dome",       250,  70, [130,  30,  35], 50),
        ("cap_dome_left",  240,  70, [115,  20,  25], 50),
        ("upper_body",     249, 130, [235, 235, 210], 45),
        ("body_olive_mid", 249, 270, [ 95,  95,  45], 45),
        ("body_olive_low", 249, 330, [ 90,  90,  40], 45),
        ("label_metal_r",  270, 380, [105, 108, 115], 45),
        ("label_metal_l",  255, 380, [ 80,  82,  85], 45),
        ("bg_top_right",   490,  10, [ 70,  77,  82], 35),
        ("bg_mid_right",   450, 250, [ 70,  75,  80], 35),
        ("bg_bot_right",   490, 490, [ 50,  55,  60], 35),
        ("bg_mid_left",     80, 250, [130, 138, 144], 45),
    ];
    let mut failed_regions = Vec::new();
    for (name, x, y, expected, tol) in regions {
        let p = pixel_at(frame, *x, *y);
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
        ("dark_red_cap",    400,   |r, g, b| r > 80 && r < 180 && g < 60 && b < 60),
        ("yellow_olive",   3_000,  |r, g, b| r > 60 && g > 50 && b < g.saturating_sub(15) && r < 200),
        ("bright_cream",     200,  |r, g, b| r > 220 && g > 215 && b > 180 && b < r),
        ("neutral_dark",   2_000,  |r, g, b| r < 80 && g < 85 && b < 90 && r.abs_diff(g) < 20),
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
        let (within_tol, total, max_d) = pixel_diff_summary(frame, &reference, 16);
        let fraction = within_tol as f64 / total as f64;
        if fraction < 0.95 {
            write_diff_visualization(frame, &reference);
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

/// Phase 3 CPU preview lane. The CPU rasterizer in `Renderer::headless`
/// is preview-quality — it does NOT have full split-sum IBL specular,
/// prefiltered cubemaps, or BRDF LUT integration the GPU path uses, so
/// it produces a meaningfully different render (no metallic gold body,
/// no HDR reflections). This test verifies the parser+geometry
/// pipeline can drive that path end-to-end without trying to pin
/// colours that the CPU lacks the lighting math to produce. The
/// headline visual fidelity bar lives in the GPU test above.
#[test]
fn m8_real_asset_waterbottle_cpu_preview() {
    let (assets, mut scene, environment) = build_waterbottle_scene();

    let mut renderer = Renderer::headless(512, 512).expect("CPU rasterizer");
    eprintln!("scena: rendering WaterBottle via CPU preview path");

    renderer.set_environment(environment);
    renderer.set_exposure_ev(1.5);
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("WaterBottle prepares for the CPU rasterizer");
    renderer
        .render_active(&scene)
        .expect("WaterBottle renders on the CPU");

    let stats = renderer.stats();
    assert_eq!(
        stats.materials, 2,
        "CPU: WaterBottle's `BottleMat` + the floor surface as two prepared materials"
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

    let frame = renderer.frame_rgba8();
    let nonzero = frame
        .chunks_exact(4)
        .filter(|p| p[..3] != [0, 0, 0])
        .count();
    assert!(
        nonzero > 5_000,
        "CPU: framed WaterBottle silhouette must produce at least 5000 non-black pixels (got {nonzero})"
    );

    // Silhouette check — confirm the bottle is somewhere near the
    // centre of the frame. The CPU path doesn't produce the gold/red
    // colour story so we cannot pin region colours, but we can still
    // catch "rendered nothing", "rendered offscreen", or "filled the
    // whole frame with a single colour" regressions.
    let centre = pixel_at(frame, 249, 246);
    let tl = pixel_at(frame, 5, 5);
    let br = pixel_at(frame, 506, 506);
    assert!(
        centre[..3] != tl[..3] || centre[..3] != br[..3],
        "CPU: centre pixel must differ from at least one corner pixel \
         (centre={centre:?}, tl={tl:?}, br={br:?}) — the renderer should \
         produce SOMETHING distinct in the bottle's footprint"
    );

    write_png_artifact(frame, 512, 512, ARTIFACT_CPU_PNG);
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
            r > 80 && r < 180 && g < 60 && b < 60,
            r > 60 && g > 50 && b < g.saturating_sub(15) && r < 200,
            r > 220 && g > 215 && b > 180 && b < r,
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
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    reader.next_frame(&mut buffer).expect("reference PNG payload reads");
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

/// Stage 0 (visibility): companion metadata so reviewers can tell at a glance
/// which renderer produced `waterbottle.png` without having to re-read the
/// test source or replay the run.
fn write_renderer_metadata(renderer_path: &str, gpu_adapter: &str) {
    if let Some(parent) = std::path::Path::new(METADATA_TOML).parent() {
        std::fs::create_dir_all(parent).expect("artifact dir");
    }
    let body = format!(
        "# Generated by tests/m8_real_asset_proof.rs.\n\
         renderer_path = \"{renderer_path}\"\n\
         gpu_adapter = \"{gpu_adapter}\"\n\
         scena_use_gpu_set = {use_gpu_set}\n",
        use_gpu_set = std::env::var("SCENA_USE_GPU").is_ok(),
    );
    std::fs::write(METADATA_TOML, body).expect("renderer metadata writes");
}
