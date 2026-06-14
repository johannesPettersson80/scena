use serde::{Deserialize, Serialize};

use crate::geometry::Aabb;

use super::view_math::transform_aabb;
use super::{Transform, Vec3};

pub const SCENE_PLACEMENT_RESULT_SCHEMA_V1: &str = "scena.placement_result.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenePlacementResultV1 {
    pub schema: String,
    pub ok: bool,
    pub verb: String,
    pub import_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<Transform>,
    pub diagnostics: Vec<ScenePlacementDiagnosticV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenePlacementDiagnosticV1 {
    pub code: String,
    pub severity: String,
    pub path: String,
    pub message: String,
    pub help: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default)]
    pub auto_fixable: bool,
}

impl ScenePlacementResultV1 {
    pub fn success(
        import_id: impl Into<String>,
        verb: impl Into<String>,
        transform: Transform,
    ) -> Self {
        Self {
            schema: SCENE_PLACEMENT_RESULT_SCHEMA_V1.to_owned(),
            ok: true,
            verb: verb.into(),
            import_id: import_id.into(),
            transform: Some(transform),
            diagnostics: Vec::new(),
        }
    }

    pub fn failure(
        import_id: impl Into<String>,
        verb: impl Into<String>,
        diagnostic: ScenePlacementDiagnosticV1,
    ) -> Self {
        Self {
            schema: SCENE_PLACEMENT_RESULT_SCHEMA_V1.to_owned(),
            ok: false,
            verb: verb.into(),
            import_id: import_id.into(),
            transform: None,
            diagnostics: vec![diagnostic],
        }
    }
}

impl ScenePlacementDiagnosticV1 {
    pub fn new(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: "error".to_owned(),
            path: path.into(),
            message: message.into(),
            help: help.into(),
            suggestion: None,
            auto_fixable: false,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

pub fn placement_center_transform(bounds: Aabb, current: Transform, target: Vec3) -> Transform {
    let world_bounds = transform_aabb(bounds, current);
    current.with_translation(current.translation + (target - world_bounds.center()))
}

pub fn placement_ground_transform(bounds: Aabb, current: Transform, ground_y: f32) -> Transform {
    let world_bounds = transform_aabb(bounds, current);
    current
        .with_translation(current.translation + Vec3::new(0.0, ground_y - world_bounds.min.y, 0.0))
}

pub fn placement_look_at_transform(
    current: Transform,
    target: Vec3,
    up: Vec3,
) -> Result<Transform, Box<ScenePlacementDiagnosticV1>> {
    if !vec3_is_finite(current.translation) || !vec3_is_finite(target) || !vec3_is_finite(up) {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_transform",
            "$.verb.look_at",
            "look_at requires finite source translation, target, and up vectors",
            "pass finite --target and --up vectors",
        )));
    }
    if (target - current.translation).length_squared() <= f32::EPSILON {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "degenerate_look_at",
            "$.verb.look_at",
            "look_at target must differ from the source translation",
            "choose a target point or target import away from the source",
        )));
    }
    Ok(current.looking_at(target, up))
}

pub fn placement_align_to_feature_transform(
    current: Transform,
    source_feature: Transform,
    target_feature: Transform,
) -> Result<Transform, Box<ScenePlacementDiagnosticV1>> {
    validate_feature_transform(source_feature, "$.source")?;
    validate_feature_transform(target_feature, "$.target")?;
    if !vec3_is_finite(current.scale) || current.scale.abs().min_element() <= f32::EPSILON {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "non_invertible_transform",
            "$.imports[].transform",
            "source import transform must have finite non-zero scale",
            "use a source transform with non-zero finite scale",
        )));
    }

    let source_rotation = normalized_quat(source_feature.rotation, "$.source.rotation")?;
    let target_rotation = normalized_quat(target_feature.rotation, "$.target.rotation")?;
    let rotation = (target_rotation * source_rotation.inverse()).normalize();
    let translation =
        target_feature.translation - rotation * (source_feature.translation * current.scale);
    Ok(Transform {
        translation,
        rotation,
        scale: current.scale,
    })
}

pub fn placement_place_on_feature_transform(
    current: Transform,
    source_feature: Transform,
    target_feature: Transform,
) -> Result<Transform, Box<ScenePlacementDiagnosticV1>> {
    validate_feature_transform(source_feature, "$.source")?;
    validate_feature_transform(target_feature, "$.target")?;
    if !vec3_is_finite(current.translation) || !vec3_is_finite(current.scale) {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_transform",
            "$.imports[].transform",
            "source import transform must have finite translation and scale",
            "use a finite source transform",
        )));
    }
    let source_world = current.translation
        + current.rotation.normalize() * (source_feature.translation * current.scale);
    Ok(current.with_translation(current.translation + (target_feature.translation - source_world)))
}

pub fn placement_fit_to_size_transform(
    bounds: Aabb,
    current: Transform,
    min_size: Option<f32>,
    max_size: Option<f32>,
) -> Result<Transform, Box<ScenePlacementDiagnosticV1>> {
    let world_bounds = transform_aabb(bounds, current);
    let extent = world_bounds.max - world_bounds.min;
    let current_size = extent.x.max(extent.y).max(extent.z);
    if !current_size.is_finite() || current_size <= f32::EPSILON {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_bounds",
            "$.imports[].uri",
            "import bounds must have finite non-zero extent for fit_to_size",
            "use an asset with renderable bounds or choose center/ground instead",
        )));
    }

    if min_size.is_none() && max_size.is_none() {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_size_range",
            "$.verb.fit_to_size",
            "fit_to_size requires --min-size, --max-size, or both",
            "pass a target size range for fit_to_size",
        )));
    }
    if min_size.is_some_and(|value| !value.is_finite() || value < 0.0)
        || max_size.is_some_and(|value| !value.is_finite() || value <= 0.0)
        || min_size
            .zip(max_size)
            .is_some_and(|(min_size, max_size)| max_size < min_size)
    {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_size_range",
            "$.verb.fit_to_size",
            "fit_to_size requires finite non-negative min and positive max with max >= min",
            "pass --min-size and/or --max-size with a valid positive range",
        )));
    }

    let min_size = min_size.unwrap_or(0.0);
    let max_size = max_size.unwrap_or(f32::INFINITY);
    let scale_factor = if current_size > max_size {
        max_size / current_size
    } else if current_size < min_size {
        min_size / current_size
    } else {
        1.0
    };
    Ok(Transform {
        scale: current.scale * scale_factor,
        ..current
    })
}

fn validate_feature_transform(
    transform: Transform,
    path: &str,
) -> Result<(), Box<ScenePlacementDiagnosticV1>> {
    if !vec3_is_finite(transform.translation)
        || !vec3_is_finite(transform.scale)
        || !quat_is_finite(transform.rotation)
    {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_feature",
            path,
            "authored feature transform must be finite",
            "fix the authored anchor or connector transform",
        )));
    }
    if transform.scale.abs().min_element() <= f32::EPSILON {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "non_invertible_feature",
            path,
            "authored feature transform must have non-zero scale",
            "fix the authored anchor or connector scale",
        )));
    }
    Ok(())
}

fn normalized_quat(
    rotation: glam::Quat,
    path: &str,
) -> Result<glam::Quat, Box<ScenePlacementDiagnosticV1>> {
    let length_squared = rotation.length_squared();
    if !length_squared.is_finite() || length_squared <= f32::EPSILON {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_feature",
            path,
            "authored feature rotation must be finite and non-zero",
            "fix the authored anchor or connector rotation",
        )));
    }
    Ok(rotation.normalize())
}

fn vec3_is_finite(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn quat_is_finite(value: glam::Quat) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite() && value.w.is_finite()
}
