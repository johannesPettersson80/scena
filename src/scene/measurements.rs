use std::f32::consts::PI;

use crate::assets::Assets;
use crate::diagnostics::LookupError;
use crate::geometry::{Aabb, GeometryDesc, GeometryTopology, GeometryVertex};
use crate::material::{Color, MaterialDesc};

use super::{LabelDesc, LabelKey, NodeKey, Scene, Transform, Vec3};

const MEASUREMENT_LABEL_OFFSET_FRACTION: f32 = 0.20;
const MEASUREMENT_LABEL_OFFSET_MIN_WORLD: f32 = 0.012;
const MEASUREMENT_LABEL_OFFSET_MAX_WORLD: f32 = 0.28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementKind {
    Distance,
    Angle,
    BoundsDimension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitFormat {
    scale: f32,
    suffix: String,
    precision: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementOverlay {
    id: String,
    kind: MeasurementGeometry,
    units: UnitFormat,
    label: Option<String>,
    color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementReport {
    pub id: String,
    pub kind: MeasurementKind,
    pub value: f32,
    pub formatted_value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementOverlayReport {
    pub id: String,
    pub kind: MeasurementKind,
    pub value: f32,
    pub formatted_value: String,
    pub line_node: NodeKey,
    pub label: Option<LabelKey>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SceneMeasurementOverlayState {
    line_node: NodeKey,
    label_node: Option<NodeKey>,
}

#[derive(Debug, Clone, PartialEq)]
enum MeasurementGeometry {
    Distance { start: Vec3, end: Vec3 },
    Angle { a: Vec3, vertex: Vec3, b: Vec3 },
    BoundsDimension { bounds: Aabb, axis: MeasurementAxis },
}

impl MeasurementOverlay {
    pub fn distance(id: impl Into<String>, start: Vec3, end: Vec3) -> Self {
        Self {
            id: id.into(),
            kind: MeasurementGeometry::Distance { start, end },
            units: UnitFormat::meters(),
            label: None,
            color: Color::CYAN,
        }
    }

    pub fn angle(id: impl Into<String>, a: Vec3, vertex: Vec3, b: Vec3) -> Self {
        Self {
            id: id.into(),
            kind: MeasurementGeometry::Angle { a, vertex, b },
            units: UnitFormat::degrees(),
            label: None,
            color: Color::CYAN,
        }
    }

    pub fn bounds_dimension(id: impl Into<String>, bounds: Aabb, axis: MeasurementAxis) -> Self {
        Self {
            id: id.into(),
            kind: MeasurementGeometry::BoundsDimension { bounds, axis },
            units: UnitFormat::meters(),
            label: None,
            color: Color::CYAN,
        }
    }

    pub fn with_units(mut self, units: UnitFormat) -> Self {
        self.units = units;
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn measure(&self) -> Result<MeasurementReport, LookupError> {
        let value = self.value()?;
        Ok(MeasurementReport {
            id: self.id.clone(),
            kind: self.kind.kind(),
            value,
            formatted_value: self.units.format(value),
        })
    }

    fn value(&self) -> Result<f32, LookupError> {
        match self.kind {
            MeasurementGeometry::Distance { start, end } => {
                validate_point(start)?;
                validate_point(end)?;
                Ok(start.distance(end))
            }
            MeasurementGeometry::Angle { a, vertex, b } => angle_between(a, vertex, b),
            MeasurementGeometry::BoundsDimension { bounds, axis } => {
                validate_bounds(bounds)?;
                Ok(match axis {
                    MeasurementAxis::X => bounds.max.x - bounds.min.x,
                    MeasurementAxis::Y => bounds.max.y - bounds.min.y,
                    MeasurementAxis::Z => bounds.max.z - bounds.min.z,
                })
            }
        }
    }

    fn line_segments(&self) -> Result<Vec<(Vec3, Vec3)>, LookupError> {
        match self.kind {
            MeasurementGeometry::Distance { start, end } => {
                validate_point(start)?;
                validate_point(end)?;
                Ok(vec![(start, end)])
            }
            MeasurementGeometry::Angle { a, vertex, b } => {
                angle_between(a, vertex, b)?;
                Ok(vec![(vertex, a), (vertex, b)])
            }
            MeasurementGeometry::BoundsDimension { bounds, axis } => {
                validate_bounds(bounds)?;
                let center = bounds.center();
                let (start, end) = match axis {
                    MeasurementAxis::X => (
                        Vec3::new(bounds.min.x, center.y, center.z),
                        Vec3::new(bounds.max.x, center.y, center.z),
                    ),
                    MeasurementAxis::Y => (
                        Vec3::new(center.x, bounds.min.y, center.z),
                        Vec3::new(center.x, bounds.max.y, center.z),
                    ),
                    MeasurementAxis::Z => (
                        Vec3::new(center.x, center.y, bounds.min.z),
                        Vec3::new(center.x, center.y, bounds.max.z),
                    ),
                };
                Ok(vec![(start, end)])
            }
        }
    }

    fn label_position(&self) -> Result<Vec3, LookupError> {
        match self.kind {
            MeasurementGeometry::Distance { start, end } => {
                validate_point(start)?;
                validate_point(end)?;
                Ok((start + end) * 0.5 + segment_label_offset(start, end))
            }
            MeasurementGeometry::Angle { a, vertex, b } => {
                angle_between(a, vertex, b)?;
                let span = a.distance(vertex).max(b.distance(vertex));
                Ok((a + vertex + b) / 3.0 + Vec3::Y * measurement_label_offset_for_span(span))
            }
            MeasurementGeometry::BoundsDimension { bounds, axis } => {
                validate_bounds(bounds)?;
                let center = bounds.center();
                Ok(center + bounds_label_offset(bounds, axis))
            }
        }
    }

    fn label_text(&self, formatted_value: &str) -> String {
        self.label.as_ref().map_or_else(
            || formatted_value.to_owned(),
            |label| format!("{label}: {formatted_value}"),
        )
    }
}

impl UnitFormat {
    pub fn meters() -> Self {
        Self::custom(1.0, "m", 3)
    }

    pub fn millimeters() -> Self {
        Self::custom(1000.0, "mm", 0)
    }

    pub fn degrees() -> Self {
        Self::custom(180.0 / PI, "deg", 1)
    }

    pub fn custom(scale: f32, suffix: impl Into<String>, precision: u8) -> Self {
        Self {
            scale: if scale.is_finite() { scale } else { 1.0 },
            suffix: suffix.into(),
            precision,
        }
    }

    pub const fn with_precision(mut self, precision: u8) -> Self {
        self.precision = precision;
        self
    }

    pub fn format(&self, value: f32) -> String {
        let scaled = value * self.scale;
        let precision = usize::from(self.precision);
        let number = format!("{scaled:.precision$}");
        if self.suffix.is_empty() {
            number
        } else {
            format!("{number} {}", self.suffix)
        }
    }
}

impl Scene {
    pub fn add_measurement_overlay<F>(
        &mut self,
        assets: &Assets<F>,
        overlay: MeasurementOverlay,
    ) -> Result<MeasurementOverlayReport, LookupError> {
        let report = overlay.measure()?;
        if self.measurements.contains_key(overlay.id()) {
            self.clear_measurement_overlay(overlay.id());
        }
        let line_geometry = assets.create_geometry(line_geometry(&overlay.line_segments()?));
        let line_material = assets.create_material(MaterialDesc::line(overlay.color, 1.0));
        let line_node = self
            .mesh(line_geometry, line_material)
            .transform(Transform::IDENTITY)
            .add()?;
        let label = if overlay.label.is_some() {
            Some(
                self.add_label_node(
                    self.root(),
                    LabelDesc::new(overlay.label_text(&report.formatted_value))
                        .with_color(overlay.color)
                        .with_size(0.08),
                    Transform::at(overlay.label_position()?),
                )?,
            )
        } else {
            None
        };
        let (label_key, label_node) = match label {
            Some((label_key, label_node)) => (Some(label_key), Some(label_node)),
            None => (None, None),
        };
        self.measurements.insert(
            report.id.clone(),
            SceneMeasurementOverlayState {
                line_node,
                label_node,
            },
        );
        Ok(MeasurementOverlayReport {
            id: report.id,
            kind: report.kind,
            value: report.value,
            formatted_value: report.formatted_value,
            line_node,
            label: label_key,
        })
    }

    #[cfg(feature = "scene-host")]
    pub(crate) fn measurement_overlay_state(
        &self,
        id: &str,
    ) -> Option<&SceneMeasurementOverlayState> {
        self.measurements.get(id)
    }

    pub(crate) fn measurements_mut(
        &mut self,
    ) -> &mut std::collections::BTreeMap<String, SceneMeasurementOverlayState> {
        &mut self.measurements
    }

    pub fn clear_measurement_overlay(&mut self, id: &str) -> bool {
        let Some(measurement) = self.measurements.remove(id) else {
            return false;
        };
        remove_generated_node_if_live(self, measurement.line_node);
        if let Some(label_node) = measurement.label_node {
            remove_generated_node_if_live(self, label_node);
        }
        true
    }
}

impl SceneMeasurementOverlayState {
    #[cfg(feature = "scene-host")]
    pub const fn line_node(&self) -> NodeKey {
        self.line_node
    }

    #[cfg(feature = "scene-host")]
    pub const fn label_node(&self) -> Option<NodeKey> {
        self.label_node
    }

    pub(crate) fn touches_removed_node(
        &self,
        removed: &std::collections::BTreeSet<NodeKey>,
    ) -> bool {
        removed.contains(&self.line_node)
            || self.label_node.is_some_and(|node| removed.contains(&node))
    }
}

fn remove_generated_node_if_live(scene: &mut Scene, node: NodeKey) {
    if scene.node(node).is_some() {
        let _ = scene.remove_node(node);
    }
}

impl MeasurementGeometry {
    const fn kind(&self) -> MeasurementKind {
        match self {
            Self::Distance { .. } => MeasurementKind::Distance,
            Self::Angle { .. } => MeasurementKind::Angle,
            Self::BoundsDimension { .. } => MeasurementKind::BoundsDimension,
        }
    }
}

fn line_geometry(segments: &[(Vec3, Vec3)]) -> GeometryDesc {
    let mut vertices = Vec::with_capacity(segments.len() * 2);
    let mut indices = Vec::with_capacity(segments.len() * 2);
    for (start, end) in segments {
        let base = vertices.len() as u32;
        vertices.push(GeometryVertex {
            position: *start,
            normal: Vec3::Y,
        });
        vertices.push(GeometryVertex {
            position: *end,
            normal: Vec3::Y,
        });
        indices.push(base);
        indices.push(base + 1);
    }
    GeometryDesc::try_new(GeometryTopology::Lines, vertices, indices)
        .expect("measurement line geometry is generated as valid line pairs")
}

fn segment_label_offset(start: Vec3, end: Vec3) -> Vec3 {
    let delta = end - start;
    let planar_perpendicular = Vec3::new(-delta.y, delta.x, 0.0);
    let offset = measurement_label_offset_for_span(delta.length());
    if planar_perpendicular.length_squared() > f32::EPSILON {
        planar_perpendicular.normalize() * offset
    } else {
        Vec3::Y * offset
    }
}

fn measurement_label_offset_for_span(span: f32) -> f32 {
    if span.is_finite() {
        (span * MEASUREMENT_LABEL_OFFSET_FRACTION).clamp(
            MEASUREMENT_LABEL_OFFSET_MIN_WORLD,
            MEASUREMENT_LABEL_OFFSET_MAX_WORLD,
        )
    } else {
        MEASUREMENT_LABEL_OFFSET_MIN_WORLD
    }
}

fn bounds_label_offset(bounds: Aabb, axis: MeasurementAxis) -> Vec3 {
    let (start, end) = match axis {
        MeasurementAxis::X => (
            Vec3::new(bounds.min.x, bounds.center().y, bounds.center().z),
            Vec3::new(bounds.max.x, bounds.center().y, bounds.center().z),
        ),
        MeasurementAxis::Y => (
            Vec3::new(bounds.center().x, bounds.min.y, bounds.center().z),
            Vec3::new(bounds.center().x, bounds.max.y, bounds.center().z),
        ),
        MeasurementAxis::Z => (
            Vec3::new(bounds.center().x, bounds.center().y, bounds.min.z),
            Vec3::new(bounds.center().x, bounds.center().y, bounds.max.z),
        ),
    };
    segment_label_offset(start, end)
}

fn angle_between(a: Vec3, vertex: Vec3, b: Vec3) -> Result<f32, LookupError> {
    validate_point(a)?;
    validate_point(vertex)?;
    validate_point(b)?;
    let va = a - vertex;
    let vb = b - vertex;
    let Some(na) = va.try_normalize() else {
        return Err(LookupError::InvalidBounds {
            reason: "angle measurement requires a non-zero first vector",
        });
    };
    let Some(nb) = vb.try_normalize() else {
        return Err(LookupError::InvalidBounds {
            reason: "angle measurement requires a non-zero second vector",
        });
    };
    Ok(na.dot(nb).clamp(-1.0, 1.0).acos())
}

fn validate_point(point: Vec3) -> Result<(), LookupError> {
    if point.x.is_finite() && point.y.is_finite() && point.z.is_finite() {
        Ok(())
    } else {
        Err(LookupError::InvalidBounds {
            reason: "measurement points must be finite",
        })
    }
}

fn validate_bounds(bounds: Aabb) -> Result<(), LookupError> {
    validate_point(bounds.min)?;
    validate_point(bounds.max)?;
    if bounds.min.x <= bounds.max.x && bounds.min.y <= bounds.max.y && bounds.min.z <= bounds.max.z
    {
        Ok(())
    } else {
        Err(LookupError::InvalidBounds {
            reason: "measurement bounds min must be less than or equal to max",
        })
    }
}
