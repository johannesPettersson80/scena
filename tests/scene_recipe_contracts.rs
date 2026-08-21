#[cfg(feature = "scene-host")]
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;

#[test]
fn scene_recipe_photo_and_focus_targets_use_shared_target_grammar() {
    let direct_photo_subject: scena::SceneRecipePhotoSubjectV1 =
        serde_json::from_value(json!({ "kind": "import", "id": "subject" }))
            .expect("photo.subject accepts the canonical direct target grammar");
    assert_eq!(
        direct_photo_subject.target(),
        &scena::SceneRecipeTargetV1::Import {
            id: "subject".to_owned()
        },
        "photo.subject direct form must resolve through the canonical SceneRecipeTargetV1 grammar"
    );
    let spec_photo_subject: scena::SceneRecipePhotoSubjectV1 = serde_json::from_value(json!({
        "target": { "kind": "import", "id": "subject" },
        "fallback": "average_metering_with_warning"
    }))
    .expect("photo.subject accepts the subject spec wrapper");
    assert_eq!(
        spec_photo_subject.target(),
        &scena::SceneRecipeTargetV1::Import {
            id: "subject".to_owned()
        },
        "photo.subject spec form must resolve through the canonical SceneRecipeTargetV1 grammar"
    );
    assert_eq!(
        spec_photo_subject.fallback(),
        scena::SceneRecipeSubjectFallbackPolicyV1::AverageMeteringWithWarning,
        "photo.subject spec form must carry an explicit fallback policy"
    );
    assert_eq!(
        std::any::TypeId::of::<scena::scene::recipe::SceneRecipeDepthOfFieldTargetV1>(),
        std::any::TypeId::of::<scena::SceneRecipeTargetV1>(),
        "depth_of_field.focus.target must use the canonical SceneRecipeTargetV1 grammar"
    );
    assert_eq!(
        std::any::TypeId::of::<scena::scene::recipe::SceneRecipeMeteringTargetV1>(),
        std::any::TypeId::of::<scena::SceneRecipeTargetV1>(),
        "render.metering.target must use the canonical SceneRecipeTargetV1 grammar"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_verification_fails_when_nested_quality_report_fails() {
    let quality = scena::RenderQualityReportV1 {
        schema: scena::RENDER_QUALITY_SCHEMA_V1.to_owned(),
        ok: false,
        profile: "product".to_owned(),
        summary: scena::RenderQualitySummaryV1 {
            checks: 1,
            errors: 1,
            warnings: 0,
        },
        checks: vec![scena::RenderQualityCheckV1 {
            id: "baseline.black_crush".to_owned(),
            code: "severe_black_crush".to_owned(),
            status: scena::RenderQualityStatusV1::Failed,
            severity: "error".to_owned(),
            region: scena::RenderQualityRegionV1 {
                kind: "subject".to_owned(),
                handle: None,
                rect_css_px: None,
            },
            observed: std::collections::BTreeMap::from([("low_clip_fraction".to_owned(), 0.9)]),
            threshold: std::collections::BTreeMap::from([(
                "max_low_clip_fraction".to_owned(),
                0.45,
            )]),
            fix_hint: "raise exposure".to_owned(),
        }],
        capabilities: scena::RenderIntrospectionCapabilitiesV1 {
            backend: scena::Backend::Headless,
            gpu_device: false,
            surface_attached: false,
            hardware_tier: scena::HardwareTier::Low,
            forward_pbr: scena::CapabilityStatus::ErrorIfRequired,
            readback_headless_screenshots: scena::CapabilityStatus::Supported,
        },
    };

    let report =
        scena::SceneRecipeVerificationReportV1::new(0, Vec::new(), None, None, None, Some(quality));

    assert!(
        !report.ok,
        "top-level recipe verification must not pass while nested quality fails: {report:#?}"
    );
    assert_eq!(report.summary.errors, 1, "{report:#?}");
    assert!(
        report
            .reasons
            .iter()
            .any(|reason| reason.source == "quality" && reason.code == "severe_black_crush"),
        "nested quality failure must surface as an actionable recipe verification reason: {report:#?}"
    );
}

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
fn scene_recipe_validation_accepts_camera_behavior_photo_intent_and_rejects_bad_subjects() {
    let valid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" }
        }
    }));
    assert!(
        valid.ok,
        "camera behavior photo recipe should validate: {valid:#?}"
    );

    let legacy_intent = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "product_hero",
            "subject": { "kind": "import", "id": "subject" }
        }
    }));
    assert!(
        legacy_intent.ok,
        "legacy product_hero intent remains a compatibility alias: {legacy_intent:#?}"
    );

    let valid_authored_node_subject = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "hero_geo", "primitive": { "kind": "box", "size": [0.12, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "hero_mat", "kind": "unlit", "base_color": "#6F7F8F" }
        ],
        "nodes": [
            { "id": "hero", "geometry": "hero_geo", "material": "hero_mat" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "node", "id": "hero" }
        }
    }));
    assert!(
        valid_authored_node_subject.ok,
        "camera behavior photo recipe should accept authored node subjects: {valid_authored_node_subject:#?}"
    );

    let unknown_intent = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "catalog_turntable",
            "subject": { "kind": "import", "id": "subject" }
        }
    }));
    assert!(!unknown_intent.ok);
    assert_reason(&unknown_intent, "invalid_photo_intent", None);

    let missing_subject_import = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "missing" }
        }
    }));
    assert!(!missing_subject_import.ok);
    assert_reason(&missing_subject_import, "unknown_photo_subject", None);
}

#[test]
fn scene_recipe_final_photo_quality_contract_is_fail_closed() {
    let preview_compatible = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" }
        }
    }));
    assert!(
        preview_compatible.ok,
        "omitting photo.quality must retain preview-compatible v1 behavior: \
         {preview_compatible:#?}"
    );
    let preview_recipe = scena::parse_valid_scene_recipe_json(
        &serde_json::to_string(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
            ],
            "photo": { "intent": "camera_behavior" }
        }))
        .expect("preview recipe serializes"),
    )
    .expect("omitted photo quality parses");
    assert_eq!(
        preview_recipe.photo.expect("photo exists").quality,
        scena::SceneRecipePhotoQualityV1::Preview
    );

    let final_defaults = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "quality": "final",
            "subject": { "kind": "import", "id": "subject" }
        }
    }));
    assert!(
        final_defaults.ok,
        "final mode owns documented capture/sampling defaults: {final_defaults:#?}"
    );

    let explicit_final = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "quality": "final",
            "subject": { "kind": "import", "id": "subject" }
        },
        "capture": { "width": 3840, "height": 2520 },
        "render": {
            "anti_aliasing": "none",
            "supersample": 2,
            "reconstruction": "tent"
        }
    }));
    assert!(
        explicit_final.ok,
        "documented explicit final settings must validate: {explicit_final:#?}"
    );
    let final_recipe = scena::parse_valid_scene_recipe_json(
        &serde_json::to_string(&json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
            ],
            "photo": { "intent": "camera_behavior", "quality": "final" }
        }))
        .expect("final recipe serializes"),
    )
    .expect("final photo quality parses");
    assert_eq!(
        final_recipe.photo.expect("photo exists").quality,
        scena::SceneRecipePhotoQualityV1::Final
    );

    let unknown_quality = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "photo": {
            "intent": "camera_behavior",
            "quality": "cinematic"
        }
    }));
    assert!(!unknown_quality.ok);
    assert_reason_at(&unknown_quality, "invalid_photo_quality", "$.photo.quality");
    let non_string_quality = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "photo": {
            "intent": "camera_behavior",
            "quality": 2
        }
    }));
    assert!(!non_string_quality.ok);
    assert_reason_at(
        &non_string_quality,
        "invalid_photo_quality",
        "$.photo.quality",
    );

    let undersized = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "photo": {
            "intent": "camera_behavior",
            "quality": "final"
        },
        "capture": { "width": 2560, "height": 1680 }
    }));
    assert!(!undersized.ok);
    assert_reason_at(&undersized, "final_photo_capture_below_min", "$.capture");

    let undersampled = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "photo": {
            "intent": "camera_behavior",
            "quality": "final"
        },
        "render": {
            "supersample": 1,
            "reconstruction": "tent"
        }
    }));
    assert!(!undersampled.ok);
    assert_reason_at(
        &undersampled,
        "final_photo_supersample_below_min",
        "$.render.supersample",
    );

    let redundant_msaa = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "photo": {
            "intent": "camera_behavior",
            "quality": "final"
        },
        "render": {
            "anti_aliasing": "msaa4",
            "supersample": 2,
            "reconstruction": "tent"
        }
    }));
    assert!(!redundant_msaa.ok);
    assert_reason_at(
        &redundant_msaa,
        "final_photo_redundant_msaa",
        "$.render.anti_aliasing",
    );

    let wrong_reconstruction = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "photo": {
            "intent": "camera_behavior",
            "quality": "final"
        },
        "render": {
            "anti_aliasing": "none",
            "supersample": 2,
            "reconstruction": "box"
        }
    }));
    assert!(!wrong_reconstruction.ok);
    assert_reason_at(
        &wrong_reconstruction,
        "final_photo_reconstruction_unsupported",
        "$.render.reconstruction",
    );
}

#[test]
fn scene_recipe_validation_rejects_manual_exposure_focus_and_accepts_authored_camera() {
    let fixed_exposure = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" }
        },
        "render": {
            "exposure_ev": 2.0
        }
    }));
    assert!(!fixed_exposure.ok);
    assert_reason_at(
        &fixed_exposure,
        "conflicting_photo_intent_setting",
        "$.render.exposure_ev",
    );

    let manual_focus = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" }
        },
        "render": {
            "depth_of_field": {
                "focus_distance": 3.0,
                "aperture_f_stop": 2.8,
                "radius_px": 4
            }
        }
    }));
    assert!(!manual_focus.ok);
    assert_reason_at(
        &manual_focus,
        "conflicting_photo_intent_setting",
        "$.render.depth_of_field.focus_distance",
    );

    let authored_camera = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" }
        },
        "cameras": [{
            "id": "manual",
            "kind": "perspective",
            "active": true,
            "transform": {
                "kind": "trs",
                "translation": [0.0, 0.0, 4.0]
            }
        }]
    }));
    assert!(
        authored_camera.ok,
        "camera behavior may preserve an authored camera while it still owns metering and focus: \
         {authored_camera:#?}"
    );
}

#[test]
fn scene_recipe_validation_accepts_camera_behavior_policy_subobjects_and_rejects_manual_staging() {
    let valid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" },
            "composition": {
                "view": "three_quarter_front_right",
                "fill_fraction": { "min": 0.65, "max": 0.85 },
                "max_center_offset_fraction": 0.16
            },
            "exposure": {
                "metering": "subject",
                "mean_luminance_srgb8": { "min": 80.0, "max": 100.0 },
                "max_low_clip_fraction": 0.20,
                "max_high_clip_fraction": 0.05
            },
            "focus": {
                "mode": "subject",
                "coverage": "all",
                "strength": "subtle"
            },
            "staging": {
                "environment": "bright_product_studio",
                "background": "dark_studio",
                "ground": "matte",
                "grid": false
            }
        }
    }));
    assert!(
        valid.ok,
        "camera behavior policy subobjects should validate: {valid:#?}"
    );

    let reflective = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" },
            "staging": { "ground": "reflective" }
        }
    }));
    assert!(
        reflective.ok,
        "camera behavior must accept the bounded reflective ground intent: {reflective:#?}"
    );

    for (recipe, expected_ground) in [
        ("colored_travel_mug.recipe.json", "reflective"),
        ("valve_manifold.recipe.json", "reflective"),
        ("dark_metal_speaker.recipe.json", "matte"),
        ("demo_hero.recipe.json", "reflective"),
    ] {
        let path = Path::new("tests/assets/photo/final/recipes").join(recipe);
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).expect("final recipe reads"))
                .expect("final recipe parses");
        assert_eq!(
            value
                .pointer("/photo/staging/ground")
                .and_then(serde_json::Value::as_str),
            Some(expected_ground),
            "{recipe} must pin its agreed ground intent"
        );
    }

    let manual_grid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" },
            "staging": {
                "grid": false
            }
        },
        "scene": {
            "grid": { "enabled": true }
        }
    }));
    assert!(!manual_grid.ok);
    assert_reason_at(
        &manual_grid,
        "conflicting_photo_intent_setting",
        "$.scene.grid.enabled",
    );

    let manual_background_color = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" },
            "staging": {
                "background": "dark_studio"
            }
        },
        "scene": {
            "background": { "kind": "color", "color": "#FFFFFF" }
        }
    }));
    assert!(!manual_background_color.ok);
    assert_reason_at(
        &manual_background_color,
        "conflicting_photo_intent_setting",
        "$.scene.background",
    );
}

#[test]
fn scene_recipe_validation_accepts_auto_exposure_compensation_only_with_auto_exposure() {
    let valid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "auto_exposure": "product_studio",
            "exposure_compensation_ev": 0.35
        }
    }));
    assert!(
        valid.ok,
        "exposure compensation should compose with auto exposure: {valid:#?}"
    );

    let without_auto = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "exposure_compensation_ev": 0.35
        }
    }));
    assert!(!without_auto.ok);
    assert_reason_at(
        &without_auto,
        "conflicting_exposure_settings",
        "$.render.exposure_compensation_ev",
    );

    let with_fixed_exposure = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "exposure_ev": 1.0,
            "exposure_compensation_ev": 0.35
        }
    }));
    assert!(!with_fixed_exposure.ok);
    assert_reason_at(
        &with_fixed_exposure,
        "conflicting_exposure_settings",
        "$.render.exposure_compensation_ev",
    );
}

#[test]
fn scene_recipe_validation_accepts_metering_modes_and_rejects_invalid_forms() {
    for metering in [
        json!({ "mode": "average" }),
        json!({ "mode": "center_weighted" }),
        json!({ "mode": "highlight_weighted" }),
        json!({
            "mode": "subject",
            "target": { "kind": "import", "id": "subject" }
        }),
        json!({
            "mode": "spot",
            "rect": { "x": 0.35, "y": 0.25, "width": 0.3, "height": 0.4 }
        }),
    ] {
        let report = scena::validate_scene_recipe_value(json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
            ],
            "render": {
                "auto_exposure": "product_studio",
                "metering": metering
            }
        }));
        assert!(
            report.ok,
            "metering form should validate cleanly: {report:#?}"
        );
    }

    let without_auto_exposure = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "metering": { "mode": "average" }
        }
    }));
    assert!(!without_auto_exposure.ok);
    assert_reason_at(
        &without_auto_exposure,
        "invalid_metering",
        "$.render.metering",
    );

    let missing_subject_target = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "auto_exposure": "product_studio",
            "metering": { "mode": "subject" }
        }
    }));
    assert!(!missing_subject_target.ok);
    assert_reason_at(
        &missing_subject_target,
        "invalid_metering",
        "$.render.metering.target",
    );

    let unknown_subject_import = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "auto_exposure": "product_studio",
            "metering": {
                "mode": "subject",
                "target": { "kind": "import", "id": "missing" }
            }
        }
    }));
    assert!(!unknown_subject_import.ok);
    assert_reason_at(
        &unknown_subject_import,
        "unknown_metering_target",
        "$.render.metering.target.id",
    );

    let spot_without_rect = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "auto_exposure": "product_studio",
            "metering": { "mode": "spot" }
        }
    }));
    assert!(!spot_without_rect.ok);
    assert_reason_at(
        &spot_without_rect,
        "invalid_metering",
        "$.render.metering.rect",
    );

    let spot_outside_viewport = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "auto_exposure": "product_studio",
            "metering": {
                "mode": "spot",
                "rect": { "x": 0.9, "y": 0.1, "width": 0.2, "height": 0.2 }
            }
        }
    }));
    assert!(!spot_outside_viewport.ok);
    assert_reason_at(
        &spot_outside_viewport,
        "invalid_metering",
        "$.render.metering.rect",
    );

    let unknown_mode = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "auto_exposure": "product_studio",
            "metering": { "mode": "magic" }
        }
    }));
    assert!(!unknown_mode.ok);
    assert_reason_at(&unknown_mode, "invalid_metering", "$.render.metering.mode");
}

#[test]
fn scene_recipe_validation_accepts_subject_spec_fallbacks_and_rejects_invalid_policies() {
    let subject_import = json!({
        "id": "subject",
        "uri": "tests/assets/gltf/cad_terminal_block.gltf"
    });

    let photo_subject_spec = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [subject_import.clone()],
        "photo": {
            "intent": "camera_behavior",
            "subject": {
                "target": { "kind": "import", "id": "subject" },
                "fallback": "error"
            }
        }
    }));
    assert!(
        photo_subject_spec.ok,
        "photo subject spec should validate cleanly: {photo_subject_spec:#?}"
    );

    let photo_direct_subject_stays_compatible = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [subject_import.clone()],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "kind": "import", "id": "subject" }
        }
    }));
    assert!(
        photo_direct_subject_stays_compatible.ok,
        "existing direct photo subject targets must remain valid: {photo_direct_subject_stays_compatible:#?}"
    );

    let subject_metering_default_fallback = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [subject_import.clone()],
        "render": {
            "auto_exposure": "product_studio",
            "metering": {
                "mode": "subject",
                "target": { "kind": "import", "id": "subject" }
            }
        }
    }));
    assert!(
        subject_metering_default_fallback.ok,
        "subject metering without fallback must default to error policy: {subject_metering_default_fallback:#?}"
    );

    let subject_metering_warning_fallback = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [subject_import.clone()],
        "render": {
            "auto_exposure": "product_studio",
            "metering": {
                "mode": "subject",
                "target": { "kind": "import", "id": "subject" },
                "fallback": "average_metering_with_warning"
            }
        }
    }));
    assert!(
        subject_metering_warning_fallback.ok,
        "subject metering should accept the explicit degraded fallback policy: {subject_metering_warning_fallback:#?}"
    );

    let invalid_photo_fallback = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [subject_import.clone()],
        "photo": {
            "intent": "camera_behavior",
            "subject": {
                "target": { "kind": "import", "id": "subject" },
                "fallback": "guess"
            }
        }
    }));
    assert!(!invalid_photo_fallback.ok);
    assert_reason_at(
        &invalid_photo_fallback,
        "invalid_subject_fallback",
        "$.photo.subject.fallback",
    );

    let missing_photo_target = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [subject_import.clone()],
        "photo": {
            "intent": "camera_behavior",
            "subject": { "fallback": "error" }
        }
    }));
    assert!(!missing_photo_target.ok);
    assert_reason_at(
        &missing_photo_target,
        "invalid_photo_subject",
        "$.photo.subject.target",
    );

    let invalid_metering_fallback = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [subject_import],
        "render": {
            "auto_exposure": "product_studio",
            "metering": {
                "mode": "subject",
                "target": { "kind": "import", "id": "subject" },
                "fallback": "guess"
            }
        }
    }));
    assert!(!invalid_metering_fallback.ok);
    assert_reason_at(
        &invalid_metering_fallback,
        "invalid_subject_fallback",
        "$.render.metering.fallback",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_shared_target_resolver_handles_import_nodes_and_candidates() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ]
    }))
    .expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "memory://shared-target-resolution.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("recipe builds");
    let manifest = &build.manifest;
    let import = manifest
        .imports
        .iter()
        .find(|import| import.id == "subject")
        .expect("subject import is in manifest");

    let mut expected_import_handles = BTreeSet::new();
    expected_import_handles.insert(import.import_handle);
    expected_import_handles.extend(import.root_handles.iter().copied());
    expected_import_handles.extend(import.primary_root);
    expected_import_handles.extend(import.nodes_by_path.values().copied());
    let expected_import_handles = expected_import_handles.into_iter().collect::<Vec<_>>();

    let import_handles = scena::resolve_scene_recipe_target_handles(
        manifest,
        &scena::SceneRecipeTargetV1::Import {
            id: "subject".to_owned(),
        },
        scena::SceneRecipeTargetResolutionMode::Subject,
    )
    .expect("whole import target resolves");
    assert_eq!(
        import_handles, expected_import_handles,
        "whole-import subject resolution must include every addressable subject handle"
    );

    let (node_path, node_handle) = import
        .nodes_by_path
        .iter()
        .next()
        .expect("fixture exposes imported node paths");
    let node_handles = scena::resolve_scene_recipe_target_handles(
        manifest,
        &scena::SceneRecipeTargetV1::Node {
            id: node_path.clone(),
        },
        scena::SceneRecipeTargetResolutionMode::Subject,
    )
    .expect("imported node path resolves through the shared resolver");
    assert_eq!(node_handles, vec![*node_handle]);

    let err = scena::resolve_scene_recipe_target_handles(
        manifest,
        &scena::SceneRecipeTargetV1::Import {
            id: "subjekt".to_owned(),
        },
        scena::SceneRecipeTargetResolutionMode::Subject,
    )
    .expect_err("misspelled import must fail with candidates");
    assert_eq!(
        err.kind,
        scena::SceneRecipeTargetResolutionErrorKind::Unresolved
    );
    assert!(
        err.candidates
            .iter()
            .any(|candidate| candidate == "subject"),
        "resolver must return nearest import candidates: {err:#?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_shared_target_resolver_reports_hidden_targets_distinctly() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "box_geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [
            { "id": "box_mat", "kind": "unlit", "base_color": "#777777" }
        ],
        "nodes": [{
            "id": "hidden_box",
            "geometry": "box_geo",
            "material": "box_mat",
            "visible": false
        }]
    }))
    .expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "memory://hidden-target-resolution.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("recipe builds");

    let err = scena::resolve_scene_recipe_target_handles(
        &build.manifest,
        &scena::SceneRecipeTargetV1::Node {
            id: "hidden_box".to_owned(),
        },
        scena::SceneRecipeTargetResolutionMode::Subject,
    )
    .expect_err(
        "hidden authored targets should report hidden, not resolve or masquerade as unresolved",
    );
    assert_eq!(
        err.kind,
        scena::SceneRecipeTargetResolutionErrorKind::Hidden
    );
    assert!(
        err.message.contains("hidden"),
        "hidden target diagnostic should name hidden visibility: {err:#?}"
    );
    assert!(
        err.candidates.is_empty(),
        "hidden target is an exact target-state failure, not a name lookup miss: {err:#?}"
    );
}

#[test]
fn scene_recipe_validation_accepts_subject_focus_and_rejects_ambiguous_dof_focus() {
    let subject_focus = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "depth_of_field": {
                "focus": {
                    "mode": "subject",
                    "target": { "kind": "import", "id": "subject" }
                },
                "coverage": "all",
                "strength": "subtle"
            }
        }
    }));
    assert!(
        subject_focus.ok,
        "subject focus depth-of-field should validate: {subject_focus:#?}"
    );

    let ambiguous = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "depth_of_field": {
                "focus_distance": 3.0,
                "focus": {
                    "mode": "subject",
                    "target": { "kind": "import", "id": "subject" }
                },
                "aperture_f_stop": 2.8,
                "radius_px": 4
            }
        }
    }));
    assert!(!ambiguous.ok);
    assert_reason_at(
        &ambiguous,
        "ambiguous_depth_of_field_focus",
        "$.render.depth_of_field.focus",
    );

    let bad_mode = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "depth_of_field": {
                "focus": {
                    "mode": "nearest",
                    "target": { "kind": "import", "id": "subject" }
                },
                "coverage": "all",
                "strength": "subtle"
            }
        }
    }));
    assert!(!bad_mode.ok);
    assert_reason_at(
        &bad_mode,
        "invalid_depth_of_field_focus",
        "$.render.depth_of_field.focus.mode",
    );

    let bad_coverage = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "depth_of_field": {
                "focus": {
                    "mode": "subject",
                    "target": { "kind": "import", "id": "subject" }
                },
                "coverage": "feature",
                "strength": "subtle"
            }
        }
    }));
    assert!(!bad_coverage.ok);
    assert_reason_at(
        &bad_coverage,
        "invalid_depth_of_field_focus",
        "$.render.depth_of_field.coverage",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_applies_auto_exposure_compensation_to_renderer() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "subject", "uri": "tests/assets/gltf/cad_terminal_block.gltf" }
        ],
        "render": {
            "auto_exposure": "product_studio",
            "exposure_compensation_ev": 0.35
        }
    }))
    .expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "memory://auto-exposure-compensation.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("recipe builds");
    let config = build
        .host
        .renderer()
        .auto_exposure()
        .expect("auto exposure is configured");
    assert!(
        (config.compensation_ev() - 0.35).abs() <= 1.0e-5,
        "recipe compensation should be installed on renderer config: {config:?}",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_rejects_non_ascii_hex_without_unwinding() {
    let text = serde_json::to_string(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {"bad": "€abc"}
    }))
    .expect("Unicode recipe serializes");
    let result = std::panic::catch_unwind(|| {
        pollster::block_on(scena::SceneHostCore::build_recipe_json(
            "memory://unicode-color.recipe.json",
            &text,
            scena::RecipeBuildPolicy::testing(),
        ))
    });
    let report = result
        .expect("recipe build validation must not unwind")
        .expect_err("non-ASCII hex must reject recipe build");
    assert_build_reason(&report, "invalid_color", "$.colors.bad");
}

#[test]
fn scene_recipe_validation_reports_future_sections_as_unsupported_features() {
    for section in ["primitives", "viewer_profile", "environment", "placements"] {
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
fn scene_recipe_validation_accepts_ergonomic_backbone_fields() {
    let report = scena::validate_scene_recipe_value(ergonomic_backbone_recipe());
    assert!(
        report.ok,
        "ergonomic recipe fields should validate cleanly: {report:#?}"
    );
}

#[test]
fn scene_recipe_validation_accepts_import_double_sided_material() {
    let report = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "cad_panel",
            "uri": "tests/assets/gltf/cad_plate_drawing_scene.gltf",
            "material": {
                "base_color": "#DDE2E5",
                "roughness": 0.72,
                "metallic": 0.1,
                "double_sided": true
            }
        }]
    }));
    assert!(
        report.ok,
        "import material double_sided is the CAD inspection backface contract and must validate: {report:#?}"
    );
}

#[test]
fn scene_recipe_validation_accepts_import_material_preset() {
    let report = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "cad_panel",
            "uri": "tests/assets/gltf/cad_plate_drawing_scene.gltf",
            "material": {
                "preset": "clearcoat_plastic",
                "base_color": "#D8C69A",
                "roughness": 0.34,
                "double_sided": true
            }
        }]
    }));
    assert!(
        report.ok,
        "import material preset should be available for imported CAD meshes: {report:#?}"
    );
}

#[test]
fn scene_recipe_validation_rejects_unknown_import_material_preset() {
    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "cad_panel",
            "uri": "tests/assets/gltf/cad_plate_drawing_scene.gltf",
            "material": {
                "preset": "premium_magic_plastic"
            }
        }]
    }));

    assert!(!invalid.ok);
    assert_reason_at(
        &invalid,
        "invalid_material_preset",
        "$.imports[0].material.preset",
    );
}

#[test]
fn scene_recipe_material_imperfection_accepts_only_fixed_bounded_profiles() {
    for profile in ["dust", "smudge", "fine_scratches", "oil_film"] {
        let valid = scena::validate_scene_recipe_value(json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": "tests/assets/gltf/cad_plate_drawing_scene.gltf",
                "material": {
                    "material_pack": { "uri": "materials/steel/scena-material-pack.json" },
                    "imperfection": {
                        "profile": profile,
                        "strength": 0.16,
                        "physical_scale_m": 0.003,
                        "seed": 42
                    }
                }
            }]
        }));
        assert!(
            valid.ok,
            "fixed imperfection profile {profile} must validate: {valid:#?}"
        );
    }

    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "materials": [{
            "id": "surface",
            "preset": "brushed_steel",
            "imperfection": {
                "profile": "procedural_damage",
                "strength": 1.2,
                "physical_scale_m": 0.0,
                "seed": -1
            }
        }]
    }));
    assert!(!invalid.ok);
    assert_reason_at(
        &invalid,
        "invalid_material_imperfection_profile",
        "$.materials[0].imperfection.profile",
    );
    assert_reason_at(
        &invalid,
        "invalid_material_imperfection_strength",
        "$.materials[0].imperfection.strength",
    );
    let strength_help = invalid
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "invalid_material_imperfection_strength")
        .map(|diagnostic| diagnostic.help.as_str())
        .unwrap();
    assert!(strength_help.contains("dust 0.30"));
    assert!(strength_help.contains("oil_film 0.65"));
    assert!(!strength_help.contains("0.08 through 0.20"));
    assert_reason_at(
        &invalid,
        "invalid_material_imperfection_scale",
        "$.materials[0].imperfection.physical_scale_m",
    );
    assert_reason_at(
        &invalid,
        "invalid_material_imperfection_seed",
        "$.materials[0].imperfection.seed",
    );

    for (recipe, expected_profile) in [
        ("dark_metal_speaker.recipe.json", "dust"),
        ("colored_travel_mug.recipe.json", "smudge"),
        ("valve_manifold.recipe.json", "fine_scratches"),
        ("demo_hero.recipe.json", "oil_film"),
    ] {
        let path = Path::new("tests/assets/photo/final/recipes").join(recipe);
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let profiles = value
            .pointer("/imports/0/material_bindings")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|binding| binding.pointer("/material/imperfection/profile"))
            .chain(
                value
                    .pointer("/materials")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|material| material.pointer("/imperfection/profile")),
            )
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        assert_eq!(profiles, vec![expected_profile], "{path:?}");
    }
}

#[test]
fn scene_recipe_validation_rejects_unknown_ergonomic_presets_at_exact_paths() {
    let invalid = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.12, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "body_mat", "preset": "miracle_metal" }
        ],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "body_mat" }
        ],
        "lights": [
            { "id": "studio", "kind": "studio_rig", "preset": "softbox" }
        ],
        "cameras": [
            {
                "id": "camera",
                "kind": "perspective",
                "lens": "cinema",
                "framing": { "preset": "hero_three_quarter", "fill": 0.0 },
                "active": true
            }
        ],
        "scene": {
            "environment": { "preset": "moonlight" }
        },
        "render": {
            "auto_exposure": "studio_magic"
        }
    }));

    assert!(!invalid.ok);
    assert_reason_at(&invalid, "invalid_material_preset", "$.materials[0].preset");
    assert_reason_at(&invalid, "invalid_light_preset", "$.lights[0].preset");
    assert_reason_at(&invalid, "invalid_camera_lens", "$.cameras[0].lens");
    assert_reason_at(
        &invalid,
        "invalid_camera_framing",
        "$.cameras[0].framing.preset",
    );
    assert_reason_at(
        &invalid,
        "invalid_camera_framing",
        "$.cameras[0].framing.fill",
    );
    assert_reason_at(
        &invalid,
        "invalid_environment",
        "$.scene.environment.preset",
    );
    assert_reason_at(&invalid, "invalid_render_setting", "$.render.auto_exposure");

    let conflicting_exposure = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.12, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "body_mat", "preset": "chrome" }
        ],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "body_mat" }
        ],
        "render": {
            "exposure_ev": 0.0,
            "auto_exposure": "product_studio"
        }
    }));
    assert_reason_at(
        &conflicting_exposure,
        "conflicting_exposure_settings",
        "$.render.auto_exposure",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_builds_photographic_surface_into_renderer_material_slots() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            {
                "id": "body_geo",
                "primitive": { "kind": "box", "size": [0.16, 0.10, 0.08], "bevel": 0.008 }
            }
        ],
        "materials": [
            {
                "id": "body_mat",
                "base_color": "#aeb4ba",
                "photographic_surface": {
                    "kind": "brushed_metal",
                    "tile_size_m": 0.12,
                    "feature_scale_m": 0.00035,
                    "variation": 0.7,
                    "wear": 0.12,
                    "seed": 9182,
                    "resolution": 32
                }
            }
        ],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "body_mat" }
        ]
    });
    let validation = scena::validate_scene_recipe_value(recipe.clone());
    assert!(validation.ok, "{validation:#?}");

    let text = serde_json::to_string(&recipe).expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/photographic-surface.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("photographic surface recipe builds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);

    let inspection = build
        .host
        .scene()
        .inspect_with_assets(build.host.assets())
        .to_schema_report();
    let material = inspection.draw_list[0]
        .material
        .as_ref()
        .expect("draw material is inspectable");
    for expected_slot in [
        "baseColorTexture",
        "normalTexture",
        "metallicRoughnessTexture",
        "occlusionTexture",
    ] {
        let slot = material
            .textures
            .iter()
            .find(|slot| slot.slot == expected_slot)
            .unwrap_or_else(|| panic!("missing generated {expected_slot} slot: {material:#?}"));
        assert_eq!(slot.decoded_dimensions, Some([32, 32]));
        assert!(slot.has_decoded_pixels);
        assert!(
            slot.source_path
                .starts_with("memory://scena/photographic-surface/v1/"),
            "generated texture must be owned by scena: {slot:#?}"
        );
    }
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_routes_ergonomic_fields_through_rust_helpers() {
    let text =
        serde_json::to_string(&ergonomic_backbone_recipe()).expect("ergonomic recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/ergonomic-backbone.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("ergonomic recipe builds through SceneHostCore");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert_eq!(
        build.host.renderer().auto_exposure(),
        Some(scena::AutoExposureConfig::product_studio()),
        "scene.preset/render.auto_exposure must reach Renderer::set_auto_exposure"
    );
    assert!(
        build
            .manifest
            .materials
            .iter()
            .any(|material| material.id == "body_mat" && material.kind == "pbr_metallic_roughness"),
        "material.preset should build a real PBR material resource: {:#?}",
        build.manifest.materials
    );
    for id in ["studio.key", "studio.fill", "studio.rim"] {
        assert!(
            build.manifest.lights.iter().any(|light| light.id == id),
            "studio_rig should expand through Scene::add_studio_lighting into {id}: {:#?}",
            build.manifest.lights
        );
    }
    assert!(
        build
            .manifest
            .cameras
            .iter()
            .any(|camera| camera.id == "camera" && camera.active == Some(true)),
        "camera.framing/lens should create and activate the authored camera: {:#?}",
        build.manifest.cameras
    );
    assert!(
        build.backend_selection_report().is_none(),
        "CPU recipe construction must not fabricate GPU selection evidence"
    );
    let scena::SceneHostRecipeBuild {
        host: _,
        manifest: _,
    } = build;
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
                "path": "translation",
                "times": [0.0, 1.5],
                "values": [[0.0, 0.0, 0.0], [0.1, 0.0, 0.0]]
            }, {
                "target": { "kind": "node", "id": "cube" },
                "path": "weights",
                "times": [0.0, 1.0],
                "values": [[0.0], [1.0]]
            }]
        }]
    }));
    assert!(!invalid.ok);
    assert_reason_at(
        &invalid,
        "unknown_animation_target",
        "$.animations[0].channels[0].target.id",
    );
    assert_reason_at(
        &invalid,
        "invalid_animation_times",
        "$.animations[0].channels[0].times[2]",
    );
    assert_reason_at(
        &invalid,
        "invalid_animation_values",
        "$.animations[0].channels[0].values",
    );
    assert_reason_at(
        &invalid,
        "invalid_animation_time",
        "$.animations[0].channels[1].times[1]",
    );
    assert_reason_at(
        &invalid,
        "invalid_animation_duration",
        "$.animations[0].channels[2].times[1]",
    );
    assert_reason_at(
        &invalid,
        "invalid_animation_target",
        "$.animations[0].channels[3].target.id",
    );
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
    let expected = include_str!("assets/stable-contracts/scene_recipe_build.v1.json");

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "scene_recipe.v1.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("stable scene recipe build succeeds");

    let actual = serde_json::to_string_pretty(&build.manifest).expect("build manifest serializes");
    assert_eq!(
        actual.trim_end(),
        expected.trim_end(),
        "stable build manifest fixture must be byte-stable output from the real executor"
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

    let oversized_text = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf" }
        ]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &oversized_text,
        scena::RecipeBuildPolicy::testing().with_max_recipe_bytes(16),
    ))
    .expect_err("oversized recipe text fails before JSON parsing");
    assert_build_reason(&report, "policy_violation", "$");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_policy_rejects_authored_allocation_bypasses() {
    let huge_torus = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [{
            "id": "bomb",
            "primitive": {
                "kind": "torus",
                "major_radius": 1.0,
                "minor_radius": 0.1,
                "segments": 65535,
                "rings": 65535
            }
        }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{ "id": "node", "geometry": "bomb", "material": "mat" }]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &huge_torus,
        scena::RecipeBuildPolicy::testing()
            .with_max_vertices(10_000)
            .with_max_indices(10_000),
    ))
    .expect_err("huge primitive must fail closed before tessellation");
    assert_build_reason(&report, "policy_violation", "$.geometries[0]");

    let aggregate = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "a", "primitive": { "kind": "plane", "size": [1.0, 1.0] } },
            { "id": "b", "primitive": { "kind": "plane", "size": [1.0, 1.0] } }
        ],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [
            { "id": "a_node", "geometry": "a", "material": "mat" },
            { "id": "b_node", "geometry": "b", "material": "mat" }
        ]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &aggregate,
        scena::RecipeBuildPolicy::testing().with_max_vertices(6),
    ))
    .expect_err("aggregate authored geometry budget fails closed");
    assert_build_reason(&report, "policy_violation", "$.geometries");

    let invalid_index = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [{
            "id": "bad_mesh",
            "mesh": {
                "topology": "triangles",
                "positions": [[0.0, 0.0, 0.0]],
                "indices": [0, 1, 2]
            }
        }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{ "id": "node", "geometry": "bad_mesh", "material": "mat" }]
    }));
    assert!(!invalid_index.ok, "out-of-range mesh indices fail closed");
    assert_reason_at(
        &invalid_index,
        "invalid_index",
        "$.geometries[0].mesh.indices[1]",
    );

    let invalid_colors = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [{
            "id": "bad_mesh",
            "mesh": {
                "topology": "triangles",
                "positions": [[0.0, 0.0, 0.0], [0.2, 0.0, 0.0], [0.0, 0.2, 0.0]],
                "colors": ["white"],
                "indices": [0, 1, 2]
            }
        }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{ "id": "node", "geometry": "bad_mesh", "material": "mat" }]
    }));
    assert!(
        !invalid_colors.ok,
        "mesh companion arrays must match positions"
    );
    assert_reason_at(
        &invalid_colors,
        "invalid_color_count",
        "$.geometries[0].mesh.colors",
    );

    let oversized_mesh = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [{
            "id": "mesh_a",
            "mesh": {
                "topology": "triangles",
                "positions": [[0.0, 0.0, 0.0], [0.2, 0.0, 0.0], [0.0, 0.2, 0.0]],
                "indices": [0, 1, 2]
            }
        }, {
            "id": "mesh_b",
            "mesh": {
                "topology": "triangles",
                "positions": [[0.0, 0.0, 0.0], [0.2, 0.0, 0.0], [0.0, 0.2, 0.0]],
                "indices": [0, 1, 2]
            }
        }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [
            { "id": "node_a", "geometry": "mesh_a", "material": "mat" },
            { "id": "node_b", "geometry": "mesh_b", "material": "mat" }
        ]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &oversized_mesh,
        scena::RecipeBuildPolicy::testing().with_max_vertices(5),
    ))
    .expect_err("aggregate custom mesh vertex budget fails closed");
    assert_build_reason(&report, "policy_violation", "$.geometries");

    let keyframe_over_cap = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{ "id": "node", "geometry": "geo", "material": "mat" }],
        "animations": [{
            "id": "too_many_keys",
            "duration": 2.0,
            "channels": [{
                "target": { "kind": "node", "id": "node" },
                "path": "translation",
                "times": [0.0, 1.0, 2.0],
                "values": [[0.0, 0.0, 0.0], [0.1, 0.0, 0.0], [0.2, 0.0, 0.0]]
            }]
        }]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &keyframe_over_cap,
        scena::RecipeBuildPolicy::testing().with_max_animation_keyframes(2),
    ))
    .expect_err("animation keyframes must be capped before typed build allocation");
    assert_build_reason(&report, "policy_violation", "$.animations[0].channels[0]");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_policy_rejects_arrow_projection_underestimate() {
    let arrow_underestimate = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "a", "primitive": { "kind": "arrow", "start": [0.0, 0.0, 0.0], "end": [1.0, 0.0, 0.0] } },
            { "id": "b", "primitive": { "kind": "arrow", "start": [0.0, 0.0, 0.0], "end": [0.0, 1.0, 0.0] } }
        ],
        "materials": [{ "id": "mat", "kind": "line", "base_color": "white" }],
        "nodes": [
            { "id": "a_node", "geometry": "a", "material": "mat" },
            { "id": "b_node", "geometry": "b", "material": "mat" }
        ]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &arrow_underestimate,
        scena::RecipeBuildPolicy::testing().with_max_vertices(10),
    ))
    .expect_err("arrow projection must fail closed before builder allocation exceeds the cap");
    assert_build_reason(&report, "policy_violation", "$.geometries");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_beveled_box_and_cylinder_build_with_real_geometry() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "steel": "#9EA7B3" },
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.24, 0.12, 0.16], "bevel": 0.01 } },
            { "id": "pin_geo", "primitive": { "kind": "cylinder", "radius": 0.04, "height": 0.22, "segments": 12, "fillet": 0.006 } }
        ],
        "materials": [{ "id": "mat", "kind": "pbr_metallic_roughness", "base_color": "steel", "metallic": 0.0, "roughness": 0.48 }],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "mat" },
            { "id": "pin", "geometry": "pin_geo", "material": "mat", "transform": { "kind": "trs", "translation": [0.22, 0.0, 0.0] } }
        ],
        "lights": [{ "id": "key", "kind": "directional", "preset": "key" }],
        "capture": { "width": 160, "height": 120 }
    }))
    .expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&recipe);
    assert!(
        validation.ok,
        "beveled primitives should validate: {validation:#?}"
    );
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/beveled-primitives.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("beveled primitive recipe builds");

    let body = build
        .manifest
        .geometries
        .iter()
        .find(|geometry| geometry.id == "body_geo")
        .expect("beveled box geometry is reported");
    let pin = build
        .manifest
        .geometries
        .iter()
        .find(|geometry| geometry.id == "pin_geo")
        .expect("beveled cylinder geometry is reported");
    assert_eq!(body.vertex_count, Some(96));
    assert_eq!(body.index_count, Some(132));
    assert_eq!(pin.vertex_count, Some(216));
    assert_eq!(pin.index_count, Some(288));
}

#[test]
fn scene_recipe_rejects_inert_bevel_knobs() {
    let report = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "geometries": [
            { "id": "ball", "primitive": { "kind": "sphere", "radius": 0.1, "bevel": 0.01 } },
            { "id": "ambiguous", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1], "bevel": 0.01, "fillet": 0.01 } }
        ]
    }));
    assert!(!report.ok);
    assert_reason_at(&report, "unsupported_feature", "$.geometries[0].primitive");
    assert_reason_at(&report, "invalid_primitive", "$.geometries[1].primitive");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_polyline_validation_and_build_reject_zero_or_one_point() {
    for points in [json!([]), json!([[0.0, 0.0, 0.0]])] {
        let value = json!({
            "schema": "scena.scene_recipe.v1",
            "geometries": [{
                "id": "rail",
                "primitive": {"kind": "polyline", "points": points}
            }]
        });
        let validation = scena::validate_scene_recipe_value(value.clone());
        assert_reason_at(
            &validation,
            "invalid_points",
            "$.geometries[0].primitive.points",
        );

        let text = serde_json::to_string(&value).expect("polyline recipe serializes");
        let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
            "memory://short-polyline.recipe.json",
            &text,
            scena::RecipeBuildPolicy::testing(),
        ))
        .expect_err("short polyline recipe must not build");
        assert_build_reason(&build, "invalid_points", "$.geometries[0].primitive.points");
    }
}

#[test]
fn scene_recipe_rejects_invalid_lod_levels() {
    let report = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "high", "primitive": { "kind": "cylinder", "radius": 0.5, "height": 1.0, "segments": 48 } }
        ],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{
            "id": "part",
            "geometry": "high",
            "material": "mat",
            "lods": [
                { "geometry": "missing", "max_screen_fraction": 0.2 },
                { "geometry": "high", "max_screen_fraction": 1.5 }
            ]
        }]
    }));
    assert!(!report.ok);
    assert_reason_at(
        &report,
        "unknown_geometry_ref",
        "$.nodes[0].lods[0].geometry",
    );
    assert_reason_at(
        &report,
        "invalid_lod_threshold",
        "$.nodes[0].lods[1].max_screen_fraction",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_lod_selects_lower_triangle_geometry_when_small_on_screen() {
    let near_triangles = render_lod_recipe_triangles(1.0);
    let far_triangles = render_lod_recipe_triangles(0.1);
    assert!(
        near_triangles > 100,
        "near subject should keep the high-detail geometry, got {near_triangles}"
    );
    assert!(
        far_triangles <= 32,
        "small projected subject should use the low-detail geometry, got {far_triangles}"
    );
    assert!(
        far_triangles < near_triangles,
        "LOD should reduce prepared triangle count, near={near_triangles}, far={far_triangles}"
    );
}

#[cfg(feature = "scene-host")]
fn render_lod_recipe_triangles(scale: f64) -> u64 {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "high", "primitive": { "kind": "cylinder", "radius": 0.5, "height": 1.0, "segments": 48 } },
            { "id": "low", "primitive": { "kind": "cylinder", "radius": 0.5, "height": 1.0, "segments": 6 } }
        ],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{
            "id": "part",
            "geometry": "high",
            "material": "mat",
            "transform": { "kind": "trs", "scale": [scale, scale, scale] },
            "lods": [{ "geometry": "low", "max_screen_fraction": 0.1 }]
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "fov_degrees": 40.0,
            "active": true,
            "transform": {
                "kind": "look_at",
                "eye": [0.0, 0.0, 3.0],
                "target": "part"
            }
        }],
        "capture": { "width": 160, "height": 120 }
    }))
    .expect("recipe serializes");
    let mut build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/lod.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("LOD recipe builds");
    build.host.prepare().expect("LOD scene prepares");
    build.host.render().expect("LOD scene renders");
    build.host.renderer().stats().triangles
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_rejects_imported_weight_animation_without_morph_targets() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
        }],
        "animations": [{
            "id": "bad_weights",
            "duration": 1.0,
            "channels": [{
                "target": { "kind": "node", "id": "part:/ColoredTriangle" },
                "path": "weights",
                "times": [0.0, 1.0],
                "values": [[0.0], [1.0]]
            }]
        }]
    }))
    .expect("recipe serializes");

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/imported-weights.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect_err("weights animation on non-morph imported node must fail closed");
    assert_build_reason(
        &report,
        "invalid_animation_target",
        "$.animations[0].channels[0].target.id",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_authored_animation_recipe_applies_scale_rotation_and_interpolation_modes() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{ "id": "node", "geometry": "geo", "material": "mat" }],
        "animations": [{
            "id": "scale_step",
            "duration": 1.0,
            "channels": [{
                "target": { "kind": "node", "id": "node" },
                "path": "scale",
                "interpolation": "step",
                "times": [0.0, 1.0],
                "values": [[1.0, 1.0, 1.0], [2.0, 3.0, 4.0]]
            }]
        }, {
            "id": "rotate_linear",
            "duration": 1.0,
            "channels": [{
                "target": { "kind": "node", "id": "node" },
                "path": "rotation",
                "interpolation": "linear",
                "times": [0.0, 1.0],
                "values": [
                    [0.0, 0.0, 0.0, 1.0],
                    [
                        0.0,
                        0.0,
                        std::f64::consts::FRAC_1_SQRT_2,
                        std::f64::consts::FRAC_1_SQRT_2
                    ]
                ]
            }]
        }, {
            "id": "move_cubic",
            "duration": 1.0,
            "channels": [{
                "target": { "kind": "node", "id": "node" },
                "path": "translation",
                "interpolation": "cubic_spline",
                "times": [0.0, 1.0],
                "values": [
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0]
                ]
            }]
        }]
    }))
    .expect("recipe serializes");

    let mut build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/authored-animation.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("authored animation recipe builds");
    let node_handle = build
        .manifest
        .nodes
        .iter()
        .find(|node| node.id == "node")
        .expect("node is in manifest")
        .handle;
    let animation_handle = |id: &str, manifest: &scena::SceneRecipeBuildV1| {
        manifest
            .animations
            .iter()
            .find(|animation| animation.id == id)
            .unwrap_or_else(|| panic!("missing animation {id}: {manifest:#?}"))
            .handle
    };

    build
        .host
        .seek_animation(animation_handle("scale_step", &build.manifest), 0.5)
        .expect("step scale seek applies");
    let transform = inspected_node_transform(&build.host, node_handle);
    assert_eq!(
        transform.scale,
        scena::Vec3::new(1.0, 1.0, 1.0),
        "STEP interpolation must hold the left value before the next key"
    );
    build
        .host
        .seek_animation(animation_handle("scale_step", &build.manifest), 1.0)
        .expect("step scale final seek applies");
    let transform = inspected_node_transform(&build.host, node_handle);
    assert_eq!(transform.scale, scena::Vec3::new(2.0, 3.0, 4.0));

    build
        .host
        .seek_animation(animation_handle("rotate_linear", &build.manifest), 0.5)
        .expect("linear rotation seek applies");
    let transform = inspected_node_transform(&build.host, node_handle);
    assert!(
        transform.rotation.z.abs() > 0.35 && transform.rotation.w < 0.95,
        "rotation channel must change from identity through recipe linear mapping: {:?}",
        transform.rotation
    );

    build
        .host
        .seek_animation(animation_handle("move_cubic", &build.manifest), 0.5)
        .expect("cubic translation seek applies");
    let transform = inspected_node_transform(&build.host, node_handle);
    assert!(
        (transform.translation.x - 0.5).abs() < 0.02,
        "cubic_spline recipe mapping must sample the Hermite midpoint, got {:?}",
        transform.translation
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_policy_rejects_authored_texture_and_environment_bypasses() {
    let texture_uri = "tests/assets/gltf/khronos/TextureTransformTest/Error.png";
    let textured_material = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "materials": [{
            "id": "mat",
            "kind": "unlit",
            "base_color": "white",
            "base_color_texture": { "uri": texture_uri }
        }],
        "geometries": [{ "id": "geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }],
        "nodes": [{ "id": "node", "geometry": "geo", "material": "mat" }]
    }))
    .expect("recipe serializes");

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &textured_material,
        scena::RecipeBuildPolicy::testing().with_max_textures(0),
    ))
    .expect_err("authored texture count cap fails closed");
    assert_build_reason(
        &report,
        "policy_violation",
        "$.materials[0].base_color_texture",
    );

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &textured_material,
        scena::RecipeBuildPolicy::testing().with_max_image_dimension(16),
    ))
    .expect_err("authored texture dimension cap fails closed");
    assert_build_reason(
        &report,
        "policy_violation",
        "$.materials[0].base_color_texture",
    );

    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &textured_material,
        scena::RecipeBuildPolicy::testing().with_max_texture_bytes(16),
    ))
    .expect_err("authored texture byte cap fails closed");
    assert_build_reason(
        &report,
        "policy_violation",
        "$.materials[0].base_color_texture",
    );

    let environment = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [{ "id": "geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{ "id": "node", "geometry": "geo", "material": "mat" }],
        "scene": {
            "environment": {
                "kind": "uri",
                "uri": "tests/assets/environment/polyhaven/studio_small_08_2k.hdr"
            }
        }
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &environment,
        scena::RecipeBuildPolicy::testing().with_fetch_byte_limit(16),
    ))
    .expect_err("environment fetch cap fails closed");
    assert_build_reason(&report, "policy_violation", "$.scene.environment.uri");

    let preset_environment = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [{ "id": "geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "white" }],
        "nodes": [{ "id": "node", "geometry": "geo", "material": "mat" }],
        "scene": { "environment": { "preset": "studio" } }
    }))
    .expect("preset environment recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &preset_environment,
        scena::RecipeBuildPolicy::testing().with_fetch_byte_limit(16),
    ))
    .expect_err("bundled environment fetch cap fails closed");
    assert_build_reason(&report, "policy_violation", "$.scene.environment.preset");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_build_policy_rejects_fail_open_path_sandboxes() {
    let recipe = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "/etc/passwd" }
        ]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &recipe,
        scena::RecipeBuildPolicy::testing().with_allowed_roots([]),
    ))
    .expect_err("empty allowed roots deny absolute paths");
    assert_build_reason(&report, "policy_violation", "$.imports[0].uri");

    let unknown_builtin_uri = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "scena://bundled/not-in-the-catalog.gltf" }
        ]
    }))
    .expect("unknown builtin recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &unknown_builtin_uri,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect_err("unknown scena uri is not treated as a trusted builtin");
    assert_build_reason(&report, "policy_violation", "$.imports[0].uri");

    let host_file_uri = serde_json::to_string_pretty(&json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [
            { "id": "part", "uri": "file://example.invalid/tmp/model.gltf" }
        ]
    }))
    .expect("recipe serializes");
    let report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/policy.recipe.json",
        &host_file_uri,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect_err("file uri authority fails closed");
    assert_build_reason(&report, "policy_violation", "$.imports[0].uri");
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
            "thickness_factor": 0.35,
            "attenuation_distance": 2.0,
            "attenuation_color": "blue",
            "clearcoat_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_roughness_texture": { "uri": texture, "color_space": "linear" },
            "clearcoat_normal_texture": { "uri": texture, "color_space": "linear" },
            "sheen_color_texture": { "uri": texture, "color_space": "srgb" },
            "sheen_roughness_texture": { "uri": texture, "color_space": "linear" },
            "anisotropy_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_texture": { "uri": texture, "color_space": "linear" },
            "iridescence_thickness_texture": { "uri": texture, "color_space": "linear" }
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

    let unsupported_gpu_textures = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "base": "#7C8798" },
        "materials": [{
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "transmission_texture": { "uri": texture, "color_space": "linear" },
            "thickness_texture": { "uri": texture, "color_space": "linear" }
        }]
    }));
    assert!(!unsupported_gpu_textures.ok);
    assert_reason_at(
        &unsupported_gpu_textures,
        "unsupported_feature",
        "$.materials[0].transmission_texture",
    );
    assert_reason_at(
        &unsupported_gpu_textures,
        "unsupported_feature",
        "$.materials[0].thickness_texture",
    );

    let invalid_ior = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "base": "#7C8798" },
        "materials": [{
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "ior": 0.9
        }]
    }));
    assert!(!invalid_ior.ok);
    assert_reason_at(&invalid_ior, "invalid_ior", "$.materials[0].ior");

    let zero_ior = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "base": "#7C8798" },
        "geometries": [{
            "id": "g",
            "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] }
        }],
        "materials": [{
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base",
            "ior": 0.0
        }],
        "nodes": [{
            "id": "n",
            "geometry": "g",
            "material": "advanced"
        }]
    }));
    assert!(
        zero_ior.ok,
        "ior:0.0 is MaterialDesc's documented sentinel and should validate: {zero_ior:#?}"
    );

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
    assert_reason_at(
        &invalid,
        "invalid_unit_value",
        "$.materials[0].clearcoat_factor",
    );
    assert_reason_at(
        &invalid,
        "unknown_color_ref",
        "$.materials[0].sheen_color_factor",
    );
    assert_reason_at(
        &invalid,
        "invalid_unit_value",
        "$.materials[0].anisotropy_strength_factor",
    );
    assert_reason_at(&invalid, "invalid_number", "$.materials[0].iridescence_ior");
    assert_reason_at(
        &invalid,
        "invalid_unit_value",
        "$.materials[0].transmission_factor",
    );
    assert_reason_at(
        &invalid,
        "invalid_number",
        "$.materials[0].attenuation_distance",
    );
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
            { "id": "cone_mat", "kind": "pbr_metallic_roughness", "base_color": "cone_color", "metallic": 0.0, "roughness": 0.55 },
            { "id": "torus_mat", "kind": "pbr_metallic_roughness", "base_color": "torus_color", "metallic": 0.0, "roughness": 0.55 },
            { "id": "disc_mat", "kind": "pbr_metallic_roughness", "base_color": "disc_color", "metallic": 0.0, "roughness": 0.55 },
            { "id": "wedge_mat", "kind": "pbr_metallic_roughness", "base_color": "wedge_color", "metallic": 0.0, "roughness": 0.55 }
        ],
        "lights": [{
            "id": "key",
            "kind": "directional",
            "preset": "key",
            "illuminance_lux": 9000.0
        }],
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
            { "id": "inside_out_torus", "primitive": { "kind": "torus", "major_radius": 0.1, "minor_radius": 0.1 } },
            { "id": "bad_disc", "primitive": { "kind": "disc", "radius": 0.1, "segments": 0 } },
            { "id": "bad_wedge", "primitive": { "kind": "wedge", "size": [0.2, 0.1] } }
        ]
    }));
    assert!(!invalid.ok);
    assert_reason(&invalid, "invalid_number", None);
    assert_reason(&invalid, "invalid_integer", None);
    assert_reason(&invalid, "invalid_vector", None);
    assert_reason_at(
        &invalid,
        "invalid_primitive",
        "$.geometries[2].primitive.minor_radius",
    );
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

    let too_small_policy = policy.clone().with_fetch_byte_limit(16);
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

    let corrupt_font_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/assets/fonts/corrupt-font.ttf");
    assert!(
        corrupt_font_path.exists(),
        "corrupt font fixture must be present"
    );
    let corrupt = json!({
        "schema": "scena.scene_recipe.v1",
        "fonts": [
            { "id": "corrupt_font", "uri": path_str(&corrupt_font_path) }
        ],
        "labels": [{
            "id": "font_label",
            "text": "AVAV",
            "font": "corrupt_font"
        }]
    });
    let corrupt_text = serde_json::to_string_pretty(&corrupt).expect("recipe serializes");
    let corrupt_report = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice11.recipe.json",
        &corrupt_text,
        policy.clone(),
    ))
    .expect_err("present but malformed font should fail closed");
    assert_build_reason(&corrupt_report, "font_load_failed", "$.fonts[0]");

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
#[test]
fn scene_recipe_slice12_skin_only_changes_headless_gpu_silhouette() {
    let undeformed = render_slice12_skin_morph_gpu(0.0, 0.0);
    let skinned = render_slice12_skin_morph_gpu(0.0, 0.28);

    assert!(
        skinned.min_y + 4 < undeformed.min_y,
        "HeadlessGpu must move the silhouette from authored skinning alone, undeformed={undeformed:?}, skinned={skinned:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice12_joint_animation_rebakes_headless_gpu_vertices() {
    let (assets, mut scene, camera, joint) = slice12_skin_morph_scene(0.0, 0.0);
    let mut renderer =
        scena::Renderer::headless_gpu(180, 140).expect("HeadlessGpu renderer builds");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial skinned scene prepares");
    renderer
        .render(&scene, camera)
        .expect("initial skinned scene renders");
    let initial = slice12_gpu_bounds(renderer.frame_rgba8(), 180, 140)
        .expect("initial skinned frame is visible");

    scene
        .set_transform(
            joint,
            scena::Transform::at(scena::Vec3::new(0.0, 0.28, 0.0)),
        )
        .expect("joint transform updates");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("moved skinned scene prepares");
    renderer
        .render(&scene, camera)
        .expect("moved skinned scene renders");
    let moved = slice12_gpu_bounds(renderer.frame_rgba8(), 180, 140)
        .expect("moved skinned frame is visible");

    assert!(
        moved.min_y + 4 < initial.min_y,
        "HeadlessGpu must re-bake skinned vertices after joint transform animation, initial={initial:?}, moved={moved:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice12_authored_morph_weight_animation_changes_rendered_output() {
    let mut recipe = slice12_skin_morph_recipe(0.0, 0.0);
    recipe.as_object_mut().expect("recipe is an object").insert(
        "animations".to_owned(),
        json!([{
            "id": "grow_tri",
            "duration": 1.0,
            "channels": [{
                "target": { "kind": "node", "id": "tri" },
                "path": "weights",
                "times": [0.0, 1.0],
                "values": [[0.0], [1.0]]
            }]
        }]),
    );
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let mut build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice12-animation.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("Slice 12 animated morph recipe builds");
    let animation = build
        .manifest
        .animations
        .iter()
        .find(|animation| animation.id == "grow_tri")
        .expect("authored morph animation appears in manifest")
        .handle;

    let initial = slice12_host_render_report(&mut build.host);
    let initial_bbox = initial
        .content_bbox_css_px
        .expect("initial animated morph frame has content bbox");
    build
        .host
        .seek_animation(animation, 1.0)
        .expect("authored morph animation seeks");
    let moved = slice12_host_render_report(&mut build.host);
    let moved_bbox = moved
        .content_bbox_css_px
        .expect("morphed animation frame has content bbox");

    assert!(
        moved_bbox.height > initial_bbox.height + 8.0,
        "authored morph weight animation must alter the rendered silhouette, initial={initial_bbox:?}, moved={moved_bbox:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice12_validation_rejects_skin_indices_outside_binding() {
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
        "skins": [{
            "id": "bad_skin",
            "source_geometry": "tri_base",
            "joints": [[1, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0]],
            "weights": [[1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]]
        }],
        "materials": [{ "id": "mat", "kind": "unlit", "base_color": "red" }],
        "nodes": [
            { "id": "joint", "geometry": "tri_base", "material": "mat", "visible": false },
            {
                "id": "tri",
                "geometry": "bad_skin",
                "material": "mat",
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
        ]
    }));
    assert_reason_at(&invalid, "invalid_skin", "$.skins[0].joints[0][0]");
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
    let (assets, mut scene, camera, _) = slice12_skin_morph_scene(morph_weight, joint_lift);
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
) -> (
    scena::Assets,
    scena::Scene,
    scena::CameraKey,
    scena::NodeKey,
) {
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

    (assets, scene, camera, joint)
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
    let report = slice12_host_render_report(&mut host);

    Slice12RenderProof {
        manifest: build.manifest,
        report,
    }
}

#[cfg(feature = "scene-host")]
fn slice12_host_render_report(
    host: &mut scena::SceneHostCore,
) -> scena::RenderIntrospectionReportV1 {
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
    report
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

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_rotation_degrees_uses_non_commuting_xyz_call_order() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "box_geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [
            { "id": "box_mat", "kind": "unlit", "base_color": "white" }
        ],
        "nodes": [{
            "id": "xyz",
            "geometry": "box_geo",
            "material": "box_mat",
            "transform": { "kind": "trs", "rotation_degrees": [90.0, 45.0, 0.0] }
        }, {
            "id": "zyx_probe",
            "geometry": "box_geo",
            "material": "box_mat",
            "transform": { "kind": "trs", "rotation_degrees": [0.0, 45.0, 90.0] }
        }]
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/rotation-order.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("rotation order recipe builds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);
    let handle = |id: &str| {
        build
            .manifest
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.handle)
            .unwrap_or_else(|| panic!("node {id} exists: {:#?}", build.manifest))
    };
    let xyz = inspected_node_transform(&build.host, handle("xyz")).rotation;
    let zyx_probe = inspected_node_transform(&build.host, handle("zyx_probe")).rotation;
    let expected = scena::Transform::default()
        .rotate_x_deg(90.0)
        .rotate_y_deg(45.0)
        .rotate_z_deg(0.0)
        .rotation;

    assert!(
        xyz.dot(expected).abs() > 0.999,
        "rotation_degrees must exactly follow Transform::rotate_x_deg -> rotate_y_deg -> rotate_z_deg, got {xyz:?}, expected {expected:?}"
    );
    assert!(
        xyz.dot(zyx_probe).abs() < 0.99,
        "the pinned rotation pair must be non-commuting; xyz={xyz:?}, zyx_probe={zyx_probe:?}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_import_node_paths_validate_and_build_for_authored_targets() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
        }],
        "colors": {
            "node": "#A0C8FF",
            "label": "#FFFFFF",
            "particle": "#30D060"
        },
        "geometries": [
            { "id": "placed_geo", "primitive": { "kind": "box", "size": [0.05, 0.05, 0.05] } }
        ],
        "materials": [
            { "id": "placed_mat", "kind": "unlit", "base_color": "node" }
        ],
        "nodes": [{
            "id": "placed_on_import",
            "geometry": "placed_geo",
            "material": "placed_mat",
            "transform": { "kind": "place_on", "target": "part:/ColoredTriangle" }
        }],
        "instance_sets": [{
            "id": "instances_on_import",
            "geometry": "placed_geo",
            "material": "placed_mat",
            "parent": "part:/ColoredTriangle",
            "transform": { "kind": "place_on", "target": "part:/ColoredTriangle" },
            "instances": [
                { "id": "i0", "transform": { "kind": "look_at", "eye": [0.0, 0.1, 0.3], "target": "part:/ColoredTriangle" } }
            ]
        }],
        "particles": [{
            "id": "import_particles",
            "parent": "part:/ColoredTriangle",
            "transform": { "kind": "place_on", "target": "part:/ColoredTriangle" },
            "particles": [
                { "id": "p0", "position": [0.0, 0.0, 0.0], "color": "particle", "size_px": 18.0 }
            ]
        }],
        "labels": [{
            "id": "import_label",
            "text": "ROOT",
            "parent": "part:/ColoredTriangle",
            "color": "label",
            "size_px": 18.0,
            "transform": { "kind": "look_at", "eye": [0.0, 0.1, 0.3], "target": "part:/ColoredTriangle" }
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.4, 0.3, 0.5], "target": "part:/ColoredTriangle" }
        }],
        "capture": { "width": 180, "height": 120 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "import node path ids should validate as authored targets: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/import-targets.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("import node target recipe builds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);
    for (id, kind) in [
        ("placed_on_import", "node"),
        ("instances_on_import", "instance_set"),
        ("import_particles", "particle_set"),
        ("import_label", "label"),
    ] {
        assert!(
            build
                .manifest
                .nodes
                .iter()
                .any(|node| node.id == id && node.kind == kind),
            "{id} should be targetable in the build manifest: {:#?}",
            build.manifest
        );
    }

    let mut host = build.host;
    host.prepare().expect("import target scene prepares");
    host.render().expect("import target scene renders");
    let capture = host.capture().expect("import target scene captures");
    let inspection_json = host.inspect_json().expect("import target scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(report.ok, "import target scene should render: {report:#?}");

    let unknown = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "imports": [{
            "id": "part",
            "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
        }],
        "colors": {
            "label": "#FFFFFF",
            "particle": "#30D060"
        },
        "particles": [{
            "id": "import_particles",
            "parent": "ghost:/Mesh",
            "particles": [
                { "id": "p0", "position": [0.0, 0.0, 0.0], "color": "particle", "size_px": 18.0 }
            ]
        }],
        "labels": [{
            "id": "import_label",
            "text": "ROOT",
            "parent": "ghost:/Mesh",
            "color": "label"
        }],
        "instance_sets": [{
            "id": "import_instances",
            "geometry": "placed_geo",
            "material": "placed_mat",
            "parent": "ghost:/Mesh",
            "instances": [
                { "id": "i0" }
            ]
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.4, 0.3, 0.5], "target": "ghost:/Mesh" }
        }]
    }));
    assert!(!unknown.ok);
    assert_reason_at(&unknown, "unknown_import_ref", "$.particles[0].parent");
    assert_reason_at(&unknown, "unknown_import_ref", "$.labels[0].parent");
    assert_reason_at(&unknown, "unknown_import_ref", "$.instance_sets[0].parent");
    assert_reason_at(
        &unknown,
        "unknown_import_ref",
        "$.cameras[0].transform.target",
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

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_scene_presets_apply_environment_background_and_floor() {
    for (preset, background) in [
        ("product_studio", scena::Background::Studio),
        ("cad_studio", scena::Background::NeutralGray),
        ("industrial_studio", scena::Background::DarkStudio),
    ] {
        let recipe = json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "body_color": "#7EA7E0"
            },
            "geometries": [
                { "id": "body_geo", "primitive": { "kind": "box", "size": [0.16, 0.08, 0.08] } }
            ],
            "materials": [
                { "id": "body_mat", "kind": "pbr_metallic_roughness", "base_color": "body_color", "roughness": 0.38, "metallic": 0.0 }
            ],
            "nodes": [
                { "id": "body", "geometry": "body_geo", "material": "body_mat", "transform": { "kind": "ground" } }
            ],
            "cameras": [
                { "id": "cam", "kind": "perspective", "active": true, "transform": { "kind": "look_at", "eye": [0.35, 0.24, 0.35], "target": "body" } }
            ],
            "scene": { "preset": preset },
            "capture": { "width": 96, "height": 72 }
        });
        let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");

        let validation = scena::validate_scene_recipe_json(&text);
        assert!(
            validation.ok,
            "{preset} should validate as a first-path recipe setup: {validation:#?}"
        );

        let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
            "tests/assets/scene-preset.recipe.json",
            &text,
            scena::RecipeBuildPolicy::testing(),
        ))
        .unwrap_or_else(|manifest| panic!("{preset} recipe build failed: {manifest:#?}"));
        assert!(build.manifest.ok, "{:#?}", build.manifest);
        assert!(
            build.host.renderer().environment().is_some(),
            "{preset} should apply a real environment"
        );
        assert_eq!(
            build.host.renderer().background_color(),
            background.color(),
            "{preset} should apply a matching studio background"
        );

        let inspection_json = build
            .host
            .inspect_json()
            .expect("scene preset recipe inspects");
        let inspection: scena::SceneInspectionReportV1 =
            serde_json::from_str(&inspection_json).expect("inspection decodes");
        assert!(
            inspection.counts.visible_drawable >= 3,
            "{preset} should add floor/grid drawables as real scene output: {inspection:#?}"
        );
    }
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
            "preset": "not_a_preset",
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
    assert_reason_at(&report, "invalid_scene_preset", "$.scene.preset");
    assert_reason(&report, "invalid_number", None);
    assert_reason(&report, "invalid_render_setting", None);
}

#[test]
fn scene_recipe_depth_of_field_quality_threshold_domains_match_metrics() {
    let valid = scena::validate_scene_recipe_value(json!({
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
        "expect": {
            "expect_quality": {
                "profile": "product",
                "depth_of_field": {
                    "min_source_background_sobel": 1.25,
                    "min_background_sobel_drop": 1.1,
                    "min_background_sobel_drop_fraction": 0.25,
                    "max_focal_mean_delta": 0.08
                }
            }
        }
    }));
    assert!(
        valid.ok,
        "Sobel-energy DoF thresholds are measured non-negative values, not unit fractions: {valid:#?}"
    );

    let invalid = scena::validate_scene_recipe_value(json!({
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
        "expect": {
            "expect_quality": {
                "profile": "product",
                "depth_of_field": {
                    "min_background_sobel_drop_fraction": 1.1
                }
            }
        }
    }));
    assert_reason_at(
        &invalid,
        "invalid_expect",
        "$.expect.expect_quality.depth_of_field.min_background_sobel_drop_fraction",
    );
}

#[test]
fn scene_recipe_exposure_quality_threshold_domains_match_subject_metrics() {
    let valid = scena::validate_scene_recipe_value(json!({
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
        "expect": {
            "expect_quality": {
                "profile": "product",
                "exposure": {
                    "min_mean_luminance_srgb8": 80.0,
                    "max_mean_luminance_srgb8": 100.0,
                    "max_low_clip_fraction": 0.2,
                    "max_high_clip_fraction": 0.05
                }
            }
        }
    }));
    assert!(
        valid.ok,
        "subject luminance bands are sRGB8 thresholds, while clip thresholds stay normalized: {valid:#?}"
    );

    let invalid_luminance = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "expect": {
            "expect_quality": {
                "profile": "product",
                "exposure": { "min_mean_luminance_srgb8": 300.0 }
            }
        }
    }));
    assert_reason_at(
        &invalid_luminance,
        "invalid_expect",
        "$.expect.expect_quality.exposure.min_mean_luminance_srgb8",
    );

    let inverted_band = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "expect": {
            "expect_quality": {
                "profile": "product",
                "exposure": {
                    "min_mean_luminance_srgb8": 120.0,
                    "max_mean_luminance_srgb8": 80.0
                }
            }
        }
    }));
    assert_reason_at(
        &inverted_band,
        "invalid_expect",
        "$.expect.expect_quality.exposure.min_mean_luminance_srgb8",
    );

    let invalid_fraction = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "expect": {
            "expect_quality": {
                "profile": "product",
                "exposure": { "max_low_clip_fraction": 2.0 }
            }
        }
    }));
    assert_reason_at(
        &invalid_fraction,
        "invalid_expect",
        "$.expect.expect_quality.exposure.max_low_clip_fraction",
    );
}

#[test]
fn scene_recipe_render_reconstruction_validates_fail_closed() {
    let valid = scena::validate_scene_recipe_value(json!({
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
        "render": {
            "anti_aliasing": "msaa4",
            "supersample": 2,
            "reconstruction": "tent"
        }
    }));
    assert!(
        valid.ok,
        "documented render reconstruction value should validate: {valid:#?}"
    );

    let invalid = scena::validate_scene_recipe_value(json!({
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
        "render": {
            "supersample": 2,
            "reconstruction": "lanczos"
        }
    }));
    assert!(
        !invalid.ok,
        "unknown reconstruction filters must fail closed"
    );
    assert_reason_at(
        &invalid,
        "invalid_render_setting",
        "$.render.reconstruction",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice4_render_settings_change_pixels_through_recipe() {
    let standard = render_slice4_recipe_cpu(slice4_emissive_panel_recipe(
        json!({ "anti_aliasing": "none", "tonemapper": "standard", "exposure_ev": 0.0 }),
    ));
    let aces = render_slice4_recipe_cpu(slice4_emissive_panel_recipe(
        json!({ "anti_aliasing": "none", "tonemapper": "aces", "exposure_ev": 0.0 }),
    ));
    assert!(
        recipe_frame_abs_diff(&standard, &aces) > 100,
        "recipe tonemapper setting must change encoded pixels"
    );

    let dim = render_slice4_recipe_cpu(slice4_gray_panel_recipe(
        json!({ "anti_aliasing": "none", "tonemapper": "standard", "exposure_ev": -1.0 }),
    ));
    let bright = render_slice4_recipe_cpu(slice4_gray_panel_recipe(
        json!({ "anti_aliasing": "none", "tonemapper": "standard", "exposure_ev": 1.0 }),
    ));
    assert!(
        recipe_frame_abs_diff(&dim, &bright) > 100,
        "recipe exposure_ev setting must change pixels"
    );

    let no_bloom = render_slice4_recipe_cpu(slice4_emissive_panel_recipe(
        json!({ "anti_aliasing": "none", "tonemapper": "standard", "exposure_ev": 0.0 }),
    ));
    let with_bloom = render_slice4_recipe_cpu(slice4_emissive_panel_recipe(json!({
        "anti_aliasing": "none",
        "tonemapper": "standard",
        "exposure_ev": 0.0,
        "bloom": { "threshold_srgb": 64, "intensity": 0.65, "radius_px": 3 }
    })));
    assert!(
        recipe_frame_abs_diff(&no_bloom, &with_bloom) > 100,
        "recipe bloom setting must change pixels"
    );

    let aliased = render_slice4_recipe_cpu(slice4_split_screen_recipe("none"));
    let smoothed = render_slice4_recipe_cpu(slice4_split_screen_recipe("fxaa"));
    let aliased_edge = recipe_pixel(&aliased, 8, 4, 4);
    let smoothed_edge = recipe_pixel(&smoothed, 8, 4, 4);
    assert_eq!(aliased_edge, [0, 0, 0, 255]);
    assert!(
        smoothed_edge[0] > aliased_edge[0] + 20,
        "recipe anti_aliasing setting must smooth the hard edge; aliased={aliased_edge:?} smoothed={smoothed_edge:?}"
    );

    let (ssao_off, ssao_off_stats) =
        render_slice4_recipe_cpu_with_stats(slice4_depth_contact_recipe(false));
    let (ssao_on, ssao_on_stats) =
        render_slice4_recipe_cpu_with_stats(slice4_depth_contact_recipe(true));
    assert_eq!(
        ssao_off_stats
            .get("ambient_occlusion_passes")
            .and_then(serde_json::Value::as_u64),
        Some(0),
        "recipe without ssao must not run the SSAO pass: {ssao_off_stats:#?}"
    );
    assert_eq!(
        ssao_on_stats
            .get("ambient_occlusion_passes")
            .and_then(serde_json::Value::as_u64),
        Some(1),
        "recipe ssao setting must run the SSAO pass: {ssao_on_stats:#?}"
    );
    let contact_drop =
        recipe_max_luma_drop_ring(&ssao_off, &ssao_on, 48, (14..28, 18..30), (18..24, 20..28));
    let baseline_open = recipe_average_luma_region(&ssao_off, 48, 8..14, 20..28);
    let ssao_open = recipe_average_luma_region(&ssao_on, 48, 8..14, 20..28);
    assert!(
        contact_drop >= 4,
        "recipe ssao setting must darken the depth contact; contact_drop={contact_drop}"
    );
    assert!(
        (baseline_open - ssao_open).abs() <= 2.0,
        "recipe ssao setting should leave open floor within tolerance; baseline={baseline_open:.2} ssao={ssao_open:.2}"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice4_grid_emits_visible_line_pixels() {
    let no_grid = render_slice4_recipe_cpu(slice4_grid_recipe(false));
    let with_grid = render_slice4_recipe_cpu(slice4_grid_recipe(true));
    assert!(
        recipe_frame_abs_diff(&no_grid, &with_grid) > 100,
        "recipe grid setting must change pixels"
    );
    assert!(
        with_grid
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 180),
        "recipe grid should draw visible light grid-line pixels"
    );
}

#[test]
fn scene_recipe_light_presets_fail_closed() {
    let report = scena::validate_scene_recipe_value(json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.1, 0.1, 0.1] } }
        ],
        "materials": [
            { "id": "body_mat", "kind": "unlit", "base_color": "white" }
        ],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "body_mat" }
        ],
        "lights": [{
            "id": "typo",
            "kind": "directional",
            "preset": "sunn"
        }, {
            "id": "cross_kind",
            "kind": "directional",
            "preset": "softbox"
        }, {
            "id": "spot_unsupported",
            "kind": "spot",
            "preset": "softbox"
        }]
    }));

    assert!(!report.ok, "invalid light presets must fail closed");
    assert_reason_at(&report, "invalid_light_preset", "$.lights[0].preset");
    assert_reason_at(&report, "invalid_light_preset", "$.lights[1].preset");
    assert_reason_at(&report, "unsupported_feature", "$.lights[2].preset");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_area_lights_validate_and_build_manifest_targets() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "warm": "#FFE2BA"
        },
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.4, 0.18, 0.24] } }
        ],
        "materials": [
            {
                "id": "body_mat",
                "kind": "pbr_metallic_roughness",
                "base_color": "#60708F",
                "metallic": 0.0,
                "roughness": 0.38
            }
        ],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "body_mat" }
        ],
        "lights": [{
            "id": "softbox_rect",
            "kind": "area",
            "shape": "rect",
            "preset": "softbox",
            "color": "warm",
            "width": 1.2,
            "height": 0.6,
            "luminous_flux_lumens": 3600.0,
            "transform": {
                "kind": "look_at",
                "eye": [0.0, 1.1, 0.9],
                "target": [0.0, 0.0, 0.0]
            }
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": {
                "kind": "look_at",
                "eye": [0.8, 0.45, 1.1],
                "target": "body"
            }
        }],
        "capture": { "width": 220, "height": 160 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let validation = scena::validate_scene_recipe_json(&text);
    assert!(
        validation.ok,
        "area-light recipe must validate: {validation:#?}"
    );

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice-a3-area-light.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("area-light recipe builds");

    assert!(build.manifest.ok, "{:#?}", build.manifest);
    assert!(
        build
            .manifest
            .lights
            .iter()
            .any(|light| light.id == "softbox_rect" && light.kind == "light"),
        "area light must be present in the typed build manifest: {:#?}",
        build.manifest.lights
    );
}

#[cfg(feature = "scene-host")]
fn render_slice4_recipe_cpu(recipe: serde_json::Value) -> Vec<u8> {
    render_slice4_recipe_cpu_with_stats(recipe).0
}

#[cfg(feature = "scene-host")]
fn render_slice4_recipe_cpu_with_stats(
    recipe: serde_json::Value,
) -> (Vec<u8>, serde_json::Map<String, serde_json::Value>) {
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice4-render-proof.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .unwrap_or_else(|error| panic!("Slice 4 render proof recipe builds: {error:#?}"));
    assert!(build.manifest.ok, "{:#?}", build.manifest);
    let mut host = build.host;
    host.prepare().expect("Slice 4 render proof prepares");
    host.render().expect("Slice 4 render proof renders");
    let stats = serde_json::from_str::<serde_json::Value>(&host.stats_json())
        .expect("Slice 4 render proof stats are JSON")
        .as_object()
        .expect("Slice 4 render proof stats are an object")
        .clone();
    (
        host.capture().expect("Slice 4 render proof captures").rgba8,
        stats,
    )
}

#[cfg(feature = "scene-host")]
fn slice4_emissive_panel_recipe(render: serde_json::Value) -> serde_json::Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "black": "#000000",
            "white": "#FFFFFF"
        },
        "geometries": [
            {
                "id": "panel_geo",
                "mesh": {
                    "topology": "triangles",
                    "positions": [[-0.16, -0.16, 0.0], [0.16, -0.16, 0.0], [0.16, 0.16, 0.0], [-0.16, 0.16, 0.0]],
                    "normals": [[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
                    "indices": [0, 1, 2, 0, 2, 3]
                }
            }
        ],
        "materials": [
            {
                "id": "panel_mat",
                "kind": "pbr_metallic_roughness",
                "base_color": "black",
                "metallic": 0.0,
                "roughness": 0.5,
                "emissive": "white",
                "emissive_strength": 4.0,
                "double_sided": true
            }
        ],
        "nodes": [
            { "id": "panel", "geometry": "panel_geo", "material": "panel_mat" }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.7320508], "target": [0.0, 0.0, 0.0] }
        }],
        "scene": { "background": { "kind": "black" } },
        "render": render,
        "capture": { "width": 32, "height": 32 }
    })
}

#[cfg(feature = "scene-host")]
fn slice4_split_screen_recipe(anti_aliasing: &str) -> serde_json::Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "white": "#FFFFFF" },
        "geometries": [{
            "id": "left_geo",
            "mesh": {
                "topology": "triangles",
                "positions": [[-1.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 1.0, 0.0], [-1.0, 1.0, 0.0]],
                "indices": [0, 1, 2, 0, 2, 3]
            }
        }],
        "materials": [
            { "id": "white_mat", "kind": "unlit", "base_color": "white", "double_sided": true }
        ],
        "nodes": [
            { "id": "left", "geometry": "left_geo", "material": "white_mat" }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.7320508], "target": [0.0, 0.0, 0.0] }
        }],
        "scene": { "background": { "kind": "black" } },
        "render": { "anti_aliasing": anti_aliasing },
        "capture": { "width": 8, "height": 8 }
    })
}

#[cfg(feature = "scene-host")]
fn slice4_gray_panel_recipe(render: serde_json::Value) -> serde_json::Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "gray": "#404040" },
        "geometries": [
            {
                "id": "panel_geo",
                "mesh": {
                    "topology": "triangles",
                    "positions": [[-0.45, -0.45, 0.0], [0.45, -0.45, 0.0], [0.45, 0.45, 0.0], [-0.45, 0.45, 0.0]],
                    "indices": [0, 1, 2, 0, 2, 3]
                }
            }
        ],
        "materials": [
            { "id": "panel_mat", "kind": "unlit", "base_color": "gray", "double_sided": true }
        ],
        "nodes": [
            { "id": "panel", "geometry": "panel_geo", "material": "panel_mat" }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.7320508], "target": [0.0, 0.0, 0.0] }
        }],
        "scene": { "background": { "kind": "black" } },
        "render": render,
        "capture": { "width": 32, "height": 32 }
    })
}

#[cfg(feature = "scene-host")]
fn slice4_depth_contact_recipe(ssao: bool) -> serde_json::Value {
    let render = if ssao {
        json!({
            "anti_aliasing": "none",
            "ssao": { "radius_px": 4, "intensity": 0.8, "depth_threshold": 0.0 }
        })
    } else {
        json!({ "anti_aliasing": "none" })
    };
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "white": "#FFFFFF",
            "block_color": "#B8B8B8"
        },
        "geometries": [{
            "id": "floor_geo",
            "mesh": {
                "topology": "triangles",
                "positions": [[-0.75, -0.55, 0.0], [0.75, -0.55, 0.0], [0.75, 0.35, 0.0], [-0.75, 0.35, 0.0]],
                "indices": [0, 1, 2, 0, 2, 3]
            }
        }, {
            "id": "block_geo",
            "mesh": {
                "topology": "triangles",
                "positions": [[-0.14, -0.18, 0.16], [0.14, -0.18, 0.16], [0.14, 0.18, 0.16], [-0.14, 0.18, 0.16]],
                "indices": [0, 1, 2, 0, 2, 3]
            }
        }],
        "materials": [
            { "id": "floor_mat", "kind": "unlit", "base_color": "white", "double_sided": true },
            { "id": "block_mat", "kind": "unlit", "base_color": "block_color", "double_sided": true }
        ],
        "nodes": [
            { "id": "floor", "geometry": "floor_geo", "material": "floor_mat" },
            { "id": "block", "geometry": "block_geo", "material": "block_mat" }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.7320508], "target": [0.0, 0.0, 0.0] }
        }],
        "scene": { "background": { "kind": "black" } },
        "render": render,
        "capture": { "width": 48, "height": 48 }
    })
}

#[cfg(feature = "scene-host")]
fn slice4_grid_recipe(enabled: bool) -> serde_json::Value {
    let scene = if enabled {
        json!({
            "background": { "kind": "black" },
            "grid": {
                "enabled": true,
                "floor_y": -0.08,
                "padding": 0.04,
                "line_spacing": 0.04,
                "color": "#000000",
                "line_color": "#FFFFFF",
                "roughness": 1.0
            }
        })
    } else {
        json!({ "background": { "kind": "black" } })
    };
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "body_color": "#303030" },
        "geometries": [
            { "id": "body_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "body_mat", "kind": "unlit", "base_color": "body_color", "double_sided": true }
        ],
        "nodes": [
            { "id": "body", "geometry": "body_geo", "material": "body_mat" }
        ],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.18, 0.2, 0.28], "target": "body" }
        }],
        "scene": scene,
        "render": { "anti_aliasing": "none" },
        "capture": { "width": 64, "height": 64 }
    })
}

#[cfg(feature = "scene-host")]
fn recipe_frame_abs_diff(before: &[u8], after: &[u8]) -> u64 {
    assert_eq!(before.len(), after.len(), "frames must match");
    before
        .iter()
        .zip(after)
        .map(|(before, after)| u64::from(before.abs_diff(*after)))
        .sum()
}

#[cfg(feature = "scene-host")]
fn recipe_pixel(frame: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let start = (y * width + x) * 4;
    frame[start..start + 4]
        .try_into()
        .expect("pixel has four channels")
}

#[cfg(feature = "scene-host")]
fn recipe_luma_at(frame: &[u8], width: u32, x: u32, y: u32) -> u8 {
    let offset = ((y * width + x) * 4) as usize;
    let red = frame[offset] as f32;
    let green = frame[offset + 1] as f32;
    let blue = frame[offset + 2] as f32;
    (0.299 * red + 0.587 * green + 0.114 * blue).round() as u8
}

#[cfg(feature = "scene-host")]
fn recipe_average_luma_region(
    frame: &[u8],
    width: u32,
    region_x: std::ops::Range<u32>,
    region_y: std::ops::Range<u32>,
) -> f32 {
    let mut total = 0_u64;
    let mut count = 0_u64;
    for y in region_y {
        for x in region_x.clone() {
            total += u64::from(recipe_luma_at(frame, width, x, y));
            count += 1;
        }
    }
    total as f32 / count.max(1) as f32
}

#[cfg(feature = "scene-host")]
fn recipe_max_luma_drop_ring(
    before: &[u8],
    after: &[u8],
    width: u32,
    outer: (std::ops::Range<u32>, std::ops::Range<u32>),
    inner: (std::ops::Range<u32>, std::ops::Range<u32>),
) -> u8 {
    let mut max_drop = 0_u8;
    for y in outer.1 {
        for x in outer.0.clone() {
            if inner.0.contains(&x) && inner.1.contains(&y) {
                continue;
            }
            let drop = recipe_luma_at(before, width, x, y)
                .saturating_sub(recipe_luma_at(after, width, x, y));
            max_drop = max_drop.max(drop);
        }
    }
    max_drop
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

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice6_rejects_clipping_plane_count_above_renderer_cap() {
    let limit = scena::Capabilities::for_backend(scena::Backend::Headless).max_clipping_planes;
    let clipping_planes = (0..=limit)
        .map(|index| {
            json!({
                "id": format!("p{index}"),
                "normal": [1.0, 0.0, 0.0],
                "distance": f64::from(index) * 0.01
            })
        })
        .collect::<Vec<_>>();
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "part": "#4A90E2" },
        "geometries": [
            { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.08, 0.08, 0.08] } }
        ],
        "materials": [
            { "id": "cube_mat", "kind": "unlit", "base_color": "part" }
        ],
        "nodes": [
            { "id": "cube", "geometry": "cube_geo", "material": "cube_mat" }
        ],
        "clipping_planes": clipping_planes,
        "capture": { "width": 64, "height": 64 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice6-too-many-clipping-planes.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect_err("clipping plane count above renderer cap must fail the build");

    assert!(!build.ok, "{build:#?}");
    assert_build_reason(&build, "policy_violation", "$.clipping_planes");
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_recipe_slice6_inactive_clipping_plane_leaves_geometry_intact() {
    let recipe = json!({
        "schema": "scena.scene_recipe.v1",
        "colors": { "part": "#F6C85F" },
        "geometries": [
            { "id": "cube_geo", "primitive": { "kind": "box", "size": [0.32, 0.32, 0.32] } }
        ],
        "materials": [
            { "id": "cube_mat", "kind": "unlit", "base_color": "part", "double_sided": true }
        ],
        "nodes": [
            { "id": "cube", "geometry": "cube_geo", "material": "cube_mat" }
        ],
        "clipping_planes": [{
            "id": "would_clip_everything",
            "normal": [1.0, 0.0, 0.0],
            "distance": -10.0,
            "active": false
        }],
        "cameras": [{
            "id": "main",
            "kind": "perspective",
            "active": true,
            "transform": { "kind": "look_at", "eye": [0.0, 0.0, 1.4], "target": "cube" }
        }],
        "capture": { "width": 96, "height": 96 }
    });
    let text = serde_json::to_string_pretty(&recipe).expect("recipe serializes");
    let validation = scena::validate_scene_recipe_json(&text);
    assert!(validation.ok, "{validation:#?}");

    let build = pollster::block_on(scena::SceneHostCore::build_recipe_json(
        "tests/assets/slice6-inactive-clipping-plane.recipe.json",
        &text,
        scena::RecipeBuildPolicy::testing(),
    ))
    .expect("inactive clipping plane recipe builds");
    assert!(build.manifest.ok, "{:#?}", build.manifest);

    let mut host = build.host;
    host.prepare().expect("inactive clipping scene prepares");
    host.render().expect("inactive clipping scene renders");
    let capture = host.capture().expect("inactive clipping scene captures");
    let inspection_json = host
        .inspect_json()
        .expect("inactive clipping scene inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    let report = host.renderer().introspect_capture(
        &capture,
        &inspection,
        scena::RenderIntrospectionOptions::summary(),
    );
    assert!(report.ok, "{report:#?}");
    assert!(
        report.framing.fit_fraction > 0.2 && !report.framing.tiny_in_frame,
        "inactive clipping plane must leave the subject visibly intact: {report:#?}"
    );
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

#[cfg(feature = "scene-host")]
fn inspected_node_transform(host: &scena::SceneHostCore, handle: u64) -> scena::Transform {
    let inspection_json = host.inspect_json().expect("scene host inspects");
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_str(&inspection_json).expect("inspection decodes");
    inspection
        .nodes
        .iter()
        .find(|node| node.handle == handle)
        .unwrap_or_else(|| panic!("missing node handle {handle}: {inspection:#?}"))
        .local_transform
}

fn ergonomic_backbone_recipe() -> serde_json::Value {
    json!({
        "schema": "scena.scene_recipe.v1",
        "colors": {
            "accent": "orange",
            "floor_tint": "studio_backdrop"
        },
        "geometries": [
            {
                "id": "body_geo",
                "primitive": { "kind": "box", "size": [0.16, 0.10, 0.08], "bevel": 0.01 }
            }
        ],
        "materials": [
            {
                "id": "body_mat",
                "preset": "chrome",
                "roughness": 0.06
            },
            {
                "id": "floor_mat",
                "preset": "matte",
                "base_color": "floor_tint"
            }
        ],
        "nodes": [
            {
                "id": "product",
                "geometry": "body_geo",
                "material": "body_mat",
                "transform": { "kind": "trs", "translation": [0.0, 0.05, 0.0] }
            }
        ],
        "lights": [
            { "id": "studio", "kind": "studio_rig", "preset": "studio_rig" }
        ],
        "cameras": [
            {
                "id": "camera",
                "kind": "perspective",
                "lens": "portrait",
                "framing": {
                    "preset": "three_quarter_front_right",
                    "fill": 0.66,
                    "margin_px": 10.0
                },
                "active": true
            }
        ],
        "scene": {
            "preset": "product_studio",
            "environment": { "preset": "studio" },
            "grid": { "enabled": true, "under_bounds": true, "padding": 0.12 }
        },
        "render": {
            "auto_exposure": "product_studio",
            "screen_space_reflections": {
                "strength": 0.5,
                "roughness": 0.24,
                "horizon_fraction": 0.42,
                "fade": 0.18
            }
        },
        "capture": { "width": 320, "height": 240 },
        "expect": {
            "expect_visible": [{ "id": "product-visible", "target": { "kind": "node", "id": "product" } }],
            "expect_backend": { "backend": "headless", "gpu_device": false },
            "expect_quality": { "profile": "product" }
        }
    })
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

fn assert_reason_at(report: &scena::SceneRecipeValidationReportV1, code: &str, path: &str) {
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.path == path),
        "missing diagnostic {code} at {path}: {report:#?}",
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
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Helvetica.ttf",
        "/Library/Fonts/Arial.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\calibri.ttf",
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
