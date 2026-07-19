use crate::app::prelude::*;

pub(super) fn copy_and_validate_required_reviews(
    files: &[PathBuf],
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    copy_required_review_reports(files, output, expected_commit)?;
    validate_staged_review_bundle(output, expected_commit)
}

fn copy_required_review_reports(
    files: &[PathBuf],
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    for role in REQUIRED_REVIEW_ROLES {
        let suffix = format!("reviews/{role}/{expected_commit}.md");
        copy_one_review_source(files, output, &suffix)?;
    }
    for suffix in ["reviews/findings.json", "reviews/maintainer-signoff.toml"] {
        copy_one_review_source(files, output, suffix)?;
    }
    Ok(())
}

fn copy_one_review_source(files: &[PathBuf], output: &Path, suffix: &str) -> Result<(), String> {
    let matches = files
        .iter()
        .filter(|path| path_ends_with(path, suffix))
        .collect::<Vec<_>>();
    let source = match matches.as_slice() {
        [] => {
            return Err(format!(
                "RELEASE-REVIEWS-MISSING: downloaded release artifacts are missing required \
                 independent review evidence {suffix}"
            ));
        }
        [source] => *source,
        _ => {
            return Err(format!(
                "RELEASE-REVIEWS-AMBIGUOUS: downloaded release artifacts contain {} copies of \
                 required review evidence {suffix}; reviewer evidence must have one \
                 unambiguous source",
                matches.len()
            ));
        }
    };
    let target = output.join(suffix);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::copy(source, &target).map_err(|error| {
        format!(
            "failed to copy independent review evidence {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

fn validate_staged_review_bundle(output: &Path, expected_commit: &str) -> Result<(), String> {
    let mut findings = Vec::new();
    check_release_review_artifacts(output, &mut findings);
    if !findings.is_empty() {
        return Err(format!(
            "RELEASE-REVIEWS-INTEGRITY: downloaded release review evidence is invalid: {}",
            findings
                .iter()
                .map(|finding| format!("{}: {}", finding.rule, finding.message))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    validate_review_bundle_integrity(output, expected_commit)
}

pub(super) fn validate_review_bundle_integrity(
    output: &Path,
    expected_commit: &str,
) -> Result<(), String> {
    let signoff_path = output.join("reviews/maintainer-signoff.toml");
    let signoff_text = fs::read_to_string(&signoff_path)
        .map_err(|error| format!("failed to read {}: {error}", signoff_path.display()))?;
    let mut reviewer_identities = BTreeSet::new();
    for role in REQUIRED_REVIEW_ROLES {
        let relative = format!("reviews/{role}/{expected_commit}.md");
        let report_path = output.join(&relative);
        let text = fs::read_to_string(&report_path)
            .map_err(|error| format!("failed to read staged review {relative}: {error}"))?;
        let frontmatter = parse_release_review_frontmatter(&text)
            .ok_or_else(|| format!("staged review {relative} has no frontmatter"))?;
        let reviewed_commit = frontmatter.get("reviewed_commit").cloned();
        if reviewed_commit.as_deref() != Some(expected_commit) {
            return Err(format!(
                "staged review {relative} records reviewed_commit={reviewed_commit:?}, expected \
                {expected_commit:?}"
            ));
        }
        if frontmatter.get("blocker_status").map(String::as_str) != Some("clear") {
            return Err(format!(
                "staged review {relative} is not blocker_status=clear"
            ));
        }
        let identity = frontmatter
            .get("reviewer_identity")
            .ok_or_else(|| format!("staged review {relative} is missing reviewer_identity"))?;
        validate_human_identity(identity, &format!("reviewer for {role}"))?;
        if !reviewer_identities.insert(identity.clone()) {
            return Err(format!(
                "reviewer identity {identity:?} is reused across required review roles"
            ));
        }
        let provenance = frontmatter
            .get("reviewer_provenance")
            .ok_or_else(|| format!("staged review {relative} is missing reviewer_provenance"))?;
        if !provenance.starts_with("https://") {
            return Err(format!(
                "staged review {relative} reviewer_provenance must be an independently \
                 verifiable https URL"
            ));
        }
        let hash_key = format!("{}_sha256", role.replace('-', "_"));
        let expected_hash = scrape_toml_string_value(&signoff_text, &hash_key)
            .ok_or_else(|| format!("maintainer sign-off is missing {hash_key}"))?;
        let actual_hash = sha256_hex(&report_path).map_err(|error| error.to_string())?;
        if expected_hash != actual_hash {
            return Err(format!(
                "maintainer sign-off hash for review {role} is stale or tampered: expected \
                 {expected_hash}, actual {actual_hash}"
            ));
        }
    }

    let findings_path = output.join("reviews/findings.json");
    let findings_text = fs::read_to_string(&findings_path)
        .map_err(|error| format!("failed to read {}: {error}", findings_path.display()))?;
    let findings_value = serde_json::from_str::<Value>(&findings_text)
        .map_err(|error| format!("failed to parse {}: {error}", findings_path.display()))?;
    if findings_value
        .get("reviewed_commit")
        .and_then(Value::as_str)
        != Some(expected_commit)
    {
        return Err(format!(
            "staged reviews/findings.json does not target expected commit {expected_commit:?}"
        ));
    }
    for entry in findings_value
        .get("findings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        let status = entry.get("status").and_then(Value::as_str).unwrap_or("");
        let severity = entry.get("severity").and_then(Value::as_str).unwrap_or("");
        let closed = matches!(status, "fixed" | "closed" | "resolved");
        if matches!(severity, "blocker" | "critical") && !closed {
            return Err(format!(
                "release finding {id:?} remains open with severity {severity:?} and status \
                 {status:?}"
            ));
        }
        if entry
            .get("history")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            return Err(format!(
                "release finding {id:?} is missing non-empty status history"
            ));
        }
    }

    if scrape_toml_string_value(&signoff_text, "signed_commit").as_deref() != Some(expected_commit)
    {
        return Err(format!(
            "staged reviews/maintainer-signoff.toml does not sign expected commit \
            {expected_commit:?}"
        ));
    }
    let maintainer_identity = scrape_toml_string_value(&signoff_text, "identity")
        .ok_or_else(|| "maintainer sign-off is missing identity".to_string())?;
    validate_human_identity(&maintainer_identity, "maintainer")?;
    if reviewer_identities.contains(&maintainer_identity) {
        return Err(format!(
            "maintainer identity {maintainer_identity:?} must be distinct from required reviewers"
        ));
    }
    if scrape_toml_bool_value(&signoff_text, "all_clear") != Some(true)
        || scrape_toml_string_value(&signoff_text, "decision").as_deref() != Some("approve")
    {
        return Err(
            "maintainer sign-off must explicitly record all_clear=true and decision=approve"
                .to_string(),
        );
    }
    let expected_findings_hash = scrape_toml_string_value(&signoff_text, "findings_sha256")
        .ok_or_else(|| "maintainer sign-off is missing findings_sha256".to_string())?;
    let actual_findings_hash = sha256_hex(&findings_path).map_err(|error| error.to_string())?;
    if expected_findings_hash != actual_findings_hash {
        return Err(format!(
            "maintainer sign-off findings hash is stale or tampered: expected \
             {expected_findings_hash}, actual {actual_findings_hash}"
        ));
    }
    Ok(())
}

fn validate_human_identity(identity: &str, subject: &str) -> Result<(), String> {
    let normalized = identity.trim().to_ascii_lowercase();
    if !normalized.starts_with("github:") || normalized.len() <= "github:".len() {
        return Err(format!(
            "{subject} identity {identity:?} must use a verifiable github:<login> identity"
        ));
    }
    if ["automation", "github-actions", "release-bot", "[bot]"]
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return Err(format!(
            "{subject} identity {identity:?} is automation, not an independent human reviewer"
        ));
    }
    Ok(())
}
