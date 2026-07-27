use serde_json::json;

use super::checks::{CompositionCheckExt, checked_check, error_check, observed_pairs, round3};
use crate::{CaptureRgba8, CaptureScreenRect, SceneCompositionCheckV1};

pub(super) fn object_framing_check(
    target_path: &str,
    target_id: &str,
    handle: u64,
    capture: &CaptureRgba8,
    rect: CaptureScreenRect,
    profile: &str,
) -> SceneCompositionCheckV1 {
    let metrics = ObjectFramingMetrics::from_rect(capture, rect);
    let thresholds = ObjectFramingThresholds::for_profile(profile);
    let observed = observed_pairs([
        ("fit_fraction", json!(round3(metrics.fit_fraction))),
        ("width_fraction", json!(round3(metrics.width_fraction))),
        ("height_fraction", json!(round3(metrics.height_fraction))),
        (
            "center_offset_fraction",
            json!([
                round3(metrics.center_offset_x_fraction),
                round3(metrics.center_offset_y_fraction)
            ]),
        ),
        ("profile", json!(profile)),
    ]);
    let mut check = if metrics.fit_fraction < thresholds.min_fit_fraction {
        error_check(
            format!("{target_path}.framing"),
            "framing",
            "subject_too_small_in_frame",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "declared object occupies too little of the native-resolution frame for the selected quality profile",
                "frame the subject closer, reduce camera distance/FOV, or lower the profile only if a small subject is intentional",
            ),
        )
    } else if metrics.fit_fraction > thresholds.max_fit_fraction {
        error_check(
            format!("{target_path}.framing"),
            "framing",
            "subject_too_large_in_frame",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "declared object fills too much of the native-resolution frame for the selected quality profile",
                "pull the camera back, widen the field of view, or crop intentionally with an explicit bbox-fit expectation",
            ),
        )
    } else {
        checked_check(
            format!("{target_path}.framing"),
            "framing",
            "subject_fit_sane",
            Some(target_id.to_owned()),
            vec![handle],
            observed,
            (
                "declared object has a sane projected frame fill for the selected quality profile",
                "no action needed",
            ),
        )
    };
    check.threshold = Some(json!({
        "min_fit_fraction": round3(thresholds.min_fit_fraction),
        "max_fit_fraction": round3(thresholds.max_fit_fraction)
    }));
    check.with_region("node", Some(handle), Some(rect))
}

#[derive(Debug, Clone, Copy)]
struct ObjectFramingMetrics {
    fit_fraction: f32,
    width_fraction: f32,
    height_fraction: f32,
    center_offset_x_fraction: f32,
    center_offset_y_fraction: f32,
}

impl ObjectFramingMetrics {
    fn from_rect(capture: &CaptureRgba8, rect: CaptureScreenRect) -> Self {
        let width = capture.descriptor.width.max(1) as f32;
        let height = capture.descriptor.height.max(1) as f32;
        let width_fraction = rect.width.max(0.0) / width;
        let height_fraction = rect.height.max(0.0) / height;
        Self {
            fit_fraction: width_fraction.max(height_fraction),
            width_fraction,
            height_fraction,
            center_offset_x_fraction: ((rect.center_x / width) - 0.5).abs(),
            center_offset_y_fraction: ((rect.center_y / height) - 0.5).abs(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ObjectFramingThresholds {
    min_fit_fraction: f32,
    max_fit_fraction: f32,
}

impl ObjectFramingThresholds {
    fn for_profile(profile: &str) -> Self {
        match profile {
            "dashboard" => Self {
                min_fit_fraction: 0.030,
                max_fit_fraction: 0.96,
            },
            "cad" | "documentation" => Self {
                min_fit_fraction: 0.050,
                max_fit_fraction: 0.95,
            },
            "photo_product" => Self {
                min_fit_fraction: 0.080,
                max_fit_fraction: 0.96,
            },
            _ => Self {
                min_fit_fraction: 0.080,
                max_fit_fraction: 0.94,
            },
        }
    }
}
