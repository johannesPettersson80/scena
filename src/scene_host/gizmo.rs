use serde::{Deserialize, Serialize};

use super::inputs::validate_transform;
use super::visual_patch::VisualPatchResultV1;
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    AssetFetcher, GizmoAxis, GizmoConstraint, GizmoMode, GizmoRay, GizmoSpace, Transform,
    TransformGizmo, Vec3,
};

pub const SCENE_HOST_GIZMO_DRAG_SCHEMA_V1: &str = "scena.scene_host_gizmo_drag.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostGizmoDragV1 {
    pub schema: String,
    pub mode: SceneHostGizmoModeV1,
    #[serde(default)]
    pub space: SceneHostGizmoSpaceV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraint: Option<SceneHostGizmoConstraintV1>,
    pub start_transform: Transform,
    pub start_ray: SceneHostGizmoRayV1,
    pub current_ray: SceneHostGizmoRayV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostGizmoModeV1 {
    Translate,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostGizmoAxisV1 {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostGizmoSpaceV1 {
    #[default]
    World,
    Local,
    ViewAligned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneHostGizmoConstraintV1 {
    Axis { axis: SceneHostGizmoAxisV1 },
    Plane { axis: SceneHostGizmoAxisV1 },
    ViewPlane,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneHostGizmoRayV1 {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_gizmo_drag_json(
        &mut self,
        target: u64,
        request_json: &str,
    ) -> Result<String, SceneHostError> {
        let request: SceneHostGizmoDragV1 =
            serde_json::from_str(request_json).map_err(|error| {
                SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    format!("invalid SceneHost gizmo drag JSON: {error}"),
                )
            })?;
        let result = self.apply_gizmo_drag(target, &request)?;
        serde_json::to_string(&result).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("SceneHost gizmo drag result serialization failed: {error}"),
            )
        })
    }

    pub fn apply_gizmo_drag(
        &mut self,
        target: u64,
        request: &SceneHostGizmoDragV1,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        request.validate_schema()?;
        let start = validate_transform(request.start_transform)?;
        let gizmo = request.gizmo();
        let transformed = gizmo
            .drag_transform(
                start,
                request.start_ray.to_gizmo_ray("start_ray")?,
                request.current_ray.to_gizmo_ray("current_ray")?,
            )
            .ok_or_else(|| {
                invalid_input(
                    "gizmo drag did not resolve a finite transform; check rays and constraint",
                )
            })?;
        let patch = gizmo.to_visual_patch(target, transformed);
        self.apply_patch(&patch)
    }
}

impl SceneHostGizmoDragV1 {
    fn validate_schema(&self) -> Result<(), SceneHostError> {
        if self.schema == SCENE_HOST_GIZMO_DRAG_SCHEMA_V1 {
            return Ok(());
        }
        Err(invalid_input(format!(
            "unsupported SceneHost gizmo drag schema {}; expected {}",
            self.schema, SCENE_HOST_GIZMO_DRAG_SCHEMA_V1
        )))
    }

    fn gizmo(&self) -> TransformGizmo {
        let mut gizmo = TransformGizmo::new(self.mode.into()).with_space(self.space.into());
        if let Some(constraint) = self.constraint {
            gizmo = gizmo.with_constraint(constraint.into());
        }
        gizmo
    }
}

impl SceneHostGizmoRayV1 {
    fn to_gizmo_ray(self, field: &str) -> Result<GizmoRay, SceneHostError> {
        GizmoRay::new(
            Vec3::from_array(self.origin),
            Vec3::from_array(self.direction),
        )
        .ok_or_else(|| {
            invalid_input(format!(
                "{field} must have finite origin and non-zero finite direction"
            ))
        })
    }
}

impl From<SceneHostGizmoModeV1> for GizmoMode {
    fn from(value: SceneHostGizmoModeV1) -> Self {
        match value {
            SceneHostGizmoModeV1::Translate => Self::Translate,
            SceneHostGizmoModeV1::Rotate => Self::Rotate,
            SceneHostGizmoModeV1::Scale => Self::Scale,
        }
    }
}

impl From<SceneHostGizmoAxisV1> for GizmoAxis {
    fn from(value: SceneHostGizmoAxisV1) -> Self {
        match value {
            SceneHostGizmoAxisV1::X => Self::X,
            SceneHostGizmoAxisV1::Y => Self::Y,
            SceneHostGizmoAxisV1::Z => Self::Z,
        }
    }
}

impl From<SceneHostGizmoSpaceV1> for GizmoSpace {
    fn from(value: SceneHostGizmoSpaceV1) -> Self {
        match value {
            SceneHostGizmoSpaceV1::World => Self::World,
            SceneHostGizmoSpaceV1::Local => Self::Local,
            SceneHostGizmoSpaceV1::ViewAligned => Self::ViewAligned,
        }
    }
}

impl From<SceneHostGizmoConstraintV1> for GizmoConstraint {
    fn from(value: SceneHostGizmoConstraintV1) -> Self {
        match value {
            SceneHostGizmoConstraintV1::Axis { axis } => Self::Axis(axis.into()),
            SceneHostGizmoConstraintV1::Plane { axis } => Self::Plane(axis.into()),
            SceneHostGizmoConstraintV1::ViewPlane => Self::ViewPlane,
        }
    }
}

fn invalid_input(message: impl Into<String>) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message.into())
}
