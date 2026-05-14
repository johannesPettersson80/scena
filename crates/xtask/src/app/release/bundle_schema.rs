use crate::app::prelude::*;

pub(crate) fn check_release_artifact_bundle(artifact_root: &Path, findings: &mut Vec<Finding>) {
    if !artifact_root.is_dir() {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("missing release artifact root {}", artifact_root.display()),
        ));
        return;
    }

    let mut files = Vec::new();
    if let Err(error) =
        collect_files_with_extensions(artifact_root, &["json", "ppm", "toml"], &mut files)
    {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not collect release artifacts: {error}"),
        ));
        return;
    }

    for suffix in REQUIRED_RELEASE_ARTIFACT_SUFFIXES {
        if !files.iter().any(|path| path_ends_with(path, suffix)) {
            findings.push(Finding::new(
                "RELEASE-READY-ARTIFACTS",
                format!("downloaded release artifacts are missing {suffix}"),
            ));
        }
    }

    for suffix in REQUIRED_PASSED_STATUS_ARTIFACT_SUFFIXES {
        let matches = files
            .iter()
            .filter(|path| path_ends_with(path, suffix))
            .collect::<Vec<_>>();
        if matches.is_empty() {
            continue;
        }
        for path in matches {
            require_json_status_passed(path, suffix, findings);
        }
    }

    for suffix in RELEASE_LANE_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_release_lane_artifact_file(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_native_gpu_render_proof(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            reject_stale_json_timestamp(path, suffix, findings);
        }
    }

    let expected_commit = release_artifact_commit_label(Path::new("."));
    for suffix in REQUIRED_JSON_COMMIT_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            reject_stale_json_commit(path, suffix, &expected_commit, findings);
        }
    }

    for suffix in REQUIRED_NON_CONSTANT_PPM_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            reject_constant_ppm_artifact(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_MEASURED_CAPABILITY_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            reject_unmeasured_capability_matrix_rows(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_BENCHMARK_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_benchmark_baseline_comparison_file(path, suffix, findings);
        }
    }

    for suffix in REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_rendered_output_screenshot_metadata_file(path, suffix, findings);
        }
    }

    check_required_visual_proof_artifacts(artifact_root, findings);
    check_release_review_artifacts(artifact_root, findings);
}

pub(crate) const REQUIRED_REVIEW_ROLES: &[&str] = &[
    "scena-rfc-reviewer",
    "scena-wgpu-architect",
    "scena-gltf-animation-reviewer",
    "scena-visual-quality-validator",
    "scena-api-ergonomics-reviewer",
    "scena-doctor-reviewer",
];

pub(crate) fn check_release_review_artifacts(artifact_root: &Path, findings: &mut Vec<Finding>) {
    // RELEASE-REVIEWS-PRESENT: per docs/specs/release-reviews.md, every release
    // requires one Markdown review report per configured subagent role under
    // reviews/<role>/<commit>.md, plus a sign-off TOML and findings register
    // (the sign-off + register are already wired through
    // REQUIRED_RELEASE_ARTIFACT_SUFFIXES). This validator covers the six per-role
    // .md files which the JSON+PPM scanner does not pick up.
    let reviews_root = artifact_root.join("reviews");
    if !reviews_root.is_dir() {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            format!(
                "missing release review root {}; see docs/specs/release-reviews.md",
                reviews_root.display()
            ),
        ));
        return;
    }
    for role in REQUIRED_REVIEW_ROLES {
        let role_dir = reviews_root.join(role);
        let report_paths: Vec<PathBuf> = match fs::read_dir(&role_dir) {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("md"))
                .collect(),
            Err(_) => Vec::new(),
        };
        if report_paths.is_empty() {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                format!(
                    "missing release review report under reviews/{role}/<commit>.md; \
                     see docs/specs/release-reviews.md"
                ),
            ));
            continue;
        }
        for report_path in &report_paths {
            validate_release_review_report(role, report_path, findings);
        }
    }

    // Schema validation for findings.json (scena.release.findings.v1) and
    // maintainer-signoff.toml. These two single-file artifacts are required by
    // REQUIRED_RELEASE_ARTIFACT_SUFFIXES; here we additionally fail-close when
    // they exist but don't satisfy the schema docs/specs/release-reviews.md
    // documents.
    let findings_path = reviews_root.join("findings.json");
    if let Ok(text) = fs::read_to_string(&findings_path) {
        validate_findings_register_schema(&text, findings);
    }
    let signoff_path = reviews_root.join("maintainer-signoff.toml");
    if let Ok(text) = fs::read_to_string(&signoff_path) {
        validate_maintainer_signoff_schema(&text, findings);
    }
}

pub(crate) fn validate_findings_register_schema(text: &str, findings: &mut Vec<Finding>) {
    let parsed: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                format!(
                    "reviews/findings.json is not valid JSON ({error}); see \
                     docs/specs/release-reviews.md Section 2"
                ),
            ));
            return;
        }
    };
    let object = match parsed.as_object() {
        Some(object) => object,
        None => {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                "reviews/findings.json top-level must be a JSON object; see \
                 docs/specs/release-reviews.md Section 2",
            ));
            return;
        }
    };
    if object.get("schema").and_then(|value| value.as_str()) != Some("scena.release.findings.v1") {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            "reviews/findings.json must declare schema = \"scena.release.findings.v1\"; \
             see docs/specs/release-reviews.md Section 2",
        ));
    }
    if object
        .get("reviewed_commit")
        .and_then(|value| value.as_str())
        .is_none()
    {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            "reviews/findings.json must record reviewed_commit as a string; see \
             docs/specs/release-reviews.md Section 2",
        ));
    }
    if object
        .get("generated_at")
        .and_then(|value| value.as_str())
        .is_none()
    {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            "reviews/findings.json must record generated_at as an RFC 3339 string; \
             see docs/specs/release-reviews.md Section 2",
        ));
    }
    let entries = match object.get("findings").and_then(|value| value.as_array()) {
        Some(array) => array,
        None => {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                "reviews/findings.json must record findings as a JSON array; see \
                 docs/specs/release-reviews.md Section 2",
            ));
            return;
        }
    };
    for (index, entry) in entries.iter().enumerate() {
        let entry_object = match entry.as_object() {
            Some(object) => object,
            None => {
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "reviews/findings.json findings[{index}] must be a JSON object; \
                         see docs/specs/release-reviews.md Section 2"
                    ),
                ));
                continue;
            }
        };
        for required in [
            "id",
            "role",
            "summary",
            "severity",
            "status",
            "evidence",
            "notes",
            "deferral_target",
        ] {
            if !entry_object.contains_key(required) {
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "reviews/findings.json findings[{index}] is missing required \
                         field {required:?}; see docs/specs/release-reviews.md Section 2"
                    ),
                ));
            }
        }
        let status = entry_object.get("status").and_then(|value| value.as_str());
        if let Some("deferred") = status {
            let target = entry_object.get("deferral_target");
            let target_is_null = matches!(target, Some(serde_json::Value::Null) | None);
            if target_is_null {
                let entry_id = entry_object
                    .get("id")
                    .and_then(|value| value.as_str())
                    .unwrap_or("<unknown>");
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "reviews/findings.json finding {entry_id:?} has status = \"deferred\" \
                         but deferral_target is null; see docs/specs/release-reviews.md \
                         Section 2"
                    ),
                ));
            }
        }
    }
}

pub(crate) fn validate_maintainer_signoff_schema(text: &str, findings: &mut Vec<Finding>) {
    for required in [
        "[maintainer]",
        "name = ",
        "signed_commit = ",
        "[approval]",
        "decision = ",
    ] {
        if !text.contains(required) {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                format!(
                    "reviews/maintainer-signoff.toml is missing required schema field \
                     {required:?}; see docs/specs/release-reviews.md Section 3"
                ),
            ));
        }
    }
    if text.contains("decision = \"hold\"") {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            "reviews/maintainer-signoff.toml has decision = \"hold\"; release-readiness \
             cannot pass while the maintainer holds; see docs/specs/release-reviews.md \
             Section 3",
        ));
    }
    let decision = scrape_toml_string_value(text, "decision");
    let all_clear = scrape_toml_bool_value(text, "all_clear");
    if let (Some("approve"), Some(false)) = (decision.as_deref(), all_clear) {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            "reviews/maintainer-signoff.toml records decision = \"approve\" while \
             all_clear = false; see docs/specs/release-reviews.md Section 3 — \
             decision = \"approve\" requires all_clear = true",
        ));
    }
    if let Ok(expected_commit) = std::env::var("SCENA_RELEASE_COMMIT") {
        let signed_commit = scrape_toml_string_value(text, "signed_commit");
        match signed_commit.as_deref() {
            Some(actual) if actual == expected_commit => {}
            Some(actual) => {
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "reviews/maintainer-signoff.toml signed_commit = {actual:?} does \
                         not match SCENA_RELEASE_COMMIT = {expected_commit:?}; see \
                         docs/specs/release-reviews.md Section 3"
                    ),
                ));
            }
            None => {
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "reviews/maintainer-signoff.toml signed_commit cannot be parsed; \
                         expected match against SCENA_RELEASE_COMMIT = {expected_commit:?}; \
                         see docs/specs/release-reviews.md Section 3"
                    ),
                ));
            }
        }
    }
}

pub(crate) fn scrape_toml_string_value(text: &str, key: &str) -> Option<String> {
    let needle = format!("{key} = ");
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&needle) {
            let value = rest.trim().trim_matches(|c: char| c == '"' || c == '\'');
            return Some(value.to_string());
        }
    }
    None
}

pub(crate) fn scrape_toml_bool_value(text: &str, key: &str) -> Option<bool> {
    let needle = format!("{key} = ");
    for line in text.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&needle) {
            return match rest.trim() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
        }
    }
    None
}
