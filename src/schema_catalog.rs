use serde::{Deserialize, Serialize};
use serde_json::json;

mod agent_smoke;
mod fixtures;

pub use agent_smoke::{
    AGENT_SMOKE_TEMPLATE_SCHEMA_V1, AgentSmokeTemplateCommandV1, AgentSmokeTemplateFileV1,
    AgentSmokeTemplateV1,
};
pub use fixtures::nearest_schema_name;

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
}

pub fn schema_catalog_v1() -> SchemaCatalogV1 {
    SchemaCatalogV1 {
        schema: SCHEMA_CATALOG_SCHEMA_V1.to_owned(),
        entries: schema_catalog_entries(),
    }
}

pub fn schema_catalog_entry(schema: &str) -> Option<SchemaCatalogEntryV1> {
    schema_catalog_entries()
        .into_iter()
        .find(|entry| entry.schema == schema)
}

pub fn schema_entry_report_v1(schema: &str) -> Option<SchemaEntryReportV1> {
    let entry = schema_catalog_entry(schema)?;
    let example = fixtures::schema_fixture_json(schema)
        .and_then(|fixture| serde_json::from_str(fixture).ok())
        .unwrap_or(serde_json::Value::Null);
    Some(SchemaEntryReportV1 {
        schema: SCHEMA_ENTRY_SCHEMA_V1.to_owned(),
        entry,
        example,
        invalid_example: invalid_example_for_schema(schema),
    })
}

fn invalid_example_for_schema(schema: &str) -> Option<serde_json::Value> {
    match schema {
        "scena.scene_recipe.v1" => Some(json!({
            "schema": "scena.scene_recipe.v1",
            "importe": [{
                "id": "part",
                "uri": "tests/assets/gltf/mesh_material_vertex_color_scene.gltf"
            }]
        })),
        _ => None,
    }
}

fn schema_catalog_entries() -> Vec<SchemaCatalogEntryV1> {
    schema_entry_rows()
        .iter()
        .map(|row| SchemaCatalogEntryV1 {
            schema: row.schema.to_owned(),
            owner_module: row.owner_module.to_owned(),
            summary: row.summary.to_owned(),
            feature_flag: row.feature_flag.map(str::to_owned),
            fixture_path: row.fixture_path.map(str::to_owned),
        })
        .collect()
}

struct SchemaEntryRow {
    schema: &'static str,
    owner_module: &'static str,
    summary: &'static str,
    feature_flag: Option<&'static str>,
    fixture_path: Option<&'static str>,
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
            schema: "scena.browser_proof_run.v1",
            owner_module: "bin/scena",
            summary: "One-command browser proof wrapper result with lane, command, and artifact paths.",
            feature_flag: None,
            fixture_path: Some("tests/assets/stable-contracts/browser_proof_run.v1.json"),
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
            schema: "scena.recipe_render_result.v1",
            owner_module: "bin/scena",
            summary: "One-command recipe build, render, introspection, and verification result.",
            feature_flag: Some("scene-host"),
            fixture_path: Some("tests/assets/stable-contracts/recipe_render_result.v1.json"),
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
