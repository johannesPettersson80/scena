use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::app::core::Finding;

const ROUND_E_FIXTURE: &str = "tests/visual/references/round_e_material_fixture.toml";
const ROUND_E_THRESHOLDS: &str = "tests/visual/references/round_e_material_thresholds.toml";
const ROUND_E_CLOUDFLARE_PROOF: &str =
    "target/gate-artifacts/round-e-cloudflare-material-proof.json";
const ROUND_E_PRESETS: &[&str] = &[
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
const ROUND_E_NEIGHBOR_PAIRS: &[(&str, &str)] = &[
    ("metal", "rough_metal"),
    ("metal", "chrome"),
    ("chrome", "plastic"),
    ("clearcoat_plastic", "plastic"),
    ("clear_glass", "frosted_glass"),
    ("rubber", "plastic"),
];

pub(crate) fn check_round_e_cloudflare_material_proof(root: &Path, findings: &mut Vec<Finding>) {
    let artifact_path = root.join(ROUND_E_CLOUDFLARE_PROOF);
    if !artifact_path.is_file() {
        if round_e_material_parity_claimed_shipped(root) {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("Round E is claimed shipped but {ROUND_E_CLOUDFLARE_PROOF} is missing"),
            ));
        }
        return;
    }

    let Ok(artifact_text) = fs::read_to_string(&artifact_path) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("could not read {ROUND_E_CLOUDFLARE_PROOF}"),
        ));
        return;
    };
    let Ok(artifact) = serde_json::from_str::<Value>(&artifact_text) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_CLOUDFLARE_PROOF} is not valid JSON"),
        ));
        return;
    };
    let Ok(thresholds) = fs::read_to_string(root.join(ROUND_E_THRESHOLDS)) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("could not read {ROUND_E_THRESHOLDS} for Cloudflare proof validation"),
        ));
        return;
    };
    let Ok(fixture_text) = fs::read_to_string(root.join(ROUND_E_FIXTURE)) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("could not read {ROUND_E_FIXTURE} for Cloudflare proof validation"),
        ));
        return;
    };
    let fixture = parse_round_e_fixture(&fixture_text);

    require_json_str(
        &artifact,
        "/proof_class",
        "round-e-cloudflare-material-proof",
        findings,
    );
    require_json_str(&artifact, "/status", "pass", findings);
    require_json_bool(&artifact, "/cache_buster/bumped", true, findings);
    require_json_bool(&artifact, "/wasm/checksum_matches_build", true, findings);
    if artifact
        .pointer("/errors")
        .and_then(Value::as_array)
        .is_none_or(|errors| !errors.is_empty())
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_CLOUDFLARE_PROOF} must record an empty errors array"),
        ));
    }

    let Some(canonical_fixture) = fixture.get("matte").or_else(|| fixture.values().next()) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_FIXTURE} has no preset entries for Cloudflare proof validation"),
        ));
        return;
    };
    for (artifact_pointer, fixture_key) in [
        ("/fixture/environment_hdr_path", "environment_hdr_path"),
        ("/fixture/environment_hdr_sha256", "environment_hdr_sha256"),
        ("/fixture/tonemapper", "tonemapper"),
        ("/fixture/output_color_space", "output_color_space"),
    ] {
        if let Some(expected) = canonical_fixture
            .get(fixture_key)
            .map(|value| unquote(value))
        {
            require_json_str(&artifact, artifact_pointer, &expected, findings);
        }
    }
    if let Some(expected) = canonical_fixture
        .get("exposure_ev")
        .and_then(|value| value.parse::<f64>().ok())
    {
        require_json_f64(&artifact, "/fixture/exposure_ev", expected, findings);
    }
    let sample_floor = artifact
        .pointer("/fixture/webgl2_smooth_metal_sample_floor")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if sample_floor < 96 {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!(
                "{ROUND_E_CLOUDFLARE_PROOF} webgl2_smooth_metal_sample_floor {sample_floor} is below 96"
            ),
        ));
    }

    for preset in ROUND_E_PRESETS {
        validate_cloudflare_per_material(&artifact, &thresholds, root, preset, findings);
    }
    validate_cloudflare_neighbor_pairs(&artifact, &thresholds, findings);
    validate_cloudflare_hard_material_metrics(&artifact, &thresholds, findings);
}

fn round_e_material_parity_claimed_shipped(root: &Path) -> bool {
    fs::read_to_string(root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md"))
        .is_ok_and(|text| {
            text.contains("**Round E real-world material parity** — **[shipped")
                || text.contains("Status: **[shipped]** for complete real-world materials")
        })
}

fn validate_cloudflare_per_material(
    artifact: &Value,
    thresholds: &str,
    root: &Path,
    preset: &str,
    findings: &mut Vec<Finding>,
) {
    let Some(material) = artifact.pointer(&format!("/per_material/{preset}")) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_CLOUDFLARE_PROOF} is missing per_material.{preset}"),
        ));
        return;
    };
    let threshold = material_delta_threshold(thresholds, preset).unwrap_or(4.0) as f64;
    let delta = material
        .get("delta_e2000_vs_reference")
        .and_then(Value::as_f64)
        .unwrap_or(f64::INFINITY);
    if delta > threshold {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!(
                "{preset} delta_e2000_vs_reference {delta:.3} exceeds committed threshold {threshold:.3}"
            ),
        ));
    }
    if material.get("reference_delta_gate").and_then(Value::as_str) != Some("hard") {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset} reference_delta_gate must be hard in public Cloudflare proof"),
        ));
    }
    if material
        .get("passed_reference_delta")
        .and_then(Value::as_bool)
        != Some(true)
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset} passed_reference_delta must be true in public Cloudflare proof"),
        ));
    }
    if let Some(crop_path) = material.get("crop_path").and_then(Value::as_str)
        && !root.join(crop_path).is_file()
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset} Cloudflare crop PNG is missing at {crop_path}"),
        ));
    }
}

fn validate_cloudflare_neighbor_pairs(
    artifact: &Value,
    thresholds: &str,
    findings: &mut Vec<Finding>,
) {
    let threshold =
        parse_threshold_value(thresholds, "global.neighbor_delta_e2000_min").unwrap_or(6.0) as f64;
    let pairs = artifact
        .pointer("/neighbor_pairs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (a, b) in ROUND_E_NEIGHBOR_PAIRS {
        let Some(pair) = pairs.iter().find(|entry| json_pair_matches(entry, a, b)) else {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("{ROUND_E_CLOUDFLARE_PROOF} is missing neighbor pair {a}/{b}"),
            ));
            continue;
        };
        let delta = pair
            .get("delta_e2000")
            .and_then(Value::as_f64)
            .unwrap_or(f64::NEG_INFINITY);
        if delta < threshold {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!(
                    "{a}/{b} neighbor_delta_e2000 {delta:.3} is below committed threshold {threshold:.3}"
                ),
            ));
        }
        if pair.get("passed").and_then(Value::as_bool) != Some(true) {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("{a}/{b} neighbor pair must record passed=true"),
            ));
        }
    }
}

fn json_pair_matches(entry: &Value, a: &str, b: &str) -> bool {
    let Some(pair) = entry.get("pair").and_then(Value::as_array) else {
        return false;
    };
    if pair.len() != 2 {
        return false;
    }
    let first = pair[0].as_str();
    let second = pair[1].as_str();
    (first == Some(a) && second == Some(b)) || (first == Some(b) && second == Some(a))
}

fn validate_cloudflare_hard_material_metrics(
    artifact: &Value,
    thresholds: &str,
    findings: &mut Vec<Finding>,
) {
    require_metric_min(
        artifact,
        thresholds,
        "chrome",
        "specular_dynamic_range",
        "chrome.specular_dynamic_range",
        findings,
    );
    require_metric_max(
        artifact,
        thresholds,
        "chrome",
        "luminance_p05",
        "chrome.dark_reflection_luminance_p05_max",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "chrome",
        "luminance_p99",
        "chrome.bright_reflection_luminance_p99_min",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "brushed_steel",
        "anisotropy_aspect_ratio_ibl",
        "brushed_steel.anisotropy_aspect_ratio_ibl",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "clearcoat_plastic",
        "clearcoat_lobe_delta",
        "clearcoat_plastic.clearcoat_lobe_delta",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "leather",
        "texture_variance",
        "leather.texture_variance_min",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "rubber",
        "roughness_variance",
        "rubber.roughness_variance_min",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "satin",
        "sheen_width",
        "satin.sheen_width_min",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "clear_glass",
        "refraction_offset_px",
        "clear_glass.refraction_offset_min",
        findings,
    );
    require_material_bool(
        artifact,
        "clear_glass",
        "passed_physical_refraction",
        findings,
    );
    require_material_str(
        artifact,
        "clear_glass",
        "physical_refraction_status",
        "measured",
        findings,
    );
    require_metric_min(
        artifact,
        thresholds,
        "frosted_glass",
        "high_frequency_contrast_reduction",
        "frosted_glass.high_frequency_contrast_reduction_min",
        findings,
    );
    require_material_bool(
        artifact,
        "frosted_glass",
        "passed_high_frequency_contrast_reduction",
        findings,
    );
    require_material_str(
        artifact,
        "frosted_glass",
        "rough_transmission_status",
        "measured",
        findings,
    );
}

fn require_metric_min(
    artifact: &Value,
    thresholds: &str,
    preset: &str,
    field: &str,
    threshold_key: &str,
    findings: &mut Vec<Finding>,
) {
    let threshold =
        parse_threshold_value(thresholds, threshold_key).unwrap_or(f32::INFINITY) as f64;
    let value = artifact
        .pointer(&format!("/per_material/{preset}/{field}"))
        .and_then(Value::as_f64)
        .unwrap_or(f64::NEG_INFINITY);
    if value < threshold {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset}.{field} {value:.3} is below committed threshold {threshold:.3}"),
        ));
    }
}

fn require_metric_max(
    artifact: &Value,
    thresholds: &str,
    preset: &str,
    field: &str,
    threshold_key: &str,
    findings: &mut Vec<Finding>,
) {
    let threshold =
        parse_threshold_value(thresholds, threshold_key).unwrap_or(f32::NEG_INFINITY) as f64;
    let value = artifact
        .pointer(&format!("/per_material/{preset}/{field}"))
        .and_then(Value::as_f64)
        .unwrap_or(f64::INFINITY);
    if value > threshold {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset}.{field} {value:.3} is above committed threshold {threshold:.3}"),
        ));
    }
}

fn require_material_bool(artifact: &Value, preset: &str, field: &str, findings: &mut Vec<Finding>) {
    if artifact
        .pointer(&format!("/per_material/{preset}/{field}"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset}.{field} must be true in public Cloudflare proof"),
        ));
    }
}

fn require_material_str(
    artifact: &Value,
    preset: &str,
    field: &str,
    expected: &str,
    findings: &mut Vec<Finding>,
) {
    if artifact
        .pointer(&format!("/per_material/{preset}/{field}"))
        .and_then(Value::as_str)
        != Some(expected)
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset}.{field} must be {expected} in public Cloudflare proof"),
        ));
    }
}

fn require_json_str(artifact: &Value, pointer: &str, expected: &str, findings: &mut Vec<Finding>) {
    if artifact.pointer(pointer).and_then(Value::as_str) != Some(expected) {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_CLOUDFLARE_PROOF} {pointer} must be {expected}"),
        ));
    }
}

fn require_json_bool(artifact: &Value, pointer: &str, expected: bool, findings: &mut Vec<Finding>) {
    if artifact.pointer(pointer).and_then(Value::as_bool) != Some(expected) {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_CLOUDFLARE_PROOF} {pointer} must be {expected}"),
        ));
    }
}

fn require_json_f64(artifact: &Value, pointer: &str, expected: f64, findings: &mut Vec<Finding>) {
    let actual = artifact
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN);
    if (actual - expected).abs() > 0.001 {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_CLOUDFLARE_PROOF} {pointer} must be {expected}"),
        ));
    }
}

fn material_delta_threshold(thresholds: &str, preset: &str) -> Option<f32> {
    let preset_key = format!("{preset}.delta_e2000_max");
    parse_threshold_value(thresholds, &preset_key)
        .or_else(|| parse_threshold_value(thresholds, "global.reference_delta_e2000_max"))
}

fn parse_threshold_value(thresholds: &str, key: &str) -> Option<f32> {
    let (wanted_section, wanted_key) = key.split_once('.')?;
    let mut section = "";
    for raw_line in thresholds.lines() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            section = name.trim();
            continue;
        }
        if section == wanted_section {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if name.trim() == wanted_key {
                return value.trim().parse::<f32>().ok();
            }
        }
    }
    None
}

fn parse_round_e_fixture(fixture: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut current: Option<String> = None;
    let mut entries = BTreeMap::<String, BTreeMap<String, String>>::new();
    for raw_line in fixture.lines() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        if let Some(section) = line
            .strip_prefix("[[presets.")
            .and_then(|value| value.strip_suffix("]]"))
        {
            current = Some(section.to_string());
            entries.entry(section.to_string()).or_default();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(current) = current.as_ref() else {
            continue;
        };
        entries
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }
    entries
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}
