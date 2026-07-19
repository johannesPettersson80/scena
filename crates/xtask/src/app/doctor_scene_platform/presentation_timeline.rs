use crate::app::prelude::*;

pub(crate) fn check_c08_presentation_timeline_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "SCENE-C08";
    let required: &[(&str, &[&str])] = &[
        (
            "src/scene_host/presentation_timeline.rs",
            &[
                "struct ResolvedAnimationSegment",
                ".map(|action| self.resolve_animation_segment(action))",
                ".collect::<Result<Vec<_>, _>>()?",
                ".zip(resolved_animation_segments)",
                "fn resolve_animation_segment",
                "self.animation_timeline_binding(*mixer)?",
                "start_seconds {start_seconds} exceeds clip duration",
                "AnimationLoopMode::Once",
                "AnimationLoopMode::Repeat",
                "offset.rem_euclid(span)",
                "f64::from(f32::EPSILON)",
            ],
        ),
        (
            "src/scene_host/animation.rs",
            &[
                "fn animation_timeline_binding",
                "mixer.clip().duration_seconds()",
                "mixer.loop_mode()",
            ],
        ),
        (
            "docs/schema-contracts.md",
            &[
                "missing `end_seconds` means the clip duration",
                "zero-duration imported static clip",
                "half-open `[start,end)`",
                "not repeated per-entry",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/presentation_timeline.rs",
        &[
            "presentation_timeline_missing_end_clamps_once_clip_to_terminal_pose_without_failure",
            "presentation_timeline_validates_clip_bounds_before_any_action_is_due_or_applied",
            "presentation_timeline_once_and_repeat_segments_hold_or_wrap_at_stable_boundaries",
            "presentation_timeline_static_clip_samples_zero_and_rejects_positive_start",
        ],
    );

    if let Ok(source) = fs::read_to_string(root.join("src/scene_host/presentation_timeline.rs")) {
        let compact_source = source.split_whitespace().collect::<String>();
        let duration_clamp = "end_seconds.unwrap_or(duration_seconds).min(duration_seconds)";
        if !compact_source.contains(duration_clamp) {
            findings.push(Finding::new(
                RULE,
                format!(
                    "src/scene_host/presentation_timeline.rs is missing required duration clamp `{duration_clamp}`"
                ),
            ));
        }
        for forbidden in [
            "let mut sample_seconds = start_seconds + elapsed * speed",
            "animation action was pre-resolved\")",
        ] {
            if source.contains(forbidden) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "src/scene_host/presentation_timeline.rs contains forbidden unbounded timeline sampling `{forbidden}`"
                    ),
                ));
            }
        }
    }
}
