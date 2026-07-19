use crate::app::prelude::*;

#[test]
pub(crate) fn c10_overlay_doctor_rejects_non_transitive_generated_child_removal() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut current_findings = Vec::new();
    check_c10_overlay_ownership_contracts(&root, &mut current_findings);
    assert!(
        current_findings
            .iter()
            .all(|finding| finding.rule != "SCENE-C10"),
        "current C10 contracts must satisfy doctor before mutation: {current_findings:?}",
    );

    let fixture_root = root.join("target/xtask-doctor-regressions/c10-overlay-closure");
    let files = [
        "src/scene.rs",
        "src/scene/removal.rs",
        "src/scene/overlay_ownership.rs",
        "src/scene/callouts.rs",
        "src/scene/measurements.rs",
        "src/scene_host/core.rs",
        "tests/c10_overlay_ownership.rs",
        "docs/api.md",
    ];
    for relative in files {
        let source = root.join(relative);
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("C10 fixture parent"))
            .expect("C10 fixture directory");
        fs::copy(source, destination).expect("copy C10 doctor fixture file");
    }

    let removal = fixture_root.join("src/scene/removal.rs");
    let source = fs::read_to_string(&removal).expect("read C10 removal fixture");
    let mutated = source.replace(
        "        self.expand_overlay_removal_closure(&mut removed);\n",
        "",
    );
    assert_ne!(
        mutated, source,
        "C10 mutation must remove closure expansion"
    );
    fs::write(&removal, mutated).expect("remove overlay closure expansion");
    let mut findings = Vec::new();

    check_c10_overlay_ownership_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "SCENE-C10"
                && finding.message.contains("src/scene/removal.rs")
                && finding.message.contains("expand_overlay_removal_closure")
        }),
        "doctor must reject removal that can orphan an owned sibling: {findings:?}",
    );
}
