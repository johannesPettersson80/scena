use crate::app::prelude::*;

#[test]
fn c12_doctor_rejects_a_second_recoverable_surface_retry() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c12-surface-acquisition");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/gpu/surface_frame.rs",
        "src/render/gpu/draw.rs",
        "src/render/gpu/draw_surface.rs",
        "src/render/gpu/draw_surface_support.rs",
        "src/render/frame.rs",
        "src/render/frame/surface.rs",
        "src/diagnostics.rs",
        "src/diagnostics/stats.rs",
        "src/scene_host/reporting.rs",
        "README.md",
        "docs/lifecycle.md",
        "docs/platforms.md",
        "docs/specs/release-gates.md",
        "docs/errors.md",
        "docs/api.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C12 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C12 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_full_review_surface_acquisition_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let policy = fixture_root.join("src/render/gpu/surface_frame.rs");
    let source = fs::read_to_string(&policy).expect("C12 policy source reads");
    let mutated = source.replace(
        "SurfaceAcquireAction::FailAfterRetry(status)",
        "SurfaceAcquireAction::ReconfigureAndRetry",
    );
    assert_ne!(
        source, mutated,
        "C12 mutation must permit an unbounded retry"
    );
    fs::write(policy, mutated).expect("C12 policy mutation writes");
    findings.clear();
    check_full_review_surface_acquisition_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C12-SURFACE-ACQUISITION"
                && finding.message.contains("FailAfterRetry(status)")
        }),
        "unbounded recoverable surface retry must fail doctor: {findings:?}"
    );
}

#[test]
fn c12_doctor_rejects_msaa_depth_mismatch_and_missing_native_fault_detail() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c12-msaa-surface-depth");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/gpu/surface_frame.rs",
        "src/render/gpu/draw.rs",
        "src/render/gpu/draw_surface.rs",
        "src/render/gpu/draw_surface_support.rs",
        "src/render/frame.rs",
        "src/render/frame/surface.rs",
        "src/diagnostics.rs",
        "src/diagnostics/stats.rs",
        "src/scene_host/reporting.rs",
        "README.md",
        "docs/lifecycle.md",
        "docs/platforms.md",
        "docs/specs/release-gates.md",
        "docs/errors.md",
        "docs/api.md",
        "CHANGELOG.md",
        "docs/release-notes/v1.8.0.md",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C12 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C12 contract fixture copies");
    }

    let mut findings = Vec::new();
    check_full_review_surface_acquisition_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let draw = fixture_root.join("src/render/gpu/draw.rs");
    let source = fs::read_to_string(&draw).expect("C12 native draw source reads");
    let mutated = source.replace(
        "depth_view: surface_scene_depth_view,",
        "depth_view: resolved_depth_view,",
    );
    assert_ne!(
        source, mutated,
        "C12 mutation must bind resolved depth to the MSAA surface scene pass"
    );
    fs::write(&draw, mutated).expect("C12 native draw mutation writes");
    findings.clear();
    check_full_review_surface_acquisition_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C12-SURFACE-ACQUISITION"
                && finding.message.contains("surface_scene_depth_view")
        }),
        "resolved depth on the MSAA surface scene pass must fail doctor: {findings:?}"
    );

    fs::write(&draw, source).expect("C12 native draw source restores");
    let surface_frame = fixture_root.join("src/render/gpu/surface_frame.rs");
    let source = fs::read_to_string(&surface_frame).expect("C12 surface fault source reads");
    let native_fault_log = concat!(
        "#[cfg(not(target_arch = \"wasm32\"))]\n",
        "        eprintln!(\"scena wgpu uncaptured error: {error:?}\");",
    );
    let mutated = source.replace(
        native_fault_log,
        "let _ = format!(\"discarded native wgpu fault: {error:?}\");",
    );
    assert_ne!(
        source, mutated,
        "C12 mutation must discard native uncaptured-error detail"
    );
    fs::write(surface_frame, mutated).expect("C12 surface fault mutation writes");
    findings.clear();
    check_full_review_surface_acquisition_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C12-SURFACE-ACQUISITION"
                && finding.message.contains("scena wgpu uncaptured error")
        }),
        "discarding native uncaptured-error detail must fail doctor: {findings:?}"
    );
}

#[test]
fn q04_doctor_rejects_runtime_cross_builder_provenance_lookup() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q04-portable-source-provenance");
    let evidence = fixture_root.join("tests/c09_gpu_resource_lifecycle.rs");
    fs::create_dir_all(evidence.parent().expect("Q04 fixture has parent"))
        .expect("Q04 fixture directory creates");
    let source = fs::read_to_string(root.join("tests/c09_gpu_resource_lifecycle.rs"))
        .expect("Q04 evidence source reads");
    let mutated = source.replacen(
        "include_bytes!(\"../Cargo.lock\")",
        "std::fs::read(env!(\"CARGO_MANIFEST_DIR\")).expect(\"runtime provenance\")",
        1,
    );
    assert_ne!(
        source, mutated,
        "portable-provenance mutation must alter the Q04 evidence producer"
    );
    fs::write(evidence, mutated).expect("runtime Q04 provenance mutation writes");

    let mut findings = Vec::new();
    check_c09_gpu_resource_lifecycle_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && (finding
                    .message
                    .contains("include_bytes!(\"../Cargo.lock\")")
                    || finding.message.contains("CARGO_MANIFEST_DIR"))
        }),
        "doctor must reject runtime Q04 provenance lookup: {findings:?}",
    );
}
