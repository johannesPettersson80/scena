use serde::{Deserialize, Serialize};

pub const AGENT_SMOKE_TEMPLATE_SCHEMA_V1: &str = "scena.agent_smoke_template.v1";
pub const AGENT_TEMPLATE_CATALOG_SCHEMA_V1: &str = "scena.agent_template_catalog.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTemplateCatalogV1 {
    pub schema: String,
    pub templates: Vec<AgentTemplateCatalogEntryV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTemplateCatalogEntryV1 {
    pub name: String,
    pub aliases: Vec<String>,
    pub status: String,
    pub required_features: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSmokeTemplateV1 {
    pub schema: String,
    pub name: String,
    pub status: String,
    pub required_features: Vec<String>,
    pub files: Vec<AgentSmokeTemplateFileV1>,
    pub commands: Vec<AgentSmokeTemplateCommandV1>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSmokeTemplateFileV1 {
    pub kind: String,
    pub path: String,
    pub schema: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSmokeTemplateCommandV1 {
    pub name: String,
    pub argv: Vec<String>,
    pub expected_schema: String,
    pub expected_ok: bool,
    pub artifacts: Vec<String>,
}
