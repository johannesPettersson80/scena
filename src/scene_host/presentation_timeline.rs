use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::animation::{invalid_input, validate_time_seconds};
use super::visual_patch::{
    VISUAL_PATCH_SCHEMA_V1, VisualPatchAnimationTimeModeV1, VisualPatchAnimationTimeV1,
    VisualPatchCameraEasedV1, VisualPatchHoverV1, VisualPatchLabelV1, VisualPatchMaterialVariantV1,
    VisualPatchResultV1, VisualPatchSectionBoxV1, VisualPatchSelectionV1, VisualPatchTintEasedV1,
    VisualPatchTintV1, VisualPatchTransformEasedV1, VisualPatchTransformV1, VisualPatchV1,
    VisualPatchVisibilityV1,
};
use super::{SceneHostCameraState, SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::AssetFetcher;
use crate::animation::AnimationLoopMode;

pub const PRESENTATION_TIMELINE_SCHEMA_V1: &str = "scena.presentation_timeline.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresentationTimelineV1 {
    pub schema: String,
    #[serde(default)]
    pub camera_bookmarks: Vec<PresentationTimelineCameraBookmarkV1>,
    #[serde(default)]
    pub actions: Vec<PresentationTimelineActionV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresentationTimelineCameraBookmarkV1 {
    pub name: String,
    pub camera: SceneHostCameraState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresentationTimelineActionV1 {
    pub at_seconds: f64,
    #[serde(flatten)]
    pub action: PresentationTimelineActionKindV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PresentationTimelineActionKindV1 {
    ApplyPatch {
        patch: Box<VisualPatchV1>,
    },
    ApplyVisualState {
        name: String,
    },
    CameraBookmark {
        name: String,
    },
    AnimationClip {
        mixer: u64,
        start_seconds: f64,
        speed: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        end_seconds: Option<f64>,
    },
}

#[derive(Default)]
struct TimelinePatchBuilder {
    transforms: BTreeMap<u64, VisualPatchTransformV1>,
    tints: BTreeMap<u64, VisualPatchTintV1>,
    visibility: BTreeMap<u64, VisualPatchVisibilityV1>,
    camera: Option<SceneHostCameraState>,
    transforms_eased: BTreeMap<u64, VisualPatchTransformEasedV1>,
    tints_eased: BTreeMap<u64, VisualPatchTintEasedV1>,
    camera_eased: Option<VisualPatchCameraEasedV1>,
    animation_time: BTreeMap<u64, VisualPatchAnimationTimeV1>,
    selection: Option<VisualPatchSelectionV1>,
    hover: Option<VisualPatchHoverV1>,
    material_variants: BTreeMap<u64, VisualPatchMaterialVariantV1>,
    labels: BTreeMap<String, VisualPatchLabelV1>,
    section_box: Option<VisualPatchSectionBoxV1>,
    metadata: Option<serde_json::Value>,
    echo_metadata: bool,
}

#[derive(Debug, Clone, Copy)]
struct ResolvedAnimationSegment {
    start_seconds: f64,
    end_seconds: f64,
    loop_mode: AnimationLoopMode,
}

impl PresentationTimelineV1 {
    pub fn new() -> Self {
        Self {
            schema: PRESENTATION_TIMELINE_SCHEMA_V1.to_owned(),
            camera_bookmarks: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn with_camera_bookmark(
        mut self,
        name: impl Into<String>,
        camera: SceneHostCameraState,
    ) -> Self {
        self.camera_bookmarks
            .push(PresentationTimelineCameraBookmarkV1 {
                name: name.into(),
                camera,
            });
        self
    }

    pub fn at(mut self, at_seconds: f64, action: PresentationTimelineActionKindV1) -> Self {
        self.actions
            .push(PresentationTimelineActionV1 { at_seconds, action });
        self
    }
}

impl Default for PresentationTimelineV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl PresentationTimelineActionKindV1 {
    pub fn apply_patch(patch: VisualPatchV1) -> Self {
        Self::ApplyPatch {
            patch: Box::new(patch),
        }
    }

    pub fn apply_state(name: impl Into<String>) -> Self {
        Self::ApplyVisualState { name: name.into() }
    }

    pub fn camera_bookmark(name: impl Into<String>) -> Self {
        Self::CameraBookmark { name: name.into() }
    }

    pub fn animation_clip(
        mixer: u64,
        start_seconds: f64,
        speed: f64,
        end_seconds: Option<f64>,
    ) -> Self {
        Self::AnimationClip {
            mixer,
            start_seconds,
            speed,
            end_seconds,
        }
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn timeline_patch(
        &self,
        timeline: &PresentationTimelineV1,
        seconds: f64,
    ) -> Result<VisualPatchV1, SceneHostError> {
        let seconds = validate_time_seconds("timeline seconds", seconds)?;
        validate_timeline(timeline)?;
        let resolved_animation_segments = timeline
            .actions
            .iter()
            .map(|action| self.resolve_animation_segment(action))
            .collect::<Result<Vec<_>, _>>()?;
        let mut builder = TimelinePatchBuilder::default();
        for (action, resolved_animation) in timeline
            .actions
            .iter()
            .zip(resolved_animation_segments)
            .filter(|(action, _)| action.at_seconds <= f64::from(seconds))
        {
            let patch = self.timeline_action_patch(
                timeline,
                action,
                resolved_animation,
                f64::from(seconds),
            )?;
            builder.merge(patch);
        }
        Ok(builder.finish())
    }

    pub fn timeline_patch_json(
        &self,
        timeline_json: &str,
        seconds: f64,
    ) -> Result<String, SceneHostError> {
        let timeline = parse_timeline_json(timeline_json)?;
        serde_json::to_string(&self.timeline_patch(&timeline, seconds)?).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("timeline patch serialization failed: {error}"),
            )
        })
    }

    pub fn seek_timeline(
        &mut self,
        timeline: &PresentationTimelineV1,
        seconds: f64,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        let patch = self.timeline_patch(timeline, seconds)?;
        self.apply_patch(&patch)
    }

    pub fn seek_timeline_json(
        &mut self,
        timeline_json: &str,
        seconds: f64,
    ) -> Result<String, SceneHostError> {
        let timeline = parse_timeline_json(timeline_json)?;
        let result = self.seek_timeline(&timeline, seconds)?;
        serde_json::to_string(&result).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("timeline seek result serialization failed: {error}"),
            )
        })
    }

    pub fn advance_timeline(
        &mut self,
        timeline: &PresentationTimelineV1,
        current_seconds: f64,
        delta_seconds: f64,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        let current_seconds = validate_time_seconds("timeline current_seconds", current_seconds)?;
        let delta_seconds = validate_time_seconds("timeline delta_seconds", delta_seconds)?;
        self.seek_timeline(timeline, f64::from(current_seconds + delta_seconds))
    }

    pub fn advance_timeline_json(
        &mut self,
        timeline_json: &str,
        current_seconds: f64,
        delta_seconds: f64,
    ) -> Result<String, SceneHostError> {
        let timeline = parse_timeline_json(timeline_json)?;
        let result = self.advance_timeline(&timeline, current_seconds, delta_seconds)?;
        serde_json::to_string(&result).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("timeline advance result serialization failed: {error}"),
            )
        })
    }

    fn timeline_action_patch(
        &self,
        timeline: &PresentationTimelineV1,
        action: &PresentationTimelineActionV1,
        resolved_animation: Option<ResolvedAnimationSegment>,
        seconds: f64,
    ) -> Result<VisualPatchV1, SceneHostError> {
        match &action.action {
            PresentationTimelineActionKindV1::ApplyPatch { patch } => Ok((**patch).clone()),
            PresentationTimelineActionKindV1::ApplyVisualState { name } => {
                Ok(self.visual_state(name)?.patch.clone())
            }
            PresentationTimelineActionKindV1::CameraBookmark { name } => {
                let camera = timeline
                    .camera_bookmarks
                    .iter()
                    .find(|bookmark| bookmark.name == *name)
                    .ok_or_else(|| {
                        invalid_input(format!("timeline camera bookmark '{name}' is not defined"))
                    })?
                    .camera;
                Ok(VisualPatchV1 {
                    camera: Some(camera),
                    ..VisualPatchV1::default()
                })
            }
            PresentationTimelineActionKindV1::AnimationClip { mixer, speed, .. } => {
                let elapsed = (seconds - action.at_seconds).max(0.0);
                let segment = resolved_animation.ok_or_else(|| {
                    invalid_input("timeline animation action was not pre-resolved")
                })?;
                let sample_seconds = segment.sample(elapsed, *speed);
                Ok(VisualPatchV1 {
                    animation_time: vec![VisualPatchAnimationTimeV1 {
                        mixer: *mixer,
                        mode: VisualPatchAnimationTimeModeV1::Seek,
                        seconds: sample_seconds,
                    }],
                    ..VisualPatchV1::default()
                })
            }
        }
    }

    fn resolve_animation_segment(
        &self,
        action: &PresentationTimelineActionV1,
    ) -> Result<Option<ResolvedAnimationSegment>, SceneHostError> {
        let PresentationTimelineActionKindV1::AnimationClip {
            mixer,
            start_seconds,
            end_seconds,
            ..
        } = &action.action
        else {
            return Ok(None);
        };
        let (duration_seconds, loop_mode) = self.animation_timeline_binding(*mixer)?;
        if *start_seconds > duration_seconds {
            return Err(invalid_input(format!(
                "timeline animation mixer {mixer} start_seconds {start_seconds} exceeds clip duration {duration_seconds}"
            )));
        }
        let end_seconds = end_seconds
            .unwrap_or(duration_seconds)
            .min(duration_seconds);
        Ok(Some(ResolvedAnimationSegment {
            start_seconds: *start_seconds,
            end_seconds,
            loop_mode,
        }))
    }
}

impl ResolvedAnimationSegment {
    fn sample(self, elapsed_seconds: f64, speed: f64) -> f64 {
        let span = self.end_seconds - self.start_seconds;
        if span <= f64::EPSILON {
            return self.start_seconds;
        }
        let offset = elapsed_seconds * speed;
        match self.loop_mode {
            AnimationLoopMode::Once => (self.start_seconds + offset).min(self.end_seconds),
            AnimationLoopMode::Repeat => {
                let remainder = offset.rem_euclid(span);
                let tolerance = f64::from(f32::EPSILON) * span.max(1.0) * 4.0;
                let stable_remainder = if remainder <= tolerance || span - remainder <= tolerance {
                    0.0
                } else {
                    remainder
                };
                self.start_seconds + stable_remainder
            }
        }
    }
}

impl TimelinePatchBuilder {
    fn merge(&mut self, patch: VisualPatchV1) {
        for entry in patch.transforms {
            self.transforms.insert(entry.node, entry);
        }
        for entry in patch.tints {
            self.tints.insert(entry.node, entry);
        }
        for entry in patch.visibility {
            self.visibility.insert(entry.node, entry);
        }
        if let Some(camera) = patch.camera {
            self.camera = Some(camera);
        }
        for entry in patch.transforms_eased {
            self.transforms_eased.insert(entry.node, entry);
        }
        for entry in patch.tints_eased {
            self.tints_eased.insert(entry.node, entry);
        }
        if let Some(camera_eased) = patch.camera_eased {
            self.camera_eased = Some(camera_eased);
        }
        for entry in patch.animation_time {
            self.animation_time.insert(entry.mixer, entry);
        }
        if let Some(selection) = patch.selection {
            self.selection = Some(selection);
        }
        if let Some(hover) = patch.hover {
            self.hover = Some(hover);
        }
        for entry in patch.material_variants {
            self.material_variants.insert(entry.import, entry);
        }
        for entry in patch.labels {
            self.labels.insert(entry.id.clone(), entry);
        }
        if let Some(section_box) = patch.section_box {
            self.section_box = Some(section_box);
        }
        if patch.metadata.is_some() {
            self.metadata = patch.metadata;
        }
        self.echo_metadata |= patch.echo_metadata;
    }

    fn finish(self) -> VisualPatchV1 {
        VisualPatchV1 {
            transforms: self.transforms.into_values().collect(),
            tints: self.tints.into_values().collect(),
            visibility: self.visibility.into_values().collect(),
            camera: self.camera,
            transforms_eased: self.transforms_eased.into_values().collect(),
            tints_eased: self.tints_eased.into_values().collect(),
            camera_eased: self.camera_eased,
            animation_time: self.animation_time.into_values().collect(),
            selection: self.selection,
            hover: self.hover,
            material_variants: self.material_variants.into_values().collect(),
            labels: self.labels.into_values().collect(),
            section_box: self.section_box,
            metadata: self.metadata,
            echo_metadata: self.echo_metadata,
            ..VisualPatchV1::default()
        }
    }
}

fn parse_timeline_json(json: &str) -> Result<PresentationTimelineV1, SceneHostError> {
    serde_json::from_str(json).map_err(|error| {
        SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("invalid presentation timeline JSON: {error}"),
        )
    })
}

fn validate_timeline(timeline: &PresentationTimelineV1) -> Result<(), SceneHostError> {
    if timeline.schema != PRESENTATION_TIMELINE_SCHEMA_V1 {
        return Err(invalid_input(format!(
            "unsupported presentation timeline schema {}; expected {}",
            timeline.schema, PRESENTATION_TIMELINE_SCHEMA_V1
        )));
    }

    let mut bookmark_names = BTreeSet::new();
    for bookmark in &timeline.camera_bookmarks {
        validate_name("timeline camera bookmark name", &bookmark.name)?;
        bookmark
            .camera
            .validate()
            .map_err(|message| invalid_input(message.to_owned()))?;
        if !bookmark_names.insert(bookmark.name.clone()) {
            return Err(invalid_input(format!(
                "duplicate timeline camera bookmark '{}'",
                bookmark.name
            )));
        }
    }

    for action in &timeline.actions {
        validate_time_seconds("timeline action at_seconds", action.at_seconds)?;
        match &action.action {
            PresentationTimelineActionKindV1::ApplyPatch { patch } => {
                if patch.schema != VISUAL_PATCH_SCHEMA_V1 {
                    return Err(invalid_input(format!(
                        "timeline patch action schema must be {}; got {}",
                        VISUAL_PATCH_SCHEMA_V1, patch.schema
                    )));
                }
            }
            PresentationTimelineActionKindV1::ApplyVisualState { name }
            | PresentationTimelineActionKindV1::CameraBookmark { name } => {
                validate_name("timeline action name", name)?;
            }
            PresentationTimelineActionKindV1::AnimationClip {
                start_seconds,
                speed,
                end_seconds,
                ..
            } => {
                validate_time_seconds("timeline animation start_seconds", *start_seconds)?;
                if !speed.is_finite() || *speed <= 0.0 || *speed > f64::from(f32::MAX) {
                    return Err(invalid_input(format!(
                        "timeline animation speed must be finite, > 0, and <= f32::MAX, got {speed}"
                    )));
                }
                if let Some(end_seconds) = end_seconds {
                    validate_time_seconds("timeline animation end_seconds", *end_seconds)?;
                    if end_seconds < start_seconds {
                        return Err(invalid_input(format!(
                            "timeline animation end_seconds must be >= start_seconds, got {end_seconds} < {start_seconds}"
                        )));
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_name(label: &str, value: &str) -> Result<(), SceneHostError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(invalid_input(format!(
            "{label} must be non-empty and contain no control characters"
        )));
    }
    Ok(())
}
