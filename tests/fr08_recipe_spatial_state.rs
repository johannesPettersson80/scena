#![cfg(feature = "scene-host")]

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};

fn feature_recipe() -> Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "gray": "#707070", "yellow": "#FFD040" },
        "geometries": [{
            "id": "box_geo",
            "primitive": { "kind": "box", "size": [0.24, 0.24, 0.08] }
        }],
        "materials": [{
            "id": "box_mat", "kind": "unlit", "base_color": "gray",
            "double_sided": true
        }],
        "nodes": [
            {
                "id": "source", "geometry": "box_geo", "material": "box_mat",
                "transform": { "kind": "trs", "translation": [-0.38, 0.0, 0.0] }
            },
            {
                "id": "target", "geometry": "box_geo", "material": "box_mat",
                "transform": { "kind": "trs", "translation": [0.38, 0.0, 0.0] }
            },
            { "id": "zone" }
        ],
        "anchors": [{
            "id": "source_origin",
            "source": {
                "kind": "authored",
                "target": { "kind": "node", "id": "source" },
                "transform": { "kind": "trs", "translation": [0.12, 0.0, 0.0] }
            },
            "tags": ["mount"],
            "label": "Source origin"
        }],
        "connectors": [
            {
                "id": "source_plug",
                "source": {
                    "kind": "authored",
                    "target": { "kind": "node", "id": "source" },
                    "transform": { "kind": "trs", "translation": [0.12, 0.0, 0.0] }
                },
                "connector_kind": "plug",
                "allowed_mates": ["socket"],
                "polarity": "plug",
                "mate": { "target": "target_socket" }
            },
            {
                "id": "target_socket",
                "source": {
                    "kind": "authored",
                    "target": { "kind": "node", "id": "target" },
                    "transform": { "kind": "trs", "translation": [-0.12, 0.0, 0.0] }
                },
                "connector_kind": "socket",
                "allowed_mates": ["plug"],
                "polarity": "socket"
            }
        ],
        "bounds": [
            {
                "id": "source_computed",
                "target": { "kind": "node", "id": "source" },
                "source": "computed"
            },
            {
                "id": "zone_authored",
                "target": { "kind": "node", "id": "zone" },
                "source": "authored",
                "min": [-0.1, -0.1, -0.1],
                "max": [0.1, 0.1, 0.1]
            }
        ],
        "named_states": [
            {
                "id": "base",
                "visibility": [{
                    "target": { "kind": "node", "id": "source" },
                    "visible": true
                }]
            },
            {
                "id": "inspection",
                "inherits": "base",
                "active": true,
                "tints": [{
                    "target": { "kind": "node", "id": "source" },
                    "color": "yellow"
                }]
            }
        ],
        "cameras": [{
            "id": "main", "kind": "perspective", "active": true,
            "transform": {
                "kind": "look_at", "eye": [0.0, 0.0, 1.5],
                "target": [0.0, 0.0, 0.0]
            }
        }],
        "capture": { "width": 128, "height": 96 }
    })
}

#[test]
fn fr08_recipe_spatial_sections_validate_and_round_trip_without_unsupported_fallback() {
    let recipe = feature_recipe();
    let report = scena::validate_scene_recipe_value(recipe.clone());
    assert!(report.ok, "FR08 sections validate: {report:#?}");
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "unsupported_feature"),
        "accepted FR08 sections must not retain the old unsupported fallback: {report:#?}"
    );
    let text = serde_json::to_string_pretty(&recipe).expect("FR08 recipe serializes");
    let parsed = scena::parse_valid_scene_recipe_json(&text).expect("FR08 recipe parses");
    let round_trip = serde_json::to_value(parsed).expect("FR08 recipe round-trips");
    for section in ["anchors", "connectors", "bounds", "named_states"] {
        assert_eq!(round_trip[section], recipe[section], "round-trip {section}");
    }
}

#[test]
fn fr08_build_maps_every_owner_applies_mate_and_active_state_and_changes_pixels() {
    let recipe = feature_recipe();
    let text = serde_json::to_string_pretty(&recipe).expect("FR08 recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "memory://fr08.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .unwrap_or_else(|manifest| panic!("FR08 recipe builds: {manifest:#?}"));
    let manifest = serde_json::to_value(&build.manifest).expect("manifest serializes");
    assert_eq!(manifest["anchors"][0]["id"], "source_origin");
    assert_eq!(
        manifest["anchors"][0]["identity_scope"],
        "persistent_recipe_id"
    );
    assert_eq!(manifest["connectors"].as_array().map(Vec::len), Some(2));
    assert_eq!(manifest["connections"][0]["source"], "source_plug");
    assert_eq!(manifest["connections"][0]["target"], "target_socket");
    assert_eq!(manifest["connections"][0]["status"], "applied");
    assert_eq!(manifest["bounds"].as_array().map(Vec::len), Some(2));
    assert_eq!(manifest["bounds"][0]["units"], "scene_meters");
    assert_eq!(manifest["named_states"][1]["id"], "inspection");
    assert_eq!(manifest["named_states"][1]["active"], true);
    assert_eq!(manifest["named_states"][1]["inherited_from"], "base");

    let state = build
        .host
        .visual_state("inspection")
        .expect("active named state is stored");
    assert_eq!(state.patch.tints.len(), 1);
    assert_eq!(state.patch.visibility.len(), 1);

    let mut feature_host = build.host;
    feature_host.prepare().expect("FR08 feature scene prepares");
    feature_host.render().expect("FR08 feature scene renders");
    let feature_capture = feature_host.capture().expect("FR08 feature capture");

    let mut control = recipe;
    let object = control.as_object_mut().expect("control recipe object");
    object.remove("anchors");
    object.remove("connectors");
    object.remove("bounds");
    object.remove("named_states");
    let control_text = serde_json::to_string_pretty(&control).expect("control serializes");
    let control = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "memory://fr08-control.recipe.json",
        &control_text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("FR08 control builds");
    let mut control_host = control.host;
    control_host.prepare().expect("FR08 control prepares");
    control_host.render().expect("FR08 control renders");
    let control_capture = control_host.capture().expect("FR08 control capture");
    let diff = scena::compare_captures_with_tolerance(
        &feature_capture,
        &control_capture,
        scena::ReferenceImageTolerance::exact(),
    )
    .expect_err("mating and active tint must visibly change the output");
    let scena::CaptureBaselineError::DiffExceeded(report) = diff else {
        panic!("FR08 captures must have matching dimensions")
    };
    assert!(
        report.diff.mismatched_pixels > 100,
        "rendered FR08 delta: {report:#?}"
    );
    let artifacts = PathBuf::from("target/gate-artifacts/fr08-recipe-spatial-state");
    fs::create_dir_all(&artifacts).expect("FR08 artifact directory creates");
    feature_capture
        .write_png(artifacts.join("feature.png"))
        .expect("FR08 feature PNG writes");
    control_capture
        .write_png(artifacts.join("control.png"))
        .expect("FR08 control PNG writes");
    fs::write(
        artifacts.join("proof.json"),
        serde_json::to_vec_pretty(&json!({
            "schema": "scena.fr08_recipe_spatial_state_proof.v1",
            "manifest": manifest,
            "diff": report,
        }))
        .expect("FR08 proof serializes"),
    )
    .expect("FR08 proof writes");
}

#[test]
fn fr08_import_aliases_preserve_source_metadata_and_exact_identity() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "fixture",
            "uri": "gltf/connector_basis_scene.gltf"
        }],
        "anchors": [{
            "id": "fixture_anchor",
            "source": { "kind": "import", "import": "fixture", "name": "basis-anchor" }
        }],
        "connectors": [{
            "id": "fixture_connector",
            "source": { "kind": "import", "import": "fixture", "name": "basis-connector" }
        }]
    });
    let text = serde_json::to_string_pretty(&recipe).expect("import alias recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/fr08-import-alias.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .unwrap_or_else(|manifest| panic!("FR08 import aliases build: {manifest:#?}"));
    let manifest = serde_json::to_value(build.manifest).expect("manifest serializes");
    assert_eq!(manifest["anchors"][0]["id"], "fixture_anchor");
    assert_eq!(manifest["anchors"][0]["source"], "import");
    assert_eq!(manifest["anchors"][0]["source_units"], "meters");
    assert_eq!(
        manifest["anchors"][0]["source_coordinate_system"],
        "gltf_y_up_right_handed"
    );
    assert_eq!(manifest["connectors"][0]["id"], "fixture_connector");
    assert_eq!(manifest["connectors"][0]["source"], "import");
}

#[test]
fn fr08_all_spatial_target_kinds_round_trip() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{ "id": "part", "uri": "part.gltf" }],
        "geometries": [{
            "id": "g", "primitive": { "kind": "box", "size": [1.0, 1.0, 1.0] }
        }],
        "materials": [{ "id": "m", "kind": "unlit", "base_color": "#FFFFFF" }],
        "nodes": [{ "id": "n", "geometry": "g", "material": "m" }],
        "anchors": [
            { "id": "node_anchor", "source": { "kind": "authored", "target": { "kind": "node", "id": "n" } } },
            { "id": "root_anchor", "source": { "kind": "authored", "target": { "kind": "import_root", "id": "part" } } },
            { "id": "path_anchor", "source": { "kind": "authored", "target": { "kind": "import_node", "import": "part", "path": "Root/Flange" } } }
        ]
    });
    let report = scena::validate_scene_recipe_value(recipe.clone());
    assert!(report.ok, "all target kinds validate: {report:#?}");
    let parsed: scena::SceneRecipeV1 =
        serde_json::from_value(recipe.clone()).expect("target recipe parses");
    assert_eq!(
        serde_json::to_value(parsed).expect("target recipe serializes")["anchors"],
        recipe["anchors"]
    );
}

#[test]
fn fr08_spatial_and_state_failures_are_structured_and_atomic() {
    let cases = [
        (
            "missing_target",
            json!({
                "anchors": [{
                    "id": "bad",
                    "source": {
                        "kind": "authored",
                        "target": { "kind": "node", "id": "missing" }
                    }
                }]
            }),
            "unknown_spatial_target",
        ),
        (
            "invalid_bounds",
            json!({
                "bounds": [{
                    "id": "bad_bounds",
                    "target": { "kind": "node", "id": "zone" },
                    "source": "authored",
                    "min": [1.0, 0.0, 0.0], "max": [0.0, 1.0, 1.0]
                }]
            }),
            "invalid_authored_bounds",
        ),
        (
            "state_cycle",
            json!({
                "named_states": [
                    { "id": "a", "inherits": "b" },
                    { "id": "b", "inherits": "a" }
                ]
            }),
            "state_inheritance_cycle",
        ),
        (
            "incompatible_connector",
            json!({
                "connectors": [
                    {
                        "id": "source_plug",
                        "source": {
                            "kind": "authored",
                            "target": { "kind": "node", "id": "source" }
                        },
                        "connector_kind": "plug", "allowed_mates": ["socket"],
                        "mate": { "target": "target_socket" }
                    },
                    {
                        "id": "target_socket",
                        "source": {
                            "kind": "authored",
                            "target": { "kind": "node", "id": "target" }
                        },
                        "connector_kind": "bolt"
                    }
                ]
            }),
            "connector_mate_failed",
        ),
        (
            "snap_tolerance",
            json!({
                "connectors": [
                    {
                        "id": "source_plug",
                        "source": {
                            "kind": "authored",
                            "target": { "kind": "node", "id": "source" }
                        },
                        "connector_kind": "plug", "allowed_mates": ["socket"],
                        "snap_tolerance": 0.01,
                        "mate": { "target": "target_socket" }
                    },
                    {
                        "id": "target_socket",
                        "source": {
                            "kind": "authored",
                            "target": { "kind": "node", "id": "target" }
                        },
                        "connector_kind": "socket", "allowed_mates": ["plug"]
                    }
                ]
            }),
            "connector_mate_failed",
        ),
        (
            "bounds_override",
            json!({
                "bounds": [{
                    "id": "bad_override",
                    "target": { "kind": "node", "id": "source" },
                    "source": "authored",
                    "min": [-1.0, -1.0, -1.0], "max": [1.0, 1.0, 1.0]
                }]
            }),
            "authored_bounds_override",
        ),
        (
            "animated_state_transform",
            json!({
                "animations": [{
                    "id": "move", "duration": 1.0,
                    "channels": [{
                        "target": { "kind": "node", "id": "source" },
                        "path": "translation", "times": [0.0, 1.0],
                        "values": [[0.0, 0.0, 0.0], [0.2, 0.0, 0.0]]
                    }]
                }],
                "named_states": [{
                    "id": "conflict",
                    "transforms": [{
                        "target": { "kind": "node", "id": "source" },
                        "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] }
                    }]
                }]
            }),
            "animated_state_transform_conflict",
        ),
    ];

    for (name, mutation, expected_code) in cases {
        let mut recipe = feature_recipe();
        for (key, value) in mutation.as_object().expect("mutation object") {
            recipe
                .as_object_mut()
                .expect("recipe object")
                .insert(key.clone(), value.clone());
        }
        let text = serde_json::to_string_pretty(&recipe).expect("bad recipe serializes");
        let error = pollster::block_on(scena::SceneHostCore::build_recipe_json(
            format!("memory://fr08-{name}.recipe.json"),
            &text,
            scena::RecipeBuildPolicy::testing(),
        ))
        .expect_err("known-bad FR08 recipe fails");
        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == expected_code),
            "{name} must emit {expected_code}: {error:#?}"
        );
    }
}
