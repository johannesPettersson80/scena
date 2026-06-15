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
        let report = SceneHostMeasurementOverlayReportV1 {
            schema: SCENE_HOST_MEASUREMENT_OVERLAY_SCHEMA_V1.to_owned(),
            id: report.id,
            kind: measurement_kind_name(report.kind).to_owned(),
            value: report.value,
            formatted_value: report.formatted_value,
            line_node,
            label_text,
        };
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("measurement overlay serialization failed: {error}"),
            )
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
