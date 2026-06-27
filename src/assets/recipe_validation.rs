use super::fetch::AssetFetcher;
use super::{AssetPath, Assets};
use crate::geometry::Aabb;
use crate::scene::recipe::{
    RecipeBuildPolicy, SceneRecipeDiagnosticV1, SceneRecipeExpectedExtentV1, SceneRecipeV1,
    SceneRecipeValidationReportV1, parse_valid_scene_recipe_json_with_policy,
    validate_scene_recipe_json_with_policy,
};

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
    for (index, import) in recipe.imports.iter().enumerate() {
        let asset_uri = match policy.resolve_import_uri(
            recipe_path,
            &import.uri,
            format!("$.imports[{index}].uri"),
        ) {
            Ok(asset_uri) => asset_uri,
            Err(diagnostic) => {
                report.diagnostics.push(*diagnostic);
                continue;
            }
        };
        match assets.load_scene(AssetPath::from(asset_uri.as_str())).await {
            Ok(asset) => {
                if let (Some(expected), Some(bounds)) = (&import.expected_extent, asset.bounds()) {
                    let extent = scaled_max_extent(bounds, import.transform);
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
            }
            Err(error) if import.optional => report.diagnostics.push(diagnostic(
                "optional_import_skipped",
                "warning",
                format!("$.imports[{index}].uri"),
                format!(
                    "optional recipe asset '{}' could not be loaded: {error}",
                    import.uri
                ),
                "the import is marked optional, so callers must tolerate it being absent",
                None,
                false,
            )),
            Err(error) => report.diagnostics.push(diagnostic(
                "asset_load_failed",
                "error",
                format!("$.imports[{index}].uri"),
                format!("failed to load recipe asset '{}': {error}", import.uri),
                "fix the uri, place the asset beside the recipe, or run `scena doctor <asset>`",
                None,
                false,
            )),
        }
    }
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
        auto_fixable,
    }
}

fn update_report_ok(report: &mut SceneRecipeValidationReportV1) {
    report.ok = !report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error");
}
