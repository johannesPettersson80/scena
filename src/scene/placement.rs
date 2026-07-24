use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::geometry::Aabb;

use super::recipe::SceneRecipeTransformV1;
use super::view_math::transform_aabb;
use super::{Transform, Vec3};

mod results;
mod serialization;
use serialization::{deserialize_transform_option, serialize_transform_option};

pub const SCENE_PLACEMENT_RESULT_SCHEMA_V1: &str = "scena.placement_result.v1";
pub const SCENE_RECIPE_PATCH_SCHEMA_V1: &str = "scena.recipe_patch.v1";
pub const PLACEMENT_VERBS: &[&str] = &[
    "center",
    "ground",
    "fit_to_size",
    "look_at",
    "align_to_anchor",
    "place_on",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenePlacementResultV1 {
    pub schema: String,
    pub ok: bool,
    pub verb: String,
    pub import_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ScenePlacementTargetV1>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_transform_option",
        deserialize_with = "deserialize_transform_option"
    )]
    pub transform: Option<Transform>,
    pub diagnostics: Vec<ScenePlacementDiagnosticV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipePatchResultV1 {
    pub schema: String,
    pub ok: bool,
    pub source_path: String,
    pub source_sha256: String,
    pub import_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ScenePlacementTargetV1>,
    pub verb: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_transform_option",
        deserialize_with = "deserialize_transform_option"
    )]
    pub previous_transform: Option<Transform>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_transform_option",
        deserialize_with = "deserialize_transform_option"
    )]
    pub transform: Option<Transform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_recipe: Option<serde_json::Value>,
    pub formatting_preserved: bool,
    pub semantic_changes: Vec<SceneRecipeSemanticChangeV1>,
    pub diagnostics: Vec<ScenePlacementDiagnosticV1>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneRecipePatchSuccessInputV1 {
    pub source_path: String,
    pub source_sha256: String,
    pub import_id: String,
    pub verb: String,
    pub previous_transform: Option<Transform>,
    pub transform: Transform,
    pub updated_recipe: serde_json::Value,
    pub semantic_change: SceneRecipeSemanticChangeV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneRecipeSemanticChangeV1 {
    pub path: String,
    pub operation: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenePlacementTargetV1 {
    Import { id: String },
    Node { id: String },
}

impl ScenePlacementTargetV1 {
    pub fn import(id: impl Into<String>) -> Self {
        Self::Import { id: id.into() }
    }

    pub fn node(id: impl Into<String>) -> Self {
        Self::Node { id: id.into() }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::Import { id } | Self::Node { id } => id,
        }
    }
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
    #[serde(default)]
    pub auto_fixable: bool,
}

impl ScenePlacementResultV1 {
    pub fn success(
        import_id: impl Into<String>,
        verb: impl Into<String>,
        transform: Transform,
    ) -> Self {
        let import_id = import_id.into();
        Self {
            schema: SCENE_PLACEMENT_RESULT_SCHEMA_V1.to_owned(),
            ok: true,
            verb: verb.into(),
            target: Some(ScenePlacementTargetV1::import(import_id.clone())),
            import_id,
            transform: Some(round_transform(transform)),
            diagnostics: Vec::new(),
        }
    }

    pub fn failure(
        import_id: impl Into<String>,
        verb: impl Into<String>,
        diagnostic: ScenePlacementDiagnosticV1,
    ) -> Self {
        let import_id = import_id.into();
        Self {
            schema: SCENE_PLACEMENT_RESULT_SCHEMA_V1.to_owned(),
            ok: false,
            verb: verb.into(),
            target: Some(ScenePlacementTargetV1::import(import_id.clone())),
            import_id,
            transform: None,
            diagnostics: vec![diagnostic],
        }
    }

    pub fn success_for_target(
        target: ScenePlacementTargetV1,
        verb: impl Into<String>,
        transform: Transform,
    ) -> Self {
        let mut result = Self::success(target.id(), verb, transform);
        result.target = Some(target);
        result
    }

    pub fn failure_for_target(
        target: ScenePlacementTargetV1,
        verb: impl Into<String>,
        diagnostic: ScenePlacementDiagnosticV1,
    ) -> Self {
        let mut result = Self::failure(target.id(), verb, diagnostic);
        result.target = Some(target);
        result
    }
}

impl SceneRecipePatchResultV1 {
    pub fn validate_schema(&self) -> Result<(), String> {
        if self.schema != SCENE_RECIPE_PATCH_SCHEMA_V1 {
            return Err(format!(
                "recipe patch schema must be '{SCENE_RECIPE_PATCH_SCHEMA_V1}', got '{}'",
                self.schema
            ));
        }
        if self.source_sha256.len() != 64
            || !self
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(
                "recipe patch source_sha256 must be a 64-character hexadecimal digest".to_owned(),
            );
        }
        if !PLACEMENT_VERBS.contains(&self.verb.as_str()) {
            return Err(format!(
                "recipe patch verb must be one of {}, got '{}'",
                PLACEMENT_VERBS.join(", "),
                self.verb
            ));
        }
        if self.ok {
            if self.transform.is_none()
                || self.updated_recipe.is_none()
                || self.semantic_changes.is_empty()
                || !self.diagnostics.is_empty()
            {
                return Err("successful recipe patch requires a transform, updated_recipe, semantic change, and no diagnostics".to_owned());
            }
            let report = super::recipe::validate_scene_recipe_value(
                self.updated_recipe
                    .clone()
                    .expect("successful patch checked updated_recipe"),
            );
            if !report.ok {
                let detail = report
                    .diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .unwrap_or("unknown recipe validation failure");
                return Err(format!("recipe patch updated_recipe is invalid: {detail}"));
            }
        } else if self.diagnostics.is_empty() {
            return Err("failed recipe patch requires at least one diagnostic".to_owned());
        }
        Ok(())
    }

    pub fn success(input: SceneRecipePatchSuccessInputV1) -> Self {
        let SceneRecipePatchSuccessInputV1 {
            source_path,
            source_sha256,
            import_id,
            verb,
            previous_transform,
            transform,
            updated_recipe,
            semantic_change,
        } = input;
        Self {
            schema: SCENE_RECIPE_PATCH_SCHEMA_V1.to_owned(),
            ok: true,
            source_path,
            source_sha256,
            target: Some(ScenePlacementTargetV1::import(import_id.clone())),
            import_id,
            verb,
            previous_transform,
            transform: Some(round_transform(transform)),
            updated_recipe: Some(updated_recipe),
            formatting_preserved: false,
            semantic_changes: vec![semantic_change],
            diagnostics: Vec::new(),
        }
    }

    pub fn failure(
        source_path: impl Into<String>,
        source_sha256: impl Into<String>,
        import_id: impl Into<String>,
        verb: impl Into<String>,
        diagnostic: ScenePlacementDiagnosticV1,
    ) -> Self {
        let import_id = import_id.into();
        Self {
            schema: SCENE_RECIPE_PATCH_SCHEMA_V1.to_owned(),
            ok: false,
            source_path: source_path.into(),
            source_sha256: source_sha256.into(),
            target: Some(ScenePlacementTargetV1::import(import_id.clone())),
            import_id,
            verb: verb.into(),
            previous_transform: None,
            transform: None,
            updated_recipe: None,
            formatting_preserved: false,
            semantic_changes: Vec::new(),
            diagnostics: vec![diagnostic],
        }
    }

    pub fn success_for_target(
        input: SceneRecipePatchSuccessInputV1,
        target: ScenePlacementTargetV1,
    ) -> Self {
        let mut result = Self::success(input);
        result.target = Some(target);
        result
    }

    pub fn failure_for_target(
        source_path: impl Into<String>,
        source_sha256: impl Into<String>,
        target: ScenePlacementTargetV1,
        verb: impl Into<String>,
        diagnostic: ScenePlacementDiagnosticV1,
    ) -> Self {
        let mut result = Self::failure(source_path, source_sha256, target.id(), verb, diagnostic);
        result.target = Some(target);
        result
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

fn round_transform(transform: Transform) -> Transform {
    Transform {
        translation: round_vec3(transform.translation),
        rotation: glam::Quat::from_xyzw(
            round3_f32(transform.rotation.x),
            round3_f32(transform.rotation.y),
            round3_f32(transform.rotation.z),
            round3_f32(transform.rotation.w),
        ),
        scale: round_vec3(transform.scale),
    }
}

fn round_vec3(value: Vec3) -> Vec3 {
    Vec3::new(
        round3_f32(value.x),
        round3_f32(value.y),
        round3_f32(value.z),
    )
}

fn round3(value: f32) -> f64 {
    if value.is_finite() {
        ((value as f64) * 1000.0).round() / 1000.0
    } else {
        value as f64
    }
}

fn round3_f32(value: f32) -> f32 {
    round3(value) as f32
}
