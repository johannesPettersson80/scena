use std::path::PathBuf;

use super::scena_input::{
    RecipeReadError, capture_descriptor_path, ensure_parent_dir, path_for_json, read_recipe_text,
    render_introspection_options,
};
use super::scena_output::{CliOutcome, json_outcome};

#[path = "recipe/verification.rs"]
mod verification;

use verification::verify_recipe_expectations;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecipeRenderCommandArgs {
    recipe: PathBuf,
    out: PathBuf,
    introspect: bool,
    verify: bool,
    detail: bool,
}

pub(crate) fn run_recipe_render_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = RecipeRenderCommandArgs::parse(args)?;
    if !args.introspect || !args.verify {
        return Err(recipe_render_usage());
    }

    let policy = scena::RecipeBuildPolicy::testing();
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
    let build = match pollster::block_on(scena::SceneHostCore::build_recipe_json(
        args.recipe.display().to_string(),
        &recipe_text,
        policy,
    )) {
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
    let verification = verify_recipe_expectations(
        &mut host,
        &build.manifest,
        recipe.expect.as_ref(),
        &capture,
        &inspection,
        &introspection,
        args.detail,
    )?;
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

impl RecipeRenderCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(recipe_render_usage());
        };
        let mut out = None;
        let mut introspect = false;
        let mut verify = false;
        let mut detail = false;
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
        })
    }
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn recipe_render_usage() -> String {
    "usage: scena recipe render <recipe.json> --introspect --verify --out <png>".to_owned()
}
