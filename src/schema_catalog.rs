use crate::scene::recipe::SchemaFieldModelV1;
use serde::{Deserialize, Serialize};

mod agent_smoke;
mod entries;
mod fixtures;
mod reports;

use entries::SchemaEntryRow;

pub use agent_smoke::{
    AGENT_SMOKE_TEMPLATE_SCHEMA_V1, AGENT_TEMPLATE_CATALOG_SCHEMA_V1, AgentSmokeTemplateCommandV1,
    AgentSmokeTemplateFileV1, AgentSmokeTemplateV1, AgentTemplateCatalogEntryV1,
    AgentTemplateCatalogV1,
};
pub use fixtures::nearest_schema_name;
pub use reports::{schema_catalog_entry, schema_catalog_v1, schema_entry_report_v1};

pub const SCHEMA_CATALOG_SCHEMA_V1: &str = "scena.schema_catalog.v1";
pub const SCHEMA_ENTRY_SCHEMA_V1: &str = "scena.schema_entry.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaCatalogV1 {
    pub schema: String,
    pub entries: Vec<SchemaCatalogEntryV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaCatalogEntryV1 {
    pub schema: String,
    pub owner_module: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_flag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaEntryReportV1 {
    pub schema: String,
    pub entry: SchemaCatalogEntryV1,
    pub example: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalid_example: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_model: Option<SchemaFieldModelV1>,
}

fn schema_entry_rows() -> &'static [SchemaEntryRow] {
    &[
        SchemaEntryRow {
            schema: SCHEMA_CATALOG_SCHEMA_V1,
            owner_module: "schema_catalog",
            summary: "Catalog of public stable scena JSON contracts.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/schema_catalog.v1.json"),
        },
        SchemaEntryRow {
            schema: SCHEMA_ENTRY_SCHEMA_V1,
            owner_module: "schema_catalog",
            summary: "Single schema catalog entry with a representative example.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/schema_entry.v1.json"),
        },
        SchemaEntryRow {
            schema: crate::FIELD_MODEL_SCHEMA_V1,
            owner_module: "scene/recipe/field_model",
            summary: "Authoritative field types, requiredness, enums, ranges, defaults, deprecations, and examples.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/field_model.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.capability_report.v1",
            owner_module: "diagnostics",
            summary: "Renderer capability status and diagnostics.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/capability_report.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_inspection.v1",
            owner_module: "scene",
            summary: "Scene graph, draw-list, bounds, material, and revision inspection.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/scene_inspection.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.capture.v1",
            owner_module: "capture",
            summary: "Descriptor-bound RGBA8 capture metadata and payload hash.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/capture.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.capture_baseline.v1",
            owner_module: "capture",
            summary: "Tolerance-bound capture comparison report.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/capture_baseline.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.render_introspection.v1",
            owner_module: "render",
            summary: "Capture-bound frame visibility, luminance, framing, reason, and fix summary.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/render_introspection.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.render_quality.v1",
            owner_module: "render",
            summary: "Profile-scoped native-capture render quality checks with actionable fix hints.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/render_quality.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_composition.v1",
            owner_module: "scene_host",
            summary: "Composition-conformance checks over declared recipe elements and generated overlays.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_composition.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.visibility_diagnosis.v1",
            owner_module: "render",
            summary: "Inspection-backed visibility diagnosis with stable reasons and suggested fixes.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/visibility_diagnosis.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.visual_repair_plan.v1",
            owner_module: "render",
            summary: "Conservative repair plan over introspection and visibility diagnosis reports.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/visual_repair_plan.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.agent_loop_result.v1",
            owner_module: "render",
            summary: "Fail-closed agent repair-loop result for irreducible or non-converging cases.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/agent_loop_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.agent_smoke_template.v1",
            owner_module: "bin/scena",
            summary: "Agent smoke-template manifest with generated files, commands, and artifacts.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/agent_smoke_template.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.agent_template_catalog.v1",
            owner_module: "schema_catalog/agent_smoke",
            summary: "Canonical agent template names, aliases, status, and required features.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/agent_template_catalog.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.browser_proof_run.v1",
            owner_module: "bin/scena",
            summary: "One-command browser proof wrapper result with lane, command, and artifact paths.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/browser_proof_run.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.q01.required_webgpu_pixel_parity.v1",
            owner_module: "browser_probe",
            summary: "Source-bound CPU/WebGPU pixel comparison with mutations, adapter, and artifact provenance.",
            feature_flag: Some("browser-probe"),
            fixture_path: Some(
                "tests/assets/stable-contracts/required_webgpu_pixel_parity.v1.json",
            ),
        },
        SchemaEntryRow {
            schema: "scena.q04.required_gpu_resource_lifecycle.v1",
            owner_module: "render",
            summary: "Physical-GPU prepare, render, release, and confirmed resource-retirement evidence.",
            feature_flag: None,
            fixture_path: Some(
                "tests/assets/stable-contracts/required_gpu_resource_lifecycle.v1.json",
            ),
        },
        SchemaEntryRow {
            schema: "scena.appearance_expectation.v1",
            owner_module: "render",
            summary: "Transient expected appearance targets for first-time material verification.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/appearance_expectation.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.appearance_introspection.v1",
            owner_module: "render",
            summary: "Capture-bound material, variant, fallback, alpha, and swatch verification report.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/appearance_introspection.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.animation_introspection.v1",
            owner_module: "render",
            summary: "Host-ticked animation sampling, channel changes, and rendered-change verification report.",
            feature_flag: Some("inspection"),
            fixture_path: Some("tests/assets/stable-contracts/animation_introspection.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.interaction_expectation.v1",
            owner_module: "scene_host",
            summary: "Transient synthetic interaction steps for host-driven verification.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/interaction_expectation.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.interaction_verification.v1",
            owner_module: "scene_host",
            summary: "Synthetic pick, hover, selection, and host-event verification report.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/interaction_verification.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_host_gizmo_drag.v1",
            owner_module: "scene_host",
            summary: "SceneHost transform gizmo drag request that applies through VisualPatch.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_host_gizmo_drag.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.connector_browser.v1",
            owner_module: "scene_host",
            summary: "Connector listing, metadata compatibility, and snap-preview report.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/connector_browser.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.product_options.v1",
            owner_module: "scene_host",
            summary: "Host-owned visual product option groups that apply VisualPatch entries.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/product_options.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.presentation_timeline.v1",
            owner_module: "scene_host",
            summary: "Host-ticked presentation timeline that emits VisualPatch for seek and advance.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/presentation_timeline.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_host_grounding.v1",
            owner_module: "scene_host",
            summary: "Product-viewer grounding preset report with floor, SSAO, and explicit shadow fallbacks.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_host_grounding.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_recipe.v1",
            owner_module: "scene",
            summary: "Declarative, transient scene snapshot consumed by agent and CLI workflows.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/scene_recipe.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_recipe_validation.v1",
            owner_module: "scene",
            summary: "Fail-closed scene recipe validation diagnostics with deterministic suggestions.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/scene_recipe_validation.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_recipe_build.v1",
            owner_module: "scene_host",
            summary: "Typed recipe build manifest mapping caller ids to stable SceneHost handles.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_recipe_build.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.recipe_build_result.v1",
            owner_module: "scene_host",
            summary: "Renderer-free recipe build manifest with effective policy and execution counters.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/recipe_build_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.recipe_render_result.v1",
            owner_module: "bin/scena",
            summary: "One-command recipe build, render, introspection, and verification result.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/recipe_render_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.cad_inspection_result.v1",
            owner_module: "bin/scena",
            summary: "CAD inspection preset report binding principal-face renders, post-process presentation metrics, and a contact sheet.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/cad_inspection_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.capture_sequence_result.v1",
            owner_module: "bin/scena/recipe/capture_sequence",
            summary: "Canonical-view, turntable, and animation-clip PNG capture sequence with per-frame camera and timing metadata.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/capture_sequence_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.semantic_aov_result.v1",
            owner_module: "bin/scena/recipe/semantic_aov",
            summary: "Deterministic semantic ID, linear-depth, and world-normal images with runtime-scoped and persistent identity legend.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/semantic_aov_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_recipe_diff.v1",
            owner_module: "scene/recipe/diff",
            summary: "Renderer-free structural recipe diff with persistent semantic identity and numeric tolerance.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/scene_recipe_diff.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_recipe_diff_result.v1",
            owner_module: "bin/scena/diff",
            summary: "Typed recipe semantic changes plus optional aggregate rendered diff and conservative persistent-identity attribution.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_recipe_diff_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.placement_result.v1",
            owner_module: "scene",
            summary: "Semantic placement transform preview for declarative recipe imports.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/placement_result.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.annotation_projection.v1",
            owner_module: "scene",
            summary: "Projected annotation anchors and CSS-pixel screen positions.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/annotation_projection.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.asset_geometry_summary.v1",
            owner_module: "assets",
            summary: "Imported geometry counts, bounds, and topology summary.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/asset_geometry_summary.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.asset_load_report.v1",
            owner_module: "assets",
            summary: "Asset load warnings, external resources, and material fallback provenance.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/asset_load_report.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.asset_doctor.v1",
            owner_module: "assets",
            summary: "Runtime asset doctor findings with severity, code, help, and suggested fixes.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/asset_doctor.v1.json"),
        },
        SchemaEntryRow {
            schema: crate::ASSET_CONVERSION_SCHEMA_V1,
            owner_module: "assets/conversion",
            summary: "FBX-to-glTF conversion plan, captured tool diagnostics, and outcome.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/asset_conversion.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.asset_catalog.v1",
            owner_module: "assets",
            summary: "Host-owned asset catalog manifest consumed by Scena readiness validation.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/asset_catalog.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.asset_readiness_report.v1",
            owner_module: "assets",
            summary: "Asset catalog readiness findings derived from real asset loads and authored features.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/asset_readiness_report.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_host_asset_import.v1",
            owner_module: "scene_host",
            summary: "SceneHost import result with stable host handles and diagnostics.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_host_asset_import.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.subtree.v1",
            owner_module: "scene_host",
            summary: "SceneHost subtree report with stable node handles and tree edges.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_host_subtree.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_host_measurement_overlay.v1",
            owner_module: "scene_host",
            summary: "SceneHost measurement overlay result with stable generated node handles.",
            feature_flag: Some("scene-host"),
            fixture_path: Some(
                "tests/assets/stable-contracts/scene_host_measurement_overlay.v1.json",
            ),
        },
        SchemaEntryRow {
            schema: "scena.scene_host_section_box.v1",
            owner_module: "scene_host",
            summary: "SceneHost section box report with generated clipping planes and helper node.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_host_section_box.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_host_visual_state.v1",
            owner_module: "scene_host",
            summary: "SceneHost named visual state storing a visual patch plus metadata.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_host_visual_state.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.scene_host_visual_states.v1",
            owner_module: "scene_host",
            summary: "SceneHost deterministic list of stored named visual states.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/scene_host_visual_states.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.animation_inventory.v1",
            owner_module: "scene_host",
            summary: "SceneHost animation clip inventory.",
            feature_flag: Some("scene-host"),
            fixture_path: Some(
                "tests/assets/stable-contracts/scene_host_animation_inventory.v1.json",
            ),
        },
        SchemaEntryRow {
            schema: "scena.visual_patch.v1",
            owner_module: "scene_host",
            summary: "Batched host-to-scena visual state patch and result contract.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/visual_patch.v1.json"),
        },
        SchemaEntryRow {
            schema: "scena.host_event.v1",
            owner_module: "scene_host",
            summary: "SceneHost event batch for pick, hover, load, diagnostic, capture, and surface events.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/host_event.v1.json"),
        },
    ]
}
