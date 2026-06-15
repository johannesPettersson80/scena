use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::path::PathBuf;

use scena::{
    Aabb, AnnotationProjectionReportV1, AssetCatalogV1, AssetDoctorReportV1, AssetLoadReportV1,
    AssetProvenance, AssetReadinessReportV1, Backend, CAPABILITY_REPORT_SCHEMA_V1, Capabilities,
    CapabilityReport, CapabilityReportV1, CaptureBaselineReport, CaptureDescriptor, Color,
    GeometryTopology, Quat, SceneAssetGeometrySummary, Transform, Vec3,
};

#[test]
fn value_types_round_trip_through_stable_serde_shapes() {
    let transform = Transform {
        translation: Vec3::new(1.0, 2.0, 3.0),
        rotation: Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),
        scale: Vec3::new(2.0, 3.0, 4.0),
    };

    let transform_json = serde_json::to_value(transform).expect("transform serializes");
    assert_eq!(
        transform_json,
        json!({
            "translation": [1.0, 2.0, 3.0],
            "rotation": [0.0, 0.0, 0.0, 1.0],
            "scale": [2.0, 3.0, 4.0],
        })
    );
    let decoded_transform: Transform =
        serde_json::from_value(transform_json).expect("transform deserializes");
    assert_eq!(decoded_transform, transform);

    let bounds = Aabb {
        min: Vec3::new(-1.0, -2.0, -3.0),
        max: Vec3::new(4.0, 5.0, 6.0),
    };
    let bounds_json = serde_json::to_value(bounds).expect("aabb serializes");
    assert_eq!(
        bounds_json,
        json!({
            "min": [-1.0, -2.0, -3.0],
            "max": [4.0, 5.0, 6.0],
        })
    );
    let decoded_bounds: Aabb = serde_json::from_value(bounds_json).expect("aabb deserializes");
    assert_eq!(decoded_bounds, bounds);

    let color = Color::from_linear_rgba(0.25, 0.5, 0.75, 1.0);
    let color_json = serde_json::to_value(color).expect("color serializes");
    assert_eq!(
        color_json,
        json!({
            "r": 0.25,
            "g": 0.5,
            "b": 0.75,
            "a": 1.0,
        })
    );
    let decoded_color: Color = serde_json::from_value(color_json).expect("color deserializes");
    assert_eq!(decoded_color, color);

    let topology_json =
        serde_json::to_value(GeometryTopology::Triangles).expect("topology serializes");
    assert_eq!(topology_json, json!("triangles"));
    let decoded_topology: GeometryTopology =
        serde_json::from_value(topology_json).expect("topology deserializes");
    assert_eq!(decoded_topology, GeometryTopology::Triangles);
}

#[test]
fn capability_report_schema_is_versioned_and_round_trips() {
    let report = CapabilityReport::new(Capabilities::for_backend(Backend::Headless), None);

    let schema_json = report.to_schema_json();

    assert_eq!(schema_json["schema"], CAPABILITY_REPORT_SCHEMA_V1);
    assert_eq!(schema_json["capabilities"]["backend"], "headless");
    assert_eq!(schema_json["capabilities"]["forward_pbr"], "degraded");
    assert!(
        schema_json["diagnostics"]
            .as_array()
            .expect("diagnostics is an array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "forward_pbr_degraded"
                && diagnostic["severity"] == "warning"),
        "capability diagnostics should use stable serde names"
    );

    let decoded: CapabilityReportV1 =
        serde_json::from_value(schema_json).expect("schema report deserializes");
    assert_eq!(decoded.schema, CAPABILITY_REPORT_SCHEMA_V1);
    assert_eq!(decoded.capabilities.backend, Backend::Headless);
    assert_eq!(decoded.capabilities.color_target_format, "Rgba8UnormSrgb");
    assert_eq!(decoded.post_processing, None);
}

#[test]
fn stable_contract_golden_fixtures_are_versioned_json() {
    let fixtures = [
        (
            "tests/assets/stable-contracts/capability_report.v1.json",
            "scena.capability_report.v1",
        ),
        (
            "tests/assets/stable-contracts/schema_catalog.v1.json",
            "scena.schema_catalog.v1",
        ),
        (
            "tests/assets/stable-contracts/schema_entry.v1.json",
            "scena.schema_entry.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_inspection.v1.json",
            "scena.scene_inspection.v1",
        ),
        (
            "tests/assets/stable-contracts/capture.v1.json",
            "scena.capture.v1",
        ),
        (
            "tests/assets/stable-contracts/capture_baseline.v1.json",
            "scena.capture_baseline.v1",
        ),
        (
            "tests/assets/stable-contracts/render_introspection.v1.json",
            "scena.render_introspection.v1",
        ),
        (
            "tests/assets/stable-contracts/visibility_diagnosis.v1.json",
            "scena.visibility_diagnosis.v1",
        ),
        (
            "tests/assets/stable-contracts/visual_repair_plan.v1.json",
            "scena.visual_repair_plan.v1",
        ),
        (
            "tests/assets/stable-contracts/agent_loop_result.v1.json",
            "scena.agent_loop_result.v1",
        ),
        (
            "tests/assets/stable-contracts/appearance_expectation.v1.json",
            "scena.appearance_expectation.v1",
        ),
        (
            "tests/assets/stable-contracts/appearance_introspection.v1.json",
            "scena.appearance_introspection.v1",
        ),
        (
            "tests/assets/stable-contracts/animation_introspection.v1.json",
            "scena.animation_introspection.v1",
        ),
        (
            "tests/assets/stable-contracts/interaction_expectation.v1.json",
            "scena.interaction_expectation.v1",
        ),
        (
            "tests/assets/stable-contracts/interaction_verification.v1.json",
            "scena.interaction_verification.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_recipe.v1.json",
            "scena.scene_recipe.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_recipe_validation.v1.json",
            "scena.scene_recipe_validation.v1",
        ),
        (
            "tests/assets/stable-contracts/placement_result.v1.json",
            "scena.placement_result.v1",
        ),
        (
            "tests/assets/stable-contracts/annotation_projection.v1.json",
            "scena.annotation_projection.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_geometry_summary.v1.json",
            "scena.asset_geometry_summary.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_load_report.v1.json",
            "scena.asset_load_report.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_doctor.v1.json",
            "scena.asset_doctor.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_catalog.v1.json",
            "scena.asset_catalog.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_readiness_report.v1.json",
            "scena.asset_readiness_report.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_asset_import.v1.json",
            "scena.scene_host_asset_import.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_subtree.v1.json",
            "scena.subtree.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_measurement_overlay.v1.json",
            "scena.scene_host_measurement_overlay.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_section_box.v1.json",
            "scena.scene_host_section_box.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_visual_state.v1.json",
            "scena.scene_host_visual_state.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_visual_states.v1.json",
            "scena.scene_host_visual_states.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_animation_inventory.v1.json",
            "scena.animation_inventory.v1",
        ),
        (
            "tests/assets/stable-contracts/visual_patch.v1.json",
            "scena.visual_patch.v1",
        ),
        (
            "tests/assets/stable-contracts/visual_patch_result.v1.json",
            "scena.visual_patch.v1",
        ),
        (
            "tests/assets/stable-contracts/host_event.v1.json",
            "scena.host_event.v1",
        ),
    ];

    for (rel, schema) in fixtures {
        let fixture = read_fixture_json(rel);
        assert_eq!(fixture["schema"], schema, "{rel} schema drifted");
    }

    let provenance = read_fixture_json("tests/assets/stable-contracts/asset_provenance.json");
    for field in [
        "source_path",
        "source_sha256",
        "license",
        "generator",
        "derivatives",
    ] {
        assert!(
            provenance.get(field).is_some(),
            "asset provenance fixture missing {field}"
        );
    }
    assert!(
        provenance.get("schema").is_none(),
        "AssetProvenance is a nested value contract and must not grow a standalone schema"
    );
}

#[test]
fn capability_report_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<CapabilityReportV1>(
        "tests/assets/stable-contracts/capability_report.v1.json",
    );
}

#[test]
fn schema_catalog_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SchemaCatalogV1>(
        "tests/assets/stable-contracts/schema_catalog.v1.json",
    );
}

#[test]
fn schema_entry_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SchemaEntryReportV1>(
        "tests/assets/stable-contracts/schema_entry.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn scene_inspection_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneInspectionReportV1>(
        "tests/assets/stable-contracts/scene_inspection.v1.json",
    );
}

#[test]
fn capture_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<CaptureDescriptor>(
        "tests/assets/stable-contracts/capture.v1.json",
    );
}

#[test]
fn capture_baseline_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<CaptureBaselineReport>(
        "tests/assets/stable-contracts/capture_baseline.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn render_introspection_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::RenderIntrospectionReportV1>(
        "tests/assets/stable-contracts/render_introspection.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn visibility_diagnosis_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::VisibilityDiagnosisReportV1>(
        "tests/assets/stable-contracts/visibility_diagnosis.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn visual_repair_plan_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::VisualRepairPlanV1>(
        "tests/assets/stable-contracts/visual_repair_plan.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn agent_loop_result_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::AgentLoopResultV1>(
        "tests/assets/stable-contracts/agent_loop_result.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn appearance_expectation_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::AppearanceExpectationV1>(
        "tests/assets/stable-contracts/appearance_expectation.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn appearance_introspection_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::AppearanceIntrospectionReportV1>(
        "tests/assets/stable-contracts/appearance_introspection.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn animation_introspection_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::AnimationIntrospectionReportV1>(
        "tests/assets/stable-contracts/animation_introspection.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn interaction_expectation_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::InteractionExpectationV1>(
        "tests/assets/stable-contracts/interaction_expectation.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn interaction_verification_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::InteractionVerificationReportV1>(
        "tests/assets/stable-contracts/interaction_verification.v1.json",
    );
}

#[test]
fn agent_smoke_template_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::AgentSmokeTemplateV1>(
        "tests/assets/stable-contracts/agent_smoke_template.v1.json",
    );
}

#[test]
fn scene_recipe_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneRecipeV1>(
        "tests/assets/stable-contracts/scene_recipe.v1.json",
    );
}

#[test]
fn scene_recipe_validation_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneRecipeValidationReportV1>(
        "tests/assets/stable-contracts/scene_recipe_validation.v1.json",
    );
}

#[test]
fn placement_result_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::ScenePlacementResultV1>(
        "tests/assets/stable-contracts/placement_result.v1.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn inspection_and_capture_v1_revisions_accept_old_shape_without_appearance() {
    let mut inspection =
        read_fixture_json("tests/assets/stable-contracts/scene_inspection.v1.json");
    inspection["revisions"]
        .as_object_mut()
        .expect("inspection revisions object")
        .remove("appearance");
    for node in inspection["nodes"]
        .as_array_mut()
        .expect("inspection nodes array")
    {
        node.as_object_mut()
            .expect("inspection node object")
            .remove("tint");
    }
    inspection["draw_list"] = json!([
        {
            "node": 1,
            "topology": "triangles",
            "primitive_count": 1,
            "vertex_count": 3,
            "index_count": 3,
            "local_bounds": {
                "min": [-0.5, -0.5, 0.0],
                "max": [0.5, 0.5, 0.0]
            },
            "world_transform": {
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            },
            "visible": true
        }
    ]);
    inspection["normal_overlays"] = json!([
        {
            "node": 1,
            "length": 0.25,
            "segments": [
                [[0.0, 0.0, 0.0], [0.0, 0.25, 0.0]]
            ]
        }
    ]);
    let inspection: scena::SceneInspectionReportV1 =
        serde_json::from_value(inspection).expect("old inspection fixture shape deserializes");
    assert_eq!(
        inspection.revisions.appearance, 0,
        "additive appearance_revision defaults for old scene_inspection.v1 consumers"
    );
    assert_eq!(
        inspection.nodes[0].tint, None,
        "additive node tint defaults for old scene_inspection.v1 consumers"
    );
    assert_eq!(
        inspection.draw_list[0].instance, None,
        "additive draw-list instance defaults for old scene_inspection.v1 consumers"
    );
    assert_eq!(
        inspection.normal_overlays[0].instance, None,
        "additive normal-overlay instance defaults for old scene_inspection.v1 consumers"
    );
    assert_eq!(
        inspection.instance_sets, None,
        "additive instance_sets defaults for old scene_inspection.v1 consumers"
    );

    let mut capture = read_fixture_json("tests/assets/stable-contracts/capture.v1.json");
    capture["revisions"]
        .as_object_mut()
        .expect("capture revisions object")
        .remove("appearance");
    let capture: CaptureDescriptor =
        serde_json::from_value(capture).expect("old capture fixture shape deserializes");
    assert_eq!(
        capture.revisions.appearance, 0,
        "additive appearance_revision defaults for old capture.v1 consumers"
    );
}

#[test]
fn capability_report_v1_accepts_old_shape_without_post_processing() {
    let mut report = read_fixture_json("tests/assets/stable-contracts/capability_report.v1.json");
    report
        .as_object_mut()
        .expect("capability report is an object")
        .remove("post_processing");
    let report: CapabilityReportV1 =
        serde_json::from_value(report).expect("old capability_report.v1 shape deserializes");
    assert_eq!(
        report.post_processing, None,
        "additive post_processing defaults for old capability_report.v1 consumers"
    );
}

#[test]
fn annotation_projection_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<AnnotationProjectionReportV1>(
        "tests/assets/stable-contracts/annotation_projection.v1.json",
    );
}

#[test]
fn asset_geometry_summary_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<SceneAssetGeometrySummary>(
        "tests/assets/stable-contracts/asset_geometry_summary.v1.json",
    );
}

#[test]
fn asset_load_report_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<AssetLoadReportV1>(
        "tests/assets/stable-contracts/asset_load_report.v1.json",
    );
}

#[test]
fn asset_doctor_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<AssetDoctorReportV1>(
        "tests/assets/stable-contracts/asset_doctor.v1.json",
    );
}

#[test]
fn asset_catalog_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<AssetCatalogV1>(
        "tests/assets/stable-contracts/asset_catalog.v1.json",
    );
}

#[test]
fn asset_readiness_report_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<AssetReadinessReportV1>(
        "tests/assets/stable-contracts/asset_readiness_report.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_asset_import_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneHostAssetImportReportV1>(
        "tests/assets/stable-contracts/scene_host_asset_import.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_subtree_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneHostSubtreeReportV1>(
        "tests/assets/stable-contracts/scene_host_subtree.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_measurement_overlay_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneHostMeasurementOverlayReportV1>(
        "tests/assets/stable-contracts/scene_host_measurement_overlay.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_subtree_v1_accepts_old_shape_without_tree_edges() {
    let old_shape = json!({
        "schema": "scena.subtree.v1",
        "nodes": [
            {
                "handle": 42,
                "name": null,
                "tags": ["frame"]
            }
        ]
    });
    let decoded: scena::SceneHostSubtreeReportV1 =
        serde_json::from_value(old_shape).expect("old subtree shape deserializes");
    assert_eq!(decoded.nodes[0].parent, None);
    assert!(decoded.nodes[0].children.is_empty());
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_animation_inventory_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneHostAnimationInventoryV1>(
        "tests/assets/stable-contracts/scene_host_animation_inventory.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn visual_patch_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::VisualPatchV1>(
        "tests/assets/stable-contracts/visual_patch.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn visual_patch_result_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::VisualPatchResultV1>(
        "tests/assets/stable-contracts/visual_patch_result.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_section_box_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneHostSectionBoxReportV1>(
        "tests/assets/stable-contracts/scene_host_section_box.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_visual_state_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneHostVisualStateV1>(
        "tests/assets/stable-contracts/scene_host_visual_state.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_visual_states_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::SceneHostVisualStatesReportV1>(
        "tests/assets/stable-contracts/scene_host_visual_states.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_visual_state_v1_accepts_minimal_defaulted_patch_shape() {
    let decoded: scena::SceneHostVisualStateV1 = serde_json::from_value(json!({
        "schema": "scena.scene_host_visual_state.v1",
        "name": "assembled",
        "patch": { "schema": "scena.visual_patch.v1" }
    }))
    .expect("minimal visual state shape deserializes");
    assert!(decoded.patch.transforms.is_empty());
    assert_eq!(decoded.patch.section_box, None);
    assert_eq!(decoded.metadata, None);
}

#[cfg(feature = "scene-host")]
#[test]
fn host_event_golden_matches_live_schema_serialization() {
    assert_fixture_matches_live_serialization::<scena::HostEventBatchV1>(
        "tests/assets/stable-contracts/host_event.v1.json",
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn host_event_v1_accepts_old_diagnostic_shape_without_node_handle() {
    let batch: scena::HostEventBatchV1 = serde_json::from_value(json!({
        "schema": "scena.host_event.v1",
        "events": [
            {
                "kind": "diagnostic",
                "code": "large_scene_precision_risk",
                "severity": "warning",
                "message": "large scene diagnostic",
                "help": "use a stable origin policy"
            }
        ]
    }))
    .expect("old host_event diagnostic shape deserializes");

    let node = match &batch.events[0] {
        scena::HostEventV1::Diagnostic { node, .. } => *node,
        event => panic!("expected diagnostic event, got {event:?}"),
    };
    assert_eq!(node, None);
}

#[cfg(feature = "scene-host")]
#[test]
fn visual_patch_v1_accepts_minimal_shape_with_defaulted_optional_channels() {
    let patch: scena::VisualPatchV1 = serde_json::from_value(json!({
        "schema": "scena.visual_patch.v1"
    }))
    .expect("minimal visual_patch.v1 shape deserializes");

    assert_eq!(patch.schema, scena::VISUAL_PATCH_SCHEMA_V1);
    assert!(patch.transforms.is_empty());
    assert!(patch.tints.is_empty());
    assert!(patch.visibility.is_empty());
    assert_eq!(patch.camera, None);
    assert!(patch.transforms_eased.is_empty());
    assert!(patch.tints_eased.is_empty());
    assert_eq!(patch.camera_eased, None);
    assert!(patch.animation_time.is_empty());
    assert_eq!(patch.selection, None);
    assert_eq!(patch.hover, None);
    assert!(patch.material_variants.is_empty());
    assert!(patch.labels.is_empty());
    assert_eq!(patch.section_box, None);
    assert_eq!(patch.metadata, None);
    assert!(!patch.echo_metadata);
}

#[cfg(feature = "scene-host")]
#[test]
fn visual_patch_result_v1_accepts_old_shape_without_additive_0_1b_counts() {
    let result: scena::VisualPatchResultV1 = serde_json::from_value(json!({
        "schema": "scena.visual_patch.v1",
        "applied": {
            "transforms": 0,
            "tints": 0,
            "visibility": 0,
            "camera": 0
        },
        "failed": [],
        "revisions": {
            "structure": 0,
            "transform": 0,
            "appearance": 0,
            "visibility": 0,
            "interaction": 0
        }
    }))
    .expect("old visual_patch_result.v1 shape deserializes");

    assert_eq!(result.applied.transforms_eased, 0);
    assert_eq!(result.applied.tints_eased, 0);
    assert_eq!(result.applied.camera_eased, 0);
    assert_eq!(result.applied.animation_time, 0);
    assert_eq!(result.applied.selection, 0);
    assert_eq!(result.applied.hover, 0);
    assert_eq!(result.applied.material_variants, 0);
    assert_eq!(result.applied.labels, 0);
    assert_eq!(result.applied.section_box, 0);
    assert_eq!(result.metadata, None);
}

#[test]
fn asset_provenance_golden_matches_live_value_serialization() {
    assert_fixture_matches_live_serialization::<AssetProvenance>(
        "tests/assets/stable-contracts/asset_provenance.json",
    );
}

#[cfg(feature = "inspection")]
#[test]
fn scene_inspection_schema_uses_report_local_handles_and_topology_helpers() {
    use std::collections::BTreeMap;

    use scena::{
        Assets, GeometryDesc, MaterialDesc, SCENE_INSPECTION_SCHEMA_V1, SceneInspectionReportV1,
    };

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));

    let mut scene = scena::Scene::new();
    let frame = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .expect("frame inserts");
    scene.add_tag(frame, "frame").expect("frame tag inserts");
    let mesh = scene
        .mesh(geometry, material)
        .parent(frame)
        .transform(Transform::at(Vec3::new(0.0, 2.0, 0.0)))
        .add()
        .expect("mesh inserts");
    scene.add_tag(mesh, "part").expect("part tag inserts");
    scene
        .set_transform(mesh, Transform::at(Vec3::new(0.0, 3.0, 0.0)))
        .expect("mesh pose updates");

    let report = scene.inspect_with_assets(&assets);
    let schema = report.to_schema_report();
    let host_like_handles = BTreeMap::from([(scene.root(), 10), (frame, 42), (mesh, 84)]);
    let host_like_schema = report.to_schema_report_with_node_handles(&host_like_handles);

    assert_eq!(schema.schema, SCENE_INSPECTION_SCHEMA_V1);
    assert_eq!(
        schema.revisions.structure,
        scene.dirty_state().structure_revision
    );
    assert_eq!(
        schema.revisions.transform,
        scene.dirty_state().transform_revision
    );
    assert_eq!(
        schema.revisions.interaction,
        scene.dirty_state().interaction_revision
    );
    assert_eq!(schema.roots().len(), 1);

    let frame_node = schema
        .find_by_tag("frame")
        .into_iter()
        .next()
        .expect("frame tag resolves");
    let part_node = schema
        .find_by_tag("part")
        .into_iter()
        .next()
        .expect("part tag resolves");
    assert_ne!(frame_node.handle, 0);
    assert_eq!(part_node.parent, Some(frame_node.handle));
    assert_eq!(
        schema.children_of(frame_node.handle),
        vec![
            schema
                .node_by_handle(part_node.handle)
                .expect("part exists")
        ]
    );
    assert_eq!(
        part_node.local_transform.translation,
        Vec3::new(0.0, 3.0, 0.0)
    );
    assert_eq!(
        part_node.world_transform.translation,
        Vec3::new(1.0, 3.0, 0.0)
    );
    assert_eq!(
        host_like_schema
            .find_by_tag("part")
            .into_iter()
            .next()
            .expect("part tag resolves")
            .handle,
        84
    );
    assert_eq!(host_like_schema.draw_list[0].node, 84);

    assert_eq!(schema.draw_list.len(), 1);
    assert_eq!(schema.draw_list[0].node, part_node.handle);
    assert_eq!(schema.draw_list[0].topology, GeometryTopology::Triangles);
    assert_eq!(schema.draw_list[0].primitive_count, 12);
    assert_eq!(schema.draw_list[0].vertex_count, 24);
    assert_eq!(schema.draw_list[0].index_count, 36);

    let schema_json = report.to_schema_json();
    assert_eq!(schema_json["schema"], SCENE_INSPECTION_SCHEMA_V1);
    assert_eq!(schema_json["nodes"][0]["handle"], 1);
    assert_eq!(schema_json["nodes"][0]["parent"], serde_json::Value::Null);
    assert_eq!(schema_json["draw_list"][0]["node"], part_node.handle);
    assert_eq!(schema_json["draw_list"][0]["topology"], "triangles");
    assert_eq!(
        schema_json["revisions"],
        json!({
            "structure": scene.dirty_state().structure_revision,
            "transform": scene.dirty_state().transform_revision,
            "appearance": scene.dirty_state().appearance_revision
                + scene.dirty_state().visibility_revision,
            "interaction": scene.dirty_state().interaction_revision,
        })
    );

    let decoded: SceneInspectionReportV1 =
        serde_json::from_value(schema_json).expect("schema report deserializes");
    assert_eq!(decoded, schema);
}

fn read_fixture_json(rel: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    let text = std::fs::read_to_string(&path).expect("stable contract fixture reads");
    serde_json::from_str(&text).expect("stable contract fixture is JSON")
}

fn assert_fixture_matches_live_serialization<T>(rel: &str)
where
    T: DeserializeOwned + Serialize,
{
    let fixture = read_fixture_json(rel);
    let decoded: T = serde_json::from_value(fixture.clone()).unwrap_or_else(|error| {
        panic!("{rel} must deserialize through the live contract: {error}")
    });
    let encoded =
        serde_json::to_value(decoded).expect("live contract value serializes back to JSON");
    assert_eq!(
        encoded, fixture,
        "{rel} must match live serialization; regenerate the fixture intentionally when the contract changes"
    );
}
