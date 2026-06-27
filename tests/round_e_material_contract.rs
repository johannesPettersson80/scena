use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

const FIXTURE_PATH: &str = "tests/visual/references/round_e_material_fixture.toml";
const THRESHOLDS_PATH: &str = "tests/visual/references/round_e_material_thresholds.toml";
const FAILING_BASELINE_PATH: &str = "tests/visual/references/round_e_failing_baseline.json";
const FAILING_BASELINE_IMAGE_PATH: &str =
    "tests/visual/references/round_e_failing_baseline_glossy_grid.png";
const FAILING_BASELINE_IMAGE_SHA256: &str =
    "32d99960699b6b05fb3888e9d8fd57af3d07a27cacb5a94214d0b0b9f0ba589c";

const REQUIRED_PRESETS: &[&str] = &[
    "matte",
    "plastic",
    "metal",
    "rough_metal",
    "chrome",
    "brushed_steel",
    "clearcoat_plastic",
    "satin",
    "leather",
    "clear_glass",
    "frosted_glass",
    "rubber",
];

const REQUIRED_THRESHOLD_FLOORS: &[(&str, ThresholdBound)] = &[
    ("matte.delta_e2000_max", ThresholdBound::Max(5.0)),
    ("plastic.delta_e2000_max", ThresholdBound::Max(11.0)),
    ("metal.delta_e2000_max", ThresholdBound::Max(20.0)),
    ("rough_metal.delta_e2000_max", ThresholdBound::Max(16.0)),
    ("chrome.specular_dynamic_range", ThresholdBound::Min(2.0)),
    (
        "chrome.dark_reflection_luminance_p05_max",
        ThresholdBound::Max(85.0),
    ),
    (
        "chrome.bright_reflection_luminance_p99_min",
        ThresholdBound::Min(230.0),
    ),
    ("chrome.reflection_edge_contrast", ThresholdBound::Min(0.30)),
    ("chrome.delta_e2000_max", ThresholdBound::Max(20.0)),
    (
        "brushed_steel.anisotropy_aspect_ratio_direct",
        ThresholdBound::Min(3.0),
    ),
    (
        "brushed_steel.anisotropy_aspect_ratio_ibl",
        ThresholdBound::Min(2.0),
    ),
    ("brushed_steel.delta_e2000_max", ThresholdBound::Max(15.0)),
    (
        "clearcoat_plastic.clearcoat_lobe_delta",
        ThresholdBound::Min(0.05),
    ),
    (
        "clearcoat_plastic.delta_e2000_max",
        ThresholdBound::Max(18.0),
    ),
    (
        "clear_glass.background_delta_e2000_max",
        ThresholdBound::Max(8.0),
    ),
    (
        "clear_glass.refraction_offset_min",
        ThresholdBound::Min(4.0),
    ),
    ("clear_glass.delta_e2000_max", ThresholdBound::Max(32.0)),
    (
        "frosted_glass.high_frequency_contrast_reduction_min",
        ThresholdBound::Min(0.50),
    ),
    ("frosted_glass.delta_e2000_max", ThresholdBound::Max(28.0)),
    ("leather.texture_variance_min", ThresholdBound::Min(0.02)),
    ("leather.delta_e2000_max", ThresholdBound::Max(7.0)),
    ("rubber.roughness_variance_min", ThresholdBound::Min(0.02)),
    ("rubber.delta_e2000_max", ThresholdBound::Max(21.0)),
    ("satin.sheen_width_min", ThresholdBound::Min(0.20)),
    ("satin.delta_e2000_max", ThresholdBound::Max(6.0)),
    ("global.neighbor_delta_e2000_min", ThresholdBound::Min(6.0)),
    ("global.reference_delta_e2000_max", ThresholdBound::Max(4.0)),
];

#[test]
fn round_e_material_fixture_is_external_anchored_and_value_bounded() {
    let fixture = parse_fixture(Path::new(FIXTURE_PATH));
    let thresholds = parse_thresholds(Path::new(THRESHOLDS_PATH));
    assert_threshold_floors(&thresholds);
    assert_required_presets(&fixture);
    assert_shared_fixture_fields(&fixture);
    assert_reference_pngs_exist_and_are_pinned(&fixture);
    assert_mobile_required_presets_are_claimed(&fixture);
}

#[test]
fn round_e_failing_baseline_rejects_old_glossy_grid() {
    let thresholds = parse_thresholds(Path::new(THRESHOLDS_PATH));
    let image_bytes = fs::read(FAILING_BASELINE_IMAGE_PATH).unwrap_or_else(|err| {
        panic!("Round E failing-baseline image {FAILING_BASELINE_IMAGE_PATH} must exist: {err}")
    });
    assert_eq!(
        sha256_hex(&image_bytes),
        FAILING_BASELINE_IMAGE_SHA256,
        "Round E failing-baseline image must stay pinned"
    );
    let baseline_image = image::load_from_memory(&image_bytes)
        .expect("Round E failing-baseline image decodes as PNG");
    assert_eq!(
        (baseline_image.width(), baseline_image.height()),
        (1920, 1440),
        "Round E failing baseline must preserve the archived 4x3 glossy grid dimensions"
    );

    let text = fs::read_to_string(FAILING_BASELINE_PATH).unwrap_or_else(|err| {
        panic!("Round E failing-baseline artifact {FAILING_BASELINE_PATH} must exist: {err}")
    });
    let artifact: Value =
        serde_json::from_str(&text).expect("Round E failing-baseline artifact is JSON");
    assert_eq!(
        artifact["proof_class"], "round-e-failing-baseline",
        "Round E failing-baseline artifact must declare its proof class"
    );
    assert_eq!(
        artifact["source_image"], FAILING_BASELINE_IMAGE_PATH,
        "Round E failing-baseline artifact must point at the pinned old glossy grid"
    );
    assert_eq!(
        artifact["source_sha256"], FAILING_BASELINE_IMAGE_SHA256,
        "Round E failing-baseline artifact must pin the old glossy grid hash"
    );
    assert!(
        artifact["metric"]
            .as_str()
            .is_some_and(|metric| metric.contains("CIEDE2000")),
        "Round E failing-baseline artifact must record the color-difference metric"
    );
    assert_eq!(
        artifact["threshold_source"], THRESHOLDS_PATH,
        "Round E failing baseline must point at the same committed threshold file as the live proof"
    );
    let minimum_margin = artifact["minimum_failure_margin_delta_e2000"]
        .as_f64()
        .expect("Round E failing baseline pins a meaningful failure margin");
    assert!(
        minimum_margin >= 1.0,
        "Round E failing baseline must require a meaningful failure margin"
    );
    let failed = artifact["failed_presets"]
        .as_array()
        .expect("Round E failing baseline records failed presets");
    let threshold_for = |preset: &str| -> f64 {
        thresholds
            .get(&format!("{preset}.delta_e2000_max"))
            .or_else(|| thresholds.get("global.reference_delta_e2000_max"))
            .copied()
            .unwrap_or_else(|| panic!("missing DeltaE threshold for {preset}"))
            .into()
    };
    let meaningful_failures = failed
        .iter()
        .filter(|entry| {
            let preset = entry["preset"].as_str().unwrap_or("");
            let delta = entry["delta_e2000_vs_reference"].as_f64().unwrap_or(0.0);
            delta >= threshold_for(preset) + minimum_margin
        })
        .count();
    let minimum_failed = artifact["minimum_failed_presets"]
        .as_u64()
        .expect("Round E failing baseline pins minimum failed preset count")
        as usize;
    assert!(
        meaningful_failures >= minimum_failed && meaningful_failures >= 3,
        "Round E old glossy grid must fail at least three presets by a meaningful margin"
    );
    for required in ["chrome", "brushed_steel", "clear_glass"] {
        assert!(
            failed.iter().any(|entry| {
                entry["preset"] == required
                    && entry["delta_e2000_vs_reference"].as_f64().unwrap_or(0.0)
                        >= threshold_for(required) + minimum_margin
            }),
            "Round E failing baseline must prove the old glossy grid fails {required}"
        );
    }
}

#[test]
fn round_e_reference_generator_uses_source_backed_texture_assets() {
    let script = fs::read_to_string("scripts/generate_round_e_model_viewer_references.mjs")
        .expect("Round E external reference generator must exist");

    for required in [
        "roundETextureSlots",
        "SCENA_BLUE",
        "SCENA_LIGHT_GRAY",
        "SCENA_CYAN",
        "SCENA_WHITE",
        "SCENA_LEATHER_BASE",
        "Fabric001_512_Color.jpg",
        "Leather001_512_Color.jpg",
        "Rubber002_512_Color.jpg",
        "baseColorTexture",
        "metallicRoughnessTexture",
        "normalTexture",
        "occlusionTexture",
        "KHR_texture_transform",
        "thicknessFactor: 0.08",
    ] {
        assert!(
            script.contains(required),
            "Round E external source-backed references must include {required}; \
             otherwise model-viewer anchors scalar placeholders while Scena renders bundled textures"
        );
    }

    for required in [
        "createShowcaseGltf()",
        "cropReferencePng(",
        "pixelCropWindow(preset.id)",
        "process.env.CHROMIUM",
    ] {
        assert!(
            script.contains(required),
            "Round E external references must be cropped from the same shared 4x3 showcase \
             geometry as the browser proof. Missing {required}; isolated per-material \
             reference renders do not match the live proof crop layout."
        );
    }
    assert!(
        !script.contains("JSON.stringify(createPresetGltf(preset)"),
        "Round E external references must not render isolated per-material scenes; the browser \
         proof compares 4x3 showcase crops."
    );
}

#[test]
fn round_e_material_proof_isolates_target_component_for_reference_delta() {
    let script = fs::read_to_string("scripts/probe_cloudflare_material_presets.mjs")
        .expect("Round E material proof script must exist");

    for required in [
        "centeredForegroundComponent(",
        "isolateCenterComponent: !preset.includes(\"glass\")",
        "glassTransmissionRegion",
        "rough_transmission_region",
        "const edgeOptions = { region: glassTransmissionRegion }",
        "path.join(outDir, \"clear_glass.png\"),",
        "reference_delta_gate === \"hard\"",
        "DEFAULT_URL = \"https://scena-demo.pages.dev/proof/?sample=material-presets\"",
    ] {
        assert!(
            script.contains(required),
            "Round E material proof must keep hard reference deltas focused on the target \
             material component and the deployed proof harness. Missing {required}."
        );
    }
}

#[derive(Clone, Copy)]
enum ThresholdBound {
    Min(f32),
    Max(f32),
}

#[derive(Debug)]
struct FixturePreset {
    values: BTreeMap<String, String>,
}

fn parse_fixture(path: &Path) -> BTreeMap<String, FixturePreset> {
    let text = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "Round E material fixture is required at {}: {err}",
            path.display()
        )
    });
    let mut current: Option<String> = None;
    let mut presets = BTreeMap::<String, FixturePreset>::new();
    for raw_line in text.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(section) = line
            .strip_prefix("[[presets.")
            .and_then(|value| value.strip_suffix("]]"))
        {
            current = Some(section.to_string());
            presets.insert(
                section.to_string(),
                FixturePreset {
                    values: BTreeMap::new(),
                },
            );
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(current) = current.as_ref() else {
            continue;
        };
        presets
            .get_mut(current)
            .expect("current preset section exists")
            .values
            .insert(key.trim().to_string(), value.trim().to_string());
    }
    presets
}

fn parse_thresholds(path: &Path) -> BTreeMap<String, f32> {
    let text = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "Round E material thresholds are required at {}: {err}",
            path.display()
        )
    });
    let mut section = String::new();
    let mut values = BTreeMap::new();
    for raw_line in text.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            section = name.trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let full_key = if section.is_empty() {
            key.trim().to_string()
        } else {
            format!("{}.{}", section, key.trim())
        };
        let parsed = value
            .trim()
            .parse::<f32>()
            .unwrap_or_else(|err| panic!("threshold {full_key} must be numeric: {err}"));
        values.insert(full_key, parsed);
    }
    values
}

fn assert_threshold_floors(thresholds: &BTreeMap<String, f32>) {
    for (key, bound) in REQUIRED_THRESHOLD_FLOORS {
        let value = thresholds
            .get(*key)
            .unwrap_or_else(|| panic!("missing Round E threshold {key}"));
        match bound {
            ThresholdBound::Min(minimum) => assert!(
                *value >= *minimum,
                "threshold {key}={value} is below required floor {minimum}"
            ),
            ThresholdBound::Max(maximum) => assert!(
                *value <= *maximum,
                "threshold {key}={value} is above required ceiling {maximum}"
            ),
        }
    }
}

fn assert_required_presets(fixture: &BTreeMap<String, FixturePreset>) {
    for preset in REQUIRED_PRESETS {
        assert!(
            fixture.contains_key(*preset),
            "Round E fixture must include preset {preset}"
        );
    }
}

fn assert_shared_fixture_fields(fixture: &BTreeMap<String, FixturePreset>) {
    for (preset, entry) in fixture {
        for required in [
            "label",
            "source_surface",
            "geometry",
            "crop_window",
            "camera",
            "lighting_mode",
            "environment_hdr_path",
            "environment_hdr_sha256",
            "tonemapper",
            "output_color_space",
            "exposure_ev",
            "reference_renderer",
            "reference_renderer_version",
            "reference_command",
            "reference_path",
            "reference_sha256",
            "claimed_lanes",
        ] {
            assert!(
                entry.values.contains_key(required),
                "Round E fixture preset {preset} must pin {required}"
            );
        }
        assert_quoted_not_scena_output(preset, entry, "reference_renderer");
        assert_quoted_not_scena_output(preset, entry, "reference_command");
    }
}

fn assert_reference_pngs_exist_and_are_pinned(fixture: &BTreeMap<String, FixturePreset>) {
    for (preset, entry) in fixture {
        let reference_path = unquote(
            entry
                .values
                .get("reference_path")
                .unwrap_or_else(|| panic!("preset {preset} must pin reference_path")),
        );
        assert!(
            reference_path.starts_with("tests/visual/references/round_e/"),
            "preset {preset} reference path must stay in round_e references, got {reference_path}"
        );
        let bytes = fs::read(PathBuf::from(&reference_path)).unwrap_or_else(|err| {
            panic!("preset {preset} reference PNG {reference_path} must exist: {err}")
        });
        assert!(
            bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]),
            "preset {preset} reference {reference_path} must be a PNG"
        );
        let sha = unquote(
            entry
                .values
                .get("reference_sha256")
                .unwrap_or_else(|| panic!("preset {preset} must pin reference_sha256")),
        );
        assert!(
            sha.len() == 64 && sha.chars().all(|ch| ch.is_ascii_hexdigit()),
            "preset {preset} must pin a 64-character lowercase SHA-256"
        );
        assert!(
            sha.chars().all(|ch| !ch.is_ascii_uppercase()),
            "preset {preset} reference SHA-256 must be lowercase"
        );
        let actual_sha = sha256_hex(&bytes);
        assert_eq!(
            actual_sha, sha,
            "preset {preset} reference SHA-256 must match {reference_path}"
        );
    }
}

fn assert_mobile_required_presets_are_claimed(fixture: &BTreeMap<String, FixturePreset>) {
    for preset in [
        "chrome",
        "brushed_steel",
        "clearcoat_plastic",
        "clear_glass",
    ] {
        let entry = fixture
            .get(preset)
            .unwrap_or_else(|| panic!("required mobile preset {preset} missing"));
        let lanes = entry
            .values
            .get("claimed_lanes")
            .unwrap_or_else(|| panic!("preset {preset} must pin claimed_lanes"));
        assert!(
            lanes.contains("ios-safari") && lanes.contains("android-chrome"),
            "preset {preset} must claim ios-safari and android-chrome for public demo approval"
        );
    }
}

fn assert_quoted_not_scena_output(preset: &str, entry: &FixturePreset, key: &str) {
    let value = unquote(
        entry
            .values
            .get(key)
            .unwrap_or_else(|| panic!("preset {preset} must pin {key}")),
    );
    let lowercase = value.to_ascii_lowercase();
    assert!(
        !lowercase.contains("scena"),
        "preset {preset} {key} must not use current scena output as the external reference: {value}"
    );
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(prefix, _)| prefix)
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
