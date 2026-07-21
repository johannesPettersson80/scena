use serde::{Deserialize, Serialize};

use super::{
    OVERRIDE_FETCH_BYTE_LIMIT, OVERRIDE_MAX_ANIMATION_CHANNELS, OVERRIDE_MAX_ANIMATION_KEYFRAMES,
    OVERRIDE_MAX_ANIMATIONS, OVERRIDE_MAX_IMAGE_DIMENSION, OVERRIDE_MAX_IMPORTS,
    OVERRIDE_MAX_INDICES, OVERRIDE_MAX_INSTANCES, OVERRIDE_MAX_MATERIALS, OVERRIDE_MAX_NODES,
    OVERRIDE_MAX_OUTPUT_PIXELS, OVERRIDE_MAX_PARTICLES, OVERRIDE_MAX_RECIPE_BYTES,
    OVERRIDE_MAX_TEXTURE_BYTES, OVERRIDE_MAX_TEXTURES, OVERRIDE_MAX_VERTICES, OVERRIDE_NETWORK,
    OVERRIDE_URI_SCHEMES, RECIPE_POLICY_SCHEMA_V1, RecipeBuildPolicy,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBuildPolicyReportV1 {
    pub schema: String,
    pub network: RecipeBuildPolicyBoolV1,
    pub allowed_uri_schemes: Vec<RecipeBuildPolicyStringV1>,
    pub allowed_roots: Vec<RecipeBuildPolicyRootV1>,
    pub limits: std::collections::BTreeMap<String, RecipeBuildPolicyLimitV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBuildPolicyBoolV1 {
    pub allowed: bool,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBuildPolicyStringV1 {
    pub value: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBuildPolicyRootV1 {
    pub path: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeBuildPolicyLimitV1 {
    pub value: u64,
    pub source: String,
}

impl RecipeBuildPolicy {
    pub fn to_schema_report(&self) -> RecipeBuildPolicyReportV1 {
        let limits = [
            ("max_imports", self.max_imports as u64, OVERRIDE_MAX_IMPORTS),
            ("max_nodes", self.max_nodes as u64, OVERRIDE_MAX_NODES),
            (
                "max_vertices",
                self.max_vertices as u64,
                OVERRIDE_MAX_VERTICES,
            ),
            ("max_indices", self.max_indices as u64, OVERRIDE_MAX_INDICES),
            (
                "max_materials",
                self.max_materials as u64,
                OVERRIDE_MAX_MATERIALS,
            ),
            (
                "max_textures",
                self.max_textures as u64,
                OVERRIDE_MAX_TEXTURES,
            ),
            (
                "max_texture_bytes",
                self.max_texture_bytes as u64,
                OVERRIDE_MAX_TEXTURE_BYTES,
            ),
            (
                "max_image_dimension",
                u64::from(self.max_image_dimension),
                OVERRIDE_MAX_IMAGE_DIMENSION,
            ),
            (
                "max_instances",
                self.max_instances as u64,
                OVERRIDE_MAX_INSTANCES,
            ),
            (
                "max_particles",
                self.max_particles as u64,
                OVERRIDE_MAX_PARTICLES,
            ),
            (
                "max_animations",
                self.max_animations as u64,
                OVERRIDE_MAX_ANIMATIONS,
            ),
            (
                "max_animation_channels",
                self.max_animation_channels as u64,
                OVERRIDE_MAX_ANIMATION_CHANNELS,
            ),
            (
                "max_animation_keyframes",
                self.max_animation_keyframes as u64,
                OVERRIDE_MAX_ANIMATION_KEYFRAMES,
            ),
            (
                "max_output_pixels",
                self.max_output_pixels,
                OVERRIDE_MAX_OUTPUT_PIXELS,
            ),
            (
                "fetch_byte_limit",
                self.fetch_byte_limit as u64,
                OVERRIDE_FETCH_BYTE_LIMIT,
            ),
            (
                "max_recipe_bytes",
                self.max_recipe_bytes as u64,
                OVERRIDE_MAX_RECIPE_BYTES,
            ),
        ]
        .into_iter()
        .map(|(name, value, bit)| {
            (
                name.to_owned(),
                RecipeBuildPolicyLimitV1 {
                    value,
                    source: self.source_for(bit).to_owned(),
                },
            )
        })
        .collect();
        RecipeBuildPolicyReportV1 {
            schema: RECIPE_POLICY_SCHEMA_V1.to_owned(),
            network: RecipeBuildPolicyBoolV1 {
                allowed: self.allow_network,
                source: self.source_for(OVERRIDE_NETWORK).to_owned(),
            },
            allowed_uri_schemes: self
                .allowed_uri_schemes
                .iter()
                .map(|value| RecipeBuildPolicyStringV1 {
                    value: value.clone(),
                    source: self.source_for(OVERRIDE_URI_SCHEMES).to_owned(),
                })
                .collect(),
            allowed_roots: self
                .allowed_roots
                .iter()
                .zip(&self.allowed_root_operator_overrides)
                .map(|(root, operator_override)| RecipeBuildPolicyRootV1 {
                    path: root
                        .canonicalize()
                        .unwrap_or_else(|_| root.clone())
                        .display()
                        .to_string(),
                    source: if *operator_override {
                        "operator_override"
                    } else {
                        "compiled_default"
                    }
                    .to_owned(),
                })
                .collect(),
            limits,
        }
    }
}
