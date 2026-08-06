use std::collections::BTreeSet;

use serde_json::Value;

use super::{diagnostic, validate_import_material};
use crate::scene::recipe::types::SceneRecipeDiagnosticV1;

pub(in crate::scene::recipe::validation::imports) fn validate_import_material_bindings(
    path: String,
    bindings: &Value,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(bindings) = bindings.as_array() else {
        diagnostics.push(diagnostic(
            "invalid_import_material_bindings",
            "error",
            &path,
            "material_bindings must be an array",
            "use material_bindings:[{source_material:{index:0,name:\"steel\"},material:{material_pack:{uri:\"materials/steel/scena-material-pack.json\"}}}]",
            None,
            false,
        ));
        return;
    };
    if bindings.is_empty() {
        diagnostics.push(diagnostic(
            "invalid_import_material_bindings",
            "error",
            &path,
            "material_bindings must contain at least one binding",
            "remove material_bindings or add a source material binding",
            None,
            false,
        ));
        return;
    }

    let mut source_indices = BTreeSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        validate_material_binding(
            &format!("{path}[{index}]"),
            binding,
            &mut source_indices,
            diagnostics,
        );
    }
}

fn validate_material_binding(
    path: &str,
    binding: &Value,
    source_indices: &mut BTreeSet<u64>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(binding) = binding.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_import_material_binding",
            "error",
            path,
            "each material binding must be an object",
            "use {source_material:{index:0,name:\"steel\"},material:{material_pack:{uri:\"materials/steel/scena-material-pack.json\"}}}",
            None,
            false,
        ));
        return;
    };
    for key in binding.keys() {
        if !["source_material", "material"].contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!("material binding field '{key}' is not part of scena.scene_recipe.v1"),
                "use source_material and material",
                None,
                false,
            ));
        }
    }
    let Some(source) = binding.get("source_material") else {
        diagnostics.push(diagnostic(
            "invalid_source_material_selector",
            "error",
            format!("{path}.source_material"),
            "material binding must identify one source material",
            "use source_material:{index:0,name:\"steel\"}",
            None,
            false,
        ));
        return;
    };
    validate_source_material_selector(
        &format!("{path}.source_material"),
        source,
        source_indices,
        diagnostics,
    );
    match binding.get("material") {
        Some(material) => {
            validate_import_material(format!("{path}.material"), material, diagnostics);
        }
        None => diagnostics.push(diagnostic(
            "invalid_import_material_binding",
            "error",
            format!("{path}.material"),
            "material binding must include a replacement material",
            "add material:{material_pack:{uri:\"materials/steel/scena-material-pack.json\"}}",
            None,
            false,
        )),
    }
}

fn validate_source_material_selector(
    path: &str,
    source: &Value,
    source_indices: &mut BTreeSet<u64>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(source) = source.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_source_material_selector",
            "error",
            path,
            "source_material must be an object",
            "use source_material:{index:0,name:\"steel\"}",
            None,
            false,
        ));
        return;
    };
    for key in source.keys() {
        if !["index", "name"].contains(&key.as_str()) {
            diagnostics.push(diagnostic(
                "unknown_field",
                "error",
                format!("{path}.{key}"),
                format!(
                    "source material selector field '{key}' is not part of scena.scene_recipe.v1"
                ),
                "use index and optional name",
                None,
                false,
            ));
        }
    }
    match source.get("index").and_then(Value::as_u64) {
        Some(index) if usize::try_from(index).is_err() => diagnostics.push(diagnostic(
            "invalid_source_material_selector",
            "error",
            format!("{path}.index"),
            "source material index is too large for this platform",
            "copy material_index from `scena inspect`",
            None,
            false,
        )),
        Some(index) if !source_indices.insert(index) => diagnostics.push(diagnostic(
            "duplicate_source_material_binding",
            "error",
            format!("{path}.index"),
            format!("source material index {index} is bound more than once"),
            "keep exactly one replacement for each source material index",
            None,
            false,
        )),
        Some(_) => {}
        None => diagnostics.push(diagnostic(
            "invalid_source_material_selector",
            "error",
            format!("{path}.index"),
            "source material selector requires a non-negative integer index",
            "copy material_index from `scena inspect`",
            None,
            false,
        )),
    }
    if let Some(name) = source.get("name") {
        match name.as_str() {
            Some(name) if !name.trim().is_empty() => {}
            _ => diagnostics.push(diagnostic(
                "invalid_source_material_selector",
                "error",
                format!("{path}.name"),
                "source material name must be a non-empty string",
                "copy material_name from `scena inspect`, or omit name for unnamed source materials",
                None,
                false,
            )),
        }
    }
}
