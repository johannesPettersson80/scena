use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::add_common_commands;
use super::builder::{TemplateBuilder, write_json_file};

pub(super) fn primitive_scene(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    authored_template(
        "primitive_scene",
        out_dir,
        json!({
            "schema": scena::SCENE_RECIPE_SCHEMA_V1,
            "colors": {
                "cube_blue": "#3A7BD5",
                "sphere_gold": "#F6C85F",
                "line_gray": "#697386",
                "label_fg": "#FFFFFF",
                "label_bg": "#1D2733"
            },
            "geometries": [
                { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } },
                { "id": "sphere_geo", "primitive": { "kind": "sphere", "radius": 0.045, "segments": 24, "rings": 12 } },
                { "id": "grid_geo", "primitive": { "kind": "grid", "length": 0.42, "divisions": 8 } },
                { "id": "axes_geo", "primitive": { "kind": "axes", "length": 0.16 } }
            ],
            "materials": [
                { "id": "cube_mat", "kind": "pbr_metallic_roughness", "base_color": "cube_blue", "metallic": 0.0, "roughness": 0.55 },
                { "id": "sphere_mat", "kind": "pbr_metallic_roughness", "base_color": "sphere_gold", "metallic": 0.15, "roughness": 0.35 },
                { "id": "line_mat", "kind": "line", "base_color": "line_gray", "stroke_width_px": 1.5 }
            ],
            "nodes": [
                { "id": "grid", "geometry": "grid_geo", "material": "line_mat" },
                { "id": "axes", "geometry": "axes_geo", "material": "line_mat" },
                { "id": "cube", "geometry": "cube_geo", "material": "cube_mat", "transform": { "kind": "trs", "translation": [-0.07, 0.04, 0.0] } },
                { "id": "sphere", "geometry": "sphere_geo", "material": "sphere_mat", "transform": { "kind": "trs", "translation": [0.08, 0.045, 0.01] } }
            ],
            "labels": [
                { "id": "scene_label", "text": "primitive scene", "color": "label_fg", "background": "label_bg", "size_px": 16.0, "transform": { "kind": "trs", "translation": [0.0, 0.16, 0.0] } }
            ],
            "cameras": [
                { "id": "main", "kind": "perspective", "fov_degrees": 38.0, "active": true, "transform": { "kind": "look_at", "eye": [0.28, 0.22, 0.32], "target": [0.0, 0.05, 0.0] } }
            ],
            "scene": { "background": { "kind": "studio" }, "grid": { "padding": 0.08, "line_spacing": 0.05 } },
            "render": { "profile": "balanced", "quality": "medium", "anti_aliasing": "fxaa", "tonemapper": "pbr_neutral" },
            "capture": { "width": 320, "height": 220 },
            "metadata": { "template": "primitive_scene", "purpose": "authored primitive starter scene" }
        }),
    )
}

pub(super) fn cad_plate(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    authored_template(
        "cad_plate",
        out_dir,
        json!({
            "schema": scena::SCENE_RECIPE_SCHEMA_V1,
            "colors": {
                "plate_blue": "#3A7BD5",
                "edge_white": "#F4F7FB",
                "label_fg": "#FFFFFF",
                "label_bg": "#1D2733"
            },
            "geometries": [
                { "id": "plate_geo", "primitive": { "kind": "box", "size": [0.12, 0.008, 0.06] } },
                {
                    "id": "plate_outline_geo",
                    "primitive": {
                        "kind": "polyline",
                        "points": [[-0.06, 0.006, -0.03], [0.06, 0.006, -0.03], [0.06, 0.006, 0.03], [-0.06, 0.006, 0.03], [-0.06, 0.006, -0.03]]
                    }
                }
            ],
            "materials": [
                { "id": "plate_mat", "kind": "pbr_metallic_roughness", "base_color": "plate_blue", "metallic": 0.05, "roughness": 0.5 },
                { "id": "edge_mat", "kind": "line", "base_color": "edge_white", "stroke_width_px": 2.0 }
            ],
            "nodes": [
                { "id": "plate", "geometry": "plate_geo", "material": "plate_mat", "name": "120x60 CAD plate", "transform": { "kind": "center" } },
                { "id": "plate_outline", "geometry": "plate_outline_geo", "material": "edge_mat" }
            ],
            "section_box": {
                "target": { "kind": "node", "id": "plate" },
                "margin": 0.01,
                "helper_wireframe": true
            },
            "measurements": [
                { "id": "plate-width", "kind": "distance", "start": [-0.06, 0.012, -0.038], "end": [0.06, 0.012, -0.038], "label": "120.0 mm", "unit": "mm", "precision": 1 }
            ],
            "callouts": [
                { "id": "datum-a", "text": "datum A", "target": { "kind": "node", "id": "plate", "local_offset": [0.0, 0.006, 0.0] }, "label_offset": [0.06, 0.05, 0.0] }
            ],
            "labels": [
                { "id": "cad_label", "text": "CAD plate", "color": "label_fg", "background": "label_bg", "size_px": 16.0, "transform": { "kind": "trs", "translation": [0.0, 0.08, 0.0] } }
            ],
            "cameras": [
                { "id": "main", "kind": "perspective", "fov_degrees": 32.0, "active": true, "transform": { "kind": "look_at", "eye": [0.18, 0.14, 0.16], "target": [0.0, 0.02, 0.0] } }
            ],
            "scene": { "background": { "kind": "studio" } },
            "render": { "profile": "balanced", "quality": "medium", "anti_aliasing": "fxaa", "tonemapper": "pbr_neutral" },
            "capture": { "width": 320, "height": 220 },
            "metadata": { "template": "cad_plate", "purpose": "authored CAD inspection starter with section box, dimension, and callout" }
        }),
    )
}

pub(super) fn dashboard_bars(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    authored_template(
        "dashboard_bars",
        out_dir,
        json!({
            "schema": scena::SCENE_RECIPE_SCHEMA_V1,
            "colors": {
                "bar_green": "#33A852",
                "bar_yellow": "#F6C85F",
                "bar_red": "#D94F45",
                "base_gray": "#2F3A46",
                "label_fg": "#FFFFFF",
                "label_bg": "#111827"
            },
            "geometries": [
                { "id": "bar_geo", "primitive": { "kind": "box", "size": [0.035, 0.1, 0.035] } },
                { "id": "base_geo", "primitive": { "kind": "box", "size": [0.28, 0.01, 0.06] } }
            ],
            "materials": [
                { "id": "green_mat", "kind": "unlit", "base_color": "bar_green" },
                { "id": "yellow_mat", "kind": "unlit", "base_color": "bar_yellow" },
                { "id": "red_mat", "kind": "unlit", "base_color": "bar_red" },
                { "id": "base_mat", "kind": "unlit", "base_color": "base_gray" }
            ],
            "nodes": [
                { "id": "base", "geometry": "base_geo", "material": "base_mat" },
                { "id": "bar_a", "geometry": "bar_geo", "material": "green_mat", "transform": { "kind": "trs", "translation": [-0.09, 0.06, 0.0], "scale": [1.0, 1.2, 1.0] } },
                { "id": "bar_b", "geometry": "bar_geo", "material": "yellow_mat", "transform": { "kind": "trs", "translation": [0.0, 0.04, 0.0], "scale": [1.0, 0.8, 1.0] } },
                { "id": "bar_c", "geometry": "bar_geo", "material": "red_mat", "transform": { "kind": "trs", "translation": [0.09, 0.075, 0.0], "scale": [1.0, 1.5, 1.0] } }
            ],
            "labels": [
                { "id": "title", "text": "LINE A", "color": "label_fg", "background": "label_bg", "size_px": 18.0, "transform": { "kind": "trs", "translation": [0.0, 0.18, 0.0] } },
                { "id": "alarm", "text": "ALARM", "color": "label_fg", "background": "bar_red", "size_px": 14.0, "transform": { "kind": "trs", "translation": [0.12, 0.14, 0.0] } }
            ],
            "cameras": [
                { "id": "main", "kind": "perspective", "fov_degrees": 34.0, "active": true, "transform": { "kind": "look_at", "eye": [0.25, 0.2, 0.26], "target": [0.0, 0.07, 0.0] } }
            ],
            "scene": { "background": { "kind": "dark_studio" } },
            "render": { "profile": "balanced", "quality": "medium", "anti_aliasing": "fxaa", "tonemapper": "pbr_neutral" },
            "capture": { "width": 320, "height": 220 },
            "metadata": { "template": "dashboard_bars", "purpose": "authored industrial dashboard bars starter" }
        }),
    )
}

pub(super) fn machine_state_viewer(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    authored_template(
        "machine_state_viewer",
        out_dir,
        json!({
            "schema": scena::SCENE_RECIPE_SCHEMA_V1,
            "colors": {
                "machine_body": "#4B5B6B",
                "motor_blue": "#3A7BD5",
                "pipe_gray": "#B7C2D0",
                "ok_green": "#33A852",
                "warn_yellow": "#F6C85F",
                "alarm_red": "#D94F45",
                "label_fg": "#FFFFFF",
                "label_bg": "#1D2733"
            },
            "geometries": [
                { "id": "base_geo", "primitive": { "kind": "box", "size": [0.22, 0.04, 0.1] } },
                { "id": "motor_geo", "primitive": { "kind": "cylinder", "radius": 0.035, "height": 0.09, "segments": 24 } },
                { "id": "pipe_geo", "primitive": { "kind": "line", "start": [-0.12, 0.08, 0.0], "end": [0.12, 0.08, 0.0] } },
                { "id": "status_geo", "primitive": { "kind": "sphere", "radius": 0.012, "segments": 16, "rings": 8 } }
            ],
            "materials": [
                { "id": "body_mat", "kind": "pbr_metallic_roughness", "base_color": "machine_body", "metallic": 0.1, "roughness": 0.65 },
                { "id": "motor_mat", "kind": "pbr_metallic_roughness", "base_color": "motor_blue", "metallic": 0.2, "roughness": 0.45 },
                { "id": "pipe_mat", "kind": "line", "base_color": "pipe_gray", "stroke_width_px": 3.0 },
                { "id": "status_mat", "kind": "unlit", "base_color": "ok_green" }
            ],
            "nodes": [
                { "id": "machine_base", "geometry": "base_geo", "material": "body_mat" },
                { "id": "motor_left", "geometry": "motor_geo", "material": "motor_mat", "transform": { "kind": "trs", "translation": [-0.06, 0.06, 0.0], "rotation_degrees": [0.0, 0.0, 90.0] } },
                { "id": "motor_right", "geometry": "motor_geo", "material": "motor_mat", "transform": { "kind": "trs", "translation": [0.06, 0.06, 0.0], "rotation_degrees": [0.0, 0.0, 90.0] } },
                { "id": "pipe", "geometry": "pipe_geo", "material": "pipe_mat" }
            ],
            "instance_sets": [{
                "id": "status_lights",
                "geometry": "status_geo",
                "material": "status_mat",
                "instances": [
                    { "id": "ok", "transform": { "kind": "trs", "translation": [-0.08, 0.115, 0.045] }, "tint": "ok_green" },
                    { "id": "warn", "transform": { "kind": "trs", "translation": [0.0, 0.115, 0.045] }, "tint": "warn_yellow" },
                    { "id": "alarm", "transform": { "kind": "trs", "translation": [0.08, 0.115, 0.045] }, "tint": "alarm_red" }
                ]
            }],
            "labels": [
                { "id": "state_label", "text": "LINE 7 RUNNING", "color": "label_fg", "background": "label_bg", "size_px": 15.0, "transform": { "kind": "trs", "translation": [0.0, 0.16, 0.0] } }
            ],
            "cameras": [
                { "id": "main", "kind": "perspective", "fov_degrees": 36.0, "active": true, "transform": { "kind": "look_at", "eye": [0.26, 0.2, 0.28], "target": [0.0, 0.07, 0.0] } }
            ],
            "scene": { "background": { "kind": "studio" }, "grid": { "padding": 0.08, "line_spacing": 0.04 } },
            "render": { "profile": "balanced", "quality": "medium", "anti_aliasing": "fxaa", "tonemapper": "pbr_neutral" },
            "capture": { "width": 320, "height": 220 },
            "metadata": { "template": "machine_state_viewer", "purpose": "authored machine state starter" }
        }),
    )
}

pub(super) fn product_configurator(out_dir: &Path) -> Result<scena::AgentSmokeTemplateV1, String> {
    authored_template(
        "product_configurator",
        out_dir,
        json!({
            "schema": scena::SCENE_RECIPE_SCHEMA_V1,
            "colors": {
                "body_blue": "#3A7BD5",
                "accent_green": "#33A852",
                "trim_dark": "#1D2733",
                "hidden_gray": "#697386",
                "label_fg": "#FFFFFF",
                "label_bg": "#1D2733"
            },
            "geometries": [
                { "id": "body_geo", "primitive": { "kind": "box", "size": [0.16, 0.08, 0.08] } },
                { "id": "trim_geo", "primitive": { "kind": "box", "size": [0.17, 0.012, 0.09] } },
                { "id": "accessory_geo", "primitive": { "kind": "sphere", "radius": 0.025, "segments": 20, "rings": 10 } }
            ],
            "materials": [
                { "id": "body_mat", "kind": "pbr_metallic_roughness", "base_color": "body_blue", "metallic": 0.0, "roughness": 0.42 },
                { "id": "trim_mat", "kind": "unlit", "base_color": "accent_green" },
                { "id": "hidden_mat", "kind": "unlit", "base_color": "hidden_gray" }
            ],
            "nodes": [
                { "id": "body", "geometry": "body_geo", "material": "body_mat" },
                { "id": "accent_trim", "geometry": "trim_geo", "material": "trim_mat", "transform": { "kind": "trs", "translation": [0.0, 0.05, 0.0] } },
                { "id": "optional_knob", "geometry": "accessory_geo", "material": "hidden_mat", "visible": false, "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.065] } }
            ],
            "labels": [
                { "id": "choice_label", "text": "BLUE + GREEN", "color": "label_fg", "background": "label_bg", "size_px": 15.0, "transform": { "kind": "trs", "translation": [0.0, 0.12, 0.0] } }
            ],
            "cameras": [
                { "id": "main", "kind": "perspective", "fov_degrees": 34.0, "active": true, "transform": { "kind": "look_at", "eye": [0.24, 0.18, 0.24], "target": [0.0, 0.04, 0.0] } }
            ],
            "scene": { "background": { "kind": "studio" }, "environment": { "kind": "default" } },
            "render": { "profile": "balanced", "quality": "medium", "anti_aliasing": "fxaa", "tonemapper": "pbr_neutral" },
            "capture": { "width": 320, "height": 220 },
            "metadata": { "template": "product_configurator", "purpose": "authored product configurator starter" }
        }),
    )
}

fn authored_template(
    name: &str,
    out_dir: &Path,
    recipe_value: Value,
) -> Result<scena::AgentSmokeTemplateV1, String> {
    let mut builder = TemplateBuilder::ready(name, &["inspection", "scene-host"]);
    let recipe = write_authored_recipe(out_dir, recipe_value, &mut builder)?;
    add_common_commands(out_dir, &recipe, &mut builder);
    Ok(builder.finish())
}

fn write_authored_recipe(
    out_dir: &Path,
    value: Value,
    builder: &mut TemplateBuilder,
) -> Result<PathBuf, String> {
    let recipe = out_dir.join("recipe.json");
    write_json_file(&recipe, &value)?;
    builder.file("recipe", &recipe, scena::SCENE_RECIPE_SCHEMA_V1);
    Ok(recipe)
}
