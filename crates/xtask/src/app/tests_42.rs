use crate::app::prelude::*;

#[test]
fn c05_doctor_rejects_inward_cone_or_wedge_winding() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c05-primitive-winding");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/geometry/primitive_meshes.rs",
        "src/geometry/primitive_meshes/tests.rs",
        "tests/placeholder_regression.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C05 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C05 source fixture copies");
    }

    let mut findings = Vec::new();
    check_c05_primitive_winding_contract(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let primitives = fixture_root.join("src/geometry/primitive_meshes.rs");
    let source = fs::read_to_string(&primitives).expect("C05 primitive source reads");
    let mutated = source.replace(
        "triangle_normal(p0, tip, p1)",
        "triangle_normal(p0, p1, tip)",
    );
    assert_ne!(source, mutated, "C05 mutation must invert the cone normal");
    fs::write(primitives, mutated).expect("C05 primitive mutation writes");
    findings.clear();
    check_c05_primitive_winding_contract(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C05-OUTWARD-PRIMITIVE-WINDING"
                && finding.message.contains("triangle_normal")
        }),
        "restoring inward cone winding must fail doctor: {findings:?}"
    );
}
