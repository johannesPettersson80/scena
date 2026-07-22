#![cfg(all(not(target_arch = "wasm32"), feature = "scene-host"))]

use std::process::Command;

use scena::{AnimationError, Assets, LookupError, RenderError, Scene};
use serde_json::json;

#[test]
fn normalized_name_candidates_are_deterministic_ranked_and_capped() {
    let candidates = scena::nearest_name_candidates(
        "main_pmp",
        ["Aux Valve", "main-pump-sensor", "Main Pump", "Main Pump"],
        2,
    );
    assert_eq!(candidates, vec!["Main Pump", "main-pump-sensor"]);
}

#[test]
fn import_lookup_errors_carry_node_anchor_connector_clip_and_variant_candidates() {
    let assets = Assets::new();

    let anchor_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/anchor_debug_scene.gltf"))
            .expect("anchor fixture loads");
    let mut scene = Scene::new();
    let anchor_import = scene
        .instantiate(&anchor_asset)
        .expect("anchor fixture instantiates");
    match anchor_import.node("Rto") {
        Err(LookupError::NodeNameNotFound { name, candidates }) => {
            assert_eq!(name, "Rto");
            assert_eq!(candidates.first().map(String::as_str), Some("Root"));
        }
        other => panic!("expected node candidates, got {other:?}"),
    }
    match anchor_import.anchor("inspectoin") {
        Err(LookupError::AnchorNotFound { candidates, .. }) => {
            assert_eq!(candidates.first().map(String::as_str), Some("inspection"));
        }
        other => panic!("expected anchor candidates, got {other:?}"),
    }

    let connector_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/connector_basis_scene.gltf"))
            .expect("connector fixture loads");
    let connector_import = scene
        .instantiate(&connector_asset)
        .expect("connector fixture instantiates");
    match connector_import.connector("basis-conector") {
        Err(LookupError::ConnectorNotFound { candidates, .. }) => {
            assert_eq!(candidates, vec!["basis-connector"]);
        }
        other => panic!("expected connector candidates, got {other:?}"),
    }

    let animated_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/z_up_animated_rotation.gltf"))
            .expect("animation fixture loads");
    let animated_import = scene
        .instantiate(&animated_asset)
        .expect("animation fixture instantiates");
    match animated_import.clip("LinerZ") {
        Err(LookupError::ClipNotFound { candidates, .. }) => {
            assert_eq!(candidates.first().map(String::as_str), Some("LinearZ"));
        }
        other => panic!("expected clip candidates, got {other:?}"),
    }
    match scene.create_animation_mixer(&animated_import, "LinerZ") {
        Err(AnimationError::ClipNotFound { candidates, .. }) => {
            assert_eq!(candidates.first().map(String::as_str), Some("LinearZ"));
        }
        other => panic!("expected animation candidates, got {other:?}"),
    }

    let variant_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/material_variants_scene.gltf"))
            .expect("variant fixture loads");
    let variant_import = scene
        .instantiate(&variant_asset)
        .expect("variant fixture instantiates");
    match scene.set_active_variant(&variant_import, Some("midnigth")) {
        Err(LookupError::VariantNotFound { candidates, .. }) => {
            assert_eq!(candidates.first().map(String::as_str), Some("midnight"));
        }
        other => panic!("expected variant candidates, got {other:?}"),
    }
}

#[test]
fn recipe_lookup_diagnostics_carry_node_geometry_material_and_preset_candidates() {
    let report = scena::validate_scene_recipe_json(
        &serde_json::to_string(&json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{"id": "pump-geometry", "kind": "box"}],
            "materials": [{"id": "pump-material", "kind": "unlit"}],
            "nodes": [
                {"id": "pump-node", "geometry": "pmp-geometry", "material": "pmp-material"},
                {"id": "child", "parent": "pmp-node"}
            ],
            "scene": {"environment": {"preset": "studoi"}}
        }))
        .expect("recipe serializes"),
    );
    assert!(!report.ok);
    for (code, candidate) in [
        ("unknown_geometry_ref", "pump-geometry"),
        ("unknown_material_ref", "pump-material"),
        ("unknown_node_ref", "pump-node"),
        ("invalid_environment", "studio"),
    ] {
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("missing {code}: {report:#?}"));
        assert!(
            diagnostic.candidates.iter().any(|item| item == candidate),
            "{code} should suggest {candidate}: {diagnostic:#?}"
        );
    }
}

#[test]
fn cli_schema_and_template_lookup_errors_expose_structured_candidates() {
    let schema = run_scena(&["schema", "get", "scena.render_introspect.v1"]);
    assert_eq!(schema.status.code(), Some(2));
    let schema: serde_json::Value =
        serde_json::from_slice(&schema.stderr).expect("schema error is JSON");
    assert_eq!(schema["candidates"][0], "scena.render_introspection.v1");

    let template = run_scena(&["examples", "agent", "get", "primitive-scne"]);
    assert_eq!(template.status.code(), Some(2));
    let template: serde_json::Value =
        serde_json::from_slice(&template.stderr).expect("template error is JSON");
    assert_eq!(template["candidates"][0], "primitive-scene");
}

#[test]
fn no_active_camera_display_and_host_conversion_keep_the_remedy() {
    let message = RenderError::NoActiveCamera.to_string();
    assert!(message.contains("add_default_camera"));
    assert!(message.contains("set_active_camera"));
    let host = scena::SceneHostError::from(RenderError::NoActiveCamera);
    assert_eq!(host.code(), scena::SceneHostErrorCode::NoActiveCamera);
    assert!(host.message().contains("add_default_camera"));
    assert!(host.message().contains("set_active_camera"));
    let json = serde_json::to_value(&host).expect("host error serializes");
    assert_eq!(json["code"], "no_active_camera");
    assert!(
        json["message"]
            .as_str()
            .is_some_and(|message| message.contains("add_default_camera"))
    );
}

fn run_scena(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .expect("scena command runs")
}
