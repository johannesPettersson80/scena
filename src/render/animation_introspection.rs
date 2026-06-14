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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimationIntrospectionSummaryV1 {
    pub sample_count: usize,
    pub changed_channel_count: usize,
    pub unchanged_channel_count: usize,
    pub invalid_channel_count: usize,
    pub visible_change: bool,
    pub capture_changes: usize,
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
