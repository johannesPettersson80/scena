#[cfg(feature = "scene-host")]
use std::collections::BTreeMap;
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
        "viewer_profile",
        "environment",
        "placements",
        "named_states",
        "anchors",
        "connectors",
        "bounds",
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
fn scene_recipe_validation_accepts_authored_animation_and_rejects_bad_channels() {
    let valid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "blue": "#3A7BD5" },
        "geometries": [
            { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "cube_mat", "kind": "unlit", "base_color": "blue" }
        ],
        "nodes": [
            { "id": "cube", "geometry": "cube_geo", "material": "cube_mat" }
        ],
        "animations": [{
            "id": "move_cube",
            "duration": 1.0,
            "channels": [{
                "target": { "kind": "node", "id": "cube" },
                "path": "translation",
                "times": [0.0, 1.0],
                "values": [[0.0, 0.0, 0.0], [0.15, 0.0, 0.0]]
            }]
        }]
    }));
    assert!(
        valid.ok,
        "authored animation recipe should validate: {valid:#?}"
    );

    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "blue": "#3A7BD5" },
        "geometries": [
            { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "cube_mat", "kind": "unlit", "base_color": "blue" }
        ],
        "nodes": [
            { "id": "cube", "geometry": "cube_geo", "material": "cube_mat" }
        ],
        "animations": [{
            "id": "bad_clip",
            "duration": 1.0,
            "channels": [{
                "target": { "kind": "node", "id": "missing" },
                "path": "translation",
                "times": [0.0, 0.5, 0.5],
                "values": [[0.0, 0.0, 0.0]]
            }, {
                "target": { "kind": "node", "id": "cube" },
                "path": "scale",
                "times": [0.0, 3.5e38],
                "values": [[1.0, 1.0, 1.0], [1.2, 1.2, 1.2]]
            }, {
                "target": { "kind": "node", "id": "cube" },
                "path": "weights",
                "times": [0.0, 1.0],
                "values": [[0.0], [1.0]]
            }]
        }]
    }));
    assert!(!invalid.ok);
    assert_reason(&invalid, "unknown_animation_target", None);
    assert_reason(&invalid, "invalid_animation_time", None);
    assert_reason(&invalid, "invalid_animation_times", None);
    assert_reason(&invalid, "invalid_animation_values", None);
    assert_reason(&invalid, "invalid_animation_target", None);
}

#[test]
fn scene_recipe_validation_accepts_expect_and_rejects_malformed_expectations() {
    let valid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "red": "#DC2020"
        },
        "geometries": [
            { "id": "plate_geo", "primitive": { "kind": "box", "size": [0.2, 0.2, 0.02] } }
        ],
        "materials": [
            { "id": "plate_mat", "kind": "unlit", "base_color": "red" }
        ],
        "nodes": [
            { "id": "plate", "geometry": "plate_geo", "material": "plate_mat" }
        ],
        "expect": {
            "expect_color": [{
                "id": "plate-red",
                "target": { "kind": "node", "id": "plate" },
                "swatch_srgb8": [220, 32, 32],
                "tolerance": 0.2
            }],
            "expect_bbox_fit": { "min": 0.1, "max": 0.9 },
            "expect_pick": [{
                "id": "pick-plate",
                "x_css_px": 32.0,
                "y_css_px": 32.0,
                "target": { "kind": "node", "id": "plate" }
            }]
        }
    }));
    assert!(valid.ok, "expect is a landed root field: {valid:#?}");

    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "red": "#DC2020"
        },
        "geometries": [
            { "id": "plate_geo", "primitive": { "kind": "box", "size": [0.2, 0.2, 0.02] } }
        ],
        "materials": [
            { "id": "plate_mat", "kind": "unlit", "base_color": "red" }
        ],
        "nodes": [
            { "id": "plate", "geometry": "plate_geo", "material": "plate_mat" }
        ],
        "expect": {
            "expect_color": [{
                "id": "bad-swatch",
                "target": { "kind": "node", "id": "plate" },
                "swatch_srgb8": [999, 0],
                "tolerance": -1.0
            }],
            "expect_pick": [{
                "id": "bad-pick",
                "x_css_px": "left",
                "y_css_px": 32.0,
                "target": { "kind": "world", "position": [0.0, 0.0, 0.0] }
            }]
        }
    }));
    assert!(!invalid.ok);
    assert_reason(&invalid, "invalid_expect", None);
    assert_reason(&invalid, "unsupported_feature", None);
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
fn scene_recipe_build_manifest_golden_matches_executor_for_stable_recipe() {
    let recipe = fs::read_to_string("tests/assets/stable-contracts/scene_recipe.v1.json")
        .expect("scene recipe fixture reads");
    let expected: scena::SceneRecipeBuildV1 = serde_json::from_str(include_str!(
        "assets/stable-contracts/scene_recipe_build.v1.json"
    ))
    .expect("scene recipe build fixture parses");

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "scene_recipe.v1.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("stable scene recipe build succeeds");

    assert_eq!(
        build.manifest, expected,
        "stable build manifest fixture must be produced by the real executor"
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

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_authored_only_builds_manifest_and_renders_through_cli() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "plate_blue": "#3A7BD5"
        },
        "geometries": [
            {
                "id": "plate_geo",
                "primitive": {
                    "kind": "box",
                    "size": [0.12, 0.06, 0.004]
                }
            }
        ],
        "materials": [
            {
                "id": "plate_mat",
                "kind": "unlit",
                "base_color": "plate_blue"
            }
        ],
        "nodes": [
            {
                "id": "plate",
                "geometry": "plate_geo",
                "material": "plate_mat",
                "name": "CAD plate"
            }
        ],
        "cameras": [
            {
                "id": "main",
                "kind": "perspective",
                "fov_degrees": 40.0,
                "active": true,
                "transform": {
                    "kind": "look_at",
                    "eye": [0.2, 0.15, 0.2],
                    "target": "plate"
                }
            }
        ],
        "capture": { "width": 320, "height": 220 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "authored-only recipe should validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "target/gate-artifacts/authored-only.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("authored-only recipe build succeeds");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert!(build.manifest.imports.is_empty());
    assert_eq!(build.manifest.geometries.len(), 1);
    assert_eq!(build.manifest.materials.len(), 1);
    assert_eq!(build.manifest.nodes.len(), 1);
    assert_eq!(build.manifest.nodes[0].id, "plate");
    assert_eq!(build.manifest.cameras.len(), 1);
    assert_eq!(build.manifest.cameras[0].id, "main");

    let mut host = build.host;
    host.prepare().expect("authored scene prepares");
    host.render().expect("authored scene renders");
    let capture = host.capture().expect("authored scene captures");
    let inspection_json = host.inspect_json().expect("authored scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(report.ok, "authored render should be visible: {report:#?}");
    assert!(
        !report.framing.tiny_in_frame,
        "authored render should frame the target at useful scale: {report:#?}"
    );

    let dir = artifact_dir("authored-only-render");
    let recipe_path = dir.join("authored-only.recipe.json");
    let png_path = dir.join("authored-only.png");
    fs::write(&recipe_path, text).expect("recipe writes");
    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "render",
            path_str(&recipe_path),
            "--introspect",
            "--out",
            path_str(&png_path),
        ])
        .output()
        .expect("scena render command runs");

    assert!(
        output.status.success(),
        "authored recipe render should exit 0, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        stderr(&output)
    );
    let cli_report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("render emits JSON");
    assert_eq!(cli_report["schema"], "scena.render_introspection.v1");
    assert_eq!(cli_report["ok"], true);
    assert!(png_path.exists(), "render writes the authored-scene PNG");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice2_authoring_vocabulary_builds_and_targets_overlays() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "base": "#345E8A",
            "accent": "#E9B44C",
            "line": "#F7F7F2",
            "tint": "#FFFFFF"
        },
        "geometries": [
            { "id": "floor_geo", "primitive": { "kind": "plane", "size": [0.6, 0.4] } },
            { "id": "rail_geo", "primitive": { "kind": "polyline", "points": [[-0.25, 0.02, -0.15], [0.0, 0.08, 0.0], [0.25, 0.02, 0.15]] } },
            { "id": "axis_geo", "primitive": { "kind": "axes", "length": 0.12 } },
            {
                "id": "flag_geo",
                "mesh": {
                    "topology": "triangles",
                    "positions": [[0.0, 0.04, 0.0], [0.12, 0.04, 0.0], [0.0, 0.16, 0.0]],
                    "normals": [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
                    "indices": [0, 1, 2],
                    "colors": ["accent", "base", "line"],
                    "uvs": [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]
                }
            }
        ],
        "materials": [
            {
                "id": "floor_mat",
                "kind": "pbr_metallic_roughness",
                "base_color": "base",
                "metallic": 0.05,
                "roughness": 0.7,
                "double_sided": true,
                "emissive": "accent",
                "emissive_strength": 0.15,
                "alpha_mode": { "kind": "opaque" },
                "base_color_texture": {
                    "uri": "gltf/khronos/WaterBottle/WaterBottle_baseColor.png",
                    "color_space": "srgb"
                }
            },
            { "id": "line_mat", "kind": "line", "base_color": "line", "stroke_width_px": 3.0 },
            { "id": "flag_mat", "kind": "unlit", "base_color": "accent", "double_sided": true }
        ],
        "nodes": [
            {
                "id": "floor",
                "geometry": "floor_geo",
                "material": "floor_mat",
                "name": "inspection floor",
                "tags": ["cad", "authored"],
                "layer_mask": 3,
                "render_group": 2,
                "tint": "tint"
            },
            {
                "id": "rail",
                "geometry": "rail_geo",
                "material": "line_mat",
                "parent": "floor",
                "visible": true
            },
            {
                "id": "axes",
                "geometry": "axis_geo",
                "material": "line_mat",
                "parent": "floor"
            },
            {
                "id": "flag",
                "geometry": "flag_geo",
                "material": "flag_mat",
                "parent": "floor"
            }
        ],
        "lights": [
            {
                "id": "key",
                "kind": "directional",
                "preset": "key",
                "illuminance_lux": 9000.0,
                "color": "line",
                "transform": { "kind": "trs", "rotation_degrees": [-35.0, 25.0, 0.0] }
            }
        ],
        "section_box": {
            "target": { "kind": "node", "id": "floor" },
            "margin": 0.02,
            "helper_wireframe": true
        },
        "callouts": [{
            "id": "floor-label",
            "text": "Authored inspection floor",
            "target": { "kind": "node", "id": "floor", "local_offset": [0.0, 0.04, 0.0] },
            "label_offset": [0.08, 0.06, 0.0]
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "fov_degrees": 38.0,
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.65, 0.45, 0.55], "target": "floor" }
        }],
        "capture": { "width": 320, "height": 220 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 2 authored vocabulary should validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice2.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 2 authored recipe build succeeds");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert_eq!(build.manifest.geometries.len(), 4);
    assert_eq!(build.manifest.materials.len(), 3);
    assert_eq!(build.manifest.nodes.len(), 4);
    assert_eq!(build.manifest.lights.len(), 1);
    assert!(build.manifest.nodes.iter().any(|node| node.id == "floor"));
    assert!(
        build
            .manifest
            .geometries
            .iter()
            .any(|geometry| geometry.id == "flag_geo" && geometry.vertex_count == Some(3))
    );

    let mut host = build.host;
    host.prepare().expect("Slice 2 authored scene prepares");
    host.render().expect("Slice 2 authored scene renders");
    let capture = host.capture().expect("Slice 2 authored scene captures");
    let inspection_json = host
        .inspect_json()
        .expect("Slice 2 authored scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(report.ok, "Slice 2 render should be visible: {report:#?}");
    assert!(
        inspection
            .nodes
            .iter()
            .any(|node| node.tags.contains(&"cad".to_owned())
                && node.tags.contains(&"authored".to_owned())
                && node.layer_mask == 3
                && node.render_group == 2
                && node.tint.is_some()),
        "node attributes should reach the actual scene inspection report: {inspection:#?}"
    );
    assert!(
        inspection.counts.clipping_planes > 0,
        "authored-node section_box target should install real clipping planes: {inspection:#?}"
    );
    let projections: scena::AnnotationProjectionReportV1 = serde_json::from_str(
        &host
            .annotation_projections_json()
            .expect("projections serialize"),
    )
    .expect("projections decode");
    assert!(
        projections
            .annotations
            .iter()
            .any(|projection| projection.id == "floor-label"),
        "authored-node callout target should create a real annotation projection: {projections:#?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice9_advanced_pbr_fields_validate_build_and_reject_clamped_values() {
    let texture = "gltf/khronos/WaterBottle/WaterBottle_baseColor.png";
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "base": "#7C8798",
            "white": "#FFFFFF",
            "blue": "#BFD7FF"
        },
        "geometries": [{
            "id": "sphere_geo",
            "primitive": { "kind": "sphere", "radius": 0.18, "segments": 24, "rings": 12 }
        }],
        "materials": [{
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "metallic": 0.0,
            "roughness": 0.42,
            "clearcoat_factor": 0.8,
            "clearcoat_roughness_factor": 0.16,
            "clearcoat_normal_scale": 1.0,
            "sheen_color_factor": "white",
            "sheen_roughness_factor": 0.35,
            "anisotropy_strength_factor": 0.65,
            "anisotropy_rotation_radians": 0.3,
            "iridescence_factor": 0.45,
            "iridescence_ior": 1.45,
            "iridescence_thickness_minimum_nm": 120.0,
            "iridescence_thickness_maximum_nm": 480.0,
            "dispersion_factor": 0.02,
            "transmission_factor": 0.18,
            "ior": 1.52,
            "thickness_factor": 0.05,
            "attenuation_distance": 2.5,
            "attenuation_color": "blue",
            "clearcoat_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_roughness_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_normal_texture": { "uri": texture, "color_space": "linear" },
            "sheen_color_texture": { "uri": texture, "color_space": "srgb" },
            "sheen_roughness_texture": { "uri": texture, "color_space": "linear" },
            "anisotropy_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_thickness_texture": { "uri": texture, "color_space": "linear" },
            "transmission_texture": { "uri": texture, "color_space": "linear" },
            "thickness_texture": { "uri": texture, "color_space": "linear" }
        }],
        "nodes": [{
            "id": "sphere",
            "geometry": "sphere_geo",
            "material": "advanced",
            "transform": { "kind": "trs", "translation": [0.0, 0.0, -1.8] }
        }],
        "lights": [{
            "id": "key",
            "kind": "directional",
            "preset": "key",
            "illuminance_lux": 12000.0
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 0.0], "target": "sphere" }
        }],
        "capture": { "width": 128, "height": 96 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "advanced PBR recipe fields should validate before build: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice9.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 9 advanced PBR recipe build succeeds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert_eq!(build.manifest.materials.len(), 1);

    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "base": "#7C8798", "white": "#FFFFFF" },
        "geometries": [{
            "id": "sphere_geo",
            "primitive": { "kind": "sphere", "radius": 0.18 }
        }],
        "materials": [{
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "clearcoat_factor": 1.25,
            "sheen_color_factor": "missing_color",
            "anisotropy_strength_factor": -0.2,
            "iridescence_ior": 0.0,
            "transmission_factor": 2.0,
            "attenuation_distance": 0.0
        }],
        "nodes": [{
            "id": "sphere",
            "geometry": "sphere_geo",
            "material": "advanced"
        }]
    }));
    assert!(!invalid.ok);
    assert_reason(&invalid, "invalid_unit_value", None);
    assert_reason(&invalid, "unknown_color_ref", None);
    assert_reason(&invalid, "invalid_number", None);
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice10_primitives_validate_build_and_render_with_deterministic_counts() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "cone_color": "#E4572E",
            "torus_color": "#17BEBB",
            "disc_color": "#FFC914",
            "wedge_color": "#6A4C93"
        },
        "geometries": [
            { "id": "cone_geo", "primitive": { "kind": "cone", "radius": 0.10, "height": 0.22, "segments": 12 } },
            { "id": "torus_geo", "primitive": { "kind": "torus", "major_radius": 0.11, "minor_radius": 0.03, "segments": 12, "rings": 6 } },
            { "id": "disc_geo", "primitive": { "kind": "disc", "radius": 0.12, "segments": 16 } },
            { "id": "wedge_geo", "primitive": { "kind": "wedge", "size": [0.20, 0.12, 0.16] } }
        ],
        "materials": [
            { "id": "cone_mat", "kind": "unlit", "base_color": "cone_color", "double_sided": true },
            { "id": "torus_mat", "kind": "unlit", "base_color": "torus_color", "double_sided": true },
            { "id": "disc_mat", "kind": "unlit", "base_color": "disc_color", "double_sided": true },
            { "id": "wedge_mat", "kind": "unlit", "base_color": "wedge_color", "double_sided": true }
        ],
        "nodes": [
            { "id": "cone", "geometry": "cone_geo", "material": "cone_mat", "name": "cone", "transform": { "kind": "trs", "translation": [-0.24, 0.03, 0.0] } },
            { "id": "torus", "geometry": "torus_geo", "material": "torus_mat", "name": "torus", "transform": { "kind": "trs", "translation": [0.0, 0.04, 0.0], "rotation_degrees": [65.0, 0.0, 0.0] } },
            { "id": "disc", "geometry": "disc_geo", "material": "disc_mat", "name": "disc", "transform": { "kind": "trs", "translation": [0.24, 0.02, 0.0], "rotation_degrees": [70.0, 0.0, 0.0] } },
            { "id": "wedge", "geometry": "wedge_geo", "material": "wedge_mat", "name": "wedge", "transform": { "kind": "trs", "translation": [0.0, 0.02, -0.22] } }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.42, 0.72], "target": "torus" }
        }],
        "capture": { "width": 192, "height": 144 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 10 primitives should validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice10.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 10 primitive recipe build succeeds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);

    let counts: BTreeMap<_, _> = build
        .manifest
        .geometries
        .iter()
        .map(|geometry| {
            (
                geometry.id.as_str(),
                (
                    geometry.kind.as_str(),
                    geometry.vertex_count,
                    geometry.index_count,
                ),
            )
        })
        .collect();
    assert_eq!(counts["cone_geo"], ("cone", Some(49), Some(72)));
    assert_eq!(counts["torus_geo"], ("torus", Some(91), Some(432)));
    assert_eq!(counts["disc_geo"], ("disc", Some(17), Some(48)));
    assert_eq!(counts["wedge_geo"], ("wedge", Some(18), Some(24)));

    let node_handles: BTreeMap<_, _> = build
        .manifest
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.handle))
        .collect();

    let mut host = build.host;
    host.prepare().expect("Slice 10 primitive scene prepares");
    host.render().expect("Slice 10 primitive scene renders");
    let capture = host.capture().expect("Slice 10 primitive scene captures");
    let inspection_json = host
        .inspect_json()
        .expect("Slice 10 primitive scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(
        report.ok,
        "Slice 10 primitive render should be visible: {report:#?}"
    );
    assert!(
        !report.framing.tiny_in_frame && report.framing.fit_fraction > 0.25,
        "primitive silhouettes should occupy a measurable framed area: {report:#?}"
    );
    for node_id in ["cone", "torus", "disc", "wedge"] {
        let handle = node_handles[node_id];
        let node = inspection
            .draw_list
            .iter()
            .find(|draw| draw.node == handle)
            .unwrap_or_else(|| {
                panic!("missing inspected draw for node {node_id}: {inspection:#?}")
            });
        let bounds = &node.local_bounds;
        assert!(
            [
                bounds.min.x,
                bounds.min.y,
                bounds.min.z,
                bounds.max.x,
                bounds.max.y,
                bounds.max.z,
            ]
            .iter()
            .all(|value| value.is_finite()),
            "node {node_id} bounds must be finite: {bounds:#?}"
        );
    }

    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "bad_cone", "primitive": { "kind": "cone", "radius": -0.1, "height": 0.2 } },
            { "id": "bad_torus", "primitive": { "kind": "torus", "major_radius": 0.1, "minor_radius": 0.0 } },
            { "id": "bad_disc", "primitive": { "kind": "disc", "radius": 0.1, "segments": 0 } },
            { "id": "bad_wedge", "primitive": { "kind": "wedge", "size": [0.2, 0.1] } }
        ]
    }));
    assert!(!invalid.ok);
    assert_reason(&invalid, "invalid_number", None);
    assert_reason(&invalid, "invalid_integer", None);
    assert_reason(&invalid, "invalid_vector", None);
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice11_fonts_validate_build_render_and_fail_closed() {
    let font_path = system_test_font_path();
    let font_uri = path_str(&font_path).to_owned();
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "fonts": [
            { "id": "dejavu", "uri": font_uri }
        ],
        "labels": [{
            "id": "font_label",
            "text": "AVAV",
            "font": "dejavu",
            "size_px": 32.0,
            "color": "#19D96E",
            "transform": { "kind": "trs", "translation": [0.0, 0.0, 0.0] }
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.5], "target": "font_label" }
        }],
        "capture": { "width": 180, "height": 96 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 11 font recipe should validate: {validation:#?}"
    );

    let policy = scena::RecipeBuildPolicy::testing().with_allowed_roots([
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        PathBuf::from("/usr/share/fonts"),
    ]);
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice11.recipe.json",
        &text,
        policy.clone(),
    ))
    .expect("Slice 11 font recipe build succeeds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert!(
        build
            .manifest
            .nodes
            .iter()
            .any(|node| node.id == "font_label" && node.kind == "label"),
        "font label must be targetable in the build manifest: {:#?}",
        build.manifest
    );

    let mut host = build.host;
    host.prepare().expect("font label scene prepares");
    host.render().expect("font label scene renders");
    let capture = host.capture().expect("font label scene captures");
    let inspection_json = host.inspect_json().expect("font label scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(
        report.ok,
        "font label render should be visible: {report:#?}"
    );
    assert!(
        report.visible_pixel_fraction > 0.005,
        "font label should produce measurable glyph pixels: {report:#?}"
    );

    let too_small_policy = policy.with_fetch_byte_limit(16);
    let oversized = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice11.recipe.json",
        &text,
        too_small_policy,
    ))
    .expect_err("oversize font should fail closed under policy");
    assert_build_reason(&oversized, "policy_violation", "$.fonts[0].uri");

    let missing = json!({
        "schema": "scena.scene_recipe.v1",
        "fonts": [
            { "id": "missing_font", "uri": "tests/assets/missing-font.ttf" }
        ],
        "labels": [{
            "id": "font_label",
            "text": "AVAV",
            "font": "missing_font"
        }]
    });
    let missing_text = serde_json::to_string_pretty(&missing).expect("recipe serializes");
    let missing_report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice11.recipe.json",
        &missing_text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect_err("missing required font should fail closed");
    assert_build_reason(&missing_report, "font_load_failed", "$.fonts[0]");

    let complex = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "fonts": [
            { "id": "dejavu", "uri": font_uri }
        ],
        "labels": [{
            "id": "complex",
            "text": "سلام",
            "font": "dejavu"
        }]
    }));
    assert!(!complex.ok);
    assert_reason(&complex, "unsupported_feature", None);
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice12_skin_morph_authoring_deforms_rendered_output_and_fails_closed() {
    let undeformed = render_slice12_skin_morph_recipe(slice12_skin_morph_recipe(0.0, 0.0));
    let deformed = render_slice12_skin_morph_recipe(slice12_skin_morph_recipe(1.0, 0.28));

    assert!(
        deformed
            .manifest
            .geometries
            .iter()
            .any(|geometry| geometry.id == "tri_morph" && geometry.kind == "morph"),
        "morph-derived geometry must appear in the typed build manifest: {:#?}",
        deformed.manifest
    );
    assert!(
        deformed
            .manifest
            .geometries
            .iter()
            .any(|geometry| geometry.id == "tri_skin" && geometry.kind == "skin"),
        "skin-derived geometry must appear in the typed build manifest: {:#?}",
        deformed.manifest
    );

    let undeformed_bbox = undeformed
        .report
        .content_bbox_css_px
        .expect("undeformed render has content bbox");
    let deformed_bbox = deformed
        .report
        .content_bbox_css_px
        .expect("deformed render has content bbox");
    assert!(
        deformed_bbox.height > undeformed_bbox.height + 8.0
            && deformed_bbox.min_y < undeformed_bbox.min_y - 4.0,
        "morph + skin deformation must change the rendered silhouette, undeformed={undeformed_bbox:?}, deformed={deformed_bbox:?}"
    );

    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "red": "#E4572E" },
        "geometries": [{
            "id": "tri_base",
            "mesh": {
                "topology": "triangles",
                "positions": [[-0.42, -0.25, 0.0], [0.42, -0.25, 0.0], [0.0, 0.25, 0.0]],
                "normals": [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
                "indices": [0, 1, 2]
            }
        }],
        "morphs": [{
            "id": "bad_morph",
            "source_geometry": "tri_base",
            "targets": [{ "position_deltas": [[0.0, 0.0, 0.0]] }]
        }],
        "skins": [{
            "id": "bad_skin",
            "source_geometry": "tri_base",
            "joints": [[0, 0, 0, 0]],
            "weights": [[1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]]
        }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "red" }],
        "nodes": [{
            "id": "tri",
            "geometry": "bad_skin",
            "material": "mat",
            "morph_weights": [1.0],
            "skin_binding": {
                "joints": ["missing_joint"],
                "inverse_bind_matrices": [[
                    1.0, 0.0, 0.0, 0.0,
                    0.0, 1.0, 0.0, 0.0,
                    0.0, 0.0, 1.0, 0.0,
                    0.0, 0.0, 0.0, 1.0
                ]]
            }
        }]
    }));
    assert!(!invalid.ok);
    assert_reason(&invalid, "invalid_morph", None);
    assert_reason(&invalid, "invalid_skin", None);
    assert_reason(&invalid, "unknown_node_ref", None);
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice12_skin_morph_authoring_changes_headless_gpu_silhouette() {
    let undeformed = render_slice12_skin_morph_gpu(0.0, 0.0);
    let deformed = render_slice12_skin_morph_gpu(1.0, 0.28);

    assert!(
        deformed.height > undeformed.height + 8
            && deformed.min_y + 4 < undeformed.min_y
            && deformed.nonblack > undeformed.nonblack,
        "HeadlessGpu must render authored morph + skin deformation, undeformed={undeformed:?}, deformed={deformed:?}"
    );
}

#[cfg(feature = "scene-host")]
struct Slice12RenderProof {
    manifest: scena::SceneRecipeBuildV1,
    report: scena::RenderIntrospectionReportV1,
}

#[cfg(feature = "scene-host")]
#[derive(Debug)]
struct Slice12GpuBounds {
    min_y: usize,
    height: usize,
    nonblack: usize,
}

#[cfg(feature = "scene-host")]
fn render_slice12_skin_morph_gpu(morph_weight: f32, joint_lift: f32) -> Slice12GpuBounds {
    let (assets, mut scene, camera) = slice12_skin_morph_scene(morph_weight, joint_lift);
    let mut renderer =
        scena::Renderer::headless_gpu(180, 140).expect("HeadlessGpu renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("Slice 12 HeadlessGpu scene prepares");
    renderer
        .render(&scene, camera)
        .expect("Slice 12 HeadlessGpu scene renders");
    slice12_gpu_bounds(renderer.frame_rgba8(), 180, 140)
        .expect("Slice 12 HeadlessGpu frame is visible")
}

#[cfg(feature = "scene-host")]
fn slice12_skin_morph_scene(
    morph_weight: f32,
    joint_lift: f32,
) -> (scena::Assets, scena::Scene, scena::CameraKey) {
    let assets = scena::Assets::new();
    let vertices = vec![
        scena::GeometryVertex {
            position: scena::Vec3::new(-0.42, -0.25, 0.0),
            normal: scena::Vec3::new(0.0, 0.0, 1.0),
        },
        scena::GeometryVertex {
            position: scena::Vec3::new(0.42, -0.25, 0.0),
            normal: scena::Vec3::new(0.0, 0.0, 1.0),
        },
        scena::GeometryVertex {
            position: scena::Vec3::new(0.0, 0.25, 0.0),
            normal: scena::Vec3::new(0.0, 0.0, 1.0),
        },
    ];
    let geometry =
        scena::GeometryDesc::try_new(scena::GeometryTopology::Triangles, vertices, vec![0, 1, 2])
            .expect("base geometry builds")
            .with_morph_targets(vec![scena::GeometryMorphTarget::new(vec![
                scena::Vec3::ZERO,
                scena::Vec3::ZERO,
                scena::Vec3::new(0.0, 0.45, 0.0),
            ])])
            .expect("morph geometry builds")
            .with_skin(scena::GeometrySkin::new(
                vec![[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                vec![[1.0, 0.0, 0.0, 0.0]; 3],
            ))
            .expect("skin geometry builds");
    let geometry = assets.create_geometry(geometry);
    let material = assets.create_material(
        scena::MaterialDesc::unlit(scena::Color::from_srgb_u8(228, 87, 46)).with_double_sided(true),
    );

    let mut scene = scena::Scene::new();
    let joint = scene
        .add_empty(
            scene.root(),
            scena::Transform::at(scena::Vec3::new(0.0, joint_lift, 0.0)),
        )
        .expect("binding node inserts");
    let mesh = scene
        .mesh(geometry, material)
        .add()
        .expect("deformed mesh inserts");
    scene
        .set_morph_weights(mesh, vec![morph_weight])
        .expect("morph weight applies");
    scene
        .set_skin_binding(
            mesh,
            scena::SceneSkinBinding::new(vec![joint], vec![scena::SkinningMatrix::IDENTITY]),
        )
        .expect("skin binding applies");
    let camera = scene.add_default_camera().expect("camera inserts");

    (assets, scene, camera)
}

#[cfg(feature = "scene-host")]
fn slice12_gpu_bounds(rgba: &[u8], width: usize, height: usize) -> Option<Slice12GpuBounds> {
    let mut min_y = height;
    let mut max_y = 0usize;
    let mut nonblack = 0usize;
    for (index, pixel) in rgba.chunks_exact(4).enumerate() {
        if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
            continue;
        }
        let y = index / width;
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        nonblack += 1;
    }
    (nonblack > 0).then_some(Slice12GpuBounds {
        min_y,
        height: max_y.saturating_sub(min_y) + 1,
        nonblack,
    })
}

#[cfg(feature = "scene-host")]
fn render_slice12_skin_morph_recipe(recipe: serde_json::Value) -> Slice12RenderProof {
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 12 skin/morph recipe should validate before build: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice12.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 12 recipe build succeeds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);

    let mut host = build.host;
    host.prepare().expect("Slice 12 scene prepares");
    host.render().expect("Slice 12 scene renders");
    let capture = host.capture().expect("Slice 12 scene captures");
    let inspection_json = host.inspect_json().expect("Slice 12 scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(report.ok, "Slice 12 render should be visible: {report:#?}");

    Slice12RenderProof {
        manifest: build.manifest,
        report,
    }
}

#[cfg(feature = "scene-host")]
fn slice12_skin_morph_recipe(morph_weight: f64, joint_lift: f64) -> serde_json::Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "red": "#E4572E",
            "dark": "#1D2733"
        },
        "geometries": [
            {
                "id": "tri_base",
                "mesh": {
                    "topology": "triangles",
                    "positions": [[-0.42, -0.25, 0.0], [0.42, -0.25, 0.0], [0.0, 0.25, 0.0]],
                    "normals": [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
                    "indices": [0, 1, 2]
                }
            },
            { "id": "joint_marker_geo", "primitive": { "kind": "box", "size": [0.04, 0.04, 0.04] } }
        ],
        "morphs": [{
            "id": "tri_morph",
            "source_geometry": "tri_base",
            "targets": [{
                "position_deltas": [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.45, 0.0]]
            }]
        }],
        "skins": [{
            "id": "tri_skin",
            "source_geometry": "tri_morph",
            "joints": [[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
            "weights": [[1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]]
        }],
        "materials": [
            { "id": "tri_mat", "kind": "unlit", "base_color": "red", "double_sided": true },
            { "id": "joint_mat", "kind": "unlit", "base_color": "dark" }
        ],
        "nodes": [
            {
                "id": "joint",
                "geometry": "joint_marker_geo",
                "material": "joint_mat",
                "visible": false,
                "transform": { "kind": "trs", "translation": [0.0, joint_lift, 0.0] }
            },
            {
                "id": "tri",
                "geometry": "tri_skin",
                "material": "tri_mat",
                "morph_weights": [morph_weight],
                "skin_binding": {
                    "joints": ["joint"],
                    "inverse_bind_matrices": [[
                        1.0, 0.0, 0.0, 0.0,
                        0.0, 1.0, 0.0, 0.0,
                        0.0, 0.0, 1.0, 0.0,
                        0.0, 0.0, 0.0, 1.0
                    ]]
                }
            }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "fov_degrees": 35.0,
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.18, 2.0], "target": [0.0, 0.18, 0.0] }
        }],
        "capture": { "width": 180, "height": 140 }
    })
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice13_particles_render_per_particle_output_and_fail_closed() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "green": "#20D060",
            "yellow": "#F0C020",
            "blue": "#2050E0",
            "red": "#E03030",
            "magenta": "#D028D0"
        },
        "particles": [{
            "id": "status_particles",
            "particles": [
                { "id": "left_green", "position": [-0.35, 0.0, 0.0], "color": "green", "size_px": 18.0 },
                { "id": "right_yellow", "position": [0.35, 0.0, 0.0], "color": "yellow", "size_px": 30.0 },
                { "id": "far_blue", "position": [0.0, 0.0, -0.35], "color": "blue", "size_px": 34.0 },
                { "id": "near_red", "position": [0.0, 0.0, 0.0], "color": "red", "size_px": 18.0 },
                { "id": "rotated_magenta", "position": [0.0, -0.35, 0.0], "color": "magenta", "size_px": 24.0, "rotation_degrees": 45.0 }
            ]
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 2.0], "target": [0.0, 0.0, 0.0] }
        }],
        "capture": { "width": 160, "height": 120 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 13 particle recipe should validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice13.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 13 recipe build succeeds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert!(
        build
            .manifest
            .nodes
            .iter()
            .any(|node| node.id == "status_particles" && node.kind == "particle_set"),
        "particle set must appear as a targetable authored node: {:#?}",
        build.manifest
    );

    let mut host = build.host;
    host.prepare().expect("Slice 13 scene prepares");
    host.render().expect("Slice 13 scene renders");
    let capture = host.capture().expect("Slice 13 scene captures");
    let inspection_json = host.inspect_json().expect("Slice 13 scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(report.ok, "Slice 13 render should be visible: {report:#?}");
    assert_slice13_particle_pixels(capture.rgba8.as_slice(), 160);

    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "green": "#20D060" },
        "particles": [{
            "id": "bad_particles",
            "particles": [
                { "id": "", "position": [0.0, 0.0, 0.0], "color": "green", "size_px": -1.0 },
                { "id": "unknown_color", "position": ["x", 0.0, 0.0], "color": "missing", "size_px": 12.0 }
            ],
            "visible": "yes"
        }]
    }));
    assert!(!invalid.ok);
    assert_reason(&invalid, "invalid_id", None);
    assert_reason(&invalid, "invalid_particle", None);
    assert_reason(&invalid, "unknown_color_ref", None);
    assert_reason(&invalid, "invalid_visible", None);
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice13_particles_change_headless_gpu_pixels_by_color_size_position_and_depth() {
    let rgba = render_slice13_particles_gpu();
    assert_slice13_particle_pixels(rgba.as_slice(), 160);
}

#[cfg(feature = "scene-host")]
fn assert_slice13_particle_pixels(rgba: &[u8], width: usize) {
    let green = slice13_color_bounds(rgba, width, |pixel| {
        pixel[1] > 150 && pixel[0] < 80 && pixel[2] < 120
    })
    .expect("green particle is visible");
    let yellow = slice13_color_bounds(rgba, width, |pixel| {
        pixel[0] > 180 && pixel[1] > 130 && pixel[2] < 90
    })
    .expect("yellow particle is visible");
    let blue = slice13_color_bounds(rgba, width, |pixel| {
        pixel[2] > 140 && pixel[0] < 90 && pixel[1] < 120
    })
    .expect("far blue particle remains visible around the nearer red particle");
    let red = slice13_color_bounds(rgba, width, |pixel| {
        pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100
    })
    .expect("near red particle is visible");
    let magenta = slice13_color_bounds(rgba, width, |pixel| {
        pixel[0] > 130 && pixel[1] < 100 && pixel[2] > 130
    })
    .expect("rotated magenta particle is visible");

    assert!(
        green.center_x() < 70.0 && yellow.center_x() > 90.0,
        "particle screen positions should track their authored x positions: green={green:?}, yellow={yellow:?}"
    );
    assert!(
        (green.center_y() - 60.0).abs() < 8.0
            && (yellow.center_y() - 60.0).abs() < 8.0
            && (red.center_y() - 60.0).abs() < 8.0,
        "particle screen positions should track their authored y positions: green={green:?}, yellow={yellow:?}, red={red:?}"
    );
    assert!(
        (14..=24).contains(&green.width())
            && (25..=40).contains(&yellow.width())
            && yellow.width() > green.width() + 8,
        "particle size_px must produce distinct rendered sprite sizes: green={green:?}, yellow={yellow:?}"
    );
    assert!(
        blue.width() > 22 && blue.height() > 22,
        "larger far blue sprite should leave a visible ring for depth verification: {blue:?}"
    );
    assert!(
        (red.center_x() - 80.0).abs() < 8.0
            && (red.center_y() - 60.0).abs() < 8.0
            && (14..=24).contains(&red.width()),
        "near red particle color and placement should be independently visible: {red:?}"
    );
    assert!(
        magenta.center_y() > 70.0 && magenta.width() >= 30 && magenta.height() >= 30,
        "rotated particle should move on the y axis and expand its screen-space bbox: {magenta:?}"
    );

    let center = slice13_pixel(rgba, width, 80, 60);
    assert!(
        center[0] > 150 && center[1] < 100 && center[2] < 100,
        "near red particle must depth-test in front of far blue at the same screen position, center pixel={center:?}"
    );
}

#[cfg(feature = "scene-host")]
fn render_slice13_particles_gpu() -> Vec<u8> {
    let mut scene = scena::Scene::new();
    let particles = scena::ParticleSet::try_new(vec![
        scena::Particle::new(
            scena::Vec3::new(-0.35, 0.0, 0.0),
            scena::Color::from_srgb_u8(32, 208, 96),
            18.0,
        ),
        scena::Particle::new(
            scena::Vec3::new(0.35, 0.0, 0.0),
            scena::Color::from_srgb_u8(240, 192, 32),
            30.0,
        ),
        scena::Particle::new(
            scena::Vec3::new(0.0, 0.0, -0.35),
            scena::Color::from_srgb_u8(32, 80, 224),
            34.0,
        ),
        scena::Particle::new(
            scena::Vec3::new(0.0, 0.0, 0.0),
            scena::Color::from_srgb_u8(224, 48, 48),
            18.0,
        ),
        scena::Particle::new(
            scena::Vec3::new(0.0, -0.35, 0.0),
            scena::Color::from_srgb_u8(208, 40, 208),
            24.0,
        )
        .with_rotation_radians(std::f32::consts::FRAC_PI_4),
    ])
    .expect("particle buffer validates");
    scene
        .add_particle_set_node(scene.root(), particles, scena::Transform::IDENTITY)
        .expect("particle set inserts");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            scena::PerspectiveCamera::default(),
            scena::Transform::at(scena::Vec3::new(0.0, 0.0, 2.0))
                .looking_at(scena::Vec3::ZERO, scena::Vec3::Y),
        )
        .expect("camera inserts");
    scene.set_active_camera(camera).expect("camera activates");

    let mut renderer =
        scena::Renderer::headless_gpu(160, 120).expect("HeadlessGpu renderer builds");
    renderer
        .prepare(&mut scene)
        .expect("Slice 13 HeadlessGpu scene prepares");
    renderer
        .render(&scene, camera)
        .expect("Slice 13 HeadlessGpu scene renders");
    renderer.frame_rgba8().to_vec()
}

#[cfg(feature = "scene-host")]
#[derive(Debug)]
struct Slice13ColorBounds {
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
}

#[cfg(feature = "scene-host")]
impl Slice13ColorBounds {
    fn width(&self) -> usize {
        self.max_x.saturating_sub(self.min_x) + 1
    }

    fn height(&self) -> usize {
        self.max_y.saturating_sub(self.min_y) + 1
    }

    fn center_x(&self) -> f32 {
        (self.min_x + self.max_x) as f32 * 0.5
    }

    fn center_y(&self) -> f32 {
        (self.min_y + self.max_y) as f32 * 0.5
    }
}

#[cfg(feature = "scene-host")]
fn slice13_color_bounds(
    rgba: &[u8],
    width: usize,
    matches: impl Fn(&[u8]) -> bool,
) -> Option<Slice13ColorBounds> {
    let mut bounds: Option<Slice13ColorBounds> = None;
    for (index, pixel) in rgba.chunks_exact(4).enumerate() {
        if !matches(pixel) {
            continue;
        }
        let x = index % width;
        let y = index / width;
        bounds = Some(match bounds {
            Some(mut bounds) => {
                bounds.min_x = bounds.min_x.min(x);
                bounds.min_y = bounds.min_y.min(y);
                bounds.max_x = bounds.max_x.max(x);
                bounds.max_y = bounds.max_y.max(y);
                bounds
            }
            None => Slice13ColorBounds {
                min_x: x,
                min_y: y,
                max_x: x,
                max_y: y,
            },
        });
    }
    bounds
}

#[cfg(feature = "scene-host")]
fn slice13_pixel(rgba: &[u8], width: usize, x: usize, y: usize) -> &[u8] {
    let start = (y * width + x) * 4;
    &rgba[start..start + 4]
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice3_transform_placement_verbs_resolve_against_authored_and_imported_targets() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "anchor_asset",
            "uri": "tests/assets/gltf/anchored_triangle_scene.gltf"
        }],
        "colors": {
            "base": "#7097C8",
            "accent": "#E9B44C"
        },
        "geometries": [
            { "id": "support_geo", "primitive": { "kind": "box", "size": [0.24, 0.10, 0.24] } },
            { "id": "part_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } },
            { "id": "large_geo", "primitive": { "kind": "box", "size": [2.0, 2.0, 2.0] } }
        ],
        "materials": [
            { "id": "base_mat", "kind": "unlit", "base_color": "base" },
            { "id": "accent_mat", "kind": "unlit", "base_color": "accent" }
        ],
        "nodes": [
            {
                "id": "support",
                "geometry": "support_geo",
                "material": "base_mat",
                "transform": { "kind": "center" }
            },
            {
                "id": "placed",
                "geometry": "part_geo",
                "material": "accent_mat",
                "transform": { "kind": "place_on", "target": "support", "offset": [0.0, 0.02, 0.0] }
            },
            {
                "id": "grounded",
                "geometry": "part_geo",
                "material": "accent_mat",
                "transform": { "kind": "ground", "plane_y": -0.25 }
            },
            {
                "id": "fit",
                "geometry": "large_geo",
                "material": "base_mat",
                "transform": { "kind": "fit_to_size", "size": [0.20, 0.20, 0.20] }
            },
            {
                "id": "aligned",
                "geometry": "part_geo",
                "material": "accent_mat",
                "transform": { "kind": "align_to_anchor", "anchor": "anchor_asset.mount" }
            }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.65, 0.45, 0.55], "target": "support" }
        }],
        "capture": { "width": 320, "height": 220 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 3 placement recipe should validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice3.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 3 placement recipe build succeeds");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    let handle = |id: &str| {
        build
            .manifest
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.handle)
            .unwrap_or_else(|| panic!("node {id} exists in manifest: {:#?}", build.manifest))
    };
    let support = build
        .host
        .node_world_bounds(handle("support"))
        .expect("support bounds query succeeds")
        .expect("support has bounds");
    let placed = build
        .host
        .node_world_bounds(handle("placed"))
        .expect("placed bounds query succeeds")
        .expect("placed has bounds");
    let grounded = build
        .host
        .node_world_bounds(handle("grounded"))
        .expect("grounded bounds query succeeds")
        .expect("grounded has bounds");
    let fit = build
        .host
        .node_world_bounds(handle("fit"))
        .expect("fit bounds query succeeds")
        .expect("fit has bounds");
    assert!(
        (placed.min.y - (support.max.y + 0.02)).abs() < 0.001,
        "place_on should put the placed node on the support top plus offset: support={support:?}, placed={placed:?}"
    );
    assert!(
        (grounded.min.y + 0.25).abs() < 0.001,
        "ground should put node min_y on the requested plane: {grounded:?}"
    );
    assert!(
        (fit.max - fit.min).max_element() <= 0.201,
        "fit_to_size should uniformly shrink inside the requested box: {fit:?}"
    );

    let inspection_json = build.host.inspect_json().expect("Slice 3 scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let aligned = inspection
        .nodes
        .iter()
        .find(|node| node.handle == handle("aligned"))
        .expect("aligned node appears in inspection");
    assert!(
        aligned.world_transform.translation.length() < 0.001,
        "align_to_anchor should put the node origin at the imported anchor: {aligned:#?}"
    );
}

#[test]
fn scene_recipe_slice3_transform_refs_fail_closed_before_build() {
    let forward_ref = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "box_geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [
            { "id": "box_mat", "kind": "unlit", "base_color": "#FFFFFF" }
        ],
        "nodes": [
            {
                "id": "early",
                "geometry": "box_geo",
                "material": "box_mat",
                "transform": { "kind": "place_on", "target": "later" }
            },
            {
                "id": "later",
                "geometry": "box_geo",
                "material": "box_mat"
            }
        ]
    }));
    assert!(!forward_ref.ok, "forward placement refs must fail closed");
    assert_reason(&forward_ref, "unknown_node_ref", None);

    let self_cycle = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "box_geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [
            { "id": "box_mat", "kind": "unlit", "base_color": "#FFFFFF" }
        ],
        "nodes": [
            {
                "id": "early",
                "geometry": "box_geo",
                "material": "box_mat",
                "transform": { "kind": "place_on", "target": "early" }
            }
        ]
    }));
    assert!(!self_cycle.ok, "self placement cycles must fail closed");
    assert_reason(&self_cycle, "unknown_node_ref", None);
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice4_scene_and_render_setup_affect_real_output() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "body_color": "#E9B44C"
        },
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.18, 0.12, 0.08] } }
        ],
        "materials": [
            { "id": "body_mat", "kind": "unlit", "base_color": "body_color" }
        ],
        "nodes": [
            {
                "id": "body",
                "geometry": "body_geo",
                "material": "body_mat",
                "transform": { "kind": "ground", "plane_y": 0.0 }
            }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.35, 0.28, 0.35], "target": "body" }
        }],
        "scene": {
            "background": { "kind": "white" },
            "environment": { "kind": "default" },
            "grid": {
                "enabled": true,
                "padding": 0.08,
                "line_spacing": 0.04
            }
        },
        "render": {
            "profile": "quality",
            "quality": "high",
            "anti_aliasing": "none",
            "bloom": { "threshold_srgb": 64, "intensity": 0.25, "radius_px": 2 },
            "ssao": { "radius_px": 2, "intensity": 0.25, "depth_threshold": 0.02 },
            "exposure_ev": 1.0,
            "tonemapper": "standard"
        },
        "capture": { "width": 96, "height": 72 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 4 scene/render setup should validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice4.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 4 scene/render recipe build succeeds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);

    assert_eq!(build.host.renderer().profile(), scena::Profile::Quality);
    assert_eq!(build.host.renderer().quality(), scena::Quality::High);
    assert_eq!(
        build.host.renderer().anti_aliasing(),
        scena::AntiAliasing::None
    );
    assert!(build.host.renderer().bloom().is_some());
    assert!(
        build
            .host
            .renderer()
            .screen_space_ambient_occlusion()
            .is_some()
    );
    assert!((build.host.renderer().exposure_ev() - 1.0).abs() < 0.001);
    assert_eq!(
        build.host.renderer().tonemapper(),
        scena::Tonemapper::Standard
    );
    assert!(build.host.renderer().environment().is_some());

    let inspection_json = build.host.inspect_json().expect("Slice 4 scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    assert!(
        inspection.counts.visible_drawable >= 3,
        "grid floor should add real renderable floor/grid nodes: {inspection:#?}"
    );

    let mut host = build.host;
    host.prepare().expect("Slice 4 scene prepares");
    host.render().expect("Slice 4 scene renders");
    let capture = host.capture().expect("Slice 4 scene captures");
    let top_left = &capture.rgba8[..4];
    assert!(
        top_left[0] > 240 && top_left[1] > 240 && top_left[2] > 240,
        "white background should be visible in the captured frame corner, got {top_left:?}"
    );
}

#[test]
fn scene_recipe_slice4_scene_and_render_settings_fail_closed() {
    let report = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "body_color": "#FFFFFF"
        },
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [
            { "id": "body_mat", "kind": "unlit", "base_color": "body_color" }
        ],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "body_mat" }
        ],
        "scene": {
            "background": { "kind": "not_a_background" },
            "grid": { "enabled": true, "line_spacing": -1.0 }
        },
        "render": {
            "anti_aliasing": "sparkle",
            "bloom": { "threshold_srgb": 64, "intensity": 2.0, "radius_px": 2 },
            "ssao": { "radius_px": 2, "intensity": 0.25, "depth_threshold": -0.01 },
            "tonemapper": "filmic"
        }
    }));
    assert!(!report.ok, "invalid scene/render knobs must fail closed");
    assert_reason(&report, "invalid_background", None);
    assert_reason(&report, "invalid_number", None);
    assert_reason(&report, "invalid_render_setting", None);
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice6_instancing_labels_and_clipping_planes_build_and_render() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "part": "#4A90E2",
            "tint": "#F6C85F",
            "label_fg": "#FFFFFF",
            "label_bg": "#1D2733"
        },
        "geometries": [
            { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "cube_mat", "kind": "unlit", "base_color": "part", "double_sided": true }
        ],
        "instance_sets": [{
            "id": "cube_field",
            "geometry": "cube_geo",
            "material": "cube_mat",
            "instances": [
                {
                    "id": "left",
                    "transform": { "kind": "trs", "translation": [-0.06, 0.0, 0.0] },
                    "tint": "tint"
                },
                {
                    "id": "right-hidden",
                    "transform": { "kind": "trs", "translation": [0.06, 0.0, 0.0] },
                    "visible": false
                }
            ]
        }],
        "labels": [{
            "id": "status_label",
            "text": "OK",
            "color": "label_fg",
            "background": "label_bg",
            "size_px": 18.0,
            "transform": { "kind": "trs", "translation": [0.0, 0.1, 0.0] }
        }],
        "clipping_planes": [{
            "id": "keep-visible-half",
            "normal": [1.0, 0.0, 0.0],
            "distance": 0.2
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "fov_degrees": 40.0,
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.24, 0.2, 0.28], "target": [0.0, 0.03, 0.0] }
        }],
        "capture": { "width": 220, "height": 180 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "Slice 6 recipe should validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice6.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 6 recipe build succeeds");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert!(
        build
            .manifest
            .nodes
            .iter()
            .any(|node| node.id == "cube_field" && node.kind == "instance_set"),
        "instance set root should be targetable in the build manifest: {:#?}",
        build.manifest
    );
    assert!(
        build
            .manifest
            .nodes
            .iter()
            .any(|node| node.id == "status_label" && node.kind == "label"),
        "free-standing label should be targetable in the build manifest: {:#?}",
        build.manifest
    );

    let mut host = build.host;
    host.prepare().expect("Slice 6 scene prepares");
    host.render().expect("Slice 6 scene renders");
    let capture = host.capture().expect("Slice 6 scene captures");
    let inspection_json = host.inspect_json().expect("Slice 6 scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    assert_eq!(
        inspection.counts.clipping_planes, 1,
        "arbitrary clipping plane should be active in the scene: {inspection:#?}"
    );
    assert!(
        inspection
            .nodes
            .iter()
            .any(|node| node.kind == "InstanceSet"),
        "recipe instance set should create a real scene InstanceSet node: {inspection:#?}"
    );
    assert!(
        inspection.nodes.iter().any(|node| node.kind == "Label"),
        "recipe label should create a real scene Label node: {inspection:#?}"
    );
    let drawn_instances = inspection
        .draw_list
        .iter()
        .filter(|draw| draw.instance.is_some())
        .count();
    assert_eq!(
        drawn_instances, 1,
        "only the visible per-instance entry should draw: {inspection:#?}"
    );
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(report.ok, "Slice 6 render should be visible: {report:#?}");
}

#[test]
fn scene_recipe_slice6_validation_rejects_bad_instances_labels_and_clipping_planes() {
    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "part": "#4A90E2" },
        "geometries": [
            { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "cube_mat", "kind": "unlit", "base_color": "part" }
        ],
        "instance_sets": [{
            "id": "cube_field",
            "geometry": "missing_geo",
            "material": "cube_mat",
            "instances": [
                { "id": "", "visible": "yes", "tint": "missing_color" }
            ]
        }],
        "labels": [{
            "id": "bad_label",
            "text": "",
            "size_px": -1.0,
            "color": "missing_color",
            "transform": { "kind": "trs", "translation": [0.0, "bad", 0.0] }
        }],
        "clipping_planes": [{
            "id": "bad_plane",
            "normal": [0.0, 0.0, 0.0],
            "distance": "near"
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.24, 0.2, 0.28], "target": [0.0, 0.0, 0.0] }
        }]
    }));

    assert!(!invalid.ok);
    assert_reason(&invalid, "unknown_geometry_ref", None);
    assert_reason(&invalid, "invalid_id", None);
    assert_reason(&invalid, "invalid_visible", None);
    assert_reason(&invalid, "unknown_color_ref", None);
    assert_reason(&invalid, "invalid_label", None);
    assert_reason(&invalid, "invalid_vector", None);
    assert_reason(&invalid, "invalid_clipping_plane", None);
}

#[test]
fn scene_recipe_authoring_refs_and_future_variants_fail_before_build() {
    let unknown_ref = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [{
            "id": "plate_geo",
            "primitive": { "kind": "box", "size": [0.12, 0.06, 0.004] }
        }],
        "materials": [{
            "id": "plate_mat",
            "kind": "unlit",
            "base_color": "missing_color"
        }],
        "nodes": [{
            "id": "plate",
            "geometry": "missing_geo",
            "material": "plate_mat"
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": {
                "kind": "look_at",
                "eye": [0.2, 0.15, 0.2],
                "target": "missing_node"
            }
        }]
    }));
    assert!(!unknown_ref.ok);
    assert_reason(&unknown_ref, "unknown_color_ref", None);
    assert_reason(&unknown_ref, "unknown_geometry_ref", None);
    assert_reason(&unknown_ref, "unknown_node_ref", None);

    let unsupported_variant = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "blue": "#3A7BD5" },
        "geometries": [{
            "id": "plate_geo",
            "primitive": { "kind": "capsule", "radius": 1.0, "height": 2.0 }
        }],
        "materials": [{
            "id": "plate_mat",
            "kind": "unlit",
            "base_color": "blue",
            "roughness": 0.5
        }],
        "nodes": [{
            "id": "plate",
            "geometry": "plate_geo",
            "material": "plate_mat",
            "transform": { "kind": "center" }
        }]
    }));
    assert!(!unsupported_variant.ok);
    assert_reason(&unsupported_variant, "unsupported_feature", None);
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
        parsed
            .section_box
            .as_ref()
            .expect("section box")
            .import
            .as_deref(),
        Some("plate")
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

#[cfg(feature = "scene-host")]
fn system_test_font_path() -> PathBuf {
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    ];
    candidates
        .iter()
        .map(Path::new)
        .find(|path| path.exists())
        .map(Path::to_path_buf)
        .expect("builder must provide a TrueType test font")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
