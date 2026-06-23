use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::animation::{AnimationChannel, AnimationClip, AnimationTarget};
use crate::scene::{Quat, Transform, Vec3};

pub const ANIMATION_INTROSPECTION_SCHEMA_V1: &str = "scena.animation_introspection.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationIntrospectionReportV1 {
    pub schema: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<AnimationClipIntrospectionV1>,
    pub summary: AnimationIntrospectionSummaryV1,
    pub samples: Vec<AnimationSampleV1>,
    pub reasons: Vec<AnimationIntrospectionReasonV1>,
    pub fixes: Vec<AnimationIntrospectionFixV1>,
    pub artifacts: AnimationIntrospectionArtifactsV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationClipIntrospectionV1 {
    pub name: String,
    pub duration_seconds: f32,
    pub channel_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimationIntrospectionSummaryV1 {
    pub sample_count: usize,
    pub changed_channel_count: usize,
    pub unchanged_channel_count: usize,
    pub invalid_channel_count: usize,
    pub visible_change: bool,
    pub capture_changes: usize,
    #[serde(default)]
    pub rendered_movement: bool,
    #[serde(default)]
    pub rendered_movement_delta_px: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationSampleV1 {
    pub time_seconds: f32,
    pub transform_revision: u64,
    #[serde(default)]
    pub appearance_revision: u64,
    pub payload_fnv1a64: String,
    pub moving_node_count: usize,
    pub invalid_node_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_values: Vec<AnimationObservedValueV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationObservedValueV1 {
    pub id: String,
    pub node: u64,
    pub kind: String,
    pub transform: Transform,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered_centroid_css_px: Option<[f32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendered_coverage_px: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_translation: Option<Vec3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub within_tolerance: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationIntrospectionReasonV1 {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationIntrospectionFixV1 {
    pub action: String,
    pub help: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationIntrospectionArtifactsV1 {
    pub sampled_times: Vec<f32>,
    pub host_tick: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationChannelChangeCounts {
    pub changed: usize,
    pub unchanged: usize,
    pub invalid: usize,
}

impl AnimationIntrospectionReportV1 {
    pub fn from_samples(
        clip: AnimationClipIntrospectionV1,
        channel_counts: AnimationChannelChangeCounts,
        samples: Vec<AnimationSampleV1>,
        expect_change: bool,
    ) -> Self {
        let capture_changes = capture_changes(&samples);
        let visible_change = capture_changes > 0;
        let rendered_coverage_missing = rendered_coverage_missing(&samples);
        let rendered_movement_delta_px = rendered_movement_delta_px(&samples);
        let rendered_movement = rendered_movement_delta_px > 1.0;
        let sampled_times = samples
            .iter()
            .map(|sample| sample.time_seconds)
            .collect::<Vec<_>>();
        let mut reasons = Vec::new();
        let mut fixes = Vec::new();

        if channel_counts.invalid > 0 {
            push_reason(
                &mut reasons,
                "invalid_channel",
                "error",
                "one or more animation channels sampled invalid or non-finite values",
            );
            push_fix(
                &mut fixes,
                "inspect_animation_channels",
                "inspect source animation accessors and interpolation before trusting playback",
            );
        }
        if expect_change && !times_advance(&sampled_times) {
            push_reason(
                &mut reasons,
                "time_not_advanced",
                "error",
                "animation verification expected change but sampled fewer than two distinct times",
            );
            push_fix(
                &mut fixes,
                "sample_distinct_times",
                "sample at least two distinct times within the clip duration",
            );
        }
        if expect_change && channel_counts.changed == 0 {
            push_reason(
                &mut reasons,
                "channels_frozen",
                "error",
                "animation verification expected channel changes but no channel changed across sampled times",
            );
            push_fix(
                &mut fixes,
                "check_clip_and_times",
                "choose an animated clip and sample distinct times inside its duration",
            );
        }
        if expect_change && !visible_change {
            push_reason(
                &mut reasons,
                "no_visible_change",
                "error",
                "animation channels did not produce distinct rendered captures across sampled times",
            );
            push_fix(
                &mut fixes,
                "render_changed_samples",
                "verify the animated node is visible, framed, and rendered after each explicit seek",
            );
        }
        if expect_change && rendered_coverage_missing {
            push_reason(
                &mut reasons,
                "rendered_node_coverage_missing",
                "error",
                "animation verification could not measure rendered pixels for the selected moving node in every sample",
            );
            push_fix(
                &mut fixes,
                "render_selected_node",
                "verify the selected moving node is visible, framed, and not matching the background in every sampled frame",
            );
        } else if expect_change
            && has_rendered_centroid_samples(&samples)
            && !rendered_movement
            && channel_counts.changed > 0
        {
            push_reason(
                &mut reasons,
                "rendered_node_coverage_frozen",
                "error",
                "animation channels changed but the selected node's rendered pixel coverage did not move across sampled frames",
            );
            push_fix(
                &mut fixes,
                "refresh_rendered_transform",
                "ensure transform changes update the rendered draw state, not only scene metadata or overlay markers",
            );
        }
        if samples.iter().any(|sample| {
            sample
                .observed_values
                .iter()
                .any(|value| value.within_tolerance == Some(false))
        }) {
            push_reason(
                &mut reasons,
                "expected_value_mismatch",
                "error",
                "one or more sampled transform values differed from expected values",
            );
            push_fix(
                &mut fixes,
                "inspect_expected_animation_state",
                "check the expected sample times, transform values, and animated target selection",
            );
        }

        let errors = reasons
            .iter()
            .filter(|reason| reason.severity == "error")
            .count();
        Self {
            schema: ANIMATION_INTROSPECTION_SCHEMA_V1.to_owned(),
            ok: errors == 0,
            clip: Some(clip),
            summary: AnimationIntrospectionSummaryV1 {
                sample_count: samples.len(),
                changed_channel_count: channel_counts.changed,
                unchanged_channel_count: channel_counts.unchanged,
                invalid_channel_count: channel_counts.invalid,
                visible_change,
                capture_changes,
                rendered_movement,
                rendered_movement_delta_px: round3(rendered_movement_delta_px),
            },
            samples,
            reasons,
            fixes,
            artifacts: AnimationIntrospectionArtifactsV1 {
                sampled_times,
                host_tick: "seek_animation".to_owned(),
            },
        }
    }

    pub fn missing_clip(requested: &str, available: Vec<String>) -> Self {
        let available = if available.is_empty() {
            "no named clips were available".to_owned()
        } else {
            format!("available clips: {}", available.join(", "))
        };
        Self {
            schema: ANIMATION_INTROSPECTION_SCHEMA_V1.to_owned(),
            ok: false,
            clip: None,
            summary: AnimationIntrospectionSummaryV1 {
                sample_count: 0,
                changed_channel_count: 0,
                unchanged_channel_count: 0,
                invalid_channel_count: 0,
                visible_change: false,
                capture_changes: 0,
                rendered_movement: false,
                rendered_movement_delta_px: 0.0,
            },
            samples: Vec::new(),
            reasons: vec![AnimationIntrospectionReasonV1 {
                code: "clip_missing".to_owned(),
                severity: "error".to_owned(),
                message: format!("animation clip '{requested}' was not found; {available}"),
            }],
            fixes: vec![AnimationIntrospectionFixV1 {
                action: "list_animation_clips".to_owned(),
                help: "inspect the asset animation inventory and choose an available clip name"
                    .to_owned(),
            }],
            artifacts: AnimationIntrospectionArtifactsV1 {
                sampled_times: Vec::new(),
                host_tick: "seek_animation".to_owned(),
            },
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("animation introspection report is serializable")
    }
}

impl AnimationClipIntrospectionV1 {
    pub fn from_clip(clip_name: &str, clip: &AnimationClip) -> Self {
        Self {
            name: clip.name().unwrap_or(clip_name).to_owned(),
            duration_seconds: round3(clip.duration_seconds()),
            channel_count: clip.channels().len(),
        }
    }
}

pub fn animation_channel_change_counts(
    clip: &AnimationClip,
    times: &[f32],
    tolerance: f32,
) -> AnimationChannelChangeCounts {
    let mut counts = AnimationChannelChangeCounts {
        changed: 0,
        unchanged: 0,
        invalid: 0,
    };
    for channel in clip.channels() {
        match channel_changed(channel, times, tolerance) {
            ChannelMotion::Changed => counts.changed += 1,
            ChannelMotion::Unchanged => counts.unchanged += 1,
            ChannelMotion::Invalid => counts.invalid += 1,
        }
    }
    counts
}

pub fn transform_differs(left: Transform, right: Transform, tolerance: f32) -> bool {
    vec3_differs(left.translation, right.translation, tolerance)
        || vec3_differs(left.scale, right.scale, tolerance)
        || quat_differs(left.rotation, right.rotation, tolerance)
}

pub fn transform_is_finite(transform: Transform) -> bool {
    transform.translation.is_finite()
        && transform.scale.is_finite()
        && transform.rotation.is_finite()
}

fn channel_changed(channel: &AnimationChannel, times: &[f32], tolerance: f32) -> ChannelMotion {
    let Some((first_time, rest)) = times.split_first() else {
        return ChannelMotion::Unchanged;
    };
    let Some(first) = sample_channel(channel, *first_time) else {
        return ChannelMotion::Invalid;
    };
    if !first.is_finite() {
        return ChannelMotion::Invalid;
    }
    for time in rest {
        let Some(next) = sample_channel(channel, *time) else {
            return ChannelMotion::Invalid;
        };
        if !next.is_finite() {
            return ChannelMotion::Invalid;
        }
        if first.differs(&next, tolerance) {
            return ChannelMotion::Changed;
        }
    }
    ChannelMotion::Unchanged
}

fn sample_channel(channel: &AnimationChannel, time_seconds: f32) -> Option<ChannelSample> {
    match channel.target() {
        AnimationTarget::Translation | AnimationTarget::Scale => {
            channel.sample_vec3(time_seconds).map(ChannelSample::Vec3)
        }
        AnimationTarget::Rotation => channel.sample_quat(time_seconds).map(ChannelSample::Quat),
        AnimationTarget::Weights => channel
            .sample_weights(time_seconds)
            .map(ChannelSample::Weights),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ChannelSample {
    Vec3(Vec3),
    Quat(Quat),
    Weights(Vec<f32>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChannelMotion {
    Changed,
    Unchanged,
    Invalid,
}

impl ChannelSample {
    fn is_finite(&self) -> bool {
        match self {
            Self::Vec3(value) => value.is_finite(),
            Self::Quat(value) => value.is_finite(),
            Self::Weights(values) => values.iter().all(|value| value.is_finite()),
        }
    }

    fn differs(&self, other: &Self, tolerance: f32) -> bool {
        match (self, other) {
            (Self::Vec3(left), Self::Vec3(right)) => vec3_differs(*left, *right, tolerance),
            (Self::Quat(left), Self::Quat(right)) => quat_differs(*left, *right, tolerance),
            (Self::Weights(left), Self::Weights(right)) => {
                left.len() != right.len()
                    || left
                        .iter()
                        .zip(right)
                        .any(|(left, right)| (left - right).abs() > tolerance)
            }
            _ => true,
        }
    }
}

fn capture_changes(samples: &[AnimationSampleV1]) -> usize {
    let Some(first) = samples.first() else {
        return 0;
    };
    samples
        .iter()
        .skip(1)
        .filter(|sample| sample.payload_fnv1a64 != first.payload_fnv1a64)
        .count()
}

fn rendered_coverage_missing(samples: &[AnimationSampleV1]) -> bool {
    let values = selected_rendered_values(samples);
    !values.is_empty()
        && values.iter().any(|value| {
            value.rendered_coverage_px.unwrap_or(0) == 0 || value.rendered_centroid_css_px.is_none()
        })
}

fn has_rendered_centroid_samples(samples: &[AnimationSampleV1]) -> bool {
    selected_rendered_values(samples)
        .iter()
        .filter(|value| value.rendered_centroid_css_px.is_some())
        .count()
        >= 2
}

fn rendered_movement_delta_px(samples: &[AnimationSampleV1]) -> f32 {
    let centroids = selected_rendered_values(samples)
        .into_iter()
        .filter_map(|value| value.rendered_centroid_css_px)
        .collect::<Vec<_>>();
    let Some(first) = centroids.first().copied() else {
        return 0.0;
    };
    centroids
        .iter()
        .map(|centroid| {
            let dx = centroid[0] - first[0];
            let dy = centroid[1] - first[1];
            (dx * dx + dy * dy).sqrt()
        })
        .fold(0.0, f32::max)
}

fn selected_rendered_values(samples: &[AnimationSampleV1]) -> Vec<&AnimationObservedValueV1> {
    samples
        .iter()
        .flat_map(|sample| sample.observed_values.iter())
        .filter(|value| value.kind == "transform")
        .collect()
}

fn times_advance(times: &[f32]) -> bool {
    let mut unique = BTreeSet::new();
    for time in times {
        unique.insert(OrderedTime(*time));
    }
    unique.len() > 1
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedTime(f32);

impl Eq for OrderedTime {}

impl PartialOrd for OrderedTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedTime {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

fn vec3_differs(left: Vec3, right: Vec3, tolerance: f32) -> bool {
    (left.x - right.x).abs() > tolerance
        || (left.y - right.y).abs() > tolerance
        || (left.z - right.z).abs() > tolerance
}

fn quat_differs(left: Quat, right: Quat, tolerance: f32) -> bool {
    (left.x - right.x).abs() > tolerance
        || (left.y - right.y).abs() > tolerance
        || (left.z - right.z).abs() > tolerance
        || (left.w - right.w).abs() > tolerance
}

fn push_reason(
    reasons: &mut Vec<AnimationIntrospectionReasonV1>,
    code: &str,
    severity: &str,
    message: &str,
) {
    reasons.push(AnimationIntrospectionReasonV1 {
        code: code.to_owned(),
        severity: severity.to_owned(),
        message: message.to_owned(),
    });
}

fn push_fix(fixes: &mut Vec<AnimationIntrospectionFixV1>, action: &str, help: &str) {
    if fixes.iter().any(|fix| fix.action == action) {
        return;
    }
    fixes.push(AnimationIntrospectionFixV1 {
        action: action.to_owned(),
        help: help.to_owned(),
    });
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_fails_when_selected_rendered_coverage_is_frozen() {
        let clip = AnimationClipIntrospectionV1 {
            name: "Move".to_owned(),
            duration_seconds: 1.0,
            channel_count: 1,
        };
        let channel_counts = AnimationChannelChangeCounts {
            changed: 1,
            unchanged: 0,
            invalid: 0,
        };
        let samples = vec![
            rendered_sample(0.0, "1111111111111111", 0.0, [24.0, 24.0]),
            rendered_sample(1.0, "2222222222222222", 1.0, [24.0, 24.0]),
        ];

        let report =
            AnimationIntrospectionReportV1::from_samples(clip, channel_counts, samples, true);

        assert!(!report.ok, "frozen rendered coverage must fail");
        assert!(report.summary.visible_change);
        assert!(!report.summary.rendered_movement);
        assert!(
            report
                .reasons
                .iter()
                .any(|reason| reason.code == "rendered_node_coverage_frozen"),
            "report must explain the stale rendered-node coverage: {report:#?}"
        );
    }

    fn rendered_sample(
        time_seconds: f32,
        payload: &str,
        translation_x: f32,
        rendered_centroid_css_px: [f32; 2],
    ) -> AnimationSampleV1 {
        AnimationSampleV1 {
            time_seconds,
            transform_revision: time_seconds as u64 + 1,
            appearance_revision: 1,
            payload_fnv1a64: payload.to_owned(),
            moving_node_count: usize::from(time_seconds > 0.0),
            invalid_node_count: 0,
            observed_values: vec![AnimationObservedValueV1 {
                id: "selected-transform".to_owned(),
                node: 7,
                kind: "transform".to_owned(),
                transform: Transform {
                    translation: Vec3::new(translation_x, 0.0, 0.0),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                },
                rendered_centroid_css_px: Some(rendered_centroid_css_px),
                rendered_coverage_px: Some(128),
                expected_translation: None,
                within_tolerance: None,
            }],
        }
    }
}
