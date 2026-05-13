use crate::app::prelude::*;

#[test]
pub(crate) fn doctor_rejects_environment_lifecycle_missing_handle_regression() {
    // ARCH-ENVIRONMENT-LIFECYCLE (handle): src/render.rs must store the bound
    // EnvironmentHandle alongside the revision counter so reload propagation
    // stays observable. The earlier batch covered the revision counter; this
    // fixture ensures the typed-handle field cannot disappear silently.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/environment-lifecycle-handle-stub");
    let render_path = fixture_root.join("src/render.rs");
    fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
    fs::write(
        &render_path,
        "pub struct Renderer { pub environment_revision: u64 }\n",
    )
    .expect("render fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render.rs",
        &["environment: Option<EnvironmentHandle>"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ENVIRONMENT-LIFECYCLE"
                && finding.message.contains("EnvironmentHandle")
        }),
        "doctor must reject Renderer types that drop the typed EnvironmentHandle \
         field: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_test_first_agents_governance_regression() {
    // TEST-FIRST-AGENTS: AGENTS.md must keep the Unit-Test-First Rule + the
    // "fail for the expected reason" + the "name the test-first proof" governance
    // text so contributors do not regress to write-then-test.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/test-first-agents-stub");
    let agents_path = fixture_root.join("AGENTS.md");
    fs::create_dir_all(fixture_root.as_path()).expect("fixture dir");
    fs::write(&agents_path, "# AGENTS\n\nNo test-first rule here.\n").expect("agents fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "TEST-FIRST-AGENTS",
        "AGENTS.md",
        &[
            "## Unit Test First Rule",
            "Run the focused test and confirm it fails for the expected reason",
            "Do not mark a checklist implementation item complete without naming the test-first proof",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TEST-FIRST-AGENTS" && finding.message.contains("Unit Test First Rule")
        }),
        "doctor must reject AGENTS.md that drops the unit-test-first governance \
         contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_default_environment_manifest_missing_field_regression() {
    // VISUAL-DEFAULT-ENV: tests/assets/environment/default-environment.toml must
    // declare name = "neutral-studio" (and the rest of the manifest contract).
    // A stub without the canonical name regresses the rule.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/default-environment-stub");
    let manifest_path = fixture_root.join("tests/assets/environment/default-environment.toml");
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("fixture dir");
    fs::write(&manifest_path, "# placeholder default environment\n").expect("manifest fixture");
    let mut findings = Vec::new();

    check_default_environment_manifest(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VISUAL-DEFAULT-ENV"),
        "doctor must reject the default-environment manifest when its required \
         fields are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m2_leak_stats_missing_counters_regression() {
    // ARCH-M2-LEAK-STATS: tests/m2_lighting_depth_clipping.rs must keep the
    // resource-lifetime baseline test that watches environment cubemaps,
    // shadow maps, depth pre-pass counters, and pending destructions.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m2-leak-stats-stub");
    let m2_path = fixture_root.join("tests/m2_lighting_depth_clipping.rs");
    fs::create_dir_all(m2_path.parent().expect("m2 parent")).expect("fixture dir");
    fs::write(&m2_path, "// no leak baseline test here\n").expect("m2 fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-M2-LEAK-STATS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "m2_resource_counters_return_to_baseline_after_empty_prepare",
            "environment_cubemaps",
            "shadow_maps",
            "depth_prepass_passes",
            "released.pending_destructions, baseline.pending_destructions",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-M2-LEAK-STATS" && finding.message.contains("environment_cubemaps")
        }),
        "doctor must reject m2 lighting test that drops the resource-lifetime \
         counter contract: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_camera_depth_missing_module_export_regression() {
    // ARCH-CAMERA-DEPTH (module): src/scene.rs must keep the public
    // `mod camera;` declaration plus the `pub use camera::{Camera, ...}`
    // re-export so the typed cameras stay accessible without leaking the
    // module path.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/camera-depth-module-stub");
    let scene_path = fixture_root.join("src/scene.rs");
    fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
    fs::write(&scene_path, "pub struct Scene {}\n").expect("scene fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene.rs",
        &[
            "mod camera;",
            "pub use camera::{Camera, DepthRange, OrthographicCamera, PerspectiveCamera}",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-CAMERA-DEPTH" && finding.message.contains("pub use camera")
        }),
        "doctor must reject scene.rs that drops the camera module re-export: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_module_boundaries_missing_renderer_no_fetch_clause_regression() {
    // ARCH-MODULES: module-boundaries.md must keep the "no hidden asset fetch,
    // shader compile, or first-time GPU upload inside render()" clause so the
    // module ownership rules stay anchored in the spec.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/module-boundaries-stub");
    let module_path = fixture_root.join("docs/specs/module-boundaries.md");
    fs::create_dir_all(module_path.parent().expect("module-boundaries parent"))
        .expect("fixture dir");
    fs::write(
        &module_path,
        "# Module Boundaries\n\nNo render-fetch clause here.\n",
    )
    .expect("module-boundaries fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-MODULES",
        "docs/specs/module-boundaries.md",
        &[
            "`scene`",
            "`assets`",
            "`render`",
            "No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-MODULES" && finding.message.contains("No hidden asset fetch")
        }),
        "doctor must reject module-boundaries.md that drops the render-no-fetch \
         clause: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_render_alpha_capabilities_field_regression() {
    // ARCH-RENDER-ALPHA (capability field): capabilities.rs must keep
    // pub alpha_pipeline: AlphaPipelineStatus on Capabilities so backends can
    // structurally distinguish LinearSourceOver from BackendPassthrough.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/render-alpha-field-stub");
    let capabilities_path = fixture_root.join("src/diagnostics/capabilities.rs");
    fs::create_dir_all(capabilities_path.parent().expect("capabilities parent"))
        .expect("fixture dir");
    fs::write(
        &capabilities_path,
        "pub enum AlphaPipelineStatus { LinearSourceOver, BackendPassthrough }\npub struct Capabilities {}\n",
    )
    .expect("capabilities fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-ALPHA",
        "src/diagnostics/capabilities.rs",
        &["pub alpha_pipeline: AlphaPipelineStatus"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-ALPHA" && finding.message.contains("pub alpha_pipeline")
        }),
        "doctor must reject Capabilities that drops the alpha_pipeline field: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_render_alpha_missing_linear_frame_path_regression() {
    // ARCH-RENDER-ALPHA (CPU path): src/render.rs must keep the
    // linear_frame: Option<Vec<Color>> field plus the cpu::clear_cpu and
    // cpu::draw_primitive_cpu calls so CPU-rasterised alpha blending happens
    // in linear space before the output stage. A stub Renderer that drops
    // the field regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/render-alpha-linear-frame-stub");
    let render_path = fixture_root.join("src/render.rs");
    fs::create_dir_all(render_path.parent().expect("render parent")).expect("fixture dir");
    fs::write(&render_path, "pub struct Renderer { pub frame: Vec<u8> }\n")
        .expect("render fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-ALPHA",
        "src/render.rs",
        &[
            "linear_frame: Option<Vec<Color>>",
            "cpu::clear_cpu",
            "cpu::draw_primitive_cpu",
        ],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-ALPHA" && finding.message.contains("linear_frame")
        }),
        "doctor must reject Renderer that drops the linear_frame CPU alpha-blend \
         path: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_test_first_doctor_contract_missing_governance_regression() {
    // TEST-FIRST-DOCTOR-CONTRACT: docs/specs/doctor-contract.md must keep the
    // unit-test-first governance reference so the doctor's own contract spec
    // stays anchored to the AGENTS rule.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/test-first-doctor-contract-stub");
    let doctor_contract_path = fixture_root.join("docs/specs/doctor-contract.md");
    fs::create_dir_all(
        doctor_contract_path
            .parent()
            .expect("doctor contract parent"),
    )
    .expect("fixture dir");
    fs::write(
        &doctor_contract_path,
        "# Doctor Contract\n\nStub spec without the test-first anchor text.\n",
    )
    .expect("doctor contract fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "TEST-FIRST-DOCTOR-CONTRACT",
        "docs/specs/doctor-contract.md",
        &["unit-test-first governance"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TEST-FIRST-DOCTOR-CONTRACT"
                && finding.message.contains("unit-test-first governance")
        }),
        "doctor must reject doctor-contract.md that drops the unit-test-first \
         governance anchor: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_release_artifact_root_without_review_directory() {
    // RELEASE-REVIEWS-PRESENT: an artifact root without a reviews/ subtree must
    // surface a missing-review-root finding. This is the simplest end-to-end
    // proof that check_release_review_artifacts wires into the release-readiness
    // bundle validator alongside the existing suffix and JSON-status checks.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/release-reviews-missing-root");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding.message.contains("missing release review root")
        }),
        "release-readiness must reject an artifact bundle without a reviews/ \
         subtree: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_release_review_directory_missing_per_role_report() {
    // RELEASE-REVIEWS-PRESENT: when reviews/ exists but a configured role lacks
    // any .md report, the validator must surface a per-role finding so the
    // operator knows exactly which agent did not file.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/release-reviews-missing-role");
    let reviews_root = fixture_root.join("reviews");
    // Create five of six role directories with one .md each; leave the sixth
    // (scena-doctor-reviewer) without an .md so the validator reports a missing
    // report for that role.
    for role in [
        "scena-rfc-reviewer",
        "scena-wgpu-architect",
        "scena-gltf-animation-reviewer",
        "scena-visual-quality-validator",
        "scena-api-ergonomics-reviewer",
    ] {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(role_dir.join("placeholder.md"), "# placeholder review\n")
            .expect("placeholder review");
    }
    // Leave scena-doctor-reviewer absent.
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding.message.contains("scena-doctor-reviewer")
        }),
        "release-readiness must reject an artifact bundle missing a per-role \
         review report: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_findings_register_missing_schema_field() {
    // RELEASE-REVIEWS-PRESENT (schema): findings.json must declare the
    // scena.release.findings.v1 schema and the reviewed_commit field. A stub
    // register that drops the schema regresses the contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/release-reviews-findings-stub");
    let reviews_root = fixture_root.join("reviews");
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(role_dir.join("placeholder.md"), "# placeholder review\n")
            .expect("placeholder review");
    }
    fs::write(
        reviews_root.join("findings.json"),
        "{\"reviewed_commit\": \"abc\", \"findings\": []}\n",
    )
    .expect("findings fixture");
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding.message.contains("scena.release.findings.v1")
        }),
        "release-readiness must reject findings.json that drops the schema field: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_findings_register_with_invalid_json_regression() {
    // RELEASE-REVIEWS-PRESENT (JSON parse): findings.json must be valid
    // JSON; a malformed register fails closed even when the substring
    // "scena.release.findings.v1" happens to be present.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-reviews-findings-invalid-json");
    let reviews_root = fixture_root.join("reviews");
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(role_dir.join("placeholder.md"), "---\nrole: bogus\nreviewed_commit: abc\nsession_id: o\ndate: 2026-05-09\nblocker_status: clear\nfindings_count: 0\n---\n# placeholder\n")
            .expect("placeholder review");
    }
    fs::write(
        reviews_root.join("findings.json"),
        "{\"schema\": \"scena.release.findings.v1\", broken json,",
    )
    .expect("malformed findings");
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding
                    .message
                    .contains("reviews/findings.json is not valid JSON")
        }),
        "release-readiness must reject a malformed findings.json file: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_findings_register_finding_missing_field_regression() {
    // RELEASE-REVIEWS-PRESENT (per-finding fields): every finding object
    // in findings.json must declare id, role, summary, severity, status,
    // evidence, notes, deferral_target. A finding missing any of those
    // fails closed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-reviews-findings-missing-fields");
    let reviews_root = fixture_root.join("reviews");
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(role_dir.join("placeholder.md"), "---\nrole: bogus\nreviewed_commit: abc\nsession_id: o\ndate: 2026-05-09\nblocker_status: clear\nfindings_count: 0\n---\n# placeholder\n")
            .expect("placeholder review");
    }
    fs::write(
        reviews_root.join("findings.json"),
        "{\"schema\":\"scena.release.findings.v1\",\"reviewed_commit\":\"abc\",\
         \"generated_at\":\"2026-05-09T00:00:00Z\",\"findings\":[{\"id\":\"F1\"}]}",
    )
    .expect("findings missing fields");
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding
                    .message
                    .contains("findings[0] is missing required field \"severity\"")
        }),
        "release-readiness must reject a finding object missing required \
         severity field: {findings:?}",
    );
}
