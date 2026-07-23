use serde::{Deserialize, Serialize};

pub const AGENT_GUIDE_SCHEMA_V1: &str = "scena.agent_guide.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentGuideV1 {
    pub schema: String,
    pub name: String,
    pub version: u32,
    pub markdown: String,
    pub commands: Vec<String>,
    pub schemas: Vec<String>,
    pub policies: Vec<String>,
    pub templates: Vec<String>,
}

pub fn agent_guide_v1() -> AgentGuideV1 {
    AgentGuideV1 {
        schema: AGENT_GUIDE_SCHEMA_V1.to_owned(),
        name: "llm-app-builder".to_owned(),
        version: 1,
        markdown: include_str!("../../docs/guides/llm-app-builder.md").to_owned(),
        commands: [
            "schema get scena.scene_recipe.v1",
            "examples agent list",
            "examples agent get <template> --out <directory>",
            "validate-recipe <recipe.json> --full",
            "recipe build <recipe.json>",
            "recipe render <recipe.json> --out <png>",
            "inspect <asset-or-recipe>",
            "diagnose <asset-or-recipe> --visibility",
            "repair <asset-or-recipe> --from <report.json>",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        schemas: [
            "scena.scene_recipe.v1",
            "scena.scene_recipe_validation.v1",
            "scena.recipe_build_result.v1",
            "scena.render_introspection.v1",
            "scena.scene_inspection.v1",
            "scena.visibility_diagnosis.v1",
            "scena.visual_repair_plan.v1",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        policies: [
            "recipe paths resolve relative to the recipe",
            "use repeatable --allow-root only for operator-approved external directories",
            "treat unavailable capability evidence as unavailable, never as a pass",
            "branch on CLI code and exit_class instead of message prose",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        templates: [
            "primitive-scene",
            "cad-plate",
            "dashboard-bars",
            "machine-state-viewer",
            "product-configurator-starter",
            "interaction-proof",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    }
}
