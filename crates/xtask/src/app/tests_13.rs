use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_visual_browser_m1_missing_artifact_regression() {
    // VISUAL-BROWSER-M1: each browser-probe workflow must declare its
    // visual artifact under `target/gate-artifacts/m6-browser-visual/`
    // with a renderer/color/tolerance/source contract; absence regresses
    // the M6 browser parity gate.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-browser-m1-stub");
    let stub_path = fixture_root.join("src/browser_probe/workflows/pbr.rs");
    fs::create_dir_all(stub_path.parent().expect("workflow parent")).expect("fixture dir");
    fs::write(
        &stub_path,
        "// Stub workflow without the visual-artifact declarations.\n",
    )
    .expect("workflow fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "VISUAL-BROWSER-M1",
        "src/browser_probe/workflows/pbr.rs",
        &["pbr-environment-lit", "renderer", "tolerance"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VISUAL-BROWSER-M1"),
        "doctor must reject browser-probe workflows that drop their visual \
         artifact declarations: {findings:?}",
    );
}
