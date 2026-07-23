use crate::app::prelude::*;

pub(crate) fn run_claim_audit() -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("CLAIM-AUDIT-ROOT", message)])?;
    let artifact =
        build_claim_audit(&root).map_err(|error| vec![Finding::new("CLAIM-AUDIT", error)])?;
    let artifact_dir = root.join("target/gate-artifacts");
    fs::create_dir_all(&artifact_dir).map_err(|error| {
        vec![Finding::new(
            "CLAIM-AUDIT",
            format!("failed to create target/gate-artifacts: {error}"),
        )]
    })?;
    let artifact_path = artifact_dir.join("m10-claim-audit.json");
    let body = serde_json::to_string_pretty(&artifact)
        .map_err(|error| vec![Finding::new("CLAIM-AUDIT", error.to_string())])?;
    fs::write(&artifact_path, format!("{body}\n")).map_err(|error| {
        vec![Finding::new(
            "CLAIM-AUDIT",
            format!("failed to write {}: {error}", artifact_path.display()),
        )]
    })?;
    println!("{}", artifact_path.display());
    Ok(())
}

pub(crate) fn check_release_readiness(root: &Path, findings: &mut Vec<Finding>) {
    run_docs_doctor(root, findings);
    run_architecture_doctor(root, findings);
    check_release_readiness_adr(root, findings);
    check_release_readiness_checklists(root, findings);
}

pub(crate) fn check_release_readiness_adr(root: &Path, findings: &mut Vec<Finding>) {
    let rel = CURRENT_RELEASE_NOTES;
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        findings.push(Finding::new(
            "RELEASE-READY-M10",
            format!("could not read {rel}"),
        ));
        return;
    };
    if text.contains("Remaining Release Blockers") || text.contains("open release blocker") {
        findings.push(Finding::new(
            "RELEASE-READY-M10",
            "v1.9.0 release notes still record open release blockers",
        ));
    }
}

pub(crate) fn check_release_readiness_checklists(root: &Path, findings: &mut Vec<Finding>) {
    for rel in [
        "README.md",
        "docs/README.md",
        CURRENT_RELEASE_NOTES,
        "docs/release-notes/v1.8.0.md",
        "docs/release-notes/v1.7.2.md",
    ] {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            findings.push(Finding::new(
                "RELEASE-READY-M10",
                format!("could not read {rel}"),
            ));
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("- [ ]") || trimmed.contains("TODO") || trimmed.contains("TBD") {
                findings.push(Finding::new(
                    "RELEASE-READY-M10",
                    format!(
                        "{rel}:{} has unfinished public release text: {trimmed}",
                        index + 1
                    ),
                ));
            }
        }
    }
}
