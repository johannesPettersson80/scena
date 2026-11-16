use crate::app::prelude::*;

const RULE: &str = "GLTF-CHECKED-ACCESSOR-READS";

fn checked_reads_fixture(name: &str) -> PathBuf {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions").join(name);
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/assets/gltf.rs",
        "src/assets/gltf/typed_accessors.rs",
        "src/assets/gltf/buffers.rs",
        "src/assets/gltf/validation.rs",
        "src/assets/gltf/skins.rs",
        "tests/gltf_import_hardening.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("checked-reads fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("checked-reads fixture copies");
    }
    fixture_root
}

fn checked_reads_findings(fixture_root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_gltf_checked_accessor_reads(fixture_root, &mut findings);
    findings
}

#[test]
fn gltf_checked_reads_doctor_accepts_the_importer_as_committed() {
    let fixture_root = checked_reads_fixture("gltf-checked-reads-clean");
    assert_eq!(checked_reads_findings(&fixture_root), Vec::new());
}

#[test]
fn gltf_checked_reads_doctor_rejects_upstream_reader_calls() {
    let fixture_root = checked_reads_fixture("gltf-checked-reads-upstream-reader");
    let skins = fixture_root.join("src/assets/gltf/skins.rs");
    let mut source = fs::read_to_string(&skins).expect("skins fixture reads");
    source.push_str(
        "\nfn bypass(skin: &::gltf::Skin<'_>) {\n    let _ = skin.reader(|_| None).read_inverse_bind_matrices();\n}\n",
    );
    fs::write(&skins, source).expect("upstream reader mutation writes");

    let findings = checked_reads_findings(&fixture_root);
    for call in [".reader(", ".read_inverse_bind_matrices("] {
        assert!(
            findings.iter().any(|finding| {
                finding.rule == RULE
                    && finding.message.contains("src/assets/gltf/skins.rs")
                    && finding.message.contains(call)
            }),
            "an upstream `{call}` call in the importer must fail doctor: {findings:?}",
        );
    }
}

#[test]
fn gltf_checked_reads_doctor_rejects_raw_accessor_iterators_outside_the_checked_reader() {
    let fixture_root = checked_reads_fixture("gltf-checked-reads-raw-iterator");
    fs::write(
        fixture_root.join("src/assets/gltf/bypass.rs"),
        "use ::gltf::accessor::Iter as AccessorIter;\n",
    )
    .expect("raw iterator mutation writes");

    let findings = checked_reads_findings(&fixture_root);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == RULE
                && finding.message.contains("src/assets/gltf/bypass.rs")
                && finding.message.contains("raw accessor iterator")
        }),
        "a raw accessor iterator outside the checked reader must fail doctor: {findings:?}",
    );
}

#[test]
fn gltf_checked_reads_doctor_rejects_a_reader_that_stops_charging_the_budget() {
    let fixture_root = checked_reads_fixture("gltf-checked-reads-uncharged");
    let reader = fixture_root.join("src/assets/gltf/typed_accessors.rs");
    let source = fs::read_to_string(&reader).expect("checked reader fixture reads");
    let mutated = source.replace("buffers.charge_accessor_bytes(", "buffers.skip_budget(");
    assert_ne!(
        source, mutated,
        "the mutation must remove the budget reservation"
    );
    fs::write(&reader, mutated).expect("uncharged reader mutation writes");

    let findings = checked_reads_findings(&fixture_root);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == RULE && finding.message.contains("buffers.charge_accessor_bytes(")
        }),
        "a checked reader that stops reserving budget must fail doctor: {findings:?}",
    );
}
