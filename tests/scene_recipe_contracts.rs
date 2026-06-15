use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

#[test]
fn scene_recipe_golden_fixture_validates_and_round_trips() {
    let text = fs::read_to_string("tests/assets/stable-contracts/scene_recipe.v1.json")
        .expect("scene recipe fixture reads");
    let recipe: scena::SceneRecipeV1 =
        serde_json::from_str(&text).expect("scene recipe fixture deserializes");

    assert_eq!(recipe.schema, scena::SCENE_RECIPE_SCHEMA_V1);
    let report = scena::validate_scene_recipe_json(&text);
    assert!(report.ok, "fixture must validate cleanly: {report:#?}");
    assert_eq!(
        serde_json::to_value(&recipe).expect("recipe serializes"),
        serde_json::from_str::<serde_json::Value>(&text).expect("fixture parses")
    );
}

#[test]
fn scene_recipe_validation_reports_unknown_fields_duplicate_ids_and_suggestions() {
    let unknown = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "importe": []
    }));
    assert!(!unknown.ok);
    assert_reason(&unknown, "unknown_field", Some("imports"));

    let duplicate = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" },
            { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" }
        ]
    }));
    assert!(!duplicate.ok);
    assert_reason(&duplicate, "duplicate_id", None);

    let workflow = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" }
        ],
        "steps": []
    }));
    assert!(!workflow.ok);
    assert_reason(&workflow, "unsupported_workflow", None);
}

#[test]
fn scene_recipe_validation_reports_future_sections_as_unsupported_features() {
    for section in [
        "primitives",
        "materials",
        "cameras",
        "lights",
        "labels",
        "viewer_profile",
        "environment",
        "placements",
        "section_box",
        "measurements",
        "callouts",
        "exploded_view",
        "named_states",
        "anchors",
        "connectors",
        "bounds",
        "authored_planes",
    ] {
        let mut recipe = json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" }
            ]
        });
        recipe
            .as_object_mut()
            .expect("recipe is an object")
            .insert(section.to_owned(), json!({}));
        let report = scena::validate_scene_recipe_value(recipe);
        assert_reason(&report, "unsupported_feature", None);
    }
}

#[test]
fn scene_recipe_validation_rejects_nested_shapes_before_parse() {
    let bad_transform = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            {
                "id": "part",
                "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
                "transform": "not-a-transform"
            }
        ]
    }));
    assert!(!bad_transform.ok);
    assert_reason(&bad_transform, "invalid_transform", None);

    let bad_metadata = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" }
        ],
        "metadata": "not-an-object"
    }));
    assert!(!bad_metadata.ok);
    assert_reason(&bad_metadata, "invalid_metadata", None);
}

#[test]
fn scena_validate_recipe_cli_emits_json_and_nonzero_for_invalid_recipe() {
    let dir = artifact_dir("validate-invalid");
    let recipe_path = dir.join("invalid.recipe.json");
    fs::write(
        &recipe_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "importe": []
        }))
        .expect("invalid recipe serializes"),
    )
    .expect("invalid recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(["validate-recipe", path_str(&recipe_path)])
        .output()
        .expect("scena validate-recipe command runs");

    assert!(!output.status.success());
    assert!(
        output.stderr.is_empty(),
        "validation diagnostics stay machine-readable on stdout, stderr={}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("validate-recipe emits JSON");
    assert_eq!(report["schema"], "scena.scene_recipe_validation.v1");
    assert_eq!(report["ok"], false);
    assert!(
        report["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "unknown_field"
                && diagnostic["suggestion"] == "imports"),
        "invalid recipe should carry did-you-mean diagnostics: {report:#}"
    );
}

fn assert_reason(
    report: &scena::SceneRecipeValidationReportV1,
    code: &str,
    suggestion: Option<&str>,
) {
    assert!(
        report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == code
                && suggestion
                    .is_none_or(|expected| diagnostic.suggestion.as_deref() == Some(expected))
        }),
        "missing diagnostic {code}/{suggestion:?}: {report:#?}",
    );
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("target")
        .join("gate-artifacts")
        .join(format!("scena-recipe-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("artifact directory creates");
    dir
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path is valid UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
