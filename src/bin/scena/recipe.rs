use std::path::PathBuf;

use super::scena_input::{
    RecipeReadError, capture_descriptor_path, ensure_parent_dir, path_for_json, read_recipe_text,
    render_introspection_options,
};
use super::scena_output::{CliOutcome, json_outcome};

#[path = "recipe/verification.rs"]
mod verification;

use verification::{RecipeVerificationInput, verify_recipe_expectations};

#[path = "recipe/cad_inspection.rs"]
mod cad_inspection;
#[path = "recipe/capture_sequence.rs"]
mod capture_sequence;
#[path = "recipe/capture_shared.rs"]
mod capture_shared;
#[path = "recipe/semantic_aov.rs"]
mod semantic_aov;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecipeRenderCommandArgs {
    recipe: PathBuf,
    out: PathBuf,
    introspect: bool,
    verify: bool,
    detail: bool,
    gpu: bool,
    max_imports: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecipeBuildCommandArgs {
    recipe: PathBuf,
    max_imports: Option<usize>,
}

pub(crate) fn run_recipe_build_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = RecipeBuildCommandArgs::parse(args)?;
    let mut policy = scena::RecipeBuildPolicy::testing();
    if let Some(max_imports) = args.max_imports {
        policy = policy.with_max_imports(max_imports);
    }
    let recipe_text = match read_recipe_text(&args.recipe, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(report)) => {
            return json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            );
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(format!(
                "failed to read recipe '{}': {error}",
                args.recipe.display()
            ));
        }
    };
    let result = pollster::block_on(scena::SceneHostCore::build_recipe_manifest_json(
        args.recipe.display().to_string(),
        &recipe_text,
        policy,
    ));
    let exit_code = if result.ok { 0 } else { 1 };
    json_outcome(
        &result,
        exit_code,
        "failed to serialize recipe build result",
    )
}

pub(crate) fn run_recipe_render_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = RecipeRenderCommandArgs::parse(args)?;
    if !args.introspect {
        return Err(recipe_render_usage());
    }

    let mut policy = scena::RecipeBuildPolicy::testing();
    if let Some(max_imports) = args.max_imports {
        policy = policy.with_max_imports(max_imports);
    }
    let recipe_text = match read_recipe_text(&args.recipe, &policy) {
        Ok(text) => text,
        Err(RecipeReadError::TooLarge(report)) => {
            return json_outcome(
                &report,
                1,
                "failed to serialize scene recipe validation report",
            );
        }
        Err(RecipeReadError::Io(error)) => {
            return Err(format!(
                "failed to read recipe '{}': {error}",
                args.recipe.display()
            ));
        }
    };
    let recipe_path = args.recipe.display().to_string();
    let build = if args.gpu {
        pollster::block_on(scena::SceneHostCore::build_recipe_json_gpu(
            &recipe_path,
            &recipe_text,
            policy,
        ))
    } else {
        pollster::block_on(scena::SceneHostCore::build_recipe_json(
            &recipe_path,
            &recipe_text,
            policy,
        ))
    };
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            let result = scena::SceneRecipeRenderResultV1::build_failed(manifest);
            return json_outcome(&result, 1, "failed to serialize recipe render result");
        }
    };
    let recipe: scena::SceneRecipeV1 = serde_json::from_str(&recipe_text)
        .map_err(|error| format!("validated recipe failed to decode: {error}"))?;

    let mut host = build.host;
    if !recipe.cameras.iter().any(|camera| camera.active) {
        host.frame_all_with_overlays()
            .map_err(|error| format!("failed to frame recipe scene including overlays: {error}"))?;
    }
    host.prepare()
        .map_err(|error| format!("failed to prepare recipe scene: {error}"))?;
    host.render()
        .map_err(|error| format!("failed to render recipe scene: {error}"))?;
    let capture = host
        .capture()
        .map_err(|error| format!("failed to capture recipe scene: {error}"))?;

    ensure_parent_dir(&args.out)?;
    capture
        .write_png(&args.out)
        .map_err(|error| format!("failed to write PNG '{}': {error}", args.out.display()))?;
    let descriptor_path = capture_descriptor_path(&args.out);
    ensure_parent_dir(&descriptor_path)?;
    std::fs::write(
        &descriptor_path,
        serde_json::to_string_pretty(&capture.descriptor)
            .map_err(|error| format!("failed to serialize capture descriptor: {error}"))?,
    )
    .map_err(|error| {
        format!(
            "failed to write capture descriptor '{}': {error}",
            descriptor_path.display()
        )
    })?;

    let inspection_json = host
        .inspect_json()
        .map_err(|error| format!("failed to inspect recipe scene: {error}"))?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| format!("failed to decode recipe scene inspection report: {error}"))?;
    let introspection_options = render_introspection_options(args.detail)
        .with_capture_png_path(path_for_json(&args.out))
        .with_capture_descriptor_path(path_for_json(&descriptor_path));
    let introspection =
        host.renderer()
            .introspect_capture(&capture, &inspection, introspection_options);
    if !args.verify {
        let exit_code = if introspection.ok { 0 } else { 1 };
        return json_outcome(
            &introspection,
            exit_code,
            "failed to serialize render introspection report",
        );
    }
    let verification = verify_recipe_expectations(RecipeVerificationInput {
        host: &mut host,
        manifest: &build.manifest,
        recipe: &recipe,
        expect: recipe.expect.as_ref(),
        capture: &capture,
        inspection: &inspection,
        introspection: &introspection,
        detail: args.detail,
        recipe_path: &args.recipe,
        recipe_dir: args
            .recipe
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
    })?;
    let result = scena::SceneRecipeRenderResultV1::new(
        build.manifest,
        capture.descriptor,
        introspection,
        verification,
    );
    let exit_code = if result.ok { 0 } else { 1 };
    json_outcome(
        &result,
        exit_code,
        "failed to serialize recipe render result",
    )
}

pub(crate) fn run_recipe_inspect_cad_command(args: &[String]) -> Result<CliOutcome, String> {
    cad_inspection::run_recipe_inspect_cad_command(args)
}

pub(crate) fn run_recipe_capture_command(args: &[String]) -> Result<CliOutcome, String> {
    capture_sequence::run_recipe_capture_command(args)
}

pub(crate) fn run_recipe_aov_command(args: &[String]) -> Result<CliOutcome, String> {
    semantic_aov::run_recipe_aov_command(args)
}

impl RecipeRenderCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(recipe_render_usage());
        };
        let mut out = None;
        let mut introspect = false;
        let mut verify = false;
        let mut detail = false;
        let mut gpu = super::scena_input::gpu_requested_from_env();
        let mut max_imports = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--out" => {
                    out = Some(PathBuf::from(flag_value(args, index, "--out")?));
                    index += 2;
                }
                "--introspect" => {
                    introspect = true;
                    index += 1;
                }
                "--verify" => {
                    verify = true;
                    index += 1;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--gpu" => {
                    gpu = true;
                    index += 1;
                }
                "--max-imports" => {
                    max_imports = Some(parse_positive_usize(
                        "--max-imports",
                        flag_value(args, index, "--max-imports")?,
                    )?);
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown recipe render flag '{flag}'; {}",
                        recipe_render_usage()
                    ));
                }
            }
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            out: out.ok_or_else(|| format!("missing --out <png>; {}", recipe_render_usage()))?,
            introspect,
            verify,
            detail,
            gpu,
            max_imports,
        })
    }
}

impl RecipeBuildCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(recipe_build_usage());
        };
        let mut max_imports = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--max-imports" => {
                    max_imports = Some(parse_positive_usize(
                        "--max-imports",
                        flag_value(args, index, "--max-imports")?,
                    )?);
                    index += 2;
                }
                "--json" => index += 1,
                flag => {
                    return Err(format!(
                        "unknown recipe build flag '{flag}'; {}",
                        recipe_build_usage()
                    ));
                }
            }
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            max_imports,
        })
    }
}

fn parse_positive_usize(flag: &str, value: String) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{flag} requires a positive integer, got 0"));
    }
    Ok(parsed)
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn recipe_render_usage() -> String {
    "usage: scena recipe render <recipe.json> --introspect [--verify] --out <png> [--gpu] [--max-imports <n>]"
        .to_owned()
}

fn recipe_build_usage() -> String {
    "usage: scena recipe build <recipe.json> [--max-imports <n>]".to_owned()
}
