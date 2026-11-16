use crate::app::prelude::*;

const RULE: &str = "GLTF-IMPORT-CONFORMANCE";

fn conformance_fixture(name: &str) -> PathBuf {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions").join(name);
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/assets/gltf/external.rs",
        "src/scene/import/nodes.rs",
        "src/geometry/morph.rs",
        "src/assets/gltf/meshes.rs",
        "src/assets/gltf/meshes/primitive_modes.rs",
        "src/assets/gc.rs",
        "src/assets/texture_fetch.rs",
        "docs/assets.md",
        "tests/gltf_import_hardening.rs",
        "tests/gltf_import_conformance.rs",
        "tests/c04_gltf_deformation_contracts.rs",
        "tests/m8_stale_handle_proof.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("conformance fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("conformance fixture copies");
    }
    fixture_root
}

fn conformance_findings(fixture_root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_gltf_import_conformance(fixture_root, &mut findings);
    findings
}

/// Rewrites one fixture file and returns the findings doctor then reports.
fn findings_after(name: &str, relative: &str, from: &str, to: &str) -> Vec<Finding> {
    let fixture_root = conformance_fixture(name);
    let path = fixture_root.join(relative);
    let source = fs::read_to_string(&path).expect("conformance fixture reads");
    let mutated = source.replace(from, to);
    assert_ne!(source, mutated, "the mutation must change {relative}");
    fs::write(&path, mutated).expect("conformance mutation writes");
    conformance_findings(&fixture_root)
}

#[test]
fn gltf_conformance_doctor_accepts_the_importer_as_committed() {
    let fixture_root = conformance_fixture("gltf-conformance-clean");
    assert_eq!(conformance_findings(&fixture_root), Vec::new());
}

#[test]
fn gltf_conformance_doctor_rejects_a_strip_that_stops_alternating_its_winding() {
    let findings = findings_after(
        "gltf-conformance-strip-winding",
        "src/assets/gltf/meshes/primitive_modes.rs",
        "if triangle % 2 == 0 { [1, 2] } else { [2, 1] }",
        "[1, 2]",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == RULE
                && finding.message.contains("primitive_modes.rs")
                && finding.message.contains("triangle % 2")
        }),
        "a strip without winding alternation must fail doctor: {findings:?}",
    );
}

#[test]
fn gltf_conformance_doctor_rejects_asset_gc_that_forgets_variant_materials() {
    let findings = findings_after(
        "gltf-conformance-variant-roots",
        "src/assets/gc.rs",
        "mesh.material_variant_bindings()",
        "mesh.material_bindings()",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == RULE
                && finding.message.contains("src/assets/gc.rs")
                && finding.message.contains("material_variant_bindings")
        }),
        "asset GC that stops rooting variant materials must fail doctor: {findings:?}",
    );
}

#[test]
fn gltf_conformance_doctor_rejects_a_removed_conformance_test() {
    let findings = findings_after(
        "gltf-conformance-missing-test",
        "tests/gltf_import_conformance.rs",
        "fn a_node_with_a_mesh_and_a_light_instantiates_both()",
        "fn a_node_with_a_mesh_instantiates()",
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == RULE
                && finding
                    .message
                    .contains("a_node_with_a_mesh_and_a_light_instantiates_both")
        }),
        "a removed conformance test must fail doctor: {findings:?}",
    );
}

#[test]
fn gltf_conformance_doctor_rejects_the_old_mode_rejection() {
    let fixture_root = conformance_fixture("gltf-conformance-mode-rejection");
    let meshes = fixture_root.join("src/assets/gltf/meshes.rs");
    let mut source = fs::read_to_string(&meshes).expect("meshes fixture reads");
    source.push_str("\nconst OLD: &str = \"unsupported glTF primitive mode\";\n");
    fs::write(&meshes, source).expect("mode rejection mutation writes");

    let findings = conformance_findings(&fixture_root);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == RULE && finding.message.contains("unsupported glTF primitive mode")
        }),
        "bringing the blanket mode rejection back must fail doctor: {findings:?}",
    );
}
