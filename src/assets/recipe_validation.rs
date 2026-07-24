use super::fetch::AssetFetcher;
use super::{AssetLoadOptions, AssetPath, Assets};
use crate::geometry::Aabb;
use crate::material::TextureColorSpace;
use crate::scene::recipe::{
    RecipeBuildPolicy, RecipeResourceRole, RecipeValidationModeV1, SceneRecipeDiagnosticResourceV1,
    SceneRecipeDiagnosticV1, SceneRecipeExpectedExtentV1, SceneRecipeResourceStatusV1,
    SceneRecipeV1, SceneRecipeValidationReportV1, parse_valid_scene_recipe_json_with_policy,
    validate_scene_recipe_json_with_policy,
};
use crate::{LabelFontFace, SceneRecipeTextureColorSpaceV1};

/// Validates a `scena.scene_recipe.v1` document and checks asset-backed fields.
///
/// The scene-owned validator is deliberately structural and does no I/O. This
/// asset-owned helper performs the load-dependent checks: URI loadability and
/// `expected_extent` bounds after the import transform's maximum scale.
pub async fn validate_scene_recipe_json_with_assets<F: AssetFetcher>(
    recipe_path: impl AsRef<str>,
    text: &str,
    assets: &Assets<F>,
) -> SceneRecipeValidationReportV1 {
    validate_scene_recipe_json_with_assets_and_policy(
        recipe_path,
        text,
        assets,
        &RecipeBuildPolicy::default(),
    )
    .await
}

/// Validates a `scena.scene_recipe.v1` document with the same operator policy used by build.
///
/// This keeps `validate-recipe` from accepting local paths that the build executor would reject,
/// so agents get path-policy failures before attempting a render.
pub async fn validate_scene_recipe_json_with_assets_and_policy<F: AssetFetcher>(
    recipe_path: impl AsRef<str>,
    text: &str,
    assets: &Assets<F>,
    policy: &RecipeBuildPolicy,
) -> SceneRecipeValidationReportV1 {
    let mut report = validate_scene_recipe_json_with_policy(text, policy);
    if !report.ok {
        return report;
    }
    let recipe = match parse_valid_scene_recipe_json_with_policy(text, policy) {
        Ok(recipe) => recipe,
        Err(parse_report) => return parse_report,
    };
    report.validation_mode = RecipeValidationModeV1::FullResolution;
    report.execution_equivalent = true;
    report.policy = Some(Box::new(policy.to_schema_report()));
    add_asset_validation_diagnostics(recipe_path.as_ref(), &recipe, assets, policy, &mut report)
        .await;
    update_report_ok(&mut report);
    report
}

async fn add_asset_validation_diagnostics<F: AssetFetcher>(
    recipe_path: &str,
    recipe: &SceneRecipeV1,
    assets: &Assets<F>,
    policy: &RecipeBuildPolicy,
    report: &mut SceneRecipeValidationReportV1,
) {
    let mut plan = policy.resolve_recipe_resources(recipe_path, recipe);
    report.diagnostics.append(&mut plan.diagnostics);
    let allowed_roots = policy
        .to_schema_report()
        .allowed_roots
        .into_iter()
        .map(|root| root.path)
        .collect::<Vec<_>>();
    let options = AssetLoadOptions::default().with_fetch_byte_limit(policy.fetch_byte_limit());
    for resource in &mut plan.resources {
        let Some(normalized_uri) = resource.report.normalized_uri.clone() else {
            continue;
        };
        let result = match resource.role {
            RecipeResourceRole::BuiltinEnvironment => continue,
            RecipeResourceRole::Import(index) => {
                let import = &recipe.imports[index];
                match assets
                    .load_scene_with_options(
                        AssetPath::from(normalized_uri.as_str()),
                        options.clone(),
                    )
                    .await
                {
                    Ok(asset) => {
                        if let (Some(expected), Some(bounds)) =
                            (&import.expected_extent, asset.bounds())
                        {
                            let transform = import
                                .transform
                                .as_ref()
                                .and_then(|transform| crate::Transform::try_from(transform).ok());
                            let extent = scaled_max_extent(bounds, transform);
                            if extent < expected.min || extent > expected.max {
                                report.diagnostics.push(diagnostic(
                                    "extent_out_of_range",
                                    "error",
                                    format!("$.imports[{index}].expected_extent"),
                                    format!(
                                        "asset maximum extent {:.3}{} is outside expected range {:.3}..{:.3}{}",
                                        extent,
                                        unit_suffix(expected),
                                        expected.min,
                                        expected.max,
                                        unit_suffix(expected)
                                    ),
                                    "fix the asset units/scale, update the import transform, or widen expected_extent intentionally",
                                    None,
                                    false,
                                ));
                            }
                        }
                        Ok(())
                    }
                    Err(error) => Err(("asset_load_failed", error.to_string())),
                }
            }
            RecipeResourceRole::Environment => assets
                .validate_environment_source_with_options(
                    AssetPath::from(normalized_uri.as_str()),
                    options.clone(),
                )
                .await
                .map_err(|error| ("environment_load_failed", error.to_string())),
            RecipeResourceRole::Font(_) => std::fs::read(&normalized_uri)
                .map_err(|error| ("font_load_failed", error.to_string()))
                .and_then(|bytes| {
                    LabelFontFace::from_truetype_bytes(&bytes)
                        .map(|_| ())
                        .map_err(|error| ("font_load_failed", error.to_string()))
                }),
            RecipeResourceRole::Texture(ref color_space) => {
                let color_space = match color_space {
                    SceneRecipeTextureColorSpaceV1::Srgb => TextureColorSpace::Srgb,
                    SceneRecipeTextureColorSpaceV1::Linear => TextureColorSpace::Linear,
                };
                assets
                    .load_texture(AssetPath::from(normalized_uri.as_str()), color_space)
                    .await
                    .map(|_| ())
                    .map_err(|error| ("texture_load_failed", error.to_string()))
            }
        };
        match result {
            Ok(()) => resource.report.status = SceneRecipeResourceStatusV1::Loaded,
            Err((code, reason)) if !resource.report.required => {
                resource.report.status = SceneRecipeResourceStatusV1::OptionalSkipped;
                report.diagnostics.push(resource_diagnostic(
                    resource,
                    &allowed_roots,
                    code,
                    "warning",
                    format!(
                        "optional {} '{}' could not be loaded from '{}': {reason}",
                        resource.report.kind, resource.report.authored_uri, normalized_uri
                    ),
                    "the resource is optional; callers must tolerate its documented fallback",
                ));
            }
            Err((code, reason)) => {
                resource.report.status = SceneRecipeResourceStatusV1::LoadFailed;
                report.diagnostics.push(resource_diagnostic(
                    resource,
                    &allowed_roots,
                    code,
                    "error",
                    format!(
                        "required {} '{}' could not be loaded from '{}': {reason}",
                        resource.report.kind, resource.report.authored_uri, normalized_uri
                    ),
                    "fix the authored URI or authorize an existing resource under the reported policy roots",
                ));
            }
        }
    }
    report.resources = plan.reports();
}

fn resource_diagnostic(
    resource: &crate::scene::recipe::PlannedRecipeResource,
    allowed_roots: &[String],
    code: &str,
    severity: &str,
    message: String,
    help: &str,
) -> SceneRecipeDiagnosticV1 {
    let mut diagnostic = diagnostic(
        code,
        severity,
        resource.report.path.clone(),
        message,
        help,
        None,
        false,
    );
    diagnostic.resource = Some(SceneRecipeDiagnosticResourceV1 {
        kind: resource.report.kind.clone(),
        authored_uri: resource.report.authored_uri.clone(),
        normalized_uri: resource.report.normalized_uri.clone(),
        required: resource.report.required,
        allowed_roots: allowed_roots.to_vec(),
    });
    diagnostic
}

fn scaled_max_extent(bounds: Aabb, transform: Option<crate::scene::Transform>) -> f64 {
    let extent = bounds.max - bounds.min;
    let max_extent = extent.x.max(extent.y).max(extent.z).abs() as f64;
    let scale = transform
        .map(|transform| transform.scale.abs().max_element() as f64)
        .unwrap_or(1.0);
    max_extent * scale
}

fn unit_suffix(expected: &SceneRecipeExpectedExtentV1) -> String {
    expected
        .unit
        .as_deref()
        .filter(|unit| !unit.trim().is_empty())
        .map(|unit| format!(" {unit}"))
        .unwrap_or_default()
}

fn diagnostic(
    code: impl Into<String>,
    severity: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
    help: impl Into<String>,
    suggestion: Option<String>,
    auto_fixable: bool,
) -> SceneRecipeDiagnosticV1 {
    SceneRecipeDiagnosticV1 {
        code: code.into(),
        severity: severity.into(),
        path: path.into(),
        message: message.into(),
        help: help.into(),
        suggestion,
        candidates: Vec::new(),
        auto_fixable,
        resource: None,
    }
}

fn update_report_ok(report: &mut SceneRecipeValidationReportV1) {
    report.ok = !report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error");
}
