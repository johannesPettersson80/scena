use crate::app::prelude::*;

const ROUND_E_FAILING_BASELINE: &str = "tests/visual/references/round_e_failing_baseline.json";
const ROUND_E_FAILING_BASELINE_IMAGE: &str =
    "tests/visual/references/round_e_failing_baseline_glossy_grid.png";
const ROUND_E_FAILING_BASELINE_IMAGE_SHA256: &str =
    "32d99960699b6b05fb3888e9d8fd57af3d07a27cacb5a94214d0b0b9f0ba589c";

pub(super) fn check_material_presets_failing_baseline(root: &Path, findings: &mut Vec<Finding>) {
    let baseline_path = root.join(ROUND_E_FAILING_BASELINE);
    let image_path = root.join(ROUND_E_FAILING_BASELINE_IMAGE);
    let Ok(text) = fs::read_to_string(&baseline_path) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("could not read {ROUND_E_FAILING_BASELINE}"),
        ));
        return;
    };
    match sha256_hex(&image_path) {
        Ok(actual) if actual == ROUND_E_FAILING_BASELINE_IMAGE_SHA256 => {}
        Ok(actual) => findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!(
                "Round E failing-baseline image SHA mismatch: expected \
                 {ROUND_E_FAILING_BASELINE_IMAGE_SHA256}, actual {actual}"
            ),
        )),
        Err(error) => findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("could not hash {ROUND_E_FAILING_BASELINE_IMAGE}: {error}"),
        )),
    }

    let Ok(artifact) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_FAILING_BASELINE} is not valid JSON"),
        ));
        return;
    };
    if artifact["proof_class"] != "round-e-failing-baseline" {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_FAILING_BASELINE} must declare round-e-failing-baseline proof_class"),
        ));
    }
    if artifact["source_image"] != ROUND_E_FAILING_BASELINE_IMAGE
        || artifact["source_sha256"] != ROUND_E_FAILING_BASELINE_IMAGE_SHA256
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_FAILING_BASELINE} must pin the old glossy grid path and SHA"),
        ));
    }
    if !artifact["metric"]
        .as_str()
        .is_some_and(|metric| metric.contains("CIEDE2000"))
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_FAILING_BASELINE} must record the CIEDE2000 metric"),
        ));
    }
    let minimum_margin = artifact["minimum_failure_margin_delta_e2000"]
        .as_f64()
        .unwrap_or(0.0);
    if artifact["threshold_source"] != "tests/visual/references/round_e_material_thresholds.toml"
        || minimum_margin < 1.0
    {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!(
                "{ROUND_E_FAILING_BASELINE} must pin per-preset thresholds and failure margin >= 1.0"
            ),
        ));
    }
    let declared_minimum_failures = artifact["minimum_failed_presets"]
        .as_u64()
        .unwrap_or(3)
        .max(3);
    let failed = artifact["failed_presets"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let meaningful_failures = failed
        .iter()
        .filter(|entry| {
            let delta = entry["delta_e2000_vs_reference"].as_f64().unwrap_or(0.0);
            let threshold = entry["threshold_delta_e2000_max"]
                .as_f64()
                .unwrap_or(f64::INFINITY);
            let margin = entry["failure_margin_delta_e2000"].as_f64().unwrap_or(0.0);
            delta >= threshold + minimum_margin && margin >= minimum_margin
        })
        .count();
    if meaningful_failures < declared_minimum_failures as usize {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!(
                "{ROUND_E_FAILING_BASELINE} must show at least {declared_minimum_failures} meaningful failures"
            ),
        ));
    }
    for required in ["chrome", "brushed_steel", "clear_glass"] {
        let present = failed.iter().any(|entry| {
            entry["preset"] == required
                && entry["threshold_delta_e2000_max"].as_f64().is_some()
                && entry["failure_margin_delta_e2000"].as_f64().unwrap_or(0.0) >= minimum_margin
        });
        if !present {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("{ROUND_E_FAILING_BASELINE} must prove old glossy grid fails {required}"),
            ));
        }
    }
}
