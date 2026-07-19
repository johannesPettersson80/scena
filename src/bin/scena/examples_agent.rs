use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::scena_output::{CliOutcome, json_outcome};
#[path = "examples_agent/builder.rs"]
mod builder;
#[path = "examples_agent/data_visualization.rs"]
mod data_visualization;
#[path = "examples_agent/overlays.rs"]
mod overlays;
#[path = "examples_agent/starter.rs"]
mod starter;

use builder::{TemplateBuilder, capture_descriptor_path, path_for_json, write_json_file};
use overlays::{add_cad_overlay_recipe_sections, add_documentation_overlay_recipe_sections};

const INTERACTION_EXPECTATION_SCHEMA_V1: &str = "scena.interaction_expectation.v1";
const INTERACTION_VERIFICATION_SCHEMA_V1: &str = "scena.interaction_verification.v1";
const TEMPLATE_STUDIO_ENVIRONMENT: &str =
    "tests/assets/environment/polyhaven/studio_small_03_1k.hdr";
const TEMPLATE_CAPTURE_MIN_WIDTH: u32 = 640;
const TEMPLATE_CAPTURE_MIN_HEIGHT: u32 = 480;

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
        let Some(first) = args.first() else {
            return Err(examples_agent_usage());
        };
        let (name, mut index) = if first == "get" {
            let Some(name) = args.get(1) else {
                return Err(examples_agent_usage());
            };
            (name.clone(), 2)
        } else {
            (first.clone(), 1)
        };
        let mut out = None;
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
        let out = out.unwrap_or_else(|| PathBuf::from("target/scena-agent").join(&name));
        Ok(Self { name, out })
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
        "primitive_scene" => starter::primitive_scene(out_dir),
        "cad_plate" => starter::cad_plate(out_dir),
        "dashboard_bars" => starter::dashboard_bars(out_dir),
        "machine_state_viewer" => starter::machine_state_viewer(out_dir),
        "product_configurator" => starter::product_configurator(out_dir),
        "live-state-viewer" => live_state_viewer(out_dir),
        "web-viewer" => web_viewer(out_dir),
        "data-visualization" => data_visualization::build(out_dir),
        "animated-viewer" => animated_viewer(out_dir),
        "interaction-proof" => interaction_proof(out_dir),
        "cad-inspection" => cad_inspection(out_dir),
        "documentation-renderer" => documentation_renderer(out_dir),
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

fn cad_inspection(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("cad-inspection", &["inspection", "scene-host"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/cad_plate_drawing_scene.gltf",
        128,
        96,
        "CAD inspection load, render, and visibility smoke proof",
        &mut builder,
    )?;
    add_cad_overlay_recipe_sections(&recipe)?;
    add_common_commands(out_dir, &recipe, &mut builder);
    builder.command(
        "diagnose_visibility",
        vec!["diagnose", &path_for_json(&recipe), "--visibility"],
        scena::VISIBILITY_DIAGNOSIS_SCHEMA_V1,
        true,
        Vec::new(),
    );
    builder.notes.push(
        "This CLI template authors CAD inspection section-box, measurement, callout, and exploded-view directives through scene_recipe.v1, then verifies the rendered overlay scene."
            .to_string(),
    );
    Ok(builder.finish())
}

fn documentation_renderer(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder =
        TemplateBuilder::ready("documentation-renderer", &["inspection", "scene-host"]);
    let recipe = write_recipe(
        out_dir,
        "tests/assets/gltf/cad_plate_drawing_scene.gltf",
        128,
        96,
        "documentation base render and introspection smoke proof",
        &mut builder,
    )?;
    add_documentation_overlay_recipe_sections(&recipe)?;
    add_common_commands(out_dir, &recipe, &mut builder);
    builder.command(
        "diagnose_visibility",
        vec!["diagnose", &path_for_json(&recipe), "--visibility"],
        scena::VISIBILITY_DIAGNOSIS_SCHEMA_V1,
        true,
        Vec::new(),
    );
    builder.notes.push(
        "This CLI template authors documentation measurement, callout, section-box, and exploded-view directives through scene_recipe.v1, then verifies the rendered overlay scene."
            .to_string(),
    );
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
    let width = width.max(TEMPLATE_CAPTURE_MIN_WIDTH);
    let height = height.max(TEMPLATE_CAPTURE_MIN_HEIGHT);
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
            "lights": template_light_rig(),
            "scene": template_scene_setup("studio"),
            "render": {
                "profile": "balanced",
                "quality": "high",
                "anti_aliasing": "msaa4",
                "supersample": 2,
                "reconstruction": "tent",
                "tonemapper": "pbr_neutral"
            },
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

fn template_light_rig() -> serde_json::Value {
    json!([
        { "id": "key", "kind": "directional", "preset": "key" },
        { "id": "fill", "kind": "directional", "preset": "fill" },
        { "id": "rim", "kind": "directional", "preset": "rim" }
    ])
}

fn template_scene_setup(background: &str) -> serde_json::Value {
    json!({
        "background": { "kind": background },
        "environment": { "kind": "uri", "uri": TEMPLATE_STUDIO_ENVIRONMENT }
    })
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
            "recipe",
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

fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn template_names() -> Vec<&'static str> {
    vec![
        "product-configurator",
        "primitive_scene",
        "cad_plate",
        "dashboard_bars",
        "machine_state_viewer",
        "product_configurator",
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
    "usage: scena examples agent [get] <template> [--out <dir>]".to_string()
}
