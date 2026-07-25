use crate::scena_cli_error::CliFailure;
use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

use crate::scena_output::{
    CliBackendSelectionV1, CliOutcome, json_outcome, json_outcome_with_backend_selection,
};

#[path = "cad_inspection/image.rs"]
mod image;
#[path = "cad_inspection/view.rs"]
mod view;

const CAD_INSPECTION_SCHEMA_V1: &str = "scena.cad_inspection_result.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
struct InspectCadArgs {
    recipe: PathBuf,
    out_dir: PathBuf,
    width: u32,
    height: u32,
    gpu: bool,
    max_imports: Option<usize>,
}

pub(crate) fn run_recipe_inspect_cad_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = InspectCadArgs::parse(args)?;
    fs::create_dir_all(&args.out_dir).map_err(|error| {
        format!(
            "failed to create output directory '{}': {error}",
            args.out_dir.display()
        )
    })?;

    let recipe_text = fs::read_to_string(&args.recipe)
        .map_err(|error| format!("failed to read recipe '{}': {error}", args.recipe.display()))?;
    let mut policy = scena::RecipeBuildPolicy::testing();
    if let Some(max_imports) = args.max_imports {
        policy = policy.with_max_imports(max_imports);
    }
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        &args.recipe.display().to_string(),
        &recipe_text,
        policy,
    ));
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            let result = scena::SceneRecipeRenderResultV1::build_failed(manifest);
            return json_outcome_with_backend_selection(
                &result,
                1,
                "failed to serialize CAD inspection build failure",
                CliBackendSelectionV1::new(args.gpu, None),
            );
        }
    };
    let inspection_json = build
        .host
        .inspect_json()
        .map_err(|error| format!("failed to inspect source recipe: {error}"))?;
    let inspection: scena::SceneInspectionReportV1 = serde_json::from_str(&inspection_json)
        .map_err(|error| format!("failed to decode source inspection report: {error}"))?;
    let bounds = view::subject_bounds(&inspection)?;
    let mut base_recipe: Value = serde_json::from_str(&recipe_text).map_err(|error| {
        format!(
            "failed to decode recipe '{}': {error}",
            args.recipe.display()
        )
    })?;
    view::rewrite_relative_imports(&mut base_recipe, &args.recipe)?;

    let mut views = Vec::new();
    let mut processed_images = Vec::new();
    for kind in [
        view::ViewKind::BroadFace,
        view::ViewKind::TopFeatures,
        view::ViewKind::Overview,
    ] {
        let view_id = kind.id();
        let recipe_path = args.out_dir.join(format!("{view_id}.recipe.json"));
        let raw_png = args.out_dir.join(format!("{view_id}.raw.png"));
        let processed_png = args.out_dir.join(format!("{view_id}.png"));
        let render_result_json = args.out_dir.join(format!("{view_id}.render-result.json"));
        let camera = view::camera_for(kind, bounds);
        let view_recipe = view::inspection_recipe(
            base_recipe.clone(),
            kind,
            camera,
            args.width,
            args.height,
            bounds,
        );
        fs::write(
            &recipe_path,
            serde_json::to_string_pretty(&view_recipe)
                .map_err(|error| format!("failed to serialize {view_id} recipe: {error}"))?,
        )
        .map_err(|error| {
            format!(
                "failed to write generated recipe '{}': {error}",
                recipe_path.display()
            )
        })?;

        let mut render_args = vec![
            recipe_path.display().to_string(),
            "--introspect".to_owned(),
            "--verify".to_owned(),
            "--detail".to_owned(),
            "--out".to_owned(),
            raw_png.display().to_string(),
        ];
        if args.gpu {
            render_args.push("--gpu".to_owned());
        }
        if let Some(max_imports) = args.max_imports {
            render_args.push("--max-imports".to_owned());
            render_args.push(max_imports.to_string());
        }
        let render = super::run_recipe_render_command(&render_args)?;
        fs::write(&render_result_json, render.stdout.as_bytes()).map_err(|error| {
            format!(
                "failed to write render result '{}': {error}",
                render_result_json.display()
            )
        })?;
        let result_json: Value = serde_json::from_str(&render.stdout)
            .map_err(|error| format!("failed to decode {view_id} render result: {error}"))?;
        let (processed, metrics) = image::process_cad_png(&raw_png, &processed_png)?;
        processed_images.push(processed);
        views.push(json!({
            "id": view_id,
            "purpose": kind.purpose(),
            "recipe_json": path_for_json(&recipe_path),
            "raw_png": path_for_json(&raw_png),
            "processed_png": path_for_json(&processed_png),
            "render_result_json": path_for_json(&render_result_json),
            "camera": {
                "eye": view::vec3_json(camera.eye),
                "target": view::vec3_json(camera.target),
                "up": view::vec3_json(camera.up),
                "fov_degrees": camera.fov_degrees
            },
            "render_result": {
                "ok": result_json["ok"].as_bool().unwrap_or(false),
                "introspection_ok": result_json["introspection"]["ok"].as_bool().unwrap_or(false),
                "verification_ok": result_json["verification"]["ok"].as_bool().unwrap_or(false),
                "backend_selection": result_json["backend_selection"].clone()
            },
            "postprocess": image::postprocess_json(metrics, args.width, args.height)
        }));
    }

    let contact_sheet_png = args.out_dir.join("cad-inspection-contact-sheet.png");
    image::write_contact_sheet(&processed_images, &contact_sheet_png)?;
    let ok = views.iter().all(|view| {
        view["render_result"]["ok"].as_bool().unwrap_or(false)
            && view["render_result"]["introspection_ok"]
                .as_bool()
                .unwrap_or(false)
            && view["render_result"]["verification_ok"]
                .as_bool()
                .unwrap_or(false)
            && view["postprocess"]["foreground_pixels"]
                .as_u64()
                .unwrap_or(0)
                > 0
            && view["postprocess"]["edge_pixels"].as_u64().unwrap_or(0) > 0
    });
    let report = json!({
        "schema": CAD_INSPECTION_SCHEMA_V1,
        "ok": ok,
        "source_recipe": path_for_json(&args.recipe),
        "output_dir": path_for_json(&args.out_dir),
        "contact_sheet_png": path_for_json(&contact_sheet_png),
        "subject_bounds": {
            "min": view::vec3_json(bounds.min),
            "max": view::vec3_json(bounds.max),
            "extent": view::vec3_json(bounds.extent())
        },
        "presentation_policy": {
            "cad_truth_owned_by": "source recipe and imported geometry",
            "tone_override": "presentation_only",
            "edge_emphasis": "presentation_only",
            "geometry_modified": false
        },
        "backend_selection": views.first()
            .map(|view| view["render_result"]["backend_selection"].clone())
            .unwrap_or(Value::Null),
        "views": views
    });
    json_outcome(
        &report,
        if ok { 0 } else { 1 },
        "failed to serialize CAD inspection result",
    )
}

impl InspectCadArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(inspect_cad_usage());
        };
        let mut out_dir = None;
        let mut width = 2560;
        let mut height = 1920;
        let mut gpu = false;
        let mut max_imports = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--out-dir" => {
                    out_dir = Some(PathBuf::from(flag_value(args, index, "--out-dir")?));
                    index += 2;
                }
                "--width" => {
                    width = parse_positive_u32(&flag_value(args, index, "--width")?, "--width")?;
                    index += 2;
                }
                "--height" => {
                    height = parse_positive_u32(&flag_value(args, index, "--height")?, "--height")?;
                    index += 2;
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
                        "unknown recipe inspect-cad flag '{flag}'; {}",
                        inspect_cad_usage()
                    ));
                }
            }
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            out_dir: out_dir
                .ok_or_else(|| format!("missing --out-dir <dir>; {}", inspect_cad_usage()))?,
            width,
            height,
            gpu,
            max_imports,
        })
    }
}

fn inspect_cad_usage() -> String {
    "usage: scena recipe inspect-cad <recipe.json> --out-dir <dir> [--width 2560] [--height 1920] [--gpu] [--max-imports <n>]"
        .to_owned()
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_positive_u32(value: &str, flag: &str) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|error| format!("{flag} must be a positive integer: {error}"))?;
    if parsed == 0 {
        return Err(format!("{flag} must be greater than zero"));
    }
    Ok(parsed)
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

fn path_for_json(path: &std::path::Path) -> String {
    path.display().to_string()
}
