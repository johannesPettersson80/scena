#![cfg(feature = "scene-host")]

use scena::{SceneRecipeTransformV1, Transform};
use serde_json::json;

#[test]
fn import_and_node_trs_use_one_tagged_shape_and_intrinsic_xyz_composition() {
    let source = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "part.glb",
            "transform": {
                "kind": "trs",
                "translation": [1.0, 2.0, 3.0],
                "rotation_degrees": [10.0, 20.0, 30.0],
                "scale": [2.0, 3.0, 4.0]
            }
        }],
        "nodes": [{
            "id": "pivot",
            "transform": {
                "kind": "trs",
                "translation": [1.0, 2.0, 3.0],
                "rotation_degrees": [10.0, 20.0, 30.0],
                "scale": [2.0, 3.0, 4.0]
            }
        }]
    });
    let recipe = scena::parse_valid_scene_recipe_json(&source.to_string())
        .expect("canonical import and node transform grammar validates");
    let import_transform = recipe.imports[0]
        .transform
        .as_ref()
        .expect("import transform parses");
    let node_transform = recipe.nodes[0]
        .transform
        .as_ref()
        .expect("node transform parses");
    assert_eq!(import_transform, node_transform);

    let resolved = Transform::try_from(import_transform).expect("TRS resolves");
    let expected = Transform::IDENTITY
        .with_translation(scena::Vec3::new(1.0, 2.0, 3.0))
        .rotate_x_deg(10.0)
        .rotate_y_deg(20.0)
        .rotate_z_deg(30.0)
        .with_scale(scena::Vec3::new(2.0, 3.0, 4.0));
    assert_transform_close(resolved, expected);

    let emitted = serde_json::to_value(&recipe).expect("recipe serializes");
    assert_eq!(emitted["imports"][0]["transform"]["kind"], "trs");
    assert_eq!(emitted["nodes"][0]["transform"]["kind"], "trs");
}

#[test]
fn legacy_import_raw_shape_is_an_explicit_warning_alias_and_serializes_canonically() {
    let source = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "part.glb",
            "transform": {
                "translation": [1.0, 2.0, 3.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            }
        }]
    });
    let report = scena::validate_scene_recipe_json(&source.to_string());
    assert!(report.ok, "legacy alias remains readable: {report:#?}");
    let migration = report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "legacy_transform_shape")
        .expect("legacy import transform reports migration warning");
    assert_eq!(migration.severity, "warning");
    assert_eq!(migration.path, "$.imports[0].transform");
    assert!(migration.auto_fixable);
    assert!(
        migration
            .suggestion
            .as_deref()
            .is_some_and(|suggestion| suggestion.contains("\"kind\":\"raw\""))
    );

    let recipe = scena::parse_valid_scene_recipe_json(&source.to_string())
        .expect("legacy alias deserializes after warning");
    assert!(matches!(
        recipe.imports[0].transform,
        Some(SceneRecipeTransformV1::Raw { .. })
    ));
    let emitted = serde_json::to_value(recipe).expect("canonical recipe serializes");
    assert_eq!(emitted["imports"][0]["transform"]["kind"], "raw");
}

#[test]
fn explicit_kind_wins_and_invalid_canonical_fields_do_not_fall_back_to_legacy() {
    let source = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "part.glb",
            "transform": {
                "kind": "trs",
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            }
        }]
    });
    let report = scena::validate_scene_recipe_json(&source.to_string());
    assert!(!report.ok);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unknown_field" && diagnostic.path == "$.imports[0].transform.rotation"
    }));
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "legacy_transform_shape")
    );
}

#[test]
fn canonical_import_raw_transform_rejects_a_zero_quaternion_before_build() {
    let source = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "part.glb",
            "transform": {
                "kind": "raw",
                "rotation": [0.0, 0.0, 0.0, 0.0]
            }
        }]
    });
    let report = scena::validate_scene_recipe_json(&source.to_string());
    assert!(!report.ok);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_rotation"
            && diagnostic.path == "$.imports[0].transform.rotation"
    }));
}

#[test]
fn placement_results_emit_canonical_raw_and_migrate_the_legacy_v1_shape() {
    let result = scena::ScenePlacementResultV1::success(
        "part",
        "center",
        Transform::IDENTITY.with_translation(scena::Vec3::new(1.0, 2.0, 3.0)),
    );
    let emitted = serde_json::to_value(&result).expect("placement result serializes");
    assert_eq!(emitted["transform"]["kind"], "raw");
    assert!(
        emitted["transform"].get("scale").is_none(),
        "placement output uses the canonical raw default omission"
    );

    let legacy = json!({
        "schema": "scena.placement_result.v1",
        "ok": true,
        "verb": "center",
        "import_id": "part",
        "transform": {
            "translation": [1.0, 2.0, 3.0],
            "rotation": [0.0, 0.0, 0.0, 1.0],
            "scale": [1.0, 1.0, 1.0]
        },
        "diagnostics": []
    });
    let migrated: scena::ScenePlacementResultV1 =
        serde_json::from_value(legacy).expect("pre-discriminator placement v1 remains readable");
    let canonical = serde_json::to_value(migrated).expect("migrated placement result serializes");
    assert_eq!(canonical["transform"]["kind"], "raw");
}

fn assert_transform_close(actual: Transform, expected: Transform) {
    assert!((actual.translation - expected.translation).length() <= 1.0e-5);
    assert!((actual.scale - expected.scale).length() <= 1.0e-5);
    assert!(actual.rotation.dot(expected.rotation).abs() >= 1.0 - 1.0e-5);
}
