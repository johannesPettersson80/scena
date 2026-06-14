use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::{CliOutcome, json_outcome};

const INTERACTION_EXPECTATION_SCHEMA_V1: &str = "scena.interaction_expectation.v1";
const INTERACTION_VERIFICATION_SCHEMA_V1: &str = "scena.interaction_verification.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExamplesAgentCommandArgs {
    name: String,
    out: PathBuf,
}

pub(crate) fn run_examples_agent_command(args: &[String]) -> Result<CliOutcome, String> {
    let args = ExamplesAgentCommandArgs::parse(args)?;
    let template = build_template(&args.name, &args.out)?;
    json_outcome(
        &template,
        0,
        "failed to serialize agent smoke template manifest",
    )
}

impl ExamplesAgentCommandArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let Some(name) = args.first() else {
            return Err(examples_agent_usage());
        };
        let mut out = None;
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--out" => {
                    out = Some(PathBuf::from(flag_value(args, index, "--out")?));
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown examples agent flag '{flag}'; {}",
                        examples_agent_usage()
                    ));
                }
            }
        }
        let out = out.unwrap_or_else(|| PathBuf::from("target/scena-agent").join(name));
        Ok(Self {
            name: name.clone(),
            out,
        })
    }
}

fn build_template(name: &str, out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    fs::create_dir_all(out_dir).map_err(|error| {
        format!(
            "failed to create template dir '{}': {error}",
            out_dir.display()
        )
    })?;
    match name {
        "product-configurator" => product_configurator(out_dir),
        "live-state-viewer" => live_state_viewer(out_dir),
        "web-viewer" => web_viewer(out_dir),
        "data-visualization" => data_visualization(out_dir),
        "animated-viewer" => animated_viewer(out_dir),
        "interaction-proof" => interaction_proof(out_dir),
        "cad-inspection" => deferred_template(
            name,
            "cad-inspection depends on Phase 2 measurement, section box, exploded view, and callout helpers",
        ),
        "documentation-renderer" => deferred_template(
            name,
            "documentation-renderer depends on Phase 2 measurement, callout, annotation layout, section box, and exploded view helpers",
        ),
        other => Err(format!(
            "unknown examples agent template '{other}'; available templates: {}",
            template_names().join(", ")
        )),
    }
}

fn product_configurator(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("product-configurator", &["inspection"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/material_variants_scene.gltf",
        96,
        72,
        "product configurator material variant proof",
        &mut builder,
    )?;
    add_common_commands(out_dir, &recipe, &mut builder);
    let expectation = out_dir.join("appearance-expectation.json");
    write_json_file(
        &expectation,
        &json!({
            "schema": scena::APPEARANCE_EXPECTATION_SCHEMA_V1,
            "targets": [{
                "id": "expected-noon",
                "variant": "noon",
                "color_family": "green",
                "swatch_srgb8": [0, 255, 0],
                "require_source_material": true,
                "alpha_mode": "opaque"
            }]
        }),
    )?;
    builder.file(
        "appearance_expectation",
        &expectation,
        scena::APPEARANCE_EXPECTATION_SCHEMA_V1,
    );
    let appearance_png = out_dir.join("appearance.png");
    builder.command(
        "verify_appearance",
        vec![
            "verify",
            "appearance",
            &path_for_json(&recipe),
            "--expect",
            &path_for_json(&expectation),
            "--out",
            &path_for_json(&appearance_png),
        ],
        scena::APPEARANCE_INTROSPECTION_SCHEMA_V1,
        true,
        vec![appearance_png],
    );
    Ok(builder.finish())
}

fn live_state_viewer(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("live-state-viewer", &["inspection"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        96,
        72,
        "live-state visibility smoke proof",
        &mut builder,
    )?;
    add_common_commands(out_dir, &recipe, &mut builder);
    builder.command(
        "diagnose_visibility",
        vec!["diagnose", &path_for_json(&recipe), "--visibility"],
        scena::VISIBILITY_DIAGNOSIS_SCHEMA_V1,
        true,
        Vec::new(),
    );
    Ok(builder.finish())
}

fn web_viewer(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("web-viewer", &["inspection"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        128,
        96,
        "web viewer render smoke proof",
        &mut builder,
    )?;
    add_common_commands(out_dir, &recipe, &mut builder);
    Ok(builder.finish())
}

fn data_visualization(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("data-visualization", &["inspection"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        96,
        72,
        "data-color render smoke proof",
        &mut builder,
    )?;
    add_common_commands(out_dir, &recipe, &mut builder);
    Ok(builder.finish())
}

fn animated_viewer(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("animated-viewer", &["inspection"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/animated_triangle_scene.glb",
        96,
        72,
        "animation change smoke proof",
        &mut builder,
    )?;
    add_common_commands(out_dir, &recipe, &mut builder);
    builder.command(
        "verify_animation",
        vec![
            "verify",
            "animation",
            &path_for_json(&recipe),
            "--clip",
            "MoveTriangle",
            "--times",
            "0,0.5,1",
            "--expect-change",
        ],
        scena::ANIMATION_INTROSPECTION_SCHEMA_V1,
        true,
        Vec::new(),
    );
    Ok(builder.finish())
}

fn interaction_proof(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("interaction-proof", &["inspection", "scene-host"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
        128,
        128,
        "synthetic interaction smoke proof",
        &mut builder,
    )?;
    add_common_commands(out_dir, &recipe, &mut builder);
    let expectation = out_dir.join("interaction-expectation.json");
    write_json_file(
        &expectation,
        &json!({
            "schema": INTERACTION_EXPECTATION_SCHEMA_V1,
            "viewport": {
                "width_css_px": 128.0,
                "height_css_px": 128.0,
                "device_pixel_ratio": 1.0
            },
            "steps": [
                {
                    "action": "hover",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expected_events": ["hover"]
                },
                {
                    "action": "select",
                    "x_css_px": 64.0,
                    "y_css_px": 64.0,
                    "expect_hit": true,
                    "expect_hover": true,
                    "expect_selection": true,
                    "expected_events": ["selection_changed"]
                }
            ]
        }),
    )?;
    builder.file(
        "interaction_expectation",
        &expectation,
        INTERACTION_EXPECTATION_SCHEMA_V1,
    );
    builder.command(
        "verify_interaction",
        vec![
            "verify",
            "interaction",
            &path_for_json(&recipe),
            "--expect",
            &path_for_json(&expectation),
        ],
        INTERACTION_VERIFICATION_SCHEMA_V1,
        true,
        Vec::new(),
    );
    Ok(builder.finish())
}

fn deferred_template(name: &str, note: &str) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::new(name, "deferred", &[]);
    builder.notes.push(note.to_string());
    Ok(builder.finish())
}

fn write_recipe(
    out_dir: &Path,
    asset: &str,
    width: u32,
    height: u32,
    purpose: &str,
    builder: &mut TemplateBuilder,
) -> Result<PathBuf, String> {
    let recipe = out_dir.join("recipe.json");
    write_json_file(
        &recipe,
        &json!({
            "schema": scena::SCENE_RECIPE_SCHEMA_V1,
            "imports": [{
                "id": "primary",
                "uri": asset,
                "expected_extent": {
                    "min": 0.1,
                    "max": 10.0,
                    "unit": "m"
                }
            }],
            "capture": {
                "width": width,
                "height": height
            },
            "metadata": {
                "template": builder.name,
                "purpose": purpose
            }
        }),
    )?;
    builder.file("recipe", &recipe, scena::SCENE_RECIPE_SCHEMA_V1);
    Ok(recipe)
}

fn add_common_commands(out_dir: &Path, recipe: &Path, builder: &mut TemplateBuilder) {
    builder.command(
        "validate_recipe",
        vec!["validate-recipe", &path_for_json(recipe)],
        scena::SCENE_RECIPE_VALIDATION_SCHEMA_V1,
        true,
        Vec::new(),
    );
    let frame = out_dir.join("frame.png");
    builder.command(
        "render_introspect",
        vec![
            "render",
            &path_for_json(recipe),
            "--introspect",
            "--out",
            &path_for_json(&frame),
        ],
        scena::RENDER_INTROSPECTION_SCHEMA_V1,
        true,
        vec![frame.clone(), capture_descriptor_path(&frame)],
    );
}

fn write_json_file(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!("failed to create directory '{}': {error}", parent.display())
        })?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(value)
            .map_err(|error| format!("failed to serialize '{}': {error}", path.display()))?,
    )
    .map_err(|error| format!("failed to write '{}': {error}", path.display()))
}

fn capture_descriptor_path(png_path: &Path) -> PathBuf {
    let stem = png_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("capture");
    png_path.with_file_name(format!("{stem}.capture.json"))
}

fn path_for_json(path: &Path) -> String {
    path.display().to_string()
}

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn template_names() -> Vec<&'static str> {
    vec![
        "product-configurator",
        "live-state-viewer",
        "web-viewer",
        "data-visualization",
        "animated-viewer",
        "interaction-proof",
        "cad-inspection",
        "documentation-renderer",
    ]
}

fn examples_agent_usage() -> String {
    "usage: scena examples agent <template> [--out <dir>]".to_string()
}

struct TemplateBuilder {
    name: String,
    status: String,
    required_features: Vec<String>,
    files: Vec<scena::AgentSmokeTemplateFileV1>,
    commands: Vec<scena::AgentSmokeTemplateCommandV1>,
    notes: Vec<String>,
}

impl TemplateBuilder {
    fn ready(name: &str, required_features: &[&str]) -> Self {
        Self::new(name, "ready", required_features)
    }

    fn new(name: &str, status: &str, required_features: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            status: status.to_string(),
            required_features: required_features
                .iter()
                .map(|feature| feature.to_string())
                .collect(),
            files: Vec::new(),
            commands: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn file(&mut self, kind: &str, path: &Path, schema: &str) {
        self.files.push(scena::AgentSmokeTemplateFileV1 {
            kind: kind.to_string(),
            path: path_for_json(path),
            schema: schema.to_string(),
        });
    }

    fn command(
        &mut self,
        name: &str,
        args: Vec<&str>,
        expected_schema: &str,
        expected_ok: bool,
        artifacts: Vec<PathBuf>,
    ) {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push("scena".to_string());
        argv.extend(args.into_iter().map(str::to_string));
        self.commands.push(scena::AgentSmokeTemplateCommandV1 {
            name: name.to_string(),
            argv,
            expected_schema: expected_schema.to_string(),
            expected_ok,
            artifacts: artifacts.iter().map(|path| path_for_json(path)).collect(),
        });
    }

    fn finish(self) -> scena::AgentSmokeTemplateV1 {
        scena::AgentSmokeTemplateV1 {
            schema: scena::AGENT_SMOKE_TEMPLATE_SCHEMA_V1.to_string(),
            name: self.name,
            status: self.status,
            required_features: self.required_features,
            files: self.files,
            commands: self.commands,
            notes: self.notes,
        }
    }
}
