use crate::app::prelude::*;

#[test]
pub(crate) fn release_readiness_rejects_deferred_finding_with_null_deferral_target_regression() {
    // RELEASE-REVIEWS-PRESENT (deferred-target invariant): a finding with
    // status = "deferred" must carry a non-null deferral_target so the
    // public-claim cross-reference is auditable. A null target fails
    // closed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-reviews-deferred-null-target");
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
         \"generated_at\":\"2026-05-09T00:00:00Z\",\"findings\":[{\"id\":\"F1\",\
         \"role\":\"r\",\"summary\":\"s\",\"severity\":\"minor\",\"status\":\"deferred\",\
         \"evidence\":[],\"notes\":\"n\",\"deferral_target\":null}]}",
    )
    .expect("deferred null-target findings");
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding
                    .message
                    .contains("status = \"deferred\" but deferral_target is null")
        }),
        "release-readiness must reject deferred findings with a null \
         deferral_target: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_signoff_with_approve_decision_when_not_all_clear_regression()
 {
    // RELEASE-REVIEWS-PRESENT (decision-approve-requires-all-clear):
    // maintainer-signoff.toml decision = "approve" only when
    // all_clear = true; an approve-with-not-clear sign-off fails closed
    // because shipping while a blocker is unresolved would re-introduce
    // the silent-failure family release-reviews.md mandates we close.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-reviews-approve-without-all-clear");
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
         \"generated_at\":\"2026-05-09T00:00:00Z\",\"findings\":[]}",
    )
    .expect("findings register fixture");
    fs::write(
        reviews_root.join("maintainer-signoff.toml"),
        "[maintainer]\nname = \"Jane Smith\"\nsigned_commit = \"abc\"\n\n\
         [reviews]\nall_clear = false\n\n[approval]\ndecision = \"approve\"\n",
    )
    .expect("signoff fixture");
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding
                    .message
                    .contains("decision = \"approve\" while all_clear = false")
        }),
        "release-readiness must reject decision = approve with all_clear = \
         false: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_review_report_missing_frontmatter_regression() {
    // RELEASE-REVIEWS-PRESENT (frontmatter): a per-role review report
    // without the documented frontmatter block fails closed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-reviews-missing-frontmatter");
    let reviews_root = fixture_root.join("reviews");
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(
            role_dir.join("placeholder.md"),
            "# placeholder review without frontmatter\n",
        )
        .expect("placeholder review");
    }
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding
                    .message
                    .contains("missing the release-reviews frontmatter block")
        }),
        "release-readiness must reject a per-role review report that drops the \
         release-reviews frontmatter block: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_review_report_with_findings_count_mismatch_regression() {
    // RELEASE-REVIEWS-PRESENT (counter): findings_count must match the
    // number of `### Finding` headings; a mismatch fails closed so the
    // register cannot drift away from the report it claims to mirror.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-reviews-findings-count-mismatch");
    let reviews_root = fixture_root.join("reviews");
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(
            role_dir.join("placeholder.md"),
            "---\n\
             role: bogus\n\
             reviewed_commit: abc\n\
             session_id: operator-local\n\
             date: 2026-05-09\n\
             blocker_status: clear\n\
             findings_count: 2\n\
             ---\n\
             \n\
             # placeholder\n\
             \n\
             ### Finding F1: only one\n\
             - Severity: nit\n\
             - Status: fixed\n\
             - Evidence: src/lib.rs\n\
             - Notes: only one heading\n",
        )
        .expect("placeholder review");
    }
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding.message.contains("findings_count=2")
                && finding.message.contains("1 `### Finding`")
        }),
        "release-readiness must reject a release-review report whose \
         findings_count disagrees with its `### Finding` heading count: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_review_report_finding_missing_severity_regression() {
    // RELEASE-REVIEWS-PRESENT (per-finding fields): each `### Finding`
    // must carry Severity, Status, Evidence, Notes lines; a finding
    // missing any of those fails closed.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/release-reviews-finding-missing-severity");
    let reviews_root = fixture_root.join("reviews");
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(
            role_dir.join("placeholder.md"),
            "---\n\
             role: bogus\n\
             reviewed_commit: abc\n\
             session_id: operator-local\n\
             date: 2026-05-09\n\
             blocker_status: findings-recorded\n\
             findings_count: 1\n\
             ---\n\
             \n\
             # placeholder\n\
             \n\
             ### Finding F1: missing severity\n\
             - Status: fixed\n\
             - Evidence: src/lib.rs\n\
             - Notes: severity line missing\n",
        )
        .expect("placeholder review");
    }
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding
                    .message
                    .contains("missing required field \"Severity:\"")
        }),
        "release-readiness must reject a release-review finding that drops \
         the Severity: line: {findings:?}",
    );
}

#[test]
pub(crate) fn release_readiness_rejects_signoff_with_hold_decision() {
    // RELEASE-REVIEWS-PRESENT (schema): maintainer-signoff.toml with
    // decision = "hold" must fail-close release-readiness so a withheld
    // sign-off cannot accidentally ship as approval.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/release-reviews-hold-signoff");
    let reviews_root = fixture_root.join("reviews");
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        fs::create_dir_all(&role_dir).expect("role dir");
        fs::write(role_dir.join("placeholder.md"), "# placeholder review\n")
            .expect("placeholder review");
    }
    fs::write(
        reviews_root.join("findings.json"),
        "{\"schema\": \"scena.release.findings.v1\", \"reviewed_commit\": \"abc\", \"findings\": []}\n",
    )
    .expect("findings fixture");
    fs::write(
        reviews_root.join("maintainer-signoff.toml"),
        "[maintainer]\nname = \"Jane Smith\"\nsigned_commit = \"abc\"\n\n[approval]\ndecision = \"hold\"\n",
    )
    .expect("signoff fixture");
    let mut findings = Vec::new();

    check_release_review_artifacts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-PRESENT"
                && finding.message.contains("decision = \"hold\"")
        }),
        "release-readiness must reject a maintainer sign-off whose decision is \
         hold: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_world_baked_prepare_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/world-baked-prepare");
    let prepare_path = fixture_root.join("src/render/prepare.rs");
    fs::create_dir_all(prepare_path.parent().expect("prepare parent")).expect("fixture dir");
    fs::write(
        &prepare_path,
        "fn collect() { let _ = transform_primitive(primitive, transform, origin_shift); }\n",
    )
    .expect("prepare fixture");
    let mut findings = Vec::new();

    forbid_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-WORLD-BAKE",
        "src/render/prepare.rs",
        &["transform_primitive(primitive, transform, origin_shift)"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-WORLD-BAKE"
                && finding.message.contains("transform_primitive")
        }),
        "doctor must reject prepare.rs that bakes per-renderable world transforms into \
         vertex positions instead of stamping them through prepared_primitive(...): \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_agents_md_missing_doctor_runbook_regression() {
    // AGENTS-VALIDATION: AGENTS.md must instruct contributors to run
    // `cargo run -p xtask -- doctor --full` and reference the
    // scena-doctor skill. A workspace whose AGENTS.md drops either
    // contract must surface a finding so the doctor entrypoint never
    // becomes invisible to new agents.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/agents-md-missing-doctor");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    fs::write(
        fixture_root.join("AGENTS.md"),
        "# Stub AGENTS\n\nContributors should run tests.\n",
    )
    .expect("agents stub");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "AGENTS-VALIDATION",
        "AGENTS.md",
        &["cargo run -p xtask -- doctor --full", "Use `scena-doctor`"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "AGENTS-VALIDATION"
                && finding
                    .message
                    .contains("cargo run -p xtask -- doctor --full")
        }),
        "doctor must reject AGENTS.md that drops the doctor runbook \
         reference: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_source_file_with_out_of_scope_term_regression() {
    // ARCH-SCOPE: scena is a renderer, not a domain engine. Source
    // files referencing domain-specific terms (plc, robotics, robot,
    // etc.) drift the project outside its non-goals. The fixture
    // writes a source file containing "plc" and asserts the
    // architecture doctor surfaces a renderer-forbidden-term finding.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/source-scope-out-of-scope");
    let src_dir = fixture_root.join("src");
    fs::create_dir_all(&src_dir).expect("src dir");
    fs::write(
        src_dir.join("foo.rs"),
        "// Wires plc telemetry into the renderer.\npub fn run() {}\n",
    )
    .expect("foo source");
    let mut findings = Vec::new();

    check_source_scope(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-SCOPE"
                && finding.message.contains("src/foo.rs")
                && finding.message.contains("plc")
        }),
        "doctor must reject source files containing renderer-forbidden \
         scope terms like 'plc': {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_required_module_layout_with_missing_files_regression() {
    // ARCH-REQUIRED: the architecture doctor must reject any workspace
    // checkout missing one of the canonical source modules listed in
    // `REQUIRED_SOURCE_MODULES`. We simulate a fresh-clone-with-missing-
    // files scenario by pointing `require_files` at an empty fixture
    // root and asserting the helper surfaces a per-path finding.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/required-modules-missing");
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    let mut findings = Vec::new();

    require_files(
        &fixture_root,
        &mut findings,
        "ARCH-REQUIRED",
        &["src/lib.rs", "src/render.rs"],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ARCH-REQUIRED"
                && finding.message.contains("src/lib.rs")
                && finding.message.contains("missing required file")),
        "doctor must reject a checkout missing src/lib.rs: {findings:?}",
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ARCH-REQUIRED"
                && finding.message.contains("src/render.rs")),
        "doctor must reject a checkout missing src/render.rs: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_markdown_with_stale_doc_terms_regression() {
    // DOCS-STALE-TERM: any markdown document containing "TODO", "FIXME",
    // "TBD", or other documented stale-author markers must surface a
    // per-file finding so doc rot can never silently land. Mirrors the
    // pattern exercised by `doctor_rejects_markdown_link_to_missing_target_regression`.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/markdown-stale-terms");
    let docs_dir = fixture_root.join("docs/specs");
    fs::create_dir_all(&docs_dir).expect("docs dir");
    fs::write(fixture_root.join("README.md"), "# Fixture readme\n").expect("readme stub");
    fs::write(fixture_root.join("AGENTS.md"), "# Fixture agents\n").expect("agents stub");
    fs::write(
        docs_dir.join("stale.md"),
        "# Stale-term fixture\n\nTODO: finish this document before shipping.\n",
    )
    .expect("stale-term fixture");
    let mut findings = Vec::new();

    check_for_stale_doc_terms(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-STALE-TERM"
                && finding.message.contains("stale.md")
                && finding.message.contains("TODO")
        }),
        "doctor must reject markdown documents that retain author-stale \
         markers like TODO/FIXME/TBD so doc rot cannot ship: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_markdown_link_to_missing_target_regression() {
    // DOCS-LINKS: a markdown link to a missing relative target must surface
    // a finding so broken intra-doc references can never silently ship.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/markdown-broken-link");
    let docs_dir = fixture_root.join("docs/specs");
    fs::create_dir_all(&docs_dir).expect("docs dir");
    fs::write(fixture_root.join("README.md"), "# Fixture readme\n").expect("readme stub");
    fs::write(fixture_root.join("AGENTS.md"), "# Fixture agents\n").expect("agents stub");
    fs::write(
        docs_dir.join("broken.md"),
        "# Broken link fixture\n\nSee [docs that do not exist](does-not-exist.md).\n",
    )
    .expect("broken-link fixture");
    let mut findings = Vec::new();

    check_markdown_links(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "DOCS-LINKS" && finding.message.contains("does-not-exist.md")
        }),
        "doctor must reject markdown documents that link to missing relative \
         targets so broken intra-doc references can never ship: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_material_desc_public_field_regression() {
    // ARCH-ASSET-API: src/material.rs MaterialDesc must keep its fields
    // private so the descriptor stays an opaque builder-only value.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/material-desc-public-field");
    let material_path = fixture_root.join("src/material.rs");
    fs::create_dir_all(material_path.parent().expect("src dir")).expect("fixture dir");
    fs::write(
        &material_path,
        "pub struct MaterialDesc {\n    pub leaked_field: u32,\n}\n",
    )
    .expect("material fixture");
    let mut findings = Vec::new();

    check_material_desc_fields_private(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-ASSET-API" && finding.message.contains("leaked_field")
        }),
        "doctor must reject src/material.rs MaterialDesc declaring a public \
         field so the descriptor stays an opaque builder-only value: {findings:?}",
    );
}
