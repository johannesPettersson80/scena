use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    Aabb, Angle, AssetFetcher, Camera, DepthOfFieldConfig, FramingOptions, OrbitControls, Vec3,
};

pub const PHOTO_CANDIDATE_PLAN_SCHEMA_V1: &str = "scena.photo_candidate_plan.v1";
pub const PHOTO_PLAN_SCHEMA_V1: &str = "scena.photo_plan.v1";
pub const PHOTO_REPORT_SCHEMA_V1: &str = "scena.photo_report.v1";
pub const PHOTO_SUBJECT_REGION_SCHEMA_V1: &str = "scena.photo_subject_region.v1";
pub const PHOTO_SHADED_CANDIDATE_SELECTION_SCHEMA_V1: &str =
    "scena.photo_shaded_candidate_selection.v1";

const CAMERA_BEHAVIOR_INTENT: &str = "camera_behavior";
const PRODUCT_HERO_COMPAT_INTENT: &str = "product_hero";
const DEFAULT_MAX_CANDIDATES: usize = 24;
const DEFAULT_FILL_MIN: f64 = 0.65;
const DEFAULT_FILL_MAX: f64 = 0.85;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoCandidateRequest {
    pub intent: String,
    pub subject_bounds: Aabb,
    pub viewport: [u32; 2],
    pub preferred_view: Option<String>,
    pub fill_range: PhotoCandidateFillRangeV1,
    pub max_candidates: usize,
    pub front_hint: Option<Vec3>,
    pub up_hint: Option<Vec3>,
    pub keep_visible_anchors: Vec<String>,
    pub staging_style: Option<String>,
    #[serde(default)]
    pub preserve_authored_camera: bool,
}

impl PhotoCandidateRequest {
    pub fn camera_behavior(subject_bounds: Aabb, viewport: (u32, u32)) -> Self {
        Self {
            intent: CAMERA_BEHAVIOR_INTENT.to_owned(),
            subject_bounds,
            viewport: [viewport.0, viewport.1],
            preferred_view: None,
            fill_range: PhotoCandidateFillRangeV1 {
                min: DEFAULT_FILL_MIN,
                max: DEFAULT_FILL_MAX,
            },
            max_candidates: DEFAULT_MAX_CANDIDATES,
            front_hint: None,
            up_hint: None,
            keep_visible_anchors: Vec::new(),
            staging_style: None,
            preserve_authored_camera: false,
        }
    }

    #[doc(hidden)]
    pub fn product_hero(subject_bounds: Aabb, viewport: (u32, u32)) -> Self {
        Self::camera_behavior(subject_bounds, viewport)
    }

    pub fn preferred_view(mut self, view: impl Into<String>) -> Self {
        self.preferred_view = Some(view.into());
        self
    }

    pub const fn fill_range(mut self, min: f64, max: f64) -> Self {
        self.fill_range = PhotoCandidateFillRangeV1 { min, max };
        self
    }

    pub const fn max_candidates(mut self, max_candidates: usize) -> Self {
        self.max_candidates = max_candidates;
        self
    }

    pub const fn front_hint(mut self, front_hint: Vec3) -> Self {
        self.front_hint = Some(front_hint);
        self
    }

    pub const fn up_hint(mut self, up_hint: Vec3) -> Self {
        self.up_hint = Some(up_hint);
        self
    }

    pub fn keep_visible_anchor(mut self, anchor: impl Into<String>) -> Self {
        let anchor = anchor.into();
        if !anchor.is_empty() && !self.keep_visible_anchors.contains(&anchor) {
            self.keep_visible_anchors.push(anchor);
        }
        self
    }

    pub fn staging_style(mut self, staging_style: impl Into<String>) -> Self {
        self.staging_style = Some(staging_style.into());
        self
    }

    pub const fn preserve_authored_camera(mut self, preserve: bool) -> Self {
        self.preserve_authored_camera = preserve;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhotoCandidateFillRangeV1 {
    pub min: f64,
    pub max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoCandidatePlanV1 {
    pub schema: String,
    pub intent: String,
    pub budget: usize,
    pub selected_candidate_id: String,
    pub constraints: PhotoCandidateConstraintsV1,
    pub candidates: Vec<PhotoCompositionCandidateV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoPlanV1 {
    pub schema: String,
    pub intent: String,
    pub source: PhotoPlanSourceV1,
    pub subject: PhotoPlanSubjectV1,
    pub planning: PhotoCandidatePlanV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoring: Option<PhotoCandidateScoringReport>,
    pub selected_candidate_id: String,
    pub candidates_evaluated: usize,
    pub rejected_candidate_reasons: BTreeMap<String, Vec<String>>,
    pub staging_choices: Vec<PhotoCandidateStagingV1>,
    pub artifacts: PhotoPlanArtifactsV1,
}

impl PhotoPlanV1 {
    pub fn validate_contract(&self) -> Result<(), &'static str> {
        if self.schema != PHOTO_PLAN_SCHEMA_V1 {
            return Err("photo_plan_schema_mismatch");
        }
        if self.intent.trim().is_empty() {
            return Err("photo_plan_intent_missing");
        }
        if self.planning.schema != PHOTO_CANDIDATE_PLAN_SCHEMA_V1 {
            return Err("photo_plan_candidate_plan_schema_mismatch");
        }
        if self.planning.candidates.is_empty() {
            return Err("photo_plan_candidates_missing");
        }
        if self.candidates_evaluated != self.planning.candidates.len() {
            return Err("photo_plan_candidate_count_mismatch");
        }
        if !self
            .planning
            .candidates
            .iter()
            .any(|candidate| candidate.id == self.selected_candidate_id)
        {
            return Err("photo_plan_selected_candidate_missing");
        }
        if self.staging_choices.is_empty() {
            return Err("photo_plan_staging_choices_missing");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoPlanSourceV1 {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoPlanSubjectV1 {
    pub target: PhotoPlanTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draw_handle_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoPlanTargetV1 {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoPlanArtifactsV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emitted_recipe_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoReportV1 {
    pub schema: String,
    pub status: String,
    pub ok: bool,
    pub intent: String,
    pub source: Value,
    pub subject: Value,
    pub planning: PhotoCandidatePlanV1,
    pub shaded_selection: Value,
    pub selected: Value,
    pub candidates: Vec<Value>,
    pub retry: Value,
    pub work_metrics: Value,
    pub acceptance: Value,
    pub quality: Value,
    pub focus_report: Value,
    pub exposure_report: Value,
    pub subject_observation: Value,
    pub subject_region: PhotoSubjectRegionV1,
    pub failure_codes: Vec<String>,
    pub artifacts: Value,
    pub build: Value,
}

impl PhotoReportV1 {
    pub fn validate_contract(&self) -> Result<(), &'static str> {
        if self.schema != PHOTO_REPORT_SCHEMA_V1 {
            return Err("photo_report_schema_mismatch");
        }
        if !matches!(self.status.as_str(), "passed" | "failed") {
            return Err("photo_report_status_invalid");
        }
        if self.ok != (self.status == "passed") {
            return Err("photo_report_ok_status_mismatch");
        }
        if self.intent.trim().is_empty() {
            return Err("photo_report_intent_missing");
        }
        if self.planning.schema != PHOTO_CANDIDATE_PLAN_SCHEMA_V1 {
            return Err("photo_report_planning_schema_mismatch");
        }
        if self.planning.candidates.is_empty() {
            return Err("photo_report_candidate_plan_empty");
        }
        if !has_object_key(&self.selected, "id") || !has_object_key(&self.selected, "status") {
            return Err("photo_report_selected_candidate_missing");
        }
        if self.candidates.is_empty() {
            return Err("photo_report_candidate_list_missing");
        }
        if value_schema(&self.shaded_selection) != Some(PHOTO_SHADED_CANDIDATE_SELECTION_SCHEMA_V1)
        {
            return Err("photo_report_shaded_selection_missing");
        }
        if !self.retry.is_object() {
            return Err("photo_report_retry_missing");
        }
        if !self.work_metrics.is_object() {
            return Err("photo_report_work_metrics_missing");
        }
        if !number_at_least(&self.work_metrics, "composition_candidate_budget", 1.0)
            || !number_at_least(&self.work_metrics, "composition_candidates", 1.0)
            || !number_at_least(&self.work_metrics, "shaded_candidate_budget", 1.0)
            || !number_at_least(&self.work_metrics, "shaded_candidate_renders", 1.0)
            || !number_at_least(&self.work_metrics, "final_candidate_renders", 1.0)
            || !number_at_least(&self.work_metrics, "total_render_calls", 1.0)
            || !number_at_least(&self.work_metrics, "prepare_calls", 1.0)
            || !number_at_least(&self.work_metrics, "capture_calls", 1.0)
            || !number_at_least(&self.work_metrics, "subject_meter_samples", 1.0)
        {
            return Err("photo_report_work_metrics_incomplete");
        }
        if !self.acceptance.is_object() {
            return Err("photo_report_acceptance_missing");
        }
        if !self.quality.get("subject").is_some_and(Value::is_object) {
            return Err("photo_report_quality_verdict_missing");
        }
        if value_schema(&self.focus_report) != Some(crate::render::FOCUS_REPORT_SCHEMA_V1) {
            return Err("photo_report_focus_report_missing");
        }
        if value_schema(&self.exposure_report) != Some(crate::render::EXPOSURE_REPORT_SCHEMA_V1) {
            return Err("photo_report_exposure_report_missing");
        }
        if value_schema(&self.subject_observation)
            != Some(crate::render::SUBJECT_OBSERVATION_SCHEMA_V1)
        {
            return Err("photo_report_subject_observation_missing");
        }
        self.subject_region.validate_contract()?;
        if !self.artifacts.is_object() {
            return Err("photo_report_artifacts_missing");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoSubjectRegionV1 {
    pub schema: String,
    pub source: String,
    pub target: crate::SubjectObservationTargetV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_bounds: Option<Aabb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projected_bounds: Option<crate::SubjectObservationBoundsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible_bounds: Option<crate::SubjectObservationBoundsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pixel_quality: Option<crate::SubjectObservationPixelQualityV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_distance_m: Option<f64>,
    pub frame_key: crate::SubjectObservationFrameKeyV1,
    pub stale: bool,
    pub fallback: crate::SubjectObservationFallbackV1,
}

impl PhotoSubjectRegionV1 {
    pub fn from_subject_observation(
        world_bounds: Option<Aabb>,
        focus_distance_m: Option<f64>,
        observation: &crate::SubjectObservationV1,
    ) -> Self {
        Self {
            schema: PHOTO_SUBJECT_REGION_SCHEMA_V1.to_owned(),
            source: "subject_observation.v1".to_owned(),
            target: observation.target.clone(),
            world_bounds,
            projected_bounds: observation.projected_bounds,
            visible_bounds: observation.visible_bounds,
            pixel_quality: observation.pixel_quality,
            focus_distance_m,
            frame_key: observation.frame_key.clone(),
            stale: observation.frame_key.state_binding != "exact_readback_completion",
            fallback: observation.fallback.clone(),
        }
    }

    pub fn validate_contract(&self) -> Result<(), &'static str> {
        if self.schema != PHOTO_SUBJECT_REGION_SCHEMA_V1 {
            return Err("photo_subject_region_schema_mismatch");
        }
        if self.source != "subject_observation.v1" {
            return Err("photo_subject_region_source_mismatch");
        }
        if self.target.kind.trim().is_empty()
            || self.target.id.trim().is_empty()
            || self.target.handles.is_empty()
        {
            return Err("photo_subject_region_target_missing");
        }
        if self.stale || self.frame_key.state_binding != "exact_readback_completion" {
            return Err("photo_subject_region_stale");
        }
        if self.world_bounds.is_none()
            || self.projected_bounds.is_none()
            || self.visible_bounds.is_none()
            || self.pixel_quality.is_none()
            || !self
                .focus_distance_m
                .is_some_and(|distance| distance.is_finite() && distance > 0.0)
        {
            return Err("photo_subject_region_incomplete");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoCandidateConstraintsV1 {
    pub viewport: [u32; 2],
    pub subject_bounds: Aabb,
    pub preferred_view: Option<String>,
    pub fill_range: PhotoCandidateFillRangeV1,
    pub front_hint: Option<Vec3>,
    pub up_hint: Option<Vec3>,
    pub keep_visible_anchors: Vec<String>,
    pub staging_style: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoCompositionCandidateV1 {
    pub id: String,
    pub order: usize,
    pub view: String,
    pub lens: String,
    pub focal_length_mm: f64,
    #[serde(default)]
    pub physical_camera: PhotoPhysicalCameraV1,
    pub fill_fraction: f64,
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub subject_yaw_deg: f64,
    #[serde(default)]
    pub preserve_authored_camera: bool,
    pub front_hint: Option<Vec3>,
    pub up_hint: Option<Vec3>,
    pub staging: PhotoCandidateStagingV1,
    pub keep_visible_anchors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhotoPhysicalCameraV1 {
    pub sensor_width_mm: f64,
    pub sensor_height_mm: f64,
    pub focal_length_mm: f64,
    pub aperture_f_stop: f64,
    pub focus_distance_m: f64,
    pub shutter_seconds: f64,
    pub sensitivity_iso: f64,
    pub exposure_compensation_ev: f64,
    pub circle_of_confusion_mm: f64,
    pub aperture_blades: u8,
}

impl Default for PhotoPhysicalCameraV1 {
    fn default() -> Self {
        Self {
            sensor_width_mm: 36.0,
            sensor_height_mm: 24.0,
            focal_length_mm: 50.0,
            aperture_f_stop: 8.0,
            focus_distance_m: 1.0,
            shutter_seconds: 1.0 / 125.0,
            sensitivity_iso: 100.0,
            exposure_compensation_ev: 0.0,
            circle_of_confusion_mm: 0.03,
            aperture_blades: 9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoCandidateStagingV1 {
    pub id: String,
    pub environment: String,
    pub background: String,
    pub ground: String,
    pub grid: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhotoCandidateObservation {
    pub candidate_id: String,
    pub visible_fill_fraction: f64,
    pub center_offset_fraction: f64,
    pub low_clip_fraction: f64,
    pub high_clip_fraction: f64,
    pub luminance_stddev_srgb8: f64,
    pub luminance_range_srgb8: f64,
    pub clipped_fraction: f64,
    pub occlusion_estimate: f64,
    pub floor_fraction: f64,
    pub silhouette_area_fraction: f64,
    pub aspect_fit_error: f64,
    pub depth_variation: f64,
    pub normal_variation: f64,
    pub anchor_visibility_fraction: f64,
    pub background_separation: f64,
    pub empty_space_fraction: f64,
    pub highlight_fraction: f64,
    pub highlight_continuity: f64,
    pub highlight_distribution: f64,
    pub shadow_presence: f64,
    pub shadow_softness: f64,
    pub silhouette_separation: f64,
    pub mean_saturation: f64,
    pub color_cast: f64,
    pub reflection_washout: f64,
    pub semantic_aov: bool,
}

impl PhotoCandidateObservation {
    pub fn new(candidate_id: impl Into<String>) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            visible_fill_fraction: 0.0,
            center_offset_fraction: 0.0,
            low_clip_fraction: 0.0,
            high_clip_fraction: 0.0,
            luminance_stddev_srgb8: 18.0,
            luminance_range_srgb8: 88.0,
            clipped_fraction: 0.0,
            occlusion_estimate: 0.0,
            floor_fraction: 0.0,
            silhouette_area_fraction: 0.0,
            aspect_fit_error: 0.0,
            depth_variation: 1.0,
            normal_variation: 1.0,
            anchor_visibility_fraction: 1.0,
            background_separation: 1.0,
            empty_space_fraction: 0.35,
            highlight_fraction: 0.08,
            highlight_continuity: 0.35,
            highlight_distribution: 0.50,
            shadow_presence: 0.15,
            shadow_softness: 0.70,
            silhouette_separation: 0.50,
            mean_saturation: 0.20,
            color_cast: 0.0,
            reflection_washout: 0.0,
            semantic_aov: false,
        }
    }

    pub fn visible_fill_fraction(mut self, value: f64) -> Self {
        self.visible_fill_fraction = value;
        self
    }

    pub fn center_offset_fraction(mut self, value: f64) -> Self {
        self.center_offset_fraction = value;
        self
    }

    pub fn low_clip_fraction(mut self, value: f64) -> Self {
        self.low_clip_fraction = value;
        self
    }

    pub fn high_clip_fraction(mut self, value: f64) -> Self {
        self.high_clip_fraction = value;
        self
    }

    pub fn luminance_stddev_srgb8(mut self, value: f64) -> Self {
        self.luminance_stddev_srgb8 = value;
        self
    }

    pub fn luminance_range_srgb8(mut self, value: f64) -> Self {
        self.luminance_range_srgb8 = value;
        self
    }

    pub fn clipped_fraction(mut self, value: f64) -> Self {
        self.clipped_fraction = value;
        self
    }

    pub fn occlusion_estimate(mut self, value: f64) -> Self {
        self.occlusion_estimate = value;
        self
    }

    pub fn floor_fraction(mut self, value: f64) -> Self {
        self.floor_fraction = value;
        self
    }

    pub fn silhouette_area_fraction(mut self, value: f64) -> Self {
        self.silhouette_area_fraction = value;
        self
    }

    pub fn aspect_fit_error(mut self, value: f64) -> Self {
        self.aspect_fit_error = value;
        self
    }

    pub fn depth_variation(mut self, value: f64) -> Self {
        self.depth_variation = value;
        self
    }

    pub fn normal_variation(mut self, value: f64) -> Self {
        self.normal_variation = value;
        self
    }

    pub fn anchor_visibility_fraction(mut self, value: f64) -> Self {
        self.anchor_visibility_fraction = value;
        self
    }

    pub fn background_separation(mut self, value: f64) -> Self {
        self.background_separation = value;
        self
    }

    pub fn appearance(
        mut self,
        empty_space_fraction: f64,
        highlight: [f64; 3],
        shadow: [f64; 2],
        color: [f64; 4],
    ) -> Self {
        self.empty_space_fraction = empty_space_fraction;
        self.highlight_fraction = highlight[0];
        self.highlight_continuity = highlight[1];
        self.highlight_distribution = highlight[2];
        self.shadow_presence = shadow[0];
        self.shadow_softness = shadow[1];
        self.silhouette_separation = color[0];
        self.mean_saturation = color[1];
        self.color_cast = color[2];
        self.reflection_washout = color[3];
        self
    }

    pub const fn semantic_aov(mut self, value: bool) -> Self {
        self.semantic_aov = value;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoCandidateScoringReport {
    pub selected_candidate_id: String,
    pub degraded: bool,
    pub reason_codes: Vec<String>,
    pub scores: Vec<PhotoCandidateScore>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoCandidateScore {
    pub candidate_id: String,
    pub order: usize,
    pub score: f64,
    pub reason_codes: Vec<String>,
}

fn value_schema(value: &Value) -> Option<&str> {
    value.get("schema").and_then(Value::as_str)
}

fn has_object_key(value: &Value, key: &str) -> bool {
    value.get(key).is_some()
}

fn number_at_least(value: &Value, key: &str, minimum: f64) -> bool {
    value
        .get(key)
        .and_then(Value::as_f64)
        .is_some_and(|value| value >= minimum)
}

#[derive(Debug, Clone, Copy)]
struct ViewSpec {
    name: &'static str,
    azimuth_deg: f64,
    elevation_deg: f64,
}

#[derive(Debug, Clone, Copy)]
struct StagingSpec {
    id: &'static str,
    environment: &'static str,
    background: &'static str,
    ground: &'static str,
    grid: bool,
}

const AUTOMATIC_STAGING: StagingSpec = StagingSpec {
    id: "automatic",
    environment: "automatic",
    background: "automatic",
    ground: "automatic",
    grid: false,
};

pub fn camera_behavior_candidate_plan(
    request: PhotoCandidateRequest,
) -> Result<PhotoCandidatePlanV1, SceneHostError> {
    validate_camera_behavior_request(&request)?;
    let budget = request.max_candidates;
    let constraints = PhotoCandidateConstraintsV1 {
        viewport: request.viewport,
        subject_bounds: request.subject_bounds,
        preferred_view: request.preferred_view.clone(),
        fill_range: request.fill_range,
        front_hint: request.front_hint,
        up_hint: request.up_hint,
        keep_visible_anchors: request.keep_visible_anchors.clone(),
        staging_style: request.staging_style.clone(),
    };
    let views = derived_views(&request)?;
    let cameras = derived_physical_cameras(&request);
    let fills = fill_candidates(request.fill_range);
    let stages = staging_candidates(request.staging_style.as_deref())?;
    let mut candidates = Vec::new();

    if request.preserve_authored_camera {
        let physical_camera = cameras[0];
        candidates.push(PhotoCompositionCandidateV1 {
            id: "camera_behavior_authored_camera".to_owned(),
            order: 0,
            view: "authored_camera".to_owned(),
            lens: "authored".to_owned(),
            focal_length_mm: physical_camera.focal_length_mm,
            physical_camera,
            fill_fraction: fills[0],
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            subject_yaw_deg: 0.0,
            preserve_authored_camera: true,
            front_hint: request.front_hint,
            up_hint: request.up_hint,
            staging: PhotoCandidateStagingV1 {
                id: AUTOMATIC_STAGING.id.to_owned(),
                environment: AUTOMATIC_STAGING.environment.to_owned(),
                background: AUTOMATIC_STAGING.background.to_owned(),
                ground: AUTOMATIC_STAGING.ground.to_owned(),
                grid: false,
            },
            keep_visible_anchors: request.keep_visible_anchors.clone(),
        });
    }

    'outer: for view in views {
        for physical_camera in cameras {
            for fill_fraction in fills {
                for stage in &stages {
                    let order = candidates.len();
                    candidates.push(PhotoCompositionCandidateV1 {
                        id: candidate_id(view, physical_camera, fill_fraction, stage),
                        order,
                        view: view.name.to_owned(),
                        lens: "physical".to_owned(),
                        focal_length_mm: physical_camera.focal_length_mm,
                        physical_camera,
                        fill_fraction,
                        azimuth_deg: view.azimuth_deg,
                        elevation_deg: view.elevation_deg,
                        subject_yaw_deg: 0.0,
                        preserve_authored_camera: false,
                        front_hint: request.front_hint,
                        up_hint: request.up_hint,
                        staging: PhotoCandidateStagingV1 {
                            id: stage.id.to_owned(),
                            environment: stage.environment.to_owned(),
                            background: stage.background.to_owned(),
                            ground: stage.ground.to_owned(),
                            grid: stage.grid,
                        },
                        keep_visible_anchors: request.keep_visible_anchors.clone(),
                    });
                    if candidates.len() >= budget {
                        break 'outer;
                    }
                }
            }
        }
    }

    let selected_candidate_id = candidates
        .first()
        .map(|candidate| candidate.id.clone())
        .ok_or_else(|| {
            invalid_photo_request("photo candidate generation produced no candidates")
        })?;
    Ok(PhotoCandidatePlanV1 {
        schema: PHOTO_CANDIDATE_PLAN_SCHEMA_V1.to_owned(),
        intent: CAMERA_BEHAVIOR_INTENT.to_owned(),
        budget,
        selected_candidate_id,
        constraints,
        candidates,
    })
}

#[doc(hidden)]
pub fn product_hero_candidate_plan(
    request: PhotoCandidateRequest,
) -> Result<PhotoCandidatePlanV1, SceneHostError> {
    camera_behavior_candidate_plan(request)
}

pub fn score_camera_behavior_candidates(
    plan: &PhotoCandidatePlanV1,
    observations: &[PhotoCandidateObservation],
) -> Result<PhotoCandidateScoringReport, SceneHostError> {
    let observations_by_id = observations
        .iter()
        .map(|observation| (observation.candidate_id.as_str(), observation))
        .collect::<BTreeMap<_, _>>();
    let mut report_reasons = BTreeSet::new();
    let mut scores = Vec::with_capacity(plan.candidates.len());
    for candidate in &plan.candidates {
        let Some(observation) = observations_by_id.get(candidate.id.as_str()) else {
            report_reasons.insert("candidate_observation_missing".to_owned());
            scores.push(PhotoCandidateScore {
                candidate_id: candidate.id.clone(),
                order: candidate.order,
                score: f64::NEG_INFINITY,
                reason_codes: vec!["candidate_observation_missing".to_owned()],
            });
            continue;
        };
        let score = score_camera_behavior_observation(observation, &mut report_reasons);
        scores.push(PhotoCandidateScore {
            candidate_id: candidate.id.clone(),
            order: candidate.order,
            score: score.score,
            reason_codes: score.reason_codes,
        });
    }
    let selected_candidate_id = scores
        .iter()
        .max_by(|left, right| {
            left.score
                .total_cmp(&right.score)
                .then_with(|| right.order.cmp(&left.order))
        })
        .map(|score| score.candidate_id.clone())
        .ok_or_else(|| invalid_photo_request("photo candidate scoring requires candidates"))?;
    Ok(PhotoCandidateScoringReport {
        selected_candidate_id,
        degraded: !report_reasons.is_empty(),
        reason_codes: report_reasons.into_iter().collect(),
        scores,
    })
}

#[doc(hidden)]
pub fn score_product_hero_candidates(
    plan: &PhotoCandidatePlanV1,
    observations: &[PhotoCandidateObservation],
) -> Result<PhotoCandidateScoringReport, SceneHostError> {
    score_camera_behavior_candidates(plan, observations)
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn viewport_size(&self) -> (u32, u32) {
        (
            self.viewport.logical_width().round().max(1.0) as u32,
            self.viewport.logical_height().round().max(1.0) as u32,
        )
    }

    pub fn frame_node_with_photo_candidate(
        &mut self,
        node: u64,
        candidate: &PhotoCompositionCandidateV1,
    ) -> Result<(), SceneHostError> {
        self.ensure_active_camera()?;
        let node = self.resolve_node(node)?;
        let bounds = self
            .scene
            .node_world_bounds(node, &self.assets)?
            .ok_or(crate::LookupError::ImportHasNoBounds)?;
        if candidate.preserve_authored_camera {
            let camera_position = self
                .scene
                .camera_node(self.active_camera)
                .and_then(|node| self.scene.world_transform(node))
                .map(|transform| transform.translation)
                .ok_or(crate::LookupError::NoActiveCamera)?;
            let focus_distance = (camera_position - bounds.center()).length().max(0.001);
            self.renderer
                .set_depth_of_field(Some(DepthOfFieldConfig::physical(
                    focus_distance,
                    candidate.physical_camera.focal_length_mm as f32,
                    candidate.physical_camera.sensor_height_mm as f32,
                    candidate.physical_camera.aperture_f_stop as f32,
                    candidate.physical_camera.aperture_blades,
                    16,
                )));
            return Ok(());
        }
        let (width, height) = self.viewport_size();
        if let Some(Camera::Perspective(mut camera)) =
            self.scene.camera(self.active_camera).cloned()
        {
            let vertical_fov = 2.0
                * (candidate.physical_camera.sensor_height_mm
                    / (2.0 * candidate.physical_camera.focal_length_mm))
                    .atan();
            camera.vertical_fov = Angle::from_radians(vertical_fov as f32);
            camera.aspect = width.max(1) as f32 / height.max(1) as f32;
            self.scene
                .set_camera(self.active_camera, Camera::Perspective(camera))?;
        }
        let margin_px = photo_candidate_margin_px(width, height, candidate.fill_fraction);
        let mut options = FramingOptions::new()
            .azimuth_elevation(candidate.azimuth_deg as f32, candidate.elevation_deg as f32)
            .fill(candidate.fill_fraction as f32)
            .margin_px(margin_px)
            .viewport(width, height);
        if let Some(front_hint) = candidate.front_hint {
            options = options.look_from(front_hint);
        }
        if let Some(up_hint) = candidate.up_hint {
            options = options.up(up_hint);
        }
        let mut framing = self
            .scene
            .frame_bounds(self.active_camera, bounds, options)?;
        if let Some(visual_center) = photographic_visual_center(&self.scene, &self.assets, node) {
            let offset = visual_center - framing.target;
            if offset.length_squared().is_finite() && offset.length_squared() > 1.0e-12 {
                framing.camera_transform.translation += offset;
                framing.target = visual_center;
                if let Some(camera_node) = self.scene.camera_node(self.active_camera) {
                    self.scene
                        .set_transform(camera_node, framing.camera_transform)?;
                }
            }
        }
        let focus_distance = framing.distance.max(0.001);
        self.renderer
            .set_depth_of_field(Some(DepthOfFieldConfig::physical(
                focus_distance,
                candidate.physical_camera.focal_length_mm as f32,
                candidate.physical_camera.sensor_height_mm as f32,
                candidate.physical_camera.aperture_f_stop as f32,
                candidate.physical_camera.aperture_blades,
                16,
            )));
        self.cancel_camera_transition();
        self.camera_controls = OrbitControls::from_framing(framing);
        Ok(())
    }
}

fn photographic_visual_center<F: AssetFetcher>(
    scene: &crate::Scene,
    assets: &crate::Assets<F>,
    root: crate::NodeKey,
) -> Option<Vec3> {
    let subtree = scene
        .subtree_nodes(root)
        .ok()?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let inspection = scene.inspect_with_assets(assets);
    let mut weighted = Vec3::ZERO;
    let mut total_weight = 0.0_f32;
    for draw in inspection
        .draw_list()
        .iter()
        .filter(|draw| subtree.contains(&draw.node()))
    {
        let transform = draw.world_transform();
        let local_center = draw.local_bounds().center();
        let world_center =
            transform.translation + transform.rotation * (local_center * transform.scale);
        let extent = draw.local_bounds().max - draw.local_bounds().min;
        let geometric_weight = (extent.x * extent.y + extent.y * extent.z + extent.z * extent.x)
            .abs()
            .max(1.0e-6);
        let detail_weight = (draw.primitive_count().max(1) as f32).sqrt();
        let weight = geometric_weight * detail_weight;
        weighted += world_center * weight;
        total_weight += weight;
    }
    (total_weight > 0.0).then_some(weighted / total_weight)
}

fn photo_candidate_margin_px(width: u32, height: u32, fill_fraction: f64) -> f32 {
    let min_viewport = width.min(height) as f32;
    if fill_fraction >= 0.98 {
        return (min_viewport * 0.018).clamp(12.0, 24.0);
    }
    (min_viewport * 0.06).clamp(10.0, 48.0)
}

fn validate_camera_behavior_request(request: &PhotoCandidateRequest) -> Result<(), SceneHostError> {
    if !is_camera_behavior_intent(&request.intent) {
        return Err(invalid_photo_request(format!(
            "unsupported photo intent '{}'; expected {CAMERA_BEHAVIOR_INTENT}",
            request.intent
        )));
    }
    if request.viewport[0] == 0 || request.viewport[1] == 0 {
        return Err(invalid_photo_request(
            "photo candidate viewport dimensions must be greater than zero",
        ));
    }
    if request.max_candidates == 0 {
        return Err(invalid_photo_request(
            "photo candidate budget must be greater than zero",
        ));
    }
    if !valid_bounds(request.subject_bounds) {
        return Err(invalid_photo_request(
            "photo subject bounds must be finite and non-degenerate",
        ));
    }
    if request.front_hint.is_some_and(|hint| !valid_hint(hint)) {
        return Err(invalid_photo_request(
            "photo front hint must be finite and non-zero",
        ));
    }
    if request.up_hint.is_some_and(|hint| !valid_hint(hint)) {
        return Err(invalid_photo_request(
            "photo up hint must be finite and non-zero",
        ));
    }
    if !request.fill_range.min.is_finite()
        || !request.fill_range.max.is_finite()
        || request.fill_range.min <= 0.0
        || request.fill_range.max > 1.0
        || request.fill_range.min > request.fill_range.max
    {
        return Err(invalid_photo_request(
            "photo fill range must be finite with 0 < min <= max <= 1",
        ));
    }
    Ok(())
}

fn is_camera_behavior_intent(intent: &str) -> bool {
    matches!(
        intent,
        CAMERA_BEHAVIOR_INTENT | "camera-behavior" | PRODUCT_HERO_COMPAT_INTENT | "product-hero"
    )
}

struct CandidateScoreWork {
    score: f64,
    reason_codes: Vec<String>,
}

fn score_camera_behavior_observation(
    observation: &PhotoCandidateObservation,
    report_reasons: &mut BTreeSet<String>,
) -> CandidateScoreWork {
    let mut score = 100.0;
    let mut reasons = Vec::new();
    penalty_below(
        observation.visible_fill_fraction,
        DEFAULT_FILL_MIN,
        220.0,
        18.0,
        "subject_fill_below_min",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.visible_fill_fraction,
        DEFAULT_FILL_MAX,
        180.0,
        14.0,
        "subject_fill_above_max",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.center_offset_fraction,
        0.16,
        180.0,
        18.0,
        "subject_off_center",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.low_clip_fraction,
        0.20,
        120.0,
        24.0,
        "subject_black_crush",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.high_clip_fraction,
        0.05,
        120.0,
        20.0,
        "subject_highlight_clip",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.clipped_fraction,
        0.01,
        180.0,
        24.0,
        "subject_clipped",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.occlusion_estimate,
        0.18,
        150.0,
        18.0,
        "subject_occluded",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.floor_fraction,
        0.34,
        160.0,
        22.0,
        "floor_dominates_frame",
        &mut score,
        &mut reasons,
    );
    penalty_below(
        observation.silhouette_area_fraction,
        0.26,
        150.0,
        14.0,
        "silhouette_area_below_min",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.aspect_fit_error,
        0.24,
        100.0,
        10.0,
        "aspect_fit_poor",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.empty_space_fraction,
        0.72,
        80.0,
        8.0,
        "empty_space_excessive",
        &mut score,
        &mut reasons,
    );
    penalty_below(
        observation.highlight_continuity,
        0.08,
        90.0,
        7.0,
        "specular_structure_fragmented",
        &mut score,
        &mut reasons,
    );
    penalty_below(
        observation.highlight_distribution,
        0.25,
        50.0,
        5.0,
        "specular_structure_missing",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.reflection_washout,
        0.20,
        80.0,
        9.0,
        "surface_reflection_washout",
        &mut score,
        &mut reasons,
    );
    penalty_below(
        observation.shadow_presence,
        0.01,
        120.0,
        5.0,
        "contact_shadow_missing",
        &mut score,
        &mut reasons,
    );
    penalty_below(
        observation.shadow_softness,
        0.20,
        40.0,
        4.0,
        "shadow_too_hard",
        &mut score,
        &mut reasons,
    );
    penalty_below(
        observation.silhouette_separation,
        0.10,
        100.0,
        10.0,
        "silhouette_separation_low",
        &mut score,
        &mut reasons,
    );
    penalty_above(
        observation.color_cast,
        0.18,
        80.0,
        6.0,
        "white_balance_color_cast",
        &mut score,
        &mut reasons,
    );
    if observation.depth_variation < 0.10
        || observation.normal_variation < 0.10
        || observation.background_separation < 0.18
        || observation.luminance_stddev_srgb8 < 6.0
        || observation.luminance_range_srgb8 < 32.0
    {
        score -= (0.10 - observation.depth_variation).max(0.0) * 160.0;
        score -= (0.10 - observation.normal_variation).max(0.0) * 160.0;
        score -= (0.18 - observation.background_separation).max(0.0) * 140.0;
        score -= (6.0 - observation.luminance_stddev_srgb8).max(0.0) * 2.0;
        score -= (32.0 - observation.luminance_range_srgb8).max(0.0) * 0.6;
        score -= 18.0;
        reasons.push("subject_readability_low".to_owned());
    }
    penalty_below(
        observation.anchor_visibility_fraction,
        0.99,
        120.0,
        12.0,
        "keep_visible_anchor_hidden",
        &mut score,
        &mut reasons,
    );
    if !observation.semantic_aov {
        score -= 8.0;
        reasons.push("semantic_aov_unavailable".to_owned());
        report_reasons.insert("semantic_aov_unavailable".to_owned());
    }
    CandidateScoreWork {
        score,
        reason_codes: reasons,
    }
}

fn penalty_below(
    value: f64,
    threshold: f64,
    scale: f64,
    base: f64,
    code: &str,
    score: &mut f64,
    reasons: &mut Vec<String>,
) {
    if value < threshold {
        *score -= (threshold - value) * scale + base;
        reasons.push(code.to_owned());
    }
}

fn penalty_above(
    value: f64,
    threshold: f64,
    scale: f64,
    base: f64,
    code: &str,
    score: &mut f64,
    reasons: &mut Vec<String>,
) {
    if value > threshold {
        *score -= (value - threshold) * scale + base;
        reasons.push(code.to_owned());
    }
}

fn valid_hint(value: Vec3) -> bool {
    value.x.is_finite()
        && value.y.is_finite()
        && value.z.is_finite()
        && value.length_squared() > 1.0e-12
}

fn valid_bounds(bounds: Aabb) -> bool {
    bounds.min.x.is_finite()
        && bounds.min.y.is_finite()
        && bounds.min.z.is_finite()
        && bounds.max.x.is_finite()
        && bounds.max.y.is_finite()
        && bounds.max.z.is_finite()
        && bounds.min.x < bounds.max.x
        && bounds.min.y < bounds.max.y
        && bounds.min.z < bounds.max.z
}

fn derived_views(request: &PhotoCandidateRequest) -> Result<[ViewSpec; 4], SceneHostError> {
    if let Some(preferred) = request.preferred_view.as_deref() {
        let (azimuth_deg, elevation_deg) = match preferred {
            "three_quarter_front_right" => (45.0, 28.0),
            "three_quarter_front_left" => (-45.0, 28.0),
            "front" => (0.0, 18.0),
            "right" => (90.0, 22.0),
            _ => {
                return Err(invalid_photo_request(format!(
                    "unsupported camera behavior view '{preferred}'"
                )));
            }
        };
        return Ok([
            ViewSpec {
                name: "authored_direction",
                azimuth_deg,
                elevation_deg,
            },
            ViewSpec {
                name: "authored_direction",
                azimuth_deg: azimuth_deg - 12.0,
                elevation_deg: elevation_deg + 4.0,
            },
            ViewSpec {
                name: "authored_direction",
                azimuth_deg: azimuth_deg + 12.0,
                elevation_deg: elevation_deg - 4.0,
            },
            ViewSpec {
                name: "authored_direction",
                azimuth_deg: -azimuth_deg,
                elevation_deg,
            },
        ]);
    }
    let size = request.subject_bounds.max - request.subject_bounds.min;
    let horizontal_total = f64::from(size.x + size.z).max(1.0e-6);
    let depth_share = f64::from(size.z) / horizontal_total;
    let base_azimuth = (24.0 + depth_share * 38.0).clamp(24.0, 62.0);
    let flatness = f64::from(size.y / size.x.max(size.z).max(1.0e-6));
    let base_elevation = (32.0 - flatness * 14.0).clamp(12.0, 34.0);
    Ok([
        ViewSpec {
            name: "geometry_derived",
            azimuth_deg: base_azimuth,
            elevation_deg: base_elevation,
        },
        ViewSpec {
            name: "geometry_derived",
            azimuth_deg: -base_azimuth,
            elevation_deg: base_elevation,
        },
        ViewSpec {
            name: "geometry_derived",
            azimuth_deg: (base_azimuth * 0.68).clamp(18.0, 48.0),
            elevation_deg: (base_elevation + 7.0).clamp(12.0, 40.0),
        },
        ViewSpec {
            name: "geometry_derived",
            azimuth_deg: -(base_azimuth * 0.68).clamp(18.0, 48.0),
            elevation_deg: (base_elevation - 5.0).clamp(8.0, 34.0),
        },
    ])
}

fn derived_physical_cameras(request: &PhotoCandidateRequest) -> [PhotoPhysicalCameraV1; 3] {
    let size = request.subject_bounds.max - request.subject_bounds.min;
    let primary = f64::from(size.x.max(size.y)).max(1.0e-6);
    let depth_ratio = (f64::from(size.z) / primary).clamp(0.0, 4.0);
    let focal = (52.0 + depth_ratio * 14.0).clamp(42.0, 88.0);
    let diagonal = f64::from(size.length()).max(1.0e-6);
    let focus = diagonal * (focal / 24.0).max(1.4);
    let aperture = (5.6 + depth_ratio * 2.8).clamp(5.6, 13.0);
    [0.0, 10.0, -8.0].map(|offset| PhotoPhysicalCameraV1 {
        sensor_width_mm: 36.0,
        sensor_height_mm: 24.0,
        focal_length_mm: (focal + offset).clamp(35.0, 105.0),
        aperture_f_stop: aperture,
        focus_distance_m: focus,
        shutter_seconds: 1.0 / 125.0,
        sensitivity_iso: 100.0,
        exposure_compensation_ev: 0.0,
        circle_of_confusion_mm: 0.03,
        aperture_blades: 9,
    })
}

fn fill_candidates(range: PhotoCandidateFillRangeV1) -> [f64; 3] {
    let preferred = 0.78_f64.clamp(range.min, range.max);
    let upper = (range.max - 0.01)
        .max(range.min)
        .clamp(range.min, range.max);
    let lower = (range.min + 0.01)
        .min(range.max)
        .clamp(range.min, range.max);
    let mut fills = [preferred, upper, lower];
    fills.sort_by(|a, b| {
        distance_from_preferred(*a)
            .total_cmp(&distance_from_preferred(*b))
            .then_with(|| b.total_cmp(a))
    });
    fills
}

fn distance_from_preferred(value: f64) -> f64 {
    (value - 0.78).abs()
}

fn staging_candidates(preferred: Option<&str>) -> Result<Vec<StagingSpec>, SceneHostError> {
    match preferred {
        Some("automatic") | None => Ok(vec![AUTOMATIC_STAGING]),
        Some("dark_studio") | Some("dark_matte") => Ok(vec![AUTOMATIC_STAGING]),
        Some(other) => Err(invalid_photo_request(format!(
            "unsupported camera behavior staging style '{other}'"
        ))),
    }
}

fn candidate_id(
    view: ViewSpec,
    physical_camera: PhotoPhysicalCameraV1,
    fill_fraction: f64,
    stage: &StagingSpec,
) -> String {
    format!(
        "camera_behavior_view_{}_focal_{}_fill_{}_az_{}_elev_{}_stage_{}",
        view.name,
        integer_id(physical_camera.focal_length_mm),
        decimal_id(fill_fraction, 2),
        integer_id(view.azimuth_deg),
        integer_id(view.elevation_deg),
        stage.id
    )
}

fn decimal_id(value: f64, precision: usize) -> String {
    format!("{value:.precision$}").replace('.', "_")
}

fn integer_id(value: f64) -> String {
    format!("{value:.0}").replace('-', "neg_")
}

fn invalid_photo_request(message: impl Into<String>) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message.into())
}
