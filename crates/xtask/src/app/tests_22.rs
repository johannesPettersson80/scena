use crate::app::prelude::*;

#[test]
pub(crate) fn c11_onboarding_doctor_rejects_stale_dependency_and_missing_compile_gate() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut current_findings = Vec::new();
    check_c11_onboarding_contracts(&root, &mut current_findings);
    assert!(
        current_findings
            .iter()
            .all(|finding| finding.rule != "DOCS-C11-ONBOARDING"),
        "current C11 contracts must satisfy doctor before mutation: {current_findings:?}",
    );

    let fixture_root = root.join("target/xtask-doctor-regressions/c11-onboarding");
    let _ = fs::remove_dir_all(&fixture_root);
    let files = [
        "Cargo.toml",
        "README.md",
        "docs/getting-started.md",
        "docs/feature-flags.md",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        "src/lib.rs",
        ".github/workflows/ci.yml",
        "tests/c11_onboarding_docs.rs",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C11 fixture parent"))
            .expect("C11 fixture directory");
        fs::copy(source, destination).expect("copy C11 doctor fixture file");
    }

    let readme = fixture_root.join("README.md");
    let source = fs::read_to_string(&readme).expect("read README fixture");
    fs::write(&readme, format!("{source}\nscena = \"1.5\"\n"))
        .expect("inject stale dependency pin");
    let workflow = fixture_root.join(".github/workflows/ci.yml");
    let source = fs::read_to_string(&workflow).expect("read workflow fixture");
    fs::write(
        &workflow,
        source.replace("cargo test --doc", "cargo test --lib"),
    )
    .expect("remove doctest compile gate");
    let mut findings = Vec::new();

    check_c11_onboarding_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-C11-ONBOARDING"
                && finding.message.contains("README.md")
                && finding.message.contains("workspace version is 1.10.4")
        }),
        "doctor must reject a stale numeric public dependency: {findings:?}",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-C11-ONBOARDING"
                && finding.message.contains(".github/workflows/ci.yml")
                && finding.message.contains("cargo test --doc")
        }),
        "doctor must reject removal of the explicit snippet compile gate: {findings:?}",
    );
}

#[test]
pub(crate) fn reverse_status_doctor_rejects_lowercase_historical_aliases() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c11-reverse-status-alias");
    let _ = fs::remove_dir_all(&fixture_root);
    let authority = fixture_root.join("docs/checklists/stunning-renders-and-performance.md");
    let duplicate = fixture_root.join("docs/checklists/current-roadmap.md");
    let source = fixture_root.join("src/render/prepare/lighting/tiled.rs");
    fs::create_dir_all(authority.parent().expect("authority fixture parent"))
        .expect("checklist fixture directory");
    fs::create_dir_all(source.parent().expect("source fixture parent"))
        .expect("source fixture directory");
    fs::write(
        &authority,
        "## B2 — Clustered / tiled light culling — [shipped]\n",
    )
    .expect("authority fixture writes");
    fs::write(
        &duplicate,
        "Roadmap closeout: [deferred]\nclustered/tiled culling now points to a future backend lane.\n",
    )
    .expect("duplicate fixture writes");
    fs::write(&source, "fn collect_tiled_light_assignment() {}\n").expect("source fixture writes");
    let mut findings = Vec::new();

    check_shipped_feature_status_drift(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-REVERSE-STATUS-DRIFT"
                && finding.message.contains("Clustered / tiled light culling")
        }),
        "doctor must reject lowercase historical aliases for shipped features: {findings:?}",
    );
}
