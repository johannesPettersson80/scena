use crate::scena_cli_error::{CliErrorKind, CliFailure};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::capture_shared::{write_png_gray16, write_png_rgba8};
use crate::scena_input::{RecipeReadError, read_recipe_text};
use crate::scena_output::{CliOutcome, json_outcome};

const SEMANTIC_AOV_RESULT_SCHEMA_V1: &str = "scena.semantic_aov_result.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AovPass {
    Id,
    Depth,
    Normal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticAovArgs {
    recipe: PathBuf,
    out_dir: PathBuf,
    passes: BTreeSet<AovPass>,
    max_imports: Option<usize>,
}

pub(crate) fn run_recipe_aov_command(args: &[String]) -> Result<CliOutcome, CliFailure> {
    let args = SemanticAovArgs::parse(args)?;
    std::fs::create_dir_all(&args.out_dir).map_err(|error| {
        format!(
            "failed to create semantic AOV output directory '{}': {error}",
            args.out_dir.display()
        )
    })?;
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
            return Err(CliFailure::new(
                CliErrorKind::InputNotFound,
                format!("failed to read recipe '{}': {error}", args.recipe.display()),
            ));
        }
    };
    let recipe: scena::SceneRecipeV1 = serde_json::from_str(&recipe_text)
        .map_err(|error| format!("validated semantic AOV recipe failed to decode: {error}"))?;
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        args.recipe.display().to_string(),
        &recipe_text,
        policy,
    ));
    let build = match build {
        Ok(build) => build,
        Err(manifest) => {
            let result = scena::SceneRecipeRenderResultV1::build_failed(manifest);
            return json_outcome(&result, 1, "failed to serialize semantic AOV build failure");
        }
    };
    let manifest = build.manifest;
    let mut host = build.host;
    if !recipe.cameras.iter().any(|camera| camera.active) {
        host.frame_all_with_overlays()
            .map_err(|error| format!("failed to frame semantic AOV subject: {error}"))?;
    }
    host.prepare()
        .map_err(|error| format!("failed to prepare semantic AOV subject: {error}"))?;
    let capture = host
        .capture_semantic_aovs()
        .map_err(|error| format!("failed to capture semantic AOVs: {error}"))?;

    let mut images = Map::new();
    if args.passes.contains(&AovPass::Id) {
        let path = args.out_dir.join("id.png");
        let rgba8 = capture.id_rgba8();
        write_png_rgba8(&path, capture.width, capture.height, &rgba8)?;
        images.insert(
            "id".to_owned(),
            image_report(&path, "rgba8_palette", &rgba8)?,
        );
    }
    if args.passes.contains(&AovPass::Depth) {
        let path = args.out_dir.join("depth.png");
        let samples = capture.depth_u16();
        write_png_gray16(&path, capture.width, capture.height, &samples)?;
        let bytes = samples
            .iter()
            .flat_map(|sample| sample.to_be_bytes())
            .collect::<Vec<_>>();
        images.insert("depth".to_owned(), image_report(&path, "gray16", &bytes)?);
    }
    if args.passes.contains(&AovPass::Normal) {
        let path = args.out_dir.join("normal.png");
        let rgba8 = capture.normal_rgba8();
        write_png_rgba8(&path, capture.width, capture.height, &rgba8)?;
        images.insert("normal".to_owned(), image_report(&path, "rgba8", &rgba8)?);
    }

    let persistent_nodes = persistent_node_map(&manifest);
    let persistent_instances = manifest
        .instances
        .iter()
        .map(|instance| {
            (
                (instance.set_handle, instance.instance_id),
                json!({
                    "kind": "recipe_instance",
                    "set_id": instance.set_id,
                    "instance_id": instance.id,
                }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let legend = capture
        .legend
        .iter()
        .map(|entry| {
            let persistent = entry
                .instance_id
                .and_then(|instance| {
                    persistent_instances
                        .get(&(entry.node_handle, instance))
                        .cloned()
                })
                .or_else(|| persistent_nodes.get(&entry.node_handle).cloned())
                .unwrap_or_else(|| json!({ "kind": "runtime_only" }));
            json!({
                "palette_index": entry.palette_index,
                "rgba8": entry.rgba8,
                "runtime_identity": {
                    "scope": "runtime_scoped",
                    "node_handle": entry.node_handle,
                    "instance_handle": entry.instance_handle,
                    "instance_id": entry.instance_id,
                },
                "persistent_identity": persistent,
            })
        })
        .collect::<Vec<_>>();
    let report_path = args.out_dir.join("semantic-aov-result.json");
    let report = json!({
        "schema": SEMANTIC_AOV_RESULT_SCHEMA_V1,
        "ok": true,
        "source_recipe": path_json(&args.recipe),
        "output_dir": path_json(&args.out_dir),
        "width": capture.width,
        "height": capture.height,
        "semantics": {
            "identity": "node_plus_optional_instance",
            "primitive_identity": "not_encoded_v1",
            "identity_scope": capture.identity_scope,
            "background": { "palette_index": 0, "rgba8": [0, 0, 0, 0] },
            "transparency": "excluded_and_counted",
            "strokes_labels_overlays": "excluded_and_counted",
            "sample_pattern": capture.sample_pattern,
            "msaa_resolve": "not_applied",
            "occlusion": "nearest_opaque_fragment",
            "depth": {
                "raw_convention": capture.depth_convention,
                "units": "scene_meters",
                "background_raw": "positive_infinity",
                "png_encoding": "linear_near_far_gray16_zero_background",
                "near": capture.near,
                "far": capture.far,
            },
            "normal": {
                "coordinate_space": capture.normal_space,
                "png_encoding": "normal_times_half_plus_half_rgba8",
                "background": [0, 0, 0, 0],
            },
        },
        "images": images,
        "legend": legend,
        "exclusions": capture.exclusions,
        "report": path_json(&report_path),
    });
    std::fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report)
            .map_err(|error| format!("failed to serialize semantic AOV report: {error}"))?,
    )
    .map_err(|error| {
        format!(
            "failed to write semantic AOV report '{}': {error}",
            report_path.display()
        )
    })?;
    json_outcome(&report, 0, "failed to serialize semantic AOV result")
}

impl SemanticAovArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(usage());
        };
        let mut out_dir = None;
        let mut passes = [AovPass::Id, AovPass::Depth, AovPass::Normal]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut max_imports = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--out-dir" => {
                    out_dir = Some(PathBuf::from(flag_value(args, index, "--out-dir")?));
                    index += 2;
                }
                "--passes" => {
                    passes = parse_passes(&flag_value(args, index, "--passes")?)?;
                    index += 2;
                }
                "--max-imports" => {
                    let value = flag_value(args, index, "--max-imports")?;
                    let parsed = value.parse::<usize>().map_err(|_| {
                        format!("--max-imports requires a positive integer, got '{value}'")
                    })?;
                    if parsed == 0 {
                        return Err("--max-imports requires a positive integer, got 0".to_owned());
                    }
                    max_imports = Some(parsed);
                    index += 2;
                }
                "--json" => index += 1,
                flag => return Err(format!("unknown recipe aov argument '{flag}'; {}", usage())),
            }
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            out_dir: out_dir.ok_or_else(|| format!("missing --out-dir <dir>; {}", usage()))?,
            passes,
            max_imports,
        })
    }
}

fn parse_passes(value: &str) -> Result<BTreeSet<AovPass>, String> {
    let mut passes = BTreeSet::new();
    for name in value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let pass = match name {
            "id" => AovPass::Id,
            "depth" => AovPass::Depth,
            "normal" => AovPass::Normal,
            _ => {
                return Err(format!(
                    "unknown semantic AOV pass '{name}'; expected id,depth,normal"
                ));
            }
        };
        passes.insert(pass);
    }
    if passes.is_empty() {
        return Err("--passes requires at least one of id,depth,normal".to_owned());
    }
    Ok(passes)
}

fn persistent_node_map(manifest: &scena::SceneRecipeBuildV1) -> BTreeMap<u64, Value> {
    let mut nodes = manifest
        .nodes
        .iter()
        .map(|node| {
            (
                node.handle,
                json!({ "kind": "recipe_node", "node_id": node.id }),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for import in &manifest.imports {
        for (path, handle) in &import.nodes_by_path {
            nodes.insert(
                *handle,
                json!({
                    "kind": "import_node",
                    "import_id": import.id,
                    "node_path": path,
                }),
            );
        }
    }
    nodes
}

fn image_report(path: &Path, encoding: &str, raw: &[u8]) -> Result<Value, String> {
    Ok(json!({
        "png": path_json(path),
        "encoding": encoding,
        "raw_fnv1a64": scena::fnv1a64_hex(raw),
        "png_bytes": std::fs::metadata(path)
            .map_err(|error| format!("failed to inspect PNG '{}': {error}", path.display()))?
            .len(),
    }))
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn path_json(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn usage() -> String {
    "usage: scena recipe aov <recipe.json> --out-dir <dir> [--passes id,depth,normal] [--max-imports <n>]".to_owned()
}
