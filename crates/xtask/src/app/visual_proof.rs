use crate::app::prelude::*;

pub(crate) fn run_visual_proof() -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("VISUAL-PROOF", message)])?;
    let lanes: &[(&str, &[&str])] = &[
        (
            "headless-cpu",
            &["cargo", "test", "--test", "m9_platform_release"],
        ),
        (
            "m8-real-asset",
            &["cargo", "test", "--test", "m8_real_asset_proof"],
        ),
    ];
    let mut findings = Vec::new();
    for (lane, command) in lanes {
        let command = command
            .iter()
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>();
        if let Err(mut lane_findings) = run_visual_proof_command(&root, lane, &command) {
            findings.append(&mut lane_findings);
        }
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings)
    }
}

pub(crate) fn run_visual_proof_command(
    root: &Path,
    lane: &str,
    command: &[String],
) -> Result<(), Vec<Finding>> {
    if command.is_empty() {
        return Err(vec![Finding::new(
            "VISUAL-PROOF",
            "visual-proof command is empty",
        )]);
    }
    let lane = sanitize_visual_proof_lane(lane).ok_or_else(|| {
        vec![Finding::new(
            "VISUAL-PROOF",
            "visual-proof lane must use ASCII letters, digits, '-' or '_'",
        )]
    })?;
    let artifact_dir = root.join("target/gate-artifacts/visual-proof");
    if let Err(error) = fs::create_dir_all(&artifact_dir) {
        return Err(vec![Finding::new(
            "VISUAL-PROOF",
            format!("failed to create {}: {error}", artifact_dir.display()),
        )]);
    }
    let output = ProcessCommand::new(&command[0])
        .args(&command[1..])
        .current_dir(root)
        .output()
        .map_err(|error| {
            vec![Finding::new(
                "VISUAL-PROOF",
                format!("failed to run command: {error}"),
            )]
        })?;
    let stdout_path = artifact_dir.join(format!("{lane}.stdout.log"));
    let stderr_path = artifact_dir.join(format!("{lane}.stderr.log"));
    let json_path = artifact_dir.join(format!("{lane}.json"));
    if let Err(error) = fs::write(&stdout_path, &output.stdout) {
        return Err(vec![Finding::new(
            "VISUAL-PROOF",
            format!("failed to write stdout log: {error}"),
        )]);
    }
    if let Err(error) = fs::write(&stderr_path, &output.stderr) {
        return Err(vec![Finding::new(
            "VISUAL-PROOF",
            format!("failed to write stderr log: {error}"),
        )]);
    }
    let stdout_text = String::from_utf8_lossy(&output.stdout);
    let stderr_text = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout_text}\n{stderr_text}");
    let rust_test_command = command.iter().any(|part| part == "test");
    let rust_test_output_observed = combined.contains("test result:");
    let rust_test_nonzero_pass_summary_observed =
        visual_proof_rust_test_nonzero_pass_summary_observed(&combined);
    let skip_marker_observed = combined.contains("SKIPPING")
        || combined.contains("ignored by default")
        || (rust_test_command
            && rust_test_output_observed
            && !rust_test_nonzero_pass_summary_observed);
    let signal = process_signal(&output.status);
    let status = if signal.is_some() {
        "terminated"
    } else if !output.status.success() {
        "failed"
    } else if rust_test_command && !rust_test_output_observed {
        "terminated"
    } else if skip_marker_observed {
        "skipped"
    } else {
        "passed"
    };
    let artifact = json!({
        "schema": "scena.visual_proof.v1",
        "lane": lane,
        "command": command,
        "status": status,
        "exit_code": output.status.code(),
        "signal": signal,
        "rust_test_command": rust_test_command,
        "rust_test_output_observed": rust_test_output_observed,
        "rust_test_nonzero_pass_summary_observed": rust_test_nonzero_pass_summary_observed,
        "skip_marker_observed": skip_marker_observed,
        "stdout_log": path_to_forward_slash(&stdout_path),
        "stderr_log": path_to_forward_slash(&stderr_path),
        "generated_at_unix_seconds": current_unix_seconds(),
        "commit": release_artifact_commit_label(root),
        "note": "Release visual proof fails closed: process termination, skipped markers, ignored/no-test paths, and cargo-test commands without Rust test summary are not passing proof."
    });
    let body = serde_json::to_string_pretty(&artifact)
        .map_err(|error| vec![Finding::new("VISUAL-PROOF", error.to_string())])?;
    if let Err(error) = fs::write(&json_path, format!("{body}\n")) {
        return Err(vec![Finding::new(
            "VISUAL-PROOF",
            format!("failed to write {}: {error}", json_path.display()),
        )]);
    }
    println!("{}", json_path.display());
    if status == "passed" {
        Ok(())
    } else {
        Err(vec![Finding::new(
            "VISUAL-PROOF",
            format!("visual-proof lane {lane:?} ended with status {status:?}"),
        )])
    }
}

pub(crate) fn sanitize_visual_proof_lane(lane: &str) -> Option<String> {
    (!lane.is_empty()
        && lane
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'))
    .then(|| lane.to_string())
}

pub(crate) fn process_signal(status: &std::process::ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    }
    #[cfg(not(unix))]
    {
        let _ = status;
        None
    }
}

pub(crate) fn visual_proof_rust_test_nonzero_pass_summary_observed(output: &str) -> bool {
    output.lines().any(|line| {
        line.contains("test result: ok.")
            && line
                .split(" passed;")
                .next()
                .and_then(|prefix| prefix.rsplit_once(' '))
                .and_then(|(_, passed)| passed.parse::<usize>().ok())
                .is_some_and(|passed| passed > 0)
    })
}

pub(crate) fn path_to_forward_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
