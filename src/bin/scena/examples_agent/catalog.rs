#[derive(Debug, Clone, Copy)]
struct TemplateSpec {
    canonical: &'static str,
    aliases: &'static [&'static str],
    required_features: &'static [&'static str],
    summary: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TemplateSelection {
    pub(super) canonical: &'static str,
    pub(super) alias: Option<&'static str>,
}

const TEMPLATE_SPECS: &[TemplateSpec] = &[
    TemplateSpec {
        canonical: "animated-viewer",
        aliases: &["animated_viewer"],
        required_features: &["inspection"],
        summary: "Animated asset playback and change verification.",
    },
    TemplateSpec {
        canonical: "cad-inspection",
        aliases: &["cad_inspection"],
        required_features: &["inspection", "scene-host"],
        summary: "CAD section, measurement, and callout inspection.",
    },
    TemplateSpec {
        canonical: "cad-plate",
        aliases: &["cad_plate"],
        required_features: &["inspection", "scene-host"],
        summary: "Authored CAD plate starter scene.",
    },
    TemplateSpec {
        canonical: "dashboard-bars",
        aliases: &["dashboard_bars"],
        required_features: &["inspection", "scene-host"],
        summary: "Authored industrial dashboard starter.",
    },
    TemplateSpec {
        canonical: "data-visualization",
        aliases: &["data_visualization"],
        required_features: &["inspection", "scene-host"],
        summary: "Authored data visualization with verification.",
    },
    TemplateSpec {
        canonical: "documentation-renderer",
        aliases: &["documentation_renderer"],
        required_features: &["inspection", "scene-host"],
        summary: "Documentation capture and annotation workflow.",
    },
    TemplateSpec {
        canonical: "interaction-proof",
        aliases: &["interaction_proof"],
        required_features: &["inspection", "scene-host"],
        summary: "Synthetic hover and selection verification.",
    },
    TemplateSpec {
        canonical: "live-state-viewer",
        aliases: &["live_state_viewer"],
        required_features: &["inspection"],
        summary: "Visibility and live-state diagnosis workflow.",
    },
    TemplateSpec {
        canonical: "machine-state-viewer",
        aliases: &["machine_state_viewer"],
        required_features: &["inspection", "scene-host"],
        summary: "Authored machine-state starter scene.",
    },
    TemplateSpec {
        canonical: "primitive-scene",
        aliases: &["primitive_scene"],
        required_features: &["inspection", "scene-host"],
        summary: "Authored primitive starter scene.",
    },
    TemplateSpec {
        canonical: "product-configurator",
        aliases: &[],
        required_features: &["inspection"],
        summary: "Imported material-variant product configuration proof.",
    },
    TemplateSpec {
        canonical: "product-configurator-starter",
        aliases: &["product_configurator"],
        required_features: &["inspection", "scene-host"],
        summary: "Authored-from-scratch product configurator starter.",
    },
    TemplateSpec {
        canonical: "web-viewer",
        aliases: &["web_viewer"],
        required_features: &["inspection"],
        summary: "Portable web-viewer render workflow.",
    },
];

pub(super) fn resolve_template(name: &str) -> Option<TemplateSelection> {
    TEMPLATE_SPECS.iter().find_map(|spec| {
        if spec.canonical == name {
            Some(TemplateSelection {
                canonical: spec.canonical,
                alias: None,
            })
        } else {
            spec.aliases
                .iter()
                .find(|alias| **alias == name)
                .map(|alias| TemplateSelection {
                    canonical: spec.canonical,
                    alias: Some(alias),
                })
        }
    })
}

pub(super) fn template_name_candidates(name: &str) -> Vec<String> {
    scena::nearest_name_candidates(name, TEMPLATE_SPECS.iter().map(|spec| spec.canonical), 3)
}

pub(super) fn template_catalog() -> scena::AgentTemplateCatalogV1 {
    scena::AgentTemplateCatalogV1 {
        schema: scena::AGENT_TEMPLATE_CATALOG_SCHEMA_V1.to_owned(),
        templates: TEMPLATE_SPECS
            .iter()
            .map(|spec| scena::AgentTemplateCatalogEntryV1 {
                name: spec.canonical.to_owned(),
                aliases: spec
                    .aliases
                    .iter()
                    .map(|alias| (*alias).to_owned())
                    .collect(),
                status: "ready".to_owned(),
                required_features: spec
                    .required_features
                    .iter()
                    .map(|feature| (*feature).to_owned())
                    .collect(),
                summary: spec.summary.to_owned(),
            })
            .collect(),
    }
}
