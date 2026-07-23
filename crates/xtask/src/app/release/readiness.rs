use crate::app::prelude::*;

use super::ReleaseArtifactBundleSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedReleaseArtifactRoot {
    pub(crate) path: PathBuf,
    pub(crate) source: &'static str,
}

pub(crate) fn resolve_release_artifact_root(
    repo_root: &Path,
    cli_root: Option<&str>,
    environment_root: Option<&str>,
) -> Result<ResolvedReleaseArtifactRoot, Finding> {
    let (configured, source) = if let Some(configured) = cli_root {
        (configured, "cli")
    } else if let Some(configured) = environment_root {
        (configured, "environment")
    } else {
        return Err(missing_artifact_root_finding());
    };
    let configured = configured.trim();
    if configured.is_empty() {
        return Err(missing_artifact_root_finding());
    }
    let configured_path = PathBuf::from(configured);
    let path = if configured_path.is_absolute() {
        configured_path
    } else {
        repo_root.join(configured_path)
    };
    Ok(ResolvedReleaseArtifactRoot { path, source })
}

fn missing_artifact_root_finding() -> Finding {
    Finding::new(
        "RELEASE-READY-ARTIFACT-ROOT",
        "release-readiness requires --artifact-root <dir> or a non-empty SCENA_RELEASE_ARTIFACT_ROOT",
    )
}

pub(crate) fn release_readiness_report(
    artifact_root: Option<&Path>,
    artifact_root_source: Option<&str>,
    summary: ReleaseArtifactBundleSummary,
    findings: &[Finding],
) -> serde_json::Value {
    let ok = findings.is_empty()
        && summary.validated_artifact_count > 0
        && summary.validated_artifact_count == summary.required_artifact_count;
    json!({
        "schema": "scena.release_readiness.v1",
        "ok": ok,
        "status": if ok { "passed" } else { "failed" },
        "artifact_root": artifact_root.map(|path| path.display().to_string()),
        "artifact_root_source": artifact_root_source,
        "discovered_artifact_count": summary.discovered_artifact_count,
        "required_artifact_count": summary.required_artifact_count,
        "validated_artifact_count": summary.validated_artifact_count,
        "findings": findings.iter().map(|finding| json!({
            "rule": finding.rule,
            "message": finding.message,
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn run_release_readiness(cli_root: Option<&str>) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("RELEASE-READY-ROOT", message)])?;
    let environment_root = env::var("SCENA_RELEASE_ARTIFACT_ROOT").ok();
    let mut findings = Vec::new();
    check_release_readiness(&root, &mut findings);
    let resolved = match resolve_release_artifact_root(&root, cli_root, environment_root.as_deref())
    {
        Ok(resolved) => Some(resolved),
        Err(finding) => {
            findings.push(finding);
            None
        }
    };
    if let Some(resolved) = &resolved {
        if env::var("SCENA_REQUIRE_CI_PROVENANCE").as_deref() != Ok("1") {
            findings.push(Finding::new(
                "RELEASE-CI-PROVENANCE",
                "release-readiness requires SCENA_REQUIRE_CI_PROVENANCE=1 and live verification of the staged CI attestation",
            ));
        } else if let Err(message) = super::ci_provenance::verify_staged_ci_provenance(
            &root,
            &resolved.path,
            &release_artifact_commit_label(&root),
        ) {
            findings.push(Finding::new("RELEASE-CI-PROVENANCE", message));
        }
    }
    let summary = resolved.as_ref().map_or(
        ReleaseArtifactBundleSummary {
            discovered_artifact_count: 0,
            required_artifact_count: REQUIRED_RELEASE_ARTIFACT_SUFFIXES.len(),
            validated_artifact_count: 0,
        },
        |resolved| check_release_artifact_bundle_with_summary(&resolved.path, &mut findings),
    );
    let report = release_readiness_report(
        resolved.as_ref().map(|resolved| resolved.path.as_path()),
        resolved.as_ref().map(|resolved| resolved.source),
        summary,
        &findings,
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .expect("release-readiness JSON value must always serialize")
    );
    if report["ok"] == true {
        Ok(())
    } else {
        Err(findings)
    }
}
