use crate::app::prelude::*;

const RULE: &str = "Q03-M2-LOCAL-STRUCTURE";
const FIXTURES: &[&str] = &[
    "direct-lights-pbr",
    "shadowed-directional-light",
    "ibl-environment",
    "fxaa-edge",
    "anti-aliasing-on-off",
    "bloom-on-off",
    "ssao-contact-on-off",
    "oit-overlap-order-invariance",
    "clipping-half-space",
];

pub(crate) fn check_q03_m2_local_structure(root: &Path, findings: &mut Vec<Finding>) {
    require_tokens(
        root,
        findings,
        "tests/m2_visual_proof.rs",
        &[
            "m2_headless_local_structure_references_match_current_fixtures",
            "q03_structure_oracle_rejects_quadrant_mean_preserving_mutations",
            "local-structure-v2",
            "STRUCTURE_MEAN_SSIM_MIN: f32 = 0.97",
            "STRUCTURE_WORST_WINDOW_SSIM_MIN: f32 = 0.70",
            "STRUCTURE_EDGE_IOU_MIN: f32 = 0.85",
            "STRUCTURE_FOREGROUND_IOU_MIN: f32 = 0.95",
            "window_ssim",
            "sobel_edge_mask",
            "foreground_mask",
            "rotate_each_quadrant_180",
            "sort_each_quadrant_by_luminance",
            "debug_candidate_quadrants",
            "quadrant_metrics_role\": \"debug-only",
            "reference-diff.ppm",
            "reference-metrics.json",
            "worst_region",
        ],
    );
    for metadata in [
        "tests/visual/fixtures/m2-headless-core.toml",
        "tests/visual/references/m2-headless-core.toml",
    ] {
        require_tokens(
            root,
            findings,
            metadata,
            &[
                "reference_mode = \"local-structure-v2\"",
                "reference_frames = \"tests/visual/references/m2-headless-core-frames.toml\"",
                "window_size = 8",
                "window_stride = 4",
                "mean_window_ssim_min = 0.97",
                "worst_window_ssim_min = 0.70",
                "edge_iou_min = 0.85",
                "foreground_iou_min = 0.95",
            ],
        );
    }
    require_tokens(
        root,
        findings,
        "tests/visual/references/m2-headless-core.toml",
        &["quadrant_metrics_role = \"debug-only\""],
    );
    let catalog = "tests/visual/references/m2-headless-core-frames.toml";
    require_tokens(
        root,
        findings,
        catalog,
        &[
            "encoding = \"png-base64-srgb8\"",
            "source = \"deterministic Renderer::headless fixture output\"",
        ],
    );
    for fixture in FIXTURES {
        require_tokens(
            root,
            findings,
            catalog,
            &[&format!("name = \"{fixture}\""), "png_base64 = \""],
        );
    }
    require_tokens(
        root,
        findings,
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "local-structure-v2",
            "windowed SSIM",
            "quadrant means are diagnostic only",
            "mean-preserving structure mutations",
            "Q03-M2-LOCAL-STRUCTURE",
        ],
    );
    require_tokens(
        root,
        findings,
        "README.md",
        &["local M2 structure", "worst-region boxes"],
    );
    require_tokens(
        root,
        findings,
        "CHANGELOG.md",
        &[
            "Replace the M2 quadrant-mean reference oracle",
            "mean-preserving mutations",
        ],
    );
    require_tokens(
        root,
        findings,
        "docs/release-notes/v1.8.0.md",
        &[
            "quadrant means while moving or collapsing structure",
            "local SSIM/edge/foreground",
        ],
    );
}

fn require_tokens(root: &Path, findings: &mut Vec<Finding>, relative: &str, required: &[&str]) {
    let text = match fs::read_to_string(root.join(relative)) {
        Ok(text) => text,
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not read {relative}: {error}"),
            ));
            return;
        }
    };
    for token in required {
        if !text.contains(token) {
            findings.push(Finding::new(RULE, format!("{relative} is missing {token}")));
        }
    }
}
