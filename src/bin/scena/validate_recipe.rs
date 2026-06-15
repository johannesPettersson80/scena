use std::fs;

use super::scena_args::ValidateRecipeCommandArgs;
use super::{CliOutcome, json_outcome, resolve_recipe_asset_uri};

pub(crate) fn run_validate_recipe_command(args: &[String]) -> Result<CliOutcome, String> {
    let recipe_path = ValidateRecipeCommandArgs::parse(args)?.recipe;
    let text = fs::read_to_string(&recipe_path)
        .map_err(|error| format!("failed to read recipe '{}': {error}", recipe_path.display()))?;
    let mut report = scena::validate_scene_recipe_json(&text);
    if report.ok {
        let recipe = match scena::parse_valid_scene_recipe_json(&text) {
            Ok(recipe) => recipe,
            Err(parse_report) => {
                report = parse_report;
                update_report_ok(&mut report);
                return emit_report(report);
            }
        };
        let recipe_path = recipe_path
            .to_str()
            .ok_or_else(|| format!("recipe path '{}' is not valid UTF-8", recipe_path.display()))?;
        add_asset_validation_diagnostics(recipe_path, &recipe, &mut report);
    }
    update_report_ok(&mut report);
    emit_report(report)
}

fn emit_report(report: scena::SceneRecipeValidationReportV1) -> Result<CliOutcome, String> {
    let exit_code = if report.ok { 0 } else { 1 };
    json_outcome(
        &report,
        exit_code,
        "failed to serialize scene recipe validation report",
    )
}

fn add_asset_validation_diagnostics(
    recipe_path: &str,
    recipe: &scena::SceneRecipeV1,
    report: &mut scena::SceneRecipeValidationReportV1,
) {
    let assets = scena::Assets::new();
    for (index, import) in recipe.imports.iter().enumerate() {
        let asset_uri = resolve_recipe_asset_uri(recipe_path, &import.uri);
        match pollster::block_on(assets.load_scene(asset_uri.as_str())) {
            Ok(asset) => {
                if let (Some(expected), Some(bounds)) = (&import.expected_extent, asset.bounds()) {
                    let extent = scaled_max_extent(bounds, import.transform);
                    if extent < expected.min || extent > expected.max {
                        report.diagnostics.push(diagnostic(
                            "extent_out_of_range",
                            "warning",
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

fn scaled_max_extent(bounds: scena::Aabb, transform: Option<scena::Transform>) -> f64 {
    let extent = bounds.max - bounds.min;
    let max_extent = extent.x.max(extent.y).max(extent.z).abs() as f64;
    let scale = transform
        .map(|transform| transform.scale.abs().max_element() as f64)
        .unwrap_or(1.0);
    max_extent * scale
}

fn unit_suffix(expected: &scena::SceneRecipeExpectedExtentV1) -> String {
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
) -> scena::SceneRecipeDiagnosticV1 {
    scena::SceneRecipeDiagnosticV1 {
        code: code.into(),
        severity: severity.into(),
        path: path.into(),
        message: message.into(),
        help: help.into(),
        suggestion,
        auto_fixable,
    }
}

fn update_report_ok(report: &mut scena::SceneRecipeValidationReportV1) {
    report.ok = !report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error");
}
