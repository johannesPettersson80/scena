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
        "colors",
        "geometries",
        "primitives",
        "materials",
        "nodes",
        "cameras",
        "lights",
        "scene",
        "render",
        "expect",
        "animations",
        "fonts",
        "skins",
        "morphs",
        "particles",
        "labels",
        "viewer_profile",
        "environment",
        "placements",
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

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_builds_import_manifest_with_stable_handles() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" }
        ],
        "capture": { "width": 160, "height": 120 }
    }))
    .expect("recipe serializes");
    let policy = scena::RecipeBuildPolicy::testing();

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/recipe-import-manifest.recipe.json",
        &recipe,
        policy,
    ))
    .expect("recipe build succeeds");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert_eq!(build.manifest.schema, scena::SCENE_RECIPE_BUILD_SCHEMA_V1);
    assert_eq!(build.manifest.imports.len(), 1);
    let import = &build.manifest.imports[0];
    assert_eq!(import.id, "part");
    assert!(import.import_handle > 0);
    assert!(!import.root_handles.is_empty());
    assert_eq!(import.primary_root, import.root_handles.first().copied());
    assert!(
        import.nodes_by_path.contains_key("part:/"),
        "root path should be addressable through the shared node id namespace: {import:#?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_policy_rejects_unsafe_or_oversized_inputs() {
    let base_recipe = |uri: &str| {
        serde_json::to_string_pretty(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "part", "uri": uri }
            ],
            "capture": { "width": 160, "height": 120 }
        }))
        .expect("recipe serializes")
    };

    let oversized_capture = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" }
        ],
        "capture": { "width": 160, "height": 120 }
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &oversized_capture,
        scena::RecipeBuildPolicy::testing().with_max_output_pixels(1),
    ))
    .expect_err("oversized capture fails closed");
    assert_build_reason(&report, "policy_violation", "$.capture");

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &base_recipe("https://example.invalid/model.gltf"),
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect_err("network uri fails closed by default");
    assert_build_reason(&report, "policy_violation", "$.imports[0].uri");

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &base_recipe("memory://model.gltf"),
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect_err("disallowed scheme fails closed");
    assert_build_reason(&report, "policy_violation", "$.imports[0].uri");

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &base_recipe("Cargo.toml"),
        scena::RecipeBuildPolicy::testing()
            .with_allowed_roots([PathBuf::from("tests/assets/gltf")]),
    ))
    .expect_err("out-of-root local file fails closed");
    assert_build_reason(&report, "policy_violation", "$.imports[0].uri");

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &base_recipe("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
        scena::RecipeBuildPolicy::testing()
            .with_max_vertices(1)
            .with_max_indices(1),
    ))
    .expect_err("oversized geometry fails closed");
    assert_build_reason(&report, "policy_violation", "$.imports[0]");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_skips_only_explicit_optional_imports() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "missing", "uri": "tests/assets/gltf/not-present.gltf", "optional": true }
        ],
        "capture": { "width": 160, "height": 120 }
    }))
    .expect("recipe serializes");

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("optional missing import does not fail the build");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert!(build.manifest.imports.is_empty());
    assert_eq!(build.manifest.skipped.len(), 1);
    assert_eq!(build.manifest.skipped[0].id, "missing");
    assert_build_reason(&build.manifest, "optional_import_skipped", "$.imports[0]");
}

#[test]
fn scene_recipe_validation_accepts_overlay_authoring_sections() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "plate", "uri": "tests/assets/gltf/cad_plate_drawing_scene.gltf" }
        ],
        "section_box": {
            "import": "plate",
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
            "id": "datum",
            "text": "120 x 60 mm plate",
            "target": {
                "kind": "import_root",
                "import": "plate",
                "local_offset": [0.0, 0.02, 0.0]
            },
            "label_offset": [0.06, 0.05, 0.0]
        }],
        "exploded_view": {
            "import": "plate",
            "mode": "axis",
            "axis": [1.0, 0.0, 0.0],
            "factor": 0.2,
            "distance": 0.05
        }
    });

    let report = scena::validate_scene_recipe_value(recipe.clone());
    assert!(report.ok, "overlay recipe should validate: {report:#?}");

    let text = serde_json::to_string(&recipe).expect("recipe serializes");
    let parsed = scena::parse_valid_scene_recipe_json(&text).expect("overlay recipe parses");
    assert_eq!(
        parsed.section_box.as_ref().expect("section box").import,
        "plate"
    );
    assert_eq!(parsed.measurements.len(), 1);
    assert_eq!(parsed.callouts.len(), 1);
    assert_eq!(
        parsed.exploded_view.as_ref().expect("exploded").import,
        "plate"
    );
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

#[cfg(feature = "scene-host")]
fn assert_build_reason(report: &scena::SceneRecipeBuildV1, code: &str, path: &str) {
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.path == path),
        "expected build diagnostic {code} at {path}, got {report:#?}"
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
