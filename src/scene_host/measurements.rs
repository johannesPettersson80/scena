use serde::{Deserialize, Serialize, Serializer};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, MeasurementKind, MeasurementOverlay, UnitFormat, Vec3};

pub const SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1: &str =
    "scena.scene_host_measurement_overlay.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostMeasurementOverlayReportV1 {
    pub schema: String,
    pub id: String,
    pub kind: String,
    #[serde(serialize_with = "serialize_round3_f32")]
    pub value: f32,
    pub formatted_value: String,
    #[serde(default = "SceneHostMeasurementAuthorityV1::scene_space_visualization")]
    pub measurement_authority: SceneHostMeasurementAuthorityV1,
    pub line_node: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_projection: Option<SceneHostMeasurementLabelProjectionV1>,
}

/// Machine-readable boundary for scene-space measurement overlays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostMeasurementAuthorityV1 {
    pub scope: String,
    pub authoritative: bool,
    pub calibrated: bool,
    pub scene_units: String,
    pub source_unit_conversion: String,
    pub transform_space: String,
    pub precision: String,
    pub snapping_policy: String,
    pub occlusion_policy: String,
    pub unsupported_claims: Vec<String>,
}

impl SceneHostMeasurementAuthorityV1 {
    pub fn scene_space_visualization() -> Self {
        Self {
            scope: "scene_space_visualization".to_owned(),
            authoritative: false,
            calibrated: false,
            scene_units: "meters".to_owned(),
            source_unit_conversion: "import_policy_to_scene_meters".to_owned(),
            transform_space: "current_world_transform".to_owned(),
            precision: "f32_transformed_scene_coordinates".to_owned(),
            snapping_policy: "measurement_does_not_apply_snapping".to_owned(),
            occlusion_policy: "presentation_overlay_not_metrology_occlusion_proof".to_owned(),
            unsupported_claims: vec![
                "manufacturing_tolerance".to_owned(),
                "survey_accuracy".to_owned(),
                "calibrated_metrology".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneHostMeasurementLabelProjectionV1 {
    #[serde(serialize_with = "serialize_round3_f32")]
    pub x_css_px: f32,
    #[serde(serialize_with = "serialize_round3_f32")]
    pub y_css_px: f32,
    pub visible: bool,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn add_distance_measurement_json(
        &mut self,
        id: &str,
        start: Vec3,
        end: Vec3,
        label: Option<&str>,
        unit: &str,
        precision: u8,
    ) -> Result<String, SceneHostError> {
        let units = measurement_units(unit, precision)?;
        let mut overlay = MeasurementOverlay::distance(id, start, end).with_units(units);
        if let Some(label) = label {
            overlay = overlay.with_label(label);
        }
        let report = self.scene.add_measurement_overlay(&self.assets, overlay)?;
        let label_node = self
            .scene
            .measurement_overlay_state(&report.id)
            .and_then(|state| state.label_node());
        let line_node = self.register_node(report.line_node);
        if let Some(label_node) = label_node {
            self.register_node(label_node);
        }
        let label_text = label.map(|label| format!("{label}: {}", report.formatted_value));
        let label_projection = label
            .map(|_| self.project_measurement_label((start + end) * 0.5))
            .transpose()?;
        let report = SceneHostMeasurementOverlayReportV1 {
            schema: SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1.to_owned(),
            id: report.id,
            kind: measurement_kind_name(report.kind).to_owned(),
            value: round3(report.value),
            formatted_value: report.formatted_value,
            measurement_authority: SceneHostMeasurementAuthorityV1::scene_space_visualization(),
            line_node,
            label_text,
            label_projection,
        };
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("measurement overlay serialization failed: {error}"),
            )
        })
    }

    fn project_measurement_label(
        &self,
        world_position: Vec3,
    ) -> Result<SceneHostMeasurementLabelProjectionV1, SceneHostError> {
        let width = self.viewport.logical_width().round().max(1.0) as u32;
        let height = self.viewport.logical_height().round().max(1.0) as u32;
        let projected =
            self.scene
                .project_world_point(self.active_camera, world_position, width, height)?;
        let Some(projected) = projected else {
            return Ok(SceneHostMeasurementLabelProjectionV1 {
                x_css_px: 0.0,
                y_css_px: 0.0,
                visible: false,
            });
        };
        Ok(SceneHostMeasurementLabelProjectionV1 {
            x_css_px: round3(projected.x),
            y_css_px: round3(projected.y),
            visible: projected.ndc_x.abs() <= 1.0 && projected.ndc_y.abs() <= 1.0,
        })
    }
}

fn measurement_units(unit: &str, precision: u8) -> Result<UnitFormat, SceneHostError> {
    match unit {
        "m" | "meter" | "meters" => Ok(UnitFormat::meters().with_precision(precision)),
        "mm" | "millimeter" | "millimeters" => {
            Ok(UnitFormat::millimeters().with_precision(precision))
        }
        "" | "unit" | "units" => Ok(UnitFormat::custom(1.0, "", precision)),
        _ => Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("unsupported distance measurement unit {unit:?}; expected m, mm, or unit"),
        )),
    }
}

fn measurement_kind_name(kind: MeasurementKind) -> &'static str {
    match kind {
        MeasurementKind::Distance => "distance",
        MeasurementKind::Angle => "angle",
        MeasurementKind::BoundsDimension => "bounds_dimension",
    }
}

fn round3(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}

fn serialize_round3_f32<S>(value: &f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(stable_round3(*value))
}

fn stable_round3(value: f32) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    let rounded = (f64::from(value) * 1000.0).round() / 1000.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}
