use crate::app::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReleaseArtifactBundleSummary {
    pub(crate) discovered_artifact_count: usize,
    pub(crate) required_artifact_count: usize,
    pub(crate) validated_artifact_count: usize,
}

pub(crate) fn check_release_artifact_bundle_with_summary(
    artifact_root: &Path,
    findings: &mut Vec<Finding>,
) -> ReleaseArtifactBundleSummary {
    let mut summary = ReleaseArtifactBundleSummary {
        discovered_artifact_count: 0,
        required_artifact_count: REQUIRED_RELEASE_ARTIFACT_SUFFIXES.len(),
        validated_artifact_count: 0,
    };
    if !artifact_root.exists() {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("missing release artifact root {}", artifact_root.display()),
        ));
        return summary;
    }
    if !artifact_root.is_dir() {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!(
                "release artifact root {} is not a readable directory",
                artifact_root.display()
            ),
        ));
        return summary;
    }

    let mut files = Vec::new();
    if let Err(error) = collect_files_with_extensions(
        artifact_root,
        &["json", "jsonl", "log", "ppm", "png", "toml"],
        &mut files,
    ) {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not collect release artifacts: {error}"),
        ));
        return summary;
    }
    summary.discovered_artifact_count = files.len();
    summary.validated_artifact_count = REQUIRED_RELEASE_ARTIFACT_SUFFIXES
        .iter()
        .filter(|suffix| files.iter().any(|path| path_ends_with(path, suffix)))
        .count();

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

    for (suffix, os, arch) in [
        (
            "q11-reference-stability/linux-x86_64.json",
            "linux",
            "x86_64",
        ),
        (
            "q11-reference-stability/macos-aarch64.json",
            "macos",
            "aarch64",
        ),
        (
            "q11-reference-stability/windows-x86_64.json",
            "windows",
            "x86_64",
        ),
    ] {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            let result = fs::read_to_string(path)
                .map_err(|error| error.to_string())
                .and_then(|text| {
                    serde_json::from_str::<Value>(&text).map_err(|error| error.to_string())
                });
            match result {
                Ok(result) => {
                    if let Err(error) = validate_q11_reference_stability_result(&result, os, arch) {
                        findings.push(Finding::new(
                            "RELEASE-Q11-REFERENCE-STABILITY",
                            format!("{suffix} failed semantic validation: {error}"),
                        ));
                    }
                }
                Err(error) => findings.push(Finding::new(
                    "RELEASE-Q11-REFERENCE-STABILITY",
                    format!("could not parse {suffix}: {error}"),
                )),
            }
        }
    }

    require_verified_staging_provenance(&files, findings);

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

    for suffix in ["c09-gpu-resource-lifecycle/required-result.json"] {
        for path in files.iter().filter(|path| path_ends_with(path, suffix)) {
            require_gpu_resource_lifecycle_proof(path, suffix, findings);
        }
    }

    check_required_visual_proof_artifacts(artifact_root, findings);
    summary
}

pub(crate) fn require_verified_staging_provenance(files: &[PathBuf], findings: &mut Vec<Finding>) {
    let Some(path) = files
        .iter()
        .find(|path| path_ends_with(path, "staging-metadata.json"))
    else {
        return;
    };
    let Some(manifest_path) = files
        .iter()
        .find(|path| path_ends_with(path, "ci-provenance.json"))
    else {
        findings.push(Finding::new(
            "RELEASE-CI-PROVENANCE",
            "release readiness requires the signed ci-provenance.json manifest",
        ));
        return;
    };
    let Ok(source) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-CI-PROVENANCE",
            format!(
                "could not read verified staging metadata {}",
                path.display()
            ),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&source) else {
        findings.push(Finding::new(
            "RELEASE-CI-PROVENANCE",
            format!("staging metadata {} is not valid JSON", path.display()),
        ));
        return;
    };
    let Ok(manifest_source) = fs::read_to_string(manifest_path) else {
        findings.push(Finding::new(
            "RELEASE-CI-PROVENANCE",
            format!("could not read signed manifest {}", manifest_path.display()),
        ));
        return;
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&manifest_source) else {
        findings.push(Finding::new(
            "RELEASE-CI-PROVENANCE",
            format!(
                "signed manifest {} is not valid JSON",
                manifest_path.display()
            ),
        ));
        return;
    };
    let provenance = &value["ci_provenance"];
    let valid_hex = |field: &Value| {
        field.as_str().is_some_and(|value| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    };
    let required_nonblank = [
        "workflow_ref",
        "workflow_sha",
        "ref",
        "run_id",
        "job",
        "source_commit",
    ]
    .iter()
    .all(|field| {
        provenance[*field]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
    });
    let verified = value["release_evidence"] == true
        && value["release_rejection_codes"] == json!([])
        && provenance["schema"] == "scena.ci_provenance.v1"
        && provenance["repository"] == "johannesPettersson80/scena"
        && provenance["issuer"] == "https://token.actions.githubusercontent.com"
        && required_nonblank
        && provenance["run_attempt"]
            .as_u64()
            .is_some_and(|value| value > 0)
        && valid_hex(&provenance["artifact_digest"])
        && provenance["attestation"]["predicate_type"] == "https://slsa.dev/provenance/v1"
        && provenance["attestation"]["verification_status"] == "verified"
        && valid_hex(&provenance["attestation"]["verification_receipt_sha256"])
        && manifest["release_evidence"] == false
        && manifest["release_rejection_codes"] == json!(["CI_ATTESTATION_NOT_YET_VERIFIED"])
        && manifest["attestation"]["predicate_type"] == "https://slsa.dev/provenance/v1"
        && manifest["attestation"]["verification_status"] == "pending"
        && [
            "schema",
            "repository",
            "workflow_ref",
            "workflow_sha",
            "ref",
            "run_id",
            "run_attempt",
            "job",
            "source_commit",
            "artifact_digest",
            "artifact_file_count",
            "issuer",
            "generated_at_unix_seconds",
        ]
        .iter()
        .all(|field| provenance[*field] == manifest[*field]);
    if !verified {
        findings.push(Finding::new(
            "RELEASE-CI-PROVENANCE",
            "release readiness requires staging metadata produced after cryptographic verification of the exact CI-issued artifact-tree attestation",
        ));
    }
}
