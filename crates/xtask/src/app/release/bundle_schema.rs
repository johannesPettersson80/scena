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
        collect_files_with_extensions(artifact_root, &["json", "ppm", "png", "toml"], &mut files)
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
}
