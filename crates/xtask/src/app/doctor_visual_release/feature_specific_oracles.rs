use crate::app::prelude::*;

const VISUAL_SMOKE_ALLOWLIST: &[(&str, &str)] = &[(
    "tests/m6_browser_renderer_parity.rs",
    "Q04 owns replacement of the current attached-canvas nonblack probe with CPU-to-WebGL2 parity",
)];

const FEATURE_SPECIFIC_ORACLE_TOKENS: &[&str] = &[
    "component_count",
    "foreground_metrics",
    "difference_metrics",
    "evaluate_m3a_feature_specific_truth",
    "evaluate_m3b_pose_truth",
    "evaluate_connector_displacement",
    "evaluate_label_region_quality",
    "evaluate_line_region_quality",
    "rgba_within_tolerance",
    "center_pixel(",
    "max_luminance_in_region",
    "pixel_rect",
    "ssim_grayscale",
    "mean_delta_e2000",
];

pub(crate) fn check_feature_specific_visual_oracles(root: &Path, findings: &mut Vec<Finding>) {
    let tests_dir = root.join("tests");
    let Ok(entries) = fs::read_dir(&tests_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("rs")
        {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !release_visual_test_name(name) {
            continue;
        }
        let relative = format!("tests/{name}");
        if VISUAL_SMOKE_ALLOWLIST
            .iter()
            .any(|(allowlisted, _reason)| *allowlisted == relative)
        {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            findings.push(Finding::new(
                "Q03-FEATURE-VISUAL-ORACLE",
                format!("could not read {relative}"),
            ));
            continue;
        };
        let has_nonblack_oracle = source.contains("nonblack") || source.contains("non_black");
        let has_feature_specific_oracle = FEATURE_SPECIFIC_ORACLE_TOKENS
            .iter()
            .any(|token| source.contains(token));
        if has_nonblack_oracle && !has_feature_specific_oracle {
            findings.push(Finding::new(
                "Q03-FEATURE-VISUAL-ORACLE",
                format!(
                    "{relative} has only a nonblack visual oracle; add projected regions, component counts, localized differentials, quality evaluators, or an explicit smoke-only allowlist entry with an owner and rationale"
                ),
            ));
        }
    }
    if root.join(".github/workflows").is_dir() {
        for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
            require_contains(
                root,
                findings,
                "Q03-FEATURE-VISUAL-ORACLE",
                workflow,
                &["cargo test --features inspection --test measurement_visual_proof"],
            );
        }
    }
}

fn release_visual_test_name(name: &str) -> bool {
    name.ends_with("_visual_proof.rs")
        || name.ends_with("_browser_rendered_output.rs")
        || name.ends_with("_renderer_parity.rs")
}
