use crate::app::prelude::*;

pub(crate) fn validate_release_review_report(
    role: &str,
    report_path: &Path,
    findings: &mut Vec<Finding>,
) {
    let Ok(text) = fs::read_to_string(report_path) else {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            format!(
                "could not read release review report {}; see \
                 docs/specs/release-reviews.md",
                report_path.display()
            ),
        ));
        return;
    };

    let frontmatter = parse_release_review_frontmatter(&text);
    let display = report_path.display();
    let report_short = report_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unnamed>");
    if frontmatter.is_none() {
        findings.push(Finding::new(
            "RELEASE-REVIEWS-PRESENT",
            format!(
                "{display} is missing the release-reviews frontmatter block; see \
                 docs/specs/release-reviews.md Section 1"
            ),
        ));
        return;
    }
    let frontmatter = frontmatter.unwrap();

    for required_key in [
        "role",
        "reviewed_commit",
        "session_id",
        "date",
        "blocker_status",
        "findings_count",
    ] {
        if frontmatter.get(required_key).is_none() {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                format!(
                    "{display} frontmatter is missing required field {required_key:?}; \
                     see docs/specs/release-reviews.md Section 1"
                ),
            ));
        }
    }

    if let Some(declared_role) = frontmatter.get("role") {
        if declared_role.as_str() != role {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                format!(
                    "{display} declares role={declared_role:?} but lives under \
                     reviews/{role}/; the role slug must match the directory; \
                     see docs/specs/release-reviews.md Section 1"
                ),
            ));
        }
    }

    if let Some(declared_blocker) = frontmatter.get("blocker_status") {
        if !matches!(
            declared_blocker.as_str(),
            "clear" | "blockers-open" | "findings-recorded"
        ) {
            findings.push(Finding::new(
                "RELEASE-REVIEWS-PRESENT",
                format!(
                    "{display} blocker_status={declared_blocker:?} is not one of \
                     clear | blockers-open | findings-recorded; see \
                     docs/specs/release-reviews.md Section 1"
                ),
            ));
        }
    }

    let finding_heading_count = text
        .lines()
        .filter(|line| line.trim_start().starts_with("### Finding"))
        .count();
    if let Some(declared_count_text) = frontmatter.get("findings_count") {
        match declared_count_text.parse::<usize>() {
            Ok(declared_count) if declared_count != finding_heading_count => {
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "{display} declares findings_count={declared_count} but contains \
                         {finding_heading_count} `### Finding` heading(s); see \
                         docs/specs/release-reviews.md Section 1"
                    ),
                ));
            }
            Err(_) => {
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "{display} findings_count={declared_count_text:?} is not a non-negative \
                         integer; see docs/specs/release-reviews.md Section 1"
                    ),
                ));
            }
            _ => {}
        }
    }

    for finding_block in iterate_finding_blocks(&text) {
        for required_field in ["Severity:", "Status:", "Evidence:", "Notes:"] {
            if !finding_block.body.lines().any(|line| {
                line.trim_start_matches(['-', '*', ' '])
                    .starts_with(required_field)
            }) {
                findings.push(Finding::new(
                    "RELEASE-REVIEWS-PRESENT",
                    format!(
                        "{report_short} finding {:?} is missing required field {required_field:?}; \
                         see docs/specs/release-reviews.md Section 1",
                        finding_block.heading.trim()
                    ),
                ));
            }
        }
    }
}

pub(crate) fn parse_release_review_frontmatter(
    text: &str,
) -> Option<std::collections::BTreeMap<String, String>> {
    let trimmed_start = text.trim_start_matches('\u{feff}').trim_start();
    let after_open = trimmed_start.strip_prefix("---\n")?;
    let close_offset = after_open
        .find("\n---\n")
        .or_else(|| after_open.find("\n---"))?;
    let yaml_block = &after_open[..close_offset];
    let mut map = std::collections::BTreeMap::new();
    for line in yaml_block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        map.insert(key.trim().to_string(), value);
    }
    Some(map)
}

pub(crate) struct ReleaseFindingBlock<'a> {
    pub(crate) heading: &'a str,
    pub(crate) body: &'a str,
}

pub(crate) fn iterate_finding_blocks(text: &str) -> Vec<ReleaseFindingBlock<'_>> {
    let mut blocks = Vec::new();
    let mut start_indices = Vec::new();
    for (index, line_start) in text.match_indices("### Finding") {
        if index == 0 || text.as_bytes()[index - 1] == b'\n' {
            let heading_end = text[index..]
                .find('\n')
                .map(|relative| index + relative)
                .unwrap_or(text.len());
            start_indices.push((index, heading_end, line_start));
        }
    }
    for (block_index, &(start, heading_end, _)) in start_indices.iter().enumerate() {
        let next_start = start_indices
            .get(block_index + 1)
            .map(|next| next.0)
            .unwrap_or_else(|| {
                text[heading_end..]
                    .find("\n## ")
                    .map(|relative| heading_end + relative)
                    .unwrap_or(text.len())
            });
        blocks.push(ReleaseFindingBlock {
            heading: &text[start..heading_end],
            body: &text[heading_end..next_start],
        });
    }
    blocks
}

pub(crate) const REQUIRED_RELEASE_ARTIFACT_SUFFIXES: &[&str] = &[
    "release-lanes/linux-native-vulkan.json",
    "release-lanes/headless-cpu.json",
    "release-lanes/linux-webgl2-chromium.json",
    "release-lanes/linux-webgpu-chromium.json",
    "release-lanes/wasm32-unknown-unknown.json",
    "release-lanes/macos-metal.json",
    "release-lanes/windows-dx12.json",
    "m5-benchmarks.json",
    "m5-public-api-freeze.json",
    "examples-visual/cad_plate_drawing_import.ppm",
    "m6-rust-wasm-renderer-probe.json",
    "m9-wasm-size.json",
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/m9-benchmarks.json",
    "m9-platform/m9-benchmarks-4k.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/linux-native-vulkan/surface-context-loss.json",
    "m9-platform/linux-native-vulkan/default-scene.ppm",
    "m9-platform/linux-native-vulkan/static-gltf.ppm",
    "m9-platform/linux-native-vulkan/pbr-directional-red.ppm",
    "m9-platform/linux-native-vulkan/pbr-point-green.ppm",
    "m9-platform/linux-native-vulkan/pbr-spot-blue.ppm",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "m9-platform/headless-cpu/default-scene.ppm",
    "m9-platform/headless-cpu/static-gltf.ppm",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/macos-metal/surface-context-loss.json",
    "m9-platform/macos-metal/default-scene.ppm",
    "m9-platform/macos-metal/static-gltf.ppm",
    "m9-platform/macos-metal/pbr-directional-red.ppm",
    "m9-platform/macos-metal/pbr-point-green.ppm",
    "m9-platform/macos-metal/pbr-spot-blue.ppm",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
    "m9-platform/windows-dx12/surface-context-loss.json",
    "m9-platform/windows-dx12/default-scene.ppm",
    "m9-platform/windows-dx12/static-gltf.ppm",
    "m9-platform/windows-dx12/pbr-directional-red.ppm",
    "m9-platform/windows-dx12/pbr-point-green.ppm",
    "m9-platform/windows-dx12/pbr-spot-blue.ppm",
    // Phase 6 paperwork: per docs/specs/release-reviews.md, the findings register and
    // maintainer sign-off must accompany the release-lane artifacts before publish.
    // Per-subagent <role>/<commit>.md reports are validated separately by
    // RELEASE-REVIEWS-PRESENT in a follow-up batch; these two single-file contracts
    // ride the existing missing-suffix path without scanning the reviews/ subtree.
    "reviews/findings.json",
    "reviews/maintainer-signoff.toml",
    "visual-proof/waterbottle-gpu.json",
    "visual-proof/browser-webgpu.json",
    "visual-proof/browser-webgl2.json",
    "visual-proof/native-gpu.json",
];

pub(crate) const REQUIRED_PASSED_STATUS_ARTIFACT_SUFFIXES: &[&str] = &[
    "m6-rust-wasm-renderer-probe.json",
    "m9-platform/m9-capability-matrix.json",
];

pub(crate) const RELEASE_LANE_ARTIFACT_SUFFIXES: &[&str] = &[
    "release-lanes/linux-native-vulkan.json",
    "release-lanes/headless-cpu.json",
    "release-lanes/linux-webgl2-chromium.json",
    "release-lanes/linux-webgpu-chromium.json",
    "release-lanes/wasm32-unknown-unknown.json",
    "release-lanes/macos-metal.json",
    "release-lanes/windows-dx12.json",
];

pub(crate) const REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/windows-dx12/rendered-output.json",
];

pub(crate) const REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
];
pub(crate) const REQUIRED_JSON_COMMIT_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/m9-capability-matrix.json",
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/linux-native-vulkan/capabilities.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/headless-cpu/capabilities.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/macos-metal/capabilities.json",
    "m9-platform/windows-dx12/rendered-output.json",
    "m9-platform/windows-dx12/capabilities.json",
    "visual-proof/waterbottle-gpu.json",
    "visual-proof/browser-webgpu.json",
    "visual-proof/browser-webgl2.json",
    "visual-proof/native-gpu.json",
];

pub(crate) const REQUIRED_NON_CONSTANT_PPM_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/linux-native-vulkan/default-scene.ppm",
    "m9-platform/linux-native-vulkan/static-gltf.ppm",
    "m9-platform/linux-native-vulkan/pbr-directional-red.ppm",
    "m9-platform/linux-native-vulkan/pbr-point-green.ppm",
    "m9-platform/linux-native-vulkan/pbr-spot-blue.ppm",
    "m9-platform/headless-cpu/default-scene.ppm",
    "m9-platform/headless-cpu/static-gltf.ppm",
    "m9-platform/macos-metal/default-scene.ppm",
    "m9-platform/macos-metal/static-gltf.ppm",
    "m9-platform/macos-metal/pbr-directional-red.ppm",
    "m9-platform/macos-metal/pbr-point-green.ppm",
    "m9-platform/macos-metal/pbr-spot-blue.ppm",
    "m9-platform/windows-dx12/default-scene.ppm",
    "m9-platform/windows-dx12/static-gltf.ppm",
    "m9-platform/windows-dx12/pbr-directional-red.ppm",
    "m9-platform/windows-dx12/pbr-point-green.ppm",
    "m9-platform/windows-dx12/pbr-spot-blue.ppm",
];

pub(crate) const REQUIRED_MEASURED_CAPABILITY_ARTIFACT_SUFFIXES: &[&str] =
    &["m9-platform/m9-capability-matrix.json"];

pub(crate) const REQUIRED_BENCHMARK_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/m9-benchmarks.json",
    "m9-platform/m9-benchmarks-4k.json",
];
pub(crate) const REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES: &[&str] = &[
    "m9-platform/linux-native-vulkan/rendered-output.json",
    "m9-platform/headless-cpu/rendered-output.json",
    "m9-platform/macos-metal/rendered-output.json",
    "m9-platform/windows-dx12/rendered-output.json",
];
pub(crate) const REQUIRED_VISUAL_PROOF_ARTIFACT_SUFFIXES: &[&str] = &[
    "visual-proof/waterbottle-gpu.json",
    "visual-proof/browser-webgpu.json",
    "visual-proof/browser-webgl2.json",
    "visual-proof/native-gpu.json",
];
pub(crate) const MIN_BENCHMARK_SAMPLE_COUNT: u64 = 100;

pub(crate) const RELEASE_ARTIFACT_MAX_AGE_SECONDS: u64 = 24 * 60 * 60;
pub(crate) const RELEASE_ARTIFACT_MAX_FUTURE_SKEW_SECONDS: u64 = 60 * 60;

pub(crate) fn require_json_status_passed(path: &Path, suffix: &str, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("could not read downloaded artifact {}", path.display()),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded artifact {} is not valid JSON", path.display()),
        ));
        return;
    };
    if value.get("status").and_then(serde_json::Value::as_str) != Some("passed") {
        findings.push(Finding::new(
            "RELEASE-READY-ARTIFACTS",
            format!("downloaded release artifact {suffix} does not have status 'passed'"),
        ));
    }
}
