use crate::app::prelude::*;

const RULE: &str = "Q07-FEATURE-OWNERSHIP";
const OWNERSHIP_PATH: &str = "docs/specs/feature-ownership.json";

pub(crate) fn check_feature_ownership_contracts(root: &Path, findings: &mut Vec<Finding>) {
    let manifest = match fs::read_to_string(root.join("Cargo.toml")) {
        Ok(text) => text,
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not read Cargo.toml: {error}"),
            ));
            return;
        }
    };
    let cargo_features = manifest_features(&manifest);
    let ownership_text = match fs::read_to_string(root.join(OWNERSHIP_PATH)) {
        Ok(text) => text,
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not read {OWNERSHIP_PATH}: {error}"),
            ));
            return;
        }
    };
    let ownership = match serde_json::from_str::<Value>(&ownership_text) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not parse {OWNERSHIP_PATH}: {error}"),
            ));
            return;
        }
    };
    if ownership.get("schema").and_then(Value::as_str) != Some("scena.feature_ownership.v1") {
        findings.push(Finding::new(
            RULE,
            format!("{OWNERSHIP_PATH} must use schema scena.feature_ownership.v1"),
        ));
    }
    let Some(entries) = ownership.get("features").and_then(Value::as_array) else {
        findings.push(Finding::new(
            RULE,
            format!("{OWNERSHIP_PATH} must contain a features array"),
        ));
        return;
    };
    let mut mapped = BTreeSet::new();
    for entry in entries {
        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if name.is_empty() {
            findings.push(Finding::new(RULE, "feature ownership entry has no name"));
            continue;
        }
        if !mapped.insert(name.to_owned()) {
            findings.push(Finding::new(
                RULE,
                format!("feature {name} has duplicate ownership entries"),
            ));
        }
        if entry
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .is_empty()
        {
            findings.push(Finding::new(
                RULE,
                format!("feature {name} has no owner module"),
            ));
        }
        check_reference(root, findings, name, entry, "implementation", false);
        check_reference(root, findings, name, entry, "test", true);
        check_reference(root, findings, name, entry, "documentation", false);
    }
    for feature in cargo_features.difference(&mapped) {
        findings.push(Finding::new(
            RULE,
            format!("Cargo feature {feature} has no ownership/proof entry"),
        ));
    }
    for feature in mapped.difference(&cargo_features) {
        findings.push(Finding::new(
            RULE,
            format!("ownership entry {feature} does not name a current Cargo feature"),
        ));
    }
}

pub(crate) fn check_q07_claim_truth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, forbidden) in [
        ("Cargo.toml", &["lcms2", "icc ="] as &[&str]),
        ("Cargo.lock", &["name = \"lcms2\"", "name = \"lcms2-sys\""]),
        ("README.md", &["`icc`", "lcms2"]),
        ("docs/feature-flags.md", &["`icc`", "lcms2"]),
        (
            "src/render/quality/tests/frame_reference.rs",
            &[
                "CARDINE_BROKEN_RENDER_ROOT",
                "#[ignore =",
                "cardine_terminal_block_review_images",
            ],
        ),
        (
            "src/render/quality/tests.rs",
            &["CARDINE_BROKEN_RENDER_ROOT", "cardine_broken_render_root"],
        ),
    ] {
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            findings.push(Finding::new(
                "Q07-CLAIM-TRUTH",
                format!("could not read {relative}"),
            ));
            continue;
        };
        for token in forbidden {
            if text.contains(token) {
                findings.push(Finding::new(
                    "Q07-CLAIM-TRUTH",
                    format!("{relative} retains forbidden unimplemented/external claim {token}"),
                ));
            }
        }
    }
    for (relative, required) in [
        (
            "tests/scena_cli_recipe.rs",
            &[
                "scena_recipe_render_verify_accepts_live_ssim_reference_and_rejects_scene_mutations",
                "camera",
                "material",
                "geometry",
                "reference_ssim_too_low",
            ] as &[&str],
        ),
        (
            "src/render/quality/tests/frame_reference.rs",
            &["committed_minimal_product_quality_fixture_replaces_external_review_data"],
        ),
        (
            "src/capture/png.rs",
            &[
                "cfg(target_arch = \"wasm32\")",
                "write_png",
                "unsupported on wasm32",
            ],
        ),
        (
            ".github/workflows/ci.yml",
            &["cargo check --target wasm32-unknown-unknown --all-features"],
        ),
        (
            ".github/workflows/release.yml",
            &["cargo check --target wasm32-unknown-unknown --all-features"],
        ),
    ] {
        let Ok(text) = fs::read_to_string(root.join(relative)) else {
            findings.push(Finding::new(
                "Q07-CLAIM-TRUTH",
                format!("could not read {relative}"),
            ));
            continue;
        };
        for token in required {
            if !text.contains(token) {
                findings.push(Finding::new(
                    "Q07-CLAIM-TRUTH",
                    format!("{relative} is missing required Q07 contract {token}"),
                ));
            }
        }
    }
}

fn manifest_features(manifest: &str) -> BTreeSet<String> {
    let mut features = BTreeSet::new();
    let mut in_features = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            continue;
        }
        if !in_features || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, _value)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name != "default" && !name.is_empty() {
            features.insert(name.to_owned());
        }
    }
    features
}

fn check_reference(
    root: &Path,
    findings: &mut Vec<Finding>,
    feature: &str,
    entry: &Value,
    field: &str,
    must_be_test: bool,
) {
    let reference = entry.get(field);
    let relative = reference
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let token = reference
        .and_then(|value| value.get("token"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        findings.push(Finding::new(
            RULE,
            format!("feature {feature} has invalid {field} path {relative}"),
        ));
        return;
    }
    let Ok(text) = fs::read_to_string(root.join(path)) else {
        findings.push(Finding::new(
            RULE,
            format!("feature {feature} {field} path {relative} is unreadable"),
        ));
        return;
    };
    if token.is_empty() || !text.contains(token) {
        findings.push(Finding::new(
            RULE,
            format!("feature {feature} {field} {relative} is missing token {token}"),
        ));
    }
    if must_be_test && !text.contains("#[test]") && !text.contains("#[wasm_bindgen_test") {
        findings.push(Finding::new(
            RULE,
            format!("feature {feature} test path {relative} contains no active test attribute"),
        ));
    }
}
