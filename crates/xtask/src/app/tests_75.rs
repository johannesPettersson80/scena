use crate::app::prelude::*;

#[test]
fn q12_doctor_rejects_stale_release_version_and_missing_current_doc() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();
    check_q12_semantic_doctor_contracts(&root, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q12 semantic doctor contract must satisfy doctor: {findings:?}"
    );

    let fixture = root.join("target/xtask-doctor-regressions/q12-current-release-docs");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q12 release-doc fixture");
    }
    for relative in [
        "Cargo.toml",
        CURRENT_RELEASE_NOTES,
        CURRENT_REVIEW_REPORT,
        CURRENT_REMEDIATION_CHECKLIST,
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q12 fixture parent");
        fs::copy(root.join(relative), destination).expect("copy Q12 fixture source");
    }
    check_current_release_document_version(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current release version fixture must pass: {findings:?}"
    );

    let manifest = fixture.join("Cargo.toml");
    let source = fs::read_to_string(&manifest).expect("Q12 manifest reads");
    let mutated = source.replacen("version = \"1.9.0\"", "version = \"1.9.1\"", 1);
    assert_ne!(
        source, mutated,
        "stale-version mutation must change package version"
    );
    fs::write(&manifest, mutated).expect("Q12 stale version mutation writes");
    findings.clear();
    check_current_release_document_version(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-CURRENT-RELEASE-VERSION"),
        "stale versioned docs must fail doctor: {findings:?}"
    );

    fs::write(&manifest, source).expect("Q12 manifest restores");
    fs::remove_file(fixture.join(CURRENT_REVIEW_REPORT))
        .expect("Q12 current review fixture removes");
    findings.clear();
    check_current_release_document_version(&fixture, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-CURRENT-RELEASE-VERSION"
                && finding.message.contains(CURRENT_REVIEW_REPORT)
        }),
        "missing current review document must fail doctor: {findings:?}"
    );
}

#[test]
fn q07_doctor_rejects_noop_antialiasing_oracle_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture = root.join("target/xtask-doctor-regressions/q07-antialiasing-effect");
    if fixture.exists() {
        fs::remove_dir_all(&fixture).expect("remove prior Q07 doctor fixture");
    }
    for relative in [
        "tests/q07_antialiasing_effect.rs",
        "tests/browser/pf01_output_toggle_validation.js",
        "scripts/build_windows_complete_hardware_bundle.sh",
        "scripts/run_windows_complete_hardware_proof.ps1",
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
    ] {
        let destination = fixture.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture parent"))
            .expect("create Q07 fixture parent");
        fs::copy(root.join(relative), destination).expect("copy Q07 source");
    }
    let mut findings = Vec::new();
    check_q07_antialiasing_effect_contract(&fixture, &mut findings);
    assert!(
        findings.is_empty(),
        "current Q07 effect proof must satisfy doctor: {findings:?}"
    );

    let native_oracle = fixture.join("tests/q07_antialiasing_effect.rs");
    let native_source = fs::read_to_string(&native_oracle).expect("read native Q07 oracle");
    let provenance_mutated = native_source.replace("std::env::var(\"GITHUB_SHA\").ok()", "None");
    assert_ne!(
        native_source, provenance_mutated,
        "Q07 mutation must remove the GitHub commit fallback"
    );
    fs::write(&native_oracle, provenance_mutated).expect("write Q07 provenance mutation");
    findings.clear();
    check_q07_antialiasing_effect_contract(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q07-ANTIALIASING-EFFECT"),
        "doctor must reject a Q07 artifact writer that loses CI commit provenance: \
         {findings:?}"
    );
    fs::write(&native_oracle, &native_source).expect("restore native Q07 oracle");

    let native_mutated = native_source.replace(
        ".saturating_add(baseline.hard_transition_count.saturating_mul(6))",
        ".max(baseline.hard_transition_count.saturating_mul(6))",
    );
    assert_ne!(
        native_source, native_mutated,
        "Q07 mutation must remove the baseline-relative edge budget"
    );
    fs::write(&native_oracle, native_mutated).expect("write Q07 edge-budget mutation");
    findings.clear();
    check_q07_antialiasing_effect_contract(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q07-ANTIALIASING-EFFECT"),
        "doctor must reject an absolute intermediate-pixel ceiling: {findings:?}"
    );
    fs::write(native_oracle, native_source).expect("restore native Q07 oracle");

    let bundle_builder = fixture.join("scripts/build_windows_complete_hardware_bundle.sh");
    let bundle_source = fs::read_to_string(&bundle_builder).expect("read Windows bundle builder");
    let bundle_mutated = bundle_source.replace(
        "cp tests/q07_antialiasing_effect.rs \"$bundle_root/tests/\"",
        "# omitted Q07 runtime checksum source",
    );
    assert_ne!(
        bundle_source, bundle_mutated,
        "Q07 mutation must remove the runtime checksum source"
    );
    fs::write(&bundle_builder, bundle_mutated).expect("write Q07 bundle mutation");
    findings.clear();
    check_q07_antialiasing_effect_contract(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q07-ANTIALIASING-EFFECT"),
        "doctor must reject a Windows bundle that omits the Q07 runtime checksum source: \
         {findings:?}"
    );
    fs::write(bundle_builder, bundle_source).expect("restore Windows bundle builder");

    let runner = fixture.join("scripts/run_windows_complete_hardware_proof.ps1");
    let runner_source = fs::read_to_string(&runner).expect("read Windows proof runner");
    let runner_mutated = runner_source.replace(
        "Copy-Item -Path (Join-Path $bundleRoot \"tests\\*.rs\")",
        "# omitted installed Q07 runtime checksum source",
    );
    assert_ne!(
        runner_source, runner_mutated,
        "Q07 mutation must remove the installer copy"
    );
    fs::write(&runner, runner_mutated).expect("write Q07 installer mutation");
    findings.clear();
    check_q07_antialiasing_effect_contract(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q07-ANTIALIASING-EFFECT"),
        "doctor must reject an installer that omits the Q07 runtime checksum source: \
         {findings:?}"
    );
    fs::write(runner, runner_source).expect("restore Windows proof runner");

    let evaluator = fixture.join("tests/browser/pf01_output_toggle_validation.js");
    let source = fs::read_to_string(&evaluator).expect("read Q07 evaluator");
    let mutated = source.replace("validateFxaaEffect", "acceptFxaaHashDifferenceOnly");
    assert_ne!(source, mutated, "Q07 mutation must remove effect evaluator");
    fs::write(evaluator, mutated).expect("write Q07 mutation");
    findings.clear();
    check_q07_antialiasing_effect_contract(&fixture, &mut findings);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "Q07-ANTIALIASING-EFFECT"),
        "doctor must reject hash-only FXAA evidence: {findings:?}"
    );
}

#[test]
fn windows_bundle_packages_and_installs_runtime_provenance_sources() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let builder =
        fs::read_to_string(root.join("scripts/build_windows_complete_hardware_bundle.sh"))
            .expect("Windows bundle builder reads");
    let runner = fs::read_to_string(root.join("scripts/run_windows_complete_hardware_proof.ps1"))
        .expect("Windows proof runner reads");

    for source in [
        "tests/q07_antialiasing_effect.rs",
        "tests/transmission_parity.rs",
        "tests/c13_depth_clipping_parity.rs",
        "tests/dynamic_transform_parity.rs",
        "tests/pbr_brdf_parity.rs",
        "tests/pf08_texture_bake_parity.rs",
    ] {
        assert!(
            builder.contains(source),
            "the Windows proof executables checksum {source} at runtime, so the complete \
             hardware bundle must carry that producer source"
        );
    }
    assert!(
        runner.contains("Copy-Item -Path (Join-Path $bundleRoot \"tests\\*.rs\")"),
        "the Windows proof installer must copy every manifest-bound Rust producer source into \
         ProofRoot before validating the installed workspace"
    );
    for source in [
        "tests/pbr_brdf_parity.rs",
        "tests/pf08_texture_bake_parity.rs",
    ] {
        let body = fs::read_to_string(root.join(source)).expect("portable parity source reads");
        assert!(
            !body.contains("PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"))"),
            "the cross-compiled Windows proof {source} must write artifacts relative to the \
             runtime ProofRoot, not the Linux builder path"
        );
    }
}
