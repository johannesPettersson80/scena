use crate::app::prelude::*;

const DEMO_HDR_PATH: &str = "demo/samples/environment/white_studio_03_1k.hdr";
const DEMO_HDR_SIDECAR_PATH: &str = "demo/samples/environment/white_studio_03_1k.hdr.prefilter.bin";
const SHOWCASE_IMAGE_QUALITY_RULE: &str = "PUBLIC-SHOWCASE-CARD-IMAGE-QUALITY";
const PUBLIC_SHOWCASE_WASM_PATH: &str = "demo/pkg/scena_bg.wasm";
const PROOF_HARNESS_WASM_PATH: &str = "demo/proof/pkg/scena_bg.wasm";
const PUBLIC_SHOWCASE_WASM_BASELINE_RAW_BYTES: u64 = 4_318_556;
const PUBLIC_SHOWCASE_WASM_BASELINE_BROTLI_BYTES: u64 = 1_193_980;
const PROOF_HARNESS_WASM_BASELINE_RAW_BYTES: u64 = 5_355_128;
const PROOF_HARNESS_WASM_BASELINE_BROTLI_BYTES: u64 = 1_534_095;
const PUBLIC_SHOWCASE_WASM_RAW_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PUBLIC_SHOWCASE_WASM_BASELINE_RAW_BYTES);
const PUBLIC_SHOWCASE_WASM_BROTLI_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PUBLIC_SHOWCASE_WASM_BASELINE_BROTLI_BYTES);
const PROOF_HARNESS_WASM_RAW_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PROOF_HARNESS_WASM_BASELINE_RAW_BYTES);
const PROOF_HARNESS_WASM_BROTLI_BUDGET_BYTES: u64 =
    ten_percent_growth_budget(PROOF_HARNESS_WASM_BASELINE_BROTLI_BYTES);

const fn ten_percent_growth_budget(baseline: u64) -> u64 {
    baseline + baseline / 10
}

pub(super) fn check_showcase_performance_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_demo_hdr_sidecar_current(root, findings);
    check_showcase_card_image_quality(root, findings);
    check_wasm_size_budget(root, findings);
    require_contains(
        root,
        findings,
        SHOWCASE_IMAGE_QUALITY_RULE,
        "examples/easy_scene_showcase.rs",
        &[
            "REFLECTIVE_SHOWCASE_SUPERSAMPLE_FACTOR: u32 = 2",
            "configure_reflective_showcase_renderer",
            "load_environment_preset(EnvironmentPreset::Studio)",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        "crates/xtask/src/app/core.rs",
        &["prerender-environment <input.hdr> [--resolution <face_px>]"],
    );
    require_contains(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        "crates/xtask/src/app/prerender_environment.rs",
        &[
            "run_prerender_environment",
            "precompute_environment_sidecar",
            "EnvironmentSidecarProfile::InteractiveWebGl2",
            "from_equirectangular_hdr_bytes",
            "with_cubemap_resolution",
            ".prefilter.bin",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        "src/assets/environment_loading.rs",
        &[
            "try_load_environment_sidecar",
            "sidecar_path_for_environment",
            "EnvironmentPrefilterSidecar::parse",
            "from_equirectangular_hdr_sidecar_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        "src/render/prepare/environment.rs",
        &[
            "prefilter_sidecar",
            "load_prefilter_sidecar",
            "bake_environment_ibl",
            "EnvironmentIblBakeRequest",
            "precompute_environment_sidecar",
        ],
    );
    require_contains(
        root,
        findings,
        "PUBLIC-SHOWCASE-WASM-SIZE",
        "scripts/build_demo_wasm.js",
        &[
            "demo/proof/pkg",
            "proof-harness,browser-probe",
            "--strip-debug",
            "--strip-dwarf",
            "--strip-producers",
            "CARGO_PROFILE_RELEASE_OPT_LEVEL",
            "process.env.CARGO_PROFILE_RELEASE_OPT_LEVEL || \"z\"",
            "stampCacheBusters(writeSizeManifest())",
            "wasm=${manifest.sha256}",
            "demo/proof/index.html",
            "demo/proof.js",
            "demo/index.html",
            "demo/main.js",
        ],
    );
    require_contains(
        root,
        findings,
        "PUBLIC-SHOWCASE-CONNECTOR-REPLAY-HOT-PATH",
        "src/demo_page.rs",
        &[
            "let floor = scene\n        .add_grid_floor(",
            "scene\n        .set_visible(floor.grid, false)",
            "connector replay keeps the animated scene on the dynamic GPU prepare path",
        ],
    );
}

fn check_demo_hdr_sidecar_current(root: &Path, findings: &mut Vec<Finding>) {
    check_hdr_sidecar_current(
        root,
        findings,
        "DEMO-HDR-SIDECAR-CURRENT",
        DEMO_HDR_PATH,
        DEMO_HDR_SIDECAR_PATH,
        None,
    );
}

fn check_hdr_sidecar_current(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    hdr_relative_path: &str,
    sidecar_relative_path: &str,
    expected_resolution: Option<u32>,
) {
    let hdr_path = root.join(hdr_relative_path);
    let sidecar_path = root.join(sidecar_relative_path);
    if !sidecar_path.exists() {
        findings.push(Finding::new(
            rule,
            format!("{sidecar_relative_path} must exist and be generated from {hdr_relative_path}"),
        ));
        return;
    }
    let hdr_sha = match sha256_hex(&hdr_path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                rule,
                format!("could not hash {hdr_relative_path}: {error}"),
            ));
            return;
        }
    };
    let sidecar_bytes = match fs::read(&sidecar_path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                rule,
                format!("could not read {sidecar_relative_path}: {error}"),
            ));
            return;
        }
    };
    let header = match scena::parse_sidecar_header(sidecar_relative_path, &sidecar_bytes) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                rule,
                format!("could not parse {sidecar_relative_path} header: {error:?}"),
            ));
            return;
        }
    };
    if header.source_sha256_hex() != hdr_sha {
        findings.push(Finding::new(
            rule,
            format!(
                "{sidecar_relative_path} source HDR SHA mismatch: header {}, actual {hdr_sha}",
                header.source_sha256_hex()
            ),
        ));
    }
    if header.profile_name() != "InteractiveWebGl2" {
        findings.push(Finding::new(
            rule,
            format!(
                "{sidecar_relative_path} must use InteractiveWebGl2, got {}",
                header.profile_name()
            ),
        ));
    }
    if let Some(expected_resolution) = expected_resolution
        && header.cubemap_resolution() != expected_resolution
    {
        findings.push(Finding::new(
            rule,
            format!(
                "{sidecar_relative_path} must use {expected_resolution}px faces, got {}",
                header.cubemap_resolution()
            ),
        ));
    }
}

#[derive(Clone, Copy)]
struct ShowcaseCardSpec {
    path: &'static str,
    width: u32,
    height: u32,
    tile_width: Option<u32>,
    min_luma_stddev: f32,
    min_edge_mean: f32,
    max_low_clip_fraction: f32,
    max_high_clip_fraction: f32,
    /// Minimum fraction of bright reflected pixels (luma > 0.6) required for a
    /// mirror-metal "chrome read" — bright reflection cards must be present.
    /// `None` skips the check (non-chrome cards).
    min_bright_fraction: Option<f32>,
    /// Minimum fraction of dark pixels (luma < 0.2) required for a chrome read —
    /// dark edge/flag falloff must be present so the subject is not a flat gray.
    min_dark_fraction: Option<f32>,
}

#[derive(Clone, Copy)]
struct ImageQualityMetrics {
    luma_stddev: f32,
    edge_mean: f32,
    low_clip_fraction: f32,
    high_clip_fraction: f32,
    bright_fraction: f32,
    dark_fraction: f32,
}

/// Chrome-read thresholds for the `material-chrome` card, set to roughly half
/// the bright/dark fractions measured on the rendered studio-HDR chrome hero so
/// the shipped card passes with margin while a flat/gray non-chrome subject
/// (≈0 bright and ≈0 dark) fails.
const CHROME_MIN_BRIGHT_FRACTION: f32 = 0.05;
const CHROME_MIN_DARK_FRACTION: f32 = 0.10;

const SHOWCASE_CARD_SPECS: &[ShowcaseCardSpec] = &[
    ShowcaseCardSpec {
        path: "docs/assets/easy-scene-showcase/lens-presets.jpg",
        width: 1_920,
        height: 480,
        tile_width: Some(480),
        min_luma_stddev: 0.06,
        min_edge_mean: 0.002,
        // Larger lens panels are mostly the dark chrome subject on a dark studio.
        max_low_clip_fraction: 0.95,
        max_high_clip_fraction: 0.05,
        min_bright_fraction: None,
        min_dark_fraction: None,
    },
    ShowcaseCardSpec {
        path: "docs/assets/easy-scene-showcase/auto-exposure-presets.jpg",
        width: 1_920,
        height: 480,
        tile_width: Some(480),
        min_luma_stddev: 0.08,
        min_edge_mean: 0.0025,
        // Studio-HDR chrome subjects on a DarkStudio backdrop are legitimately
        // mostly-dark; luma stddev + edge detail are the real guards here.
        max_low_clip_fraction: 0.95,
        max_high_clip_fraction: 0.06,
        min_bright_fraction: None,
        min_dark_fraction: None,
    },
    ShowcaseCardSpec {
        path: "docs/assets/easy-scene-showcase/environment-presets.jpg",
        width: 960,
        height: 480,
        tile_width: Some(480),
        min_luma_stddev: 0.10,
        min_edge_mean: 0.0015,
        // Dark studio backdrop dominates this card; stddev + edge are the guards.
        max_low_clip_fraction: 0.95,
        max_high_clip_fraction: 0.06,
        min_bright_fraction: None,
        min_dark_fraction: None,
    },
    ShowcaseCardSpec {
        path: "docs/assets/easy-scene-showcase/material-chrome.png",
        width: 640,
        height: 640,
        tile_width: None,
        min_luma_stddev: 0.20,
        min_edge_mean: 0.004,
        // A studio-HDR chrome hero is a dark mirror on a dark studio: mostly-dark
        // is correct here, so the dark-fraction ceiling is generous. The real
        // "is it chrome" guard is min_bright_fraction below.
        max_low_clip_fraction: 0.95,
        max_high_clip_fraction: 0.06,
        // Calibrated below from the rendered chrome card (~half the measured
        // values) so a flat/gray non-chrome card fails the chrome read.
        min_bright_fraction: Some(CHROME_MIN_BRIGHT_FRACTION),
        min_dark_fraction: Some(CHROME_MIN_DARK_FRACTION),
    },
];

fn check_showcase_card_image_quality(root: &Path, findings: &mut Vec<Finding>) {
    for spec in SHOWCASE_CARD_SPECS {
        let path = root.join(spec.path);
        let image = match image::open(&path) {
            Ok(image) => image.into_rgba8(),
            Err(error) => {
                findings.push(Finding::new(
                    SHOWCASE_IMAGE_QUALITY_RULE,
                    format!("could not decode {}: {error}", spec.path),
                ));
                continue;
            }
        };
        if image.width() != spec.width || image.height() != spec.height {
            findings.push(Finding::new(
                SHOWCASE_IMAGE_QUALITY_RULE,
                format!(
                    "{} must be {}x{}, got {}x{}",
                    spec.path,
                    spec.width,
                    spec.height,
                    image.width(),
                    image.height()
                ),
            ));
            continue;
        }
        check_metrics(
            spec.path,
            "whole image",
            image_metrics(&image, 0, spec.width),
            spec,
            findings,
        );
        if let Some(tile_width) = spec.tile_width {
            for tile in 0..(spec.width / tile_width) {
                let label = format!("tile {tile}");
                check_metrics(
                    spec.path,
                    &label,
                    image_metrics(&image, tile * tile_width, tile_width),
                    spec,
                    findings,
                );
            }
        }
    }
}

fn check_metrics(
    path: &str,
    label: &str,
    metrics: ImageQualityMetrics,
    spec: &ShowcaseCardSpec,
    findings: &mut Vec<Finding>,
) {
    if metrics.luma_stddev < spec.min_luma_stddev {
        findings.push(Finding::new(
            SHOWCASE_IMAGE_QUALITY_RULE,
            format!(
                "{path} {label} is too flat: luma stddev {:.3} < {:.3}",
                metrics.luma_stddev, spec.min_luma_stddev
            ),
        ));
    }
    if metrics.edge_mean < spec.min_edge_mean {
        findings.push(Finding::new(
            SHOWCASE_IMAGE_QUALITY_RULE,
            format!(
                "{path} {label} has insufficient visible detail: edge mean {:.4} < {:.4}",
                metrics.edge_mean, spec.min_edge_mean
            ),
        ));
    }
    if metrics.low_clip_fraction > spec.max_low_clip_fraction {
        findings.push(Finding::new(
            SHOWCASE_IMAGE_QUALITY_RULE,
            format!(
                "{path} {label} is black-crushed: low-luma fraction {:.3} > {:.3}",
                metrics.low_clip_fraction, spec.max_low_clip_fraction
            ),
        ));
    }
    if metrics.high_clip_fraction > spec.max_high_clip_fraction {
        findings.push(Finding::new(
            SHOWCASE_IMAGE_QUALITY_RULE,
            format!(
                "{path} {label} is blown out: high-luma fraction {:.3} > {:.3}",
                metrics.high_clip_fraction, spec.max_high_clip_fraction
            ),
        ));
    }
    if let Some(min_bright) = spec.min_bright_fraction
        && metrics.bright_fraction < min_bright
    {
        findings.push(Finding::new(
            SHOWCASE_IMAGE_QUALITY_RULE,
            format!(
                "{path} {label} lacks chrome read: bright reflection fraction {:.3} < {:.3}",
                metrics.bright_fraction, min_bright
            ),
        ));
    }
    if let Some(min_dark) = spec.min_dark_fraction
        && metrics.dark_fraction < min_dark
    {
        findings.push(Finding::new(
            SHOWCASE_IMAGE_QUALITY_RULE,
            format!(
                "{path} {label} lacks chrome read: dark edge fraction {:.3} < {:.3}",
                metrics.dark_fraction, min_dark
            ),
        ));
    }
}

fn image_metrics(image: &image::RgbaImage, start_x: u32, width: u32) -> ImageQualityMetrics {
    let height = image.height();
    let mut count = 0_u64;
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut low = 0_u64;
    let mut high = 0_u64;
    let mut bright = 0_u64;
    let mut dark = 0_u64;
    let mut edge_sum = 0.0_f64;
    for y in 0..height {
        for x in start_x..start_x + width {
            let luma = luma_from_pixel(image.get_pixel(x, y)) as f64;
            count += 1;
            sum += luma;
            sum_sq += luma * luma;
            if luma < 0.04 {
                low += 1;
            }
            if luma > 0.95 {
                high += 1;
            }
            if luma > 0.6 {
                bright += 1;
            }
            if luma < 0.2 {
                dark += 1;
            }
            let dx = if x + 1 < start_x + width {
                (luma_from_pixel(image.get_pixel(x + 1, y)) as f64 - luma).abs()
            } else {
                0.0
            };
            let dy = if y + 1 < height {
                (luma_from_pixel(image.get_pixel(x, y + 1)) as f64 - luma).abs()
            } else {
                0.0
            };
            edge_sum += (dx * dx + dy * dy).sqrt();
        }
    }
    let count_f = count as f64;
    let mean = sum / count_f;
    let variance = (sum_sq / count_f - mean * mean).max(0.0);
    ImageQualityMetrics {
        luma_stddev: variance.sqrt() as f32,
        edge_mean: (edge_sum / count_f) as f32,
        low_clip_fraction: low as f32 / count as f32,
        high_clip_fraction: high as f32 / count as f32,
        bright_fraction: bright as f32 / count as f32,
        dark_fraction: dark as f32 / count as f32,
    }
}

fn luma_from_pixel(pixel: &image::Rgba<u8>) -> f32 {
    let [r, g, b, _] = pixel.0;
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
}

fn check_wasm_size_budget(root: &Path, findings: &mut Vec<Finding>) {
    let public_path = root.join(PUBLIC_SHOWCASE_WASM_PATH);
    let proof_path = root.join(PROOF_HARNESS_WASM_PATH);
    let public_exists = public_path.exists();
    let proof_exists = proof_path.exists();

    if !public_exists && !proof_exists {
        return;
    }

    if public_exists != proof_exists {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "showcase WASM size budgets require both generated bundles to exist: \
                 {PUBLIC_SHOWCASE_WASM_PATH} exists={public_exists}, \
                 {PROOF_HARNESS_WASM_PATH} exists={proof_exists}"
            ),
        ));
        return;
    }

    check_one_wasm_size_budget(
        root,
        findings,
        PUBLIC_SHOWCASE_WASM_PATH,
        PUBLIC_SHOWCASE_WASM_RAW_BUDGET_BYTES,
        PUBLIC_SHOWCASE_WASM_BROTLI_BUDGET_BYTES,
        "public showcase",
    );
    check_one_wasm_size_budget(
        root,
        findings,
        PROOF_HARNESS_WASM_PATH,
        PROOF_HARNESS_WASM_RAW_BUDGET_BYTES,
        PROOF_HARNESS_WASM_BROTLI_BUDGET_BYTES,
        "proof harness",
    );
}

fn check_one_wasm_size_budget(
    root: &Path,
    findings: &mut Vec<Finding>,
    relative_path: &str,
    raw_budget_bytes: u64,
    brotli_budget_bytes: u64,
    label: &str,
) {
    let path = root.join(relative_path);
    let bytes = match fs::read(&path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "PUBLIC-SHOWCASE-WASM-SIZE",
                format!("could not read {label} WASM bundle {relative_path}: {error}"),
            ));
            return;
        }
    };
    let raw_len = bytes.len() as u64;
    let manifest_path = root.join(format!("{relative_path}.size.json"));
    let manifest = match read_wasm_size_manifest(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "PUBLIC-SHOWCASE-WASM-SIZE",
                format!(
                    "could not read {label} WASM size manifest {}: {error}",
                    path_to_forward_slash(&manifest_path)
                ),
            ));
            return;
        }
    };
    if manifest.raw_bytes != raw_len {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM size manifest raw_bytes {} does not match actual raw size {raw_len} at {relative_path}",
                manifest.raw_bytes
            ),
        ));
    }
    if manifest.brotli_quality != 11 {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM size manifest must use brotli quality 11, got {}",
                manifest.brotli_quality
            ),
        ));
    }
    if raw_len > raw_budget_bytes {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM raw size {raw_len} exceeds budget {raw_budget_bytes} at {relative_path}"
            ),
        ));
    }
    if manifest.brotli_bytes > brotli_budget_bytes {
        findings.push(Finding::new(
            "PUBLIC-SHOWCASE-WASM-SIZE",
            format!(
                "{label} WASM brotli size {} exceeds budget {brotli_budget_bytes} at {relative_path}",
                manifest.brotli_bytes
            ),
        ));
    }
}

struct WasmSizeManifest {
    raw_bytes: u64,
    brotli_quality: u64,
    brotli_bytes: u64,
}

fn read_wasm_size_manifest(path: &Path) -> Result<WasmSizeManifest, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let json: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    Ok(WasmSizeManifest {
        raw_bytes: json
            .get("raw_bytes")
            .and_then(Value::as_u64)
            .ok_or("missing numeric raw_bytes")?,
        brotli_quality: json
            .get("brotli_quality")
            .and_then(Value::as_u64)
            .ok_or("missing numeric brotli_quality")?,
        brotli_bytes: json
            .get("brotli_bytes")
            .and_then(Value::as_u64)
            .ok_or("missing numeric brotli_bytes")?,
    })
}
