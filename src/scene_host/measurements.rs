use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, MeasurementKind, MeasurementOverlay, UnitFormat, Vec3};

pub const SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1: &str =
    "scena.scene_host_measurement_overlay.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostMeasurementOverlayReportV1 {
    pub schema: String,
    pub id: String,
    pub kind: String,
    pub value: f32,
    pub formatted_value: String,
    pub line_node: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_projection: Option<SceneHostMeasurementLabelProjectionV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneHostMeasurementLabelProjectionV1 {
    pub x_css_px: f32,
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
        let line_node = self.register_node(report.line_node);
        let label_text = label.map(|label| format!("{label}: {}", report.formatted_value));
        let label_projection = label
            .map(|_| self.project_measurement_label((start + end) * 0.5))
            .transpose()?;
        let report = SceneHostMeasurementOverlayReportV1 {
            schema: SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1.to_owned(),
            id: report.id,
            kind: measurement_kind_name(report.kind).to_owned(),
            value: report.value,
            formatted_value: report.formatted_value,
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
            x_css_px: projected.x,
            y_css_px: projected.y,
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
