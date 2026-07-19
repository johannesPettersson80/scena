use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::scena_input::{RecipeReadError, path_for_json, read_recipe_text};
use super::scena_output::{CliOutcome, json_outcome};

#[path = "diff/attribution.rs"]
mod attribution;

const RECIPE_DIFF_RESULT_SCHEMA_V1: &str = "scena.scene_recipe_diff_result.v1";

#[derive(Debug, Clone, PartialEq)]
struct DiffCommandArgs {
    before: PathBuf,
    after: PathBuf,
    render: bool,
    out_dir: Option<PathBuf>,
    numeric_tolerance: f64,
    max_abs_diff: u8,
    max_mismatched_pixels: usize,
    max_imports: Option<usize>,
}

struct LoadedRecipe {
    text: String,
    recipe: scena::SceneRecipeV1,
}

struct RenderedRecipe {
    capture: scena::CaptureRgba8,
    aov: scena::SceneHostSemanticAovCaptureV1,
    manifest: scena::SceneRecipeBuildV1,
}

pub(crate) fn run_diff_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = DiffCommandArgs::parse(args)?;
    let mut policy = scena::RecipeBuildPolicy::testing();
    if let Some(max_imports) = args.max_imports {
        policy = policy.with_max_imports(max_imports);
    }
    let before = match load_recipe(&args.before, &policy)? {
        Ok(recipe) => recipe,
        Err(outcome) => return Ok(outcome),
    };
    let after = match load_recipe(&args.after, &policy)? {
        Ok(recipe) => recipe,
        Err(outcome) => return Ok(outcome),
    };
    let structural = scena::diff_scene_recipes(
        &before.recipe,
        &after.recipe,
        scena::SceneRecipeDiffOptions::new(args.numeric_tolerance),
    );
    let structural_value = serde_json::to_value(&structural)
        .map_err(|error| format!("failed to serialize typed recipe diff: {error}"))?;

    if !args.render {
        return json_outcome(
            &json!({
                "schema": RECIPE_DIFF_RESULT_SCHEMA_V1,
                "equal": structural.equal,
                "before": path_for_json(&args.before),
                "after": path_for_json(&args.after),
                "structural": structural_value,
                "visual": Value::Null,
                "execution": execution_report(0),
            }),
            0,
            "failed to serialize scene recipe diff result",
        );
    }

    let out_dir = args
        .out_dir
        .as_ref()
        .expect("rendered diff parser requires output directory");
    std::fs::create_dir_all(out_dir).map_err(|error| {
        format!(
            "failed to create recipe diff output directory '{}': {error}",
            out_dir.display()
        )
    })?;
    let before_rendered = match render_recipe(&args.before, &before, policy.clone())? {
        Ok(rendered) => rendered,
        Err(outcome) => return Ok(outcome),
    };
    let after_rendered = match render_recipe(&args.after, &after, policy)? {
        Ok(rendered) => rendered,
        Err(outcome) => return Ok(outcome),
    };

    let tolerance = scena::ReferenceImageTolerance::new()
        .with_max_abs_diff(args.max_abs_diff)
        .with_max_mismatched_pixels(args.max_mismatched_pixels);
    let aggregate = match scena::compare_captures_with_tolerance(
        &after_rendered.capture,
        &before_rendered.capture,
        tolerance,
    ) {
        Ok(report) => report,
        Err(scena::CaptureBaselineError::DiffExceeded(report)) => *report,
        Err(error) => {
            return Err(format!(
                "failed to compare rendered recipe captures: {error}"
            ));
        }
    };
    let (attribution, diff_rgba8) = attribution::attributed_visual_diff(
        &before_rendered.capture,
        &after_rendered.capture,
        &before_rendered.aov,
        &after_rendered.aov,
        &before_rendered.manifest,
        &after_rendered.manifest,
        args.max_abs_diff,
    )?;
    let before_png = out_dir.join("before.png");
    let after_png = out_dir.join("after.png");
    let diff_png = out_dir.join("diff.png");
    before_rendered
        .capture
        .write_png(&before_png)
        .map_err(|error| format!("failed to write '{}': {error}", before_png.display()))?;
    after_rendered
        .capture
        .write_png(&after_png)
        .map_err(|error| format!("failed to write '{}': {error}", after_png.display()))?;
    write_rgba8_png(
        &diff_png,
        before_rendered.capture.descriptor.width,
        before_rendered.capture.descriptor.height,
        &diff_rgba8,
    )?;

    let visual_equal = aggregate.status == "passed";
    let report_path = out_dir.join("recipe-diff-result.json");
    let report = json!({
        "schema": RECIPE_DIFF_RESULT_SCHEMA_V1,
        "equal": structural.equal && visual_equal,
        "before": path_for_json(&args.before),
        "after": path_for_json(&args.after),
        "structural": structural_value,
        "visual": {
            "aggregate": aggregate,
            "attribution": attribution,
            "artifacts": {
                "before_png": path_for_json(&before_png),
                "after_png": path_for_json(&after_png),
                "diff_png": path_for_json(&diff_png),
                "report": path_for_json(&report_path),
            },
        },
        "execution": execution_report(2),
    });
    std::fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report)
            .map_err(|error| format!("failed to encode rendered recipe diff: {error}"))?,
    )
    .map_err(|error| format!("failed to write '{}': {error}", report_path.display()))?;
    json_outcome(
        &report,
        0,
        "failed to serialize rendered scene recipe diff result",
    )
}

impl DiffCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(before) = args.first() else {
            return Err(usage());
        };
        let Some(after) = args.get(1) else {
            return Err(usage());
        };
        let mut render = false;
        let mut out_dir = None;
        let mut numeric_tolerance = 1.0e-6;
        let mut max_abs_diff = 0;
        let mut max_mismatched_pixels = 0;
        let mut max_imports = None;
        let mut index = 2;
        while index < args.len() {
            match args[index].as_str() {
                "--render" => {
                    render = true;
                    index += 1;
                }
                "--out-dir" => {
                    out_dir = Some(PathBuf::from(flag_value(args, index, "--out-dir")?));
                    index += 2;
                }
                "--numeric-tolerance" => {
                    numeric_tolerance = parse_nonnegative_f64(
                        "--numeric-tolerance",
                        flag_value(args, index, "--numeric-tolerance")?,
                    )?;
                    index += 2;
                }
                "--max-abs-diff" => {
                    max_abs_diff = flag_value(args, index, "--max-abs-diff")?
                        .parse::<u8>()
                        .map_err(|_| {
                            "--max-abs-diff requires an integer from 0 to 255".to_owned()
                        })?;
                    index += 2;
                }
                "--max-mismatched-pixels" => {
                    max_mismatched_pixels = parse_usize(
                        "--max-mismatched-pixels",
                        flag_value(args, index, "--max-mismatched-pixels")?,
                    )?;
                    index += 2;
                }
                "--max-imports" => {
                    let value =
                        parse_usize("--max-imports", flag_value(args, index, "--max-imports")?)?;
                    if value == 0 {
                        return Err("--max-imports requires a positive integer, got 0".to_owned());
                    }
                    max_imports = Some(value);
                    index += 2;
                }
                "--json" => index += 1,
                flag => return Err(format!("unknown diff argument '{flag}'; {}", usage())),
            }
        }
        if render && out_dir.is_none() {
            return Err(format!("--render requires --out-dir <dir>; {}", usage()));
        }
        if !render && out_dir.is_some() {
            return Err("--out-dir is only valid with --render".to_owned());
        }
        Ok(Self {
            before: PathBuf::from(before),
            after: PathBuf::from(after),
            render,
            out_dir,
            numeric_tolerance,
            max_abs_diff,
            max_mismatched_pixels,
            max_imports,
        })
    }
}

fn load_recipe(
    path: &Path,
    policy: &scena::RecipeBuildPolicy,
) -> Result<Result<LoadedRecipe, CliOutcome>, String> {
    let text = match read_recipe_text(path, policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(report)) => {
            return Ok(Err(json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            )?));
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(format!(
                "failed to read recipe '{}': {error}",
                path.display()
            ));
        }
    };
    match scena::parse_valid_scene_recipe_json_with_policy(&text, policy) {
        Ok(recipe) => Ok(Ok(LoadedRecipe { text, recipe })),
        Err(report) => Ok(Err(json_outcome(
            &report,
            1,
            "failed to serialize scene recipe validation report",
        )?)),
    }
}

fn render_recipe(
    path: &Path,
    loaded: &LoadedRecipe,
    policy: scena::RecipeBuildPolicy,
) -> Result<Result<RenderedRecipe, CliOutcome>, String> {
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        path.display().to_string(),
        &loaded.text,
        policy,
    ));
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            return Ok(Err(json_outcome(
                &manifest,
                1,
                "failed to serialize scene recipe build failure",
            )?));
        }
    };
    let manifest = build.manifest;
    let mut host = build.host;
    if !loaded.recipe.cameras.iter().any(|camera| camera.active) {
        host.frame_all_with_overlays()
            .map_err(|error| format!("failed to frame recipe '{}': {error}", path.display()))?;
    }
    host.prepare()
        .map_err(|error| format!("failed to prepare recipe '{}': {error}", path.display()))?;
    host.render()
        .map_err(|error| format!("failed to render recipe '{}': {error}", path.display()))?;
    let capture = host
        .capture()
        .map_err(|error| format!("failed to capture recipe '{}': {error}", path.display()))?;
    let aov = host.capture_semantic_aovs().map_err(|error| {
        format!(
            "failed to capture semantic AOV for recipe '{}': {error}",
            path.display()
        )
    })?;
    Ok(Ok(RenderedRecipe {
        capture,
        aov,
        manifest,
    }))
}

fn write_rgba8_png(path: &Path, width: u32, height: u32, rgba8: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path)
        .map_err(|error| format!("failed to create '{}': {error}", path.display()))?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to encode '{}': {error}", path.display()))?;
    writer
        .write_image_data(rgba8)
        .map_err(|error| format!("failed to encode '{}': {error}", path.display()))
}

fn execution_report(renderer_constructions: usize) -> Value {
    json!({
        "renderer_constructions": renderer_constructions,
        "prepare_calls": renderer_constructions,
        "render_calls": renderer_constructions,
        "capture_constructions": renderer_constructions,
    })
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_nonnegative_f64(flag: &str, value: String) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("{flag} requires a finite non-negative number, got '{value}'"))?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(format!(
            "{flag} requires a finite non-negative number, got '{value}'"
        ));
    }
    Ok(parsed)
}

fn parse_usize(flag: &str, value: String) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))
}

fn usage() -> String {
    "usage: scena diff <before.recipe.json> <after.recipe.json> [--numeric-tolerance <n>] [--render --out-dir <dir> [--max-abs-diff <0..255>] [--max-mismatched-pixels <n>]] [--max-imports <n>]".to_owned()
}
