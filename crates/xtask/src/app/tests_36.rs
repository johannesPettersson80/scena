use crate::app::prelude::*;

#[test]
fn pf06_doctor_rejects_restored_brute_force_spatial_loops() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf06-spatial-acceleration");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/geometry.rs",
        "src/geometry/spatial.rs",
        "src/picking.rs",
        "src/picking/geometry_hit.rs",
        "src/render/prepare.rs",
        "src/render/prepare/shadows.rs",
        "src/render/prepare/shadows/cache.rs",
        "tests/pf06_spatial_acceleration.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("PF06 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("PF06 source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf06_spatial_acceleration_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let picking = fixture_root.join("src/picking.rs");
    let mut source = fs::read_to_string(&picking).expect("picking fixture reads");
    source.push_str("\n// mutation: for indices in geometry.indices().chunks_exact(3) {}\n");
    fs::write(picking, source).expect("picking mutation writes");
    findings.clear();
    check_pf06_spatial_acceleration_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF06-SHARED-SPATIAL-ACCELERATION"
                && finding.message.contains("chunks_exact(3)")
        }),
        "restoring brute-force triangle scanning must fail: {findings:?}"
    );
}

#[test]
fn pf09_doctor_rejects_restored_all_triangles_per_band_scan() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf09-parallel-work");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/parallel.rs",
        "src/render/prepare/environment_baker.rs",
        "src/render/prepare/environment_baker/brdf.rs",
        "src/render/cpu.rs",
        "src/render/cpu_render.rs",
        "src/render/cpu_render/row_bands.rs",
        "tests/m9_platform_release.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("PF09 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("PF09 source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf09_parallel_work_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let raster = fixture_root.join("src/render/cpu_render.rs");
    let source = fs::read_to_string(&raster).expect("CPU raster fixture reads");
    let mutated = source.replace(
        "primitive_indices: Some(&row_bands.bands[chunk_index])",
        "primitive_indices: None",
    );
    assert_ne!(source, mutated, "PF09 mutation must alter the fixture");
    fs::write(raster, mutated).expect("PF09 mutation writes");
    findings.clear();
    check_pf09_parallel_work_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF09-DETERMINISTIC-PARALLEL-WORK"
                && finding.message.contains("primitive_indices")
        }),
        "restoring every-triangle band scans must fail: {findings:?}"
    );
}

#[test]
fn renderer_owned_interior_mutability_is_not_misclassified_as_a_singleton() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/render-singleton-scan");
    let source_path = fixture_root.join("src/render/cache.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source_path.parent().expect("fixture file has parent"))
        .expect("singleton fixture directory creates");
    fs::write(
        &source_path,
        "use std::cell::RefCell;\nstruct Cache { values: RefCell<Vec<u32>> }\nimpl Cache { fn new() -> Self { Self { values: RefCell::new(Vec::new()) } } }\n",
    )
    .expect("renderer-owned cache fixture writes");

    let mut findings = Vec::new();
    check_render_singleton_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    fs::write(
        source_path,
        "use std::cell::RefCell;\nstatic CACHE: RefCell<Vec<u32>> = RefCell::new(Vec::new());\n",
    )
    .expect("global singleton mutation writes");
    findings.clear();
    check_render_singleton_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-SINGLETON"
                && finding.message.contains("src/render/cache.rs:2")
        }),
        "global render state must still fail: {findings:?}"
    );
}

#[test]
fn positive_contract_pins_follow_owned_split_module_files() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/split-module-contract-pin");
    let parent = fixture_root.join("src/render.rs");
    let child = fixture_root.join("src/render/frame.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(child.parent().expect("split module parent exists"))
        .expect("split module fixture directory creates");
    fs::write(&parent, "mod frame;\n").expect("split module owner writes");
    fs::write(&child, "pub fn retained_render_contract() {}\n").expect("split module child writes");

    let mut findings = Vec::new();
    require_contains(
        &fixture_root,
        &mut findings,
        "SPLIT-MODULE-PIN",
        "src/render.rs",
        &["retained_render_contract"],
    );
    assert_eq!(findings, Vec::new());

    fs::write(child, "pub fn removed_contract() {}\n").expect("split module mutation writes");
    findings.clear();
    require_contains(
        &fixture_root,
        &mut findings,
        "SPLIT-MODULE-PIN",
        "src/render.rs",
        &["retained_render_contract"],
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SPLIT-MODULE-PIN"),
        "removing a child-module contract must still fail: {findings:?}"
    );
}
