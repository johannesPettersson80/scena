use std::path::{Path, PathBuf};

use serde_json::json;

use super::{
    TEMPLATE_CAPTURE_MIN_HEIGHT, TEMPLATE_CAPTURE_MIN_WIDTH, TemplateBuilder, add_common_commands,
    path_for_json, template_light_rig, template_scene_setup, write_json_file,
};

pub(super) fn build(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready("data-visualization", &["inspection", "scene-host"]);
    let recipe = write_recipe(out_dir, &mut builder)?;
    add_common_commands(out_dir, &recipe, &mut builder);
    let expectation = out_dir.join("appearance-expectation.json");
    write_json_file(
        &expectation,
        &json!({
            "schema": scena::APPEARANCE_EXPECTATION_SCHEMA_V1,
            "targets": [{
                "id": "data-mark-material",
                "tag": "data-mark-blue",
                "color_family": "blue",
                "swatch_srgb8": [64, 128, 191],
                "swatch_tolerance": 0.25,
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
        "verify_data_mark_appearance",
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

fn write_recipe(out_dir: &Path, builder: &mut TemplateBuilder) -> Result<PathBuf, String> {
    let recipe = out_dir.join("recipe.json");
    write_json_file(
        &recipe,
        &json!({
            "schema": scena::SCENE_RECIPE_SCHEMA_V1,
            "colors": {
                "data_blue": "#4080BF",
                "base_gray": "#2F3A46",
                "label_fg": "#FFFFFF",
                "label_bg": "#111827"
            },
            "geometries": [
                { "id": "bar_geo", "primitive": { "kind": "box", "size": [0.04, 0.1, 0.04] } },
                { "id": "base_geo", "primitive": { "kind": "box", "size": [0.32, 0.012, 0.08] } }
            ],
            "materials": [
                { "id": "data_blue_mat", "kind": "unlit", "base_color": "data_blue" },
                { "id": "base_mat", "kind": "unlit", "base_color": "base_gray" }
            ],
            "nodes": [
                {
                    "id": "data_mark_blue",
                    "geometry": "bar_geo",
                    "material": "data_blue_mat",
                    "tags": ["data-mark-blue"],
                    "transform": { "kind": "trs", "translation": [-0.09, 0.075, 0.0], "scale": [1.0, 1.5, 1.0] }
                },
                {
                    "id": "data_mark_mid",
                    "geometry": "bar_geo",
                    "material": "data_blue_mat",
                    "transform": { "kind": "trs", "translation": [0.0, 0.05, 0.0] }
                },
                {
                    "id": "data_mark_low",
                    "geometry": "bar_geo",
                    "material": "data_blue_mat",
                    "transform": { "kind": "trs", "translation": [0.09, 0.035, 0.0], "scale": [1.0, 0.7, 1.0] }
                },
                { "id": "data_base", "geometry": "base_geo", "material": "base_mat" }
            ],
            "labels": [{
                "id": "data_title",
                "text": "THROUGHPUT",
                "color": "label_fg",
                "background": "label_bg",
                "size_px": 18.0,
                "transform": { "kind": "trs", "translation": [0.0, 0.2, 0.0] }
            }],
            "cameras": [{
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 34.0,
                "active": true,
                "transform": { "kind": "look_at", "eye": [0.25, 0.2, 0.26], "target": [0.0, 0.07, 0.0] }
            }],
            "lights": template_light_rig(),
            "scene": template_scene_setup("dark_studio"),
            "render": {
                "profile": "balanced",
                "quality": "high",
                "anti_aliasing": "msaa4",
                "supersample": 2,
                "reconstruction": "tent",
                "tonemapper": "pbr_neutral"
            },
            "capture": {
                "width": TEMPLATE_CAPTURE_MIN_WIDTH,
                "height": TEMPLATE_CAPTURE_MIN_HEIGHT
            },
            "metadata": {
                "template": builder.name,
                "purpose": "authored data-color render and appearance proof"
            }
        }),
    )?;
    builder.file("recipe", &recipe, scena::SCENE_RECIPE_SCHEMA_V1);
    Ok(recipe)
}
