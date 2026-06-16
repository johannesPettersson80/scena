use std::fs;
use std::path::Path;

use serde_json::json;

use super::builder::write_json_file;

pub(super) fn add_cad_overlay_recipe_sections(recipe: &Path) -> Result<(), String> {
    merge_recipe_sections(
        recipe,
        json!({
            "section_box": {
                "import": "primary",
                "margin": 0.01,
                "helper_wireframe": true
            },
            "measurements": [{
                "id": "plate-width",
                "kind": "distance",
                "start": [-0.06, 0.0, 0.0],
                "end": [0.06, 0.0, 0.0],
                "label": "plate width",
                "unit": "mm",
                "precision": 1
            }],
            "callouts": [{
                "id": "datum-callout",
                "text": "120 x 60 mm plate",
                "target": {
                    "kind": "import_root",
                    "import": "primary",
                    "local_offset": [0.0, 0.02, 0.0]
                },
                "label_offset": [0.06, 0.05, 0.0]
            }],
            "exploded_view": {
                "import": "primary",
                "mode": "axis",
                "axis": [1.0, 0.0, 0.0],
                "factor": 0.15,
                "distance": 0.05
            }
        }),
    )
}

pub(super) fn add_documentation_overlay_recipe_sections(recipe: &Path) -> Result<(), String> {
    merge_recipe_sections(
        recipe,
        json!({
            "section_box": {
                "import": "primary",
                "margin": 0.012,
                "helper_wireframe": true
            },
            "measurements": [{
                "id": "body-width",
                "kind": "distance",
                "start": [-0.06, -0.035, 0.0],
                "end": [0.06, -0.035, 0.0],
                "label": "body width",
                "unit": "mm",
                "precision": 1
            }],
            "callouts": [{
                "id": "service-panel",
                "text": "service panel",
                "target": {
                    "kind": "import_root",
                    "import": "primary",
                    "local_offset": [0.035, 0.02, 0.0]
                },
                "label_offset": [0.06, 0.05, 0.0]
            }],
            "exploded_view": {
                "import": "primary",
                "mode": "direct_children",
                "factor": 0.0,
                "distance": 0.0
            }
        }),
    )
}

fn merge_recipe_sections(recipe: &Path, sections: serde_json::Value) -> Result<(), String> {
    let text = fs::read_to_string(recipe)
        .map_err(|error| format!("failed to read recipe '{}': {error}", recipe.display()))?;
    let mut value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse recipe '{}': {error}", recipe.display()))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| format!("recipe '{}' is not a JSON object", recipe.display()))?;
    for (key, value) in sections
        .as_object()
        .ok_or_else(|| "recipe section patch must be an object".to_string())?
    {
        object.insert(key.clone(), value.clone());
    }
    write_json_file(recipe, &value)
}
