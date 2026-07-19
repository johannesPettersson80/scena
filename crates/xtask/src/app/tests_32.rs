use crate::app::prelude::*;

#[test]
fn q07_feature_ownership_rejects_unmapped_and_unproven_features() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q07-feature-ownership");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in ["docs/specs", "docs", "src", "tests"] {
        fs::create_dir_all(fixture_root.join(directory)).expect("Q07 fixture directory");
    }
    fs::write(
        fixture_root.join("Cargo.toml"),
        "[features]\ndefault = []\nreal = []\nghost = []\n",
    )
    .expect("Q07 manifest writes");
    fs::write(
        fixture_root.join("src/real.rs"),
        "pub fn real_feature() {}\n",
    )
    .expect("Q07 implementation writes");
    fs::write(
        fixture_root.join("tests/real.rs"),
        "#[test]\nfn real_feature_works() {}\n",
    )
    .expect("Q07 test writes");
    fs::write(fixture_root.join("docs/feature-flags.md"), "`real`\n")
        .expect("Q07 feature docs write");
    fs::write(
        fixture_root.join("docs/specs/feature-ownership.json"),
        serde_json::to_string_pretty(&json!({
            "schema": "scena.feature_ownership.v1",
            "features": [{
                "name": "real",
                "owner": "assets",
                "implementation": {"path": "src/real.rs", "token": "real_feature"},
                "test": {"path": "tests/real.rs", "token": "real_feature_works"},
                "documentation": {"path": "docs/feature-flags.md", "token": "`real`"}
            }]
        }))
        .expect("Q07 weak ownership JSON serializes"),
    )
    .expect("Q07 weak ownership JSON writes");

    let mut findings = Vec::new();
    check_feature_ownership_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q07-FEATURE-OWNERSHIP" && finding.message.contains("ghost")
        }),
        "unmapped Cargo feature must fail closed: {findings:?}"
    );

    fs::write(fixture_root.join("src/ghost.rs"), "// missing call site\n")
        .expect("Q07 ghost source writes");
    fs::write(
        fixture_root.join("tests/ghost.rs"),
        "#[test]\nfn ghost_feature_works() {}\n",
    )
    .expect("Q07 ghost test writes");
    fs::write(
        fixture_root.join("docs/feature-flags.md"),
        "`real`\n`ghost`\n",
    )
    .expect("Q07 complete docs write");
    fs::write(
        fixture_root.join("docs/specs/feature-ownership.json"),
        serde_json::to_string_pretty(&json!({
            "schema": "scena.feature_ownership.v1",
            "features": [
                {
                    "name": "real",
                    "owner": "assets",
                    "implementation": {"path": "src/real.rs", "token": "real_feature"},
                    "test": {"path": "tests/real.rs", "token": "real_feature_works"},
                    "documentation": {"path": "docs/feature-flags.md", "token": "`real`"}
                },
                {
                    "name": "ghost",
                    "owner": "assets",
                    "implementation": {"path": "src/ghost.rs", "token": "ghost_feature"},
                    "test": {"path": "tests/ghost.rs", "token": "ghost_feature_works"},
                    "documentation": {"path": "docs/feature-flags.md", "token": "`ghost`"}
                }
            ]
        }))
        .expect("Q07 complete ownership JSON serializes"),
    )
    .expect("Q07 complete ownership JSON writes");

    findings.clear();
    check_feature_ownership_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q07-FEATURE-OWNERSHIP"
                && finding.message.contains("ghost")
                && finding.message.contains("implementation")
        }),
        "mapped feature without its implementation token must fail: {findings:?}"
    );

    fs::write(
        fixture_root.join("src/ghost.rs"),
        "pub fn ghost_feature() {}\n",
    )
    .expect("Q07 strong ghost source writes");
    findings.clear();
    check_feature_ownership_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());
}

#[test]
fn q07_claim_truth_rejects_false_features_and_missing_live_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q07-claim-truth");
    let _ = fs::remove_dir_all(&fixture_root);
    for directory in [
        ".github/workflows",
        "docs",
        "src/capture",
        "src/render/quality/tests",
        "tests",
    ] {
        fs::create_dir_all(fixture_root.join(directory)).expect("Q07 fixture directory");
    }
    for (relative, contents) in [
        ("Cargo.toml", "[dependencies]\nlcms2 = \"6\"\n"),
        ("Cargo.lock", ""),
        ("README.md", "feature claims\n"),
        ("docs/feature-flags.md", "feature docs\n"),
        ("src/render/quality/tests.rs", "mod frame_reference;\n"),
        (
            "src/render/quality/tests/frame_reference.rs",
            "#[test]\nfn committed_minimal_product_quality_fixture_replaces_external_review_data() {}\n",
        ),
        (
            "tests/scena_cli_recipe.rs",
            "fn scena_recipe_render_verify_accepts_live_ssim_reference_and_rejects_scene_mutations() { let _ = (\"camera\", \"material\", \"geometry\", \"reference_ssim_too_low\"); }\n",
        ),
        (
            "src/capture/png.rs",
            "#[cfg(target_arch = \"wasm32\")]\nfn write_png() { let _ = \"unsupported on wasm32\"; }\n",
        ),
        (
            ".github/workflows/ci.yml",
            "cargo check --target wasm32-unknown-unknown --all-features\n",
        ),
        (
            ".github/workflows/release.yml",
            "cargo check --target wasm32-unknown-unknown --all-features\n",
        ),
    ] {
        fs::write(fixture_root.join(relative), contents).expect("Q07 fixture file writes");
    }

    let mut findings = Vec::new();
    check_q07_claim_truth_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q07-CLAIM-TRUTH"
                && finding.message.contains("Cargo.toml")
                && finding.message.contains("lcms2")
        }),
        "dependency-only ICC claim must fail closed: {findings:?}"
    );

    fs::write(fixture_root.join("Cargo.toml"), "[dependencies]\n")
        .expect("Q07 corrected manifest writes");
    findings.clear();
    check_q07_claim_truth_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());
}
