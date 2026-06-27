use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::visual_patch::{VISUAL_PATCH_SCHEMA_V1, VisualPatchResultV1, VisualPatchV1};
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::AssetFetcher;

pub const PRODUCT_OPTIONS_SCHEMA_V1: &str = "scena.product_options.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductOptionsV1 {
    pub schema: String,
    #[serde(default)]
    pub groups: Vec<ProductOptionGroupV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductOptionGroupV1 {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub options: Vec<ProductOptionV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductOptionV1 {
    pub id: String,
    pub label: String,
    pub patch: VisualPatchV1,
    #[serde(default)]
    pub metadata: Option<Value>,
}

impl ProductOptionsV1 {
    pub fn empty() -> Self {
        Self {
            schema: PRODUCT_OPTIONS_SCHEMA_V1.to_owned(),
            groups: Vec::new(),
        }
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn store_product_options(
        &mut self,
        options: ProductOptionsV1,
    ) -> Result<ProductOptionsV1, SceneHostError> {
        validate_product_options(&options)?;
        self.product_options = options;
        Ok(self.product_options())
    }

    pub fn store_product_options_json(&mut self, json: &str) -> Result<String, SceneHostError> {
        let options: ProductOptionsV1 = serde_json::from_str(json).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("invalid product options JSON: {error}"),
            )
        })?;
        let report = self.store_product_options(options)?;
        product_options_json(&report)
    }

    pub fn product_options(&self) -> ProductOptionsV1 {
        self.product_options.clone()
    }

    pub fn product_options_json(&self) -> Result<String, SceneHostError> {
        product_options_json(&self.product_options)
    }

    pub fn apply_product_option(
        &mut self,
        group_id: &str,
        option_id: &str,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        let patch = self.product_option_patch(group_id, option_id)?;
        let result = self.apply_patch(&patch)?;
        if result.failed.is_empty() {
            self.set_active_product_option(group_id, option_id)?;
        }
        Ok(result)
    }

    pub fn apply_product_option_json(
        &mut self,
        group_id: &str,
        option_id: &str,
    ) -> Result<String, SceneHostError> {
        let result = self.apply_product_option(group_id, option_id)?;
        serde_json::to_string(&result).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("product option apply result serialization failed: {error}"),
            )
        })
    }

    fn product_option_patch(
        &self,
        group_id: &str,
        option_id: &str,
    ) -> Result<VisualPatchV1, SceneHostError> {
        let group = self
            .product_options
            .groups
            .iter()
            .find(|group| group.id == group_id)
            .ok_or_else(|| invalid_input(format!("product option group '{group_id}' not found")))?;
        let option = group
            .options
            .iter()
            .find(|option| option.id == option_id)
            .ok_or_else(|| {
                invalid_input(format!(
                    "product option '{option_id}' not found in group '{group_id}'"
                ))
            })?;
        Ok(option.patch.clone())
    }

    fn set_active_product_option(
        &mut self,
        group_id: &str,
        option_id: &str,
    ) -> Result<(), SceneHostError> {
        let group = self
            .product_options
            .groups
            .iter_mut()
            .find(|group| group.id == group_id)
            .ok_or_else(|| invalid_input(format!("product option group '{group_id}' not found")))?;
        group.active = Some(option_id.to_owned());
        Ok(())
    }
}

fn validate_product_options(options: &ProductOptionsV1) -> Result<(), SceneHostError> {
    if options.schema != PRODUCT_OPTIONS_SCHEMA_V1 {
        return Err(invalid_input(format!(
            "unsupported product options schema {}; expected {}",
            options.schema, PRODUCT_OPTIONS_SCHEMA_V1
        )));
    }

    let mut group_ids = BTreeSet::new();
    for group in &options.groups {
        validate_id("product option group id", &group.id)?;
        if !group_ids.insert(group.id.clone()) {
            return Err(invalid_input(format!(
                "duplicate product option group id '{}'",
                group.id
            )));
        }

        let mut option_ids = BTreeSet::new();
        for option in &group.options {
            validate_id("product option id", &option.id)?;
            if !option_ids.insert(option.id.clone()) {
                return Err(invalid_input(format!(
                    "duplicate product option id '{}' in group '{}'",
                    option.id, group.id
                )));
            }
            if option.patch.schema != VISUAL_PATCH_SCHEMA_V1 {
                return Err(invalid_input(format!(
                    "product option '{}.{}' patch schema must be {}; got {}",
                    group.id, option.id, VISUAL_PATCH_SCHEMA_V1, option.patch.schema
                )));
            }
        }

        if let Some(active) = &group.active
            && !option_ids.contains(active)
        {
            return Err(invalid_input(format!(
                "active product option '{active}' is not present in group '{}'",
                group.id
            )));
        }
    }
    Ok(())
}

fn validate_id(label: &str, value: &str) -> Result<(), SceneHostError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(invalid_input(format!(
            "{label} must be non-empty and contain no control characters"
        )));
    }
    Ok(())
}

fn product_options_json(options: &ProductOptionsV1) -> Result<String, SceneHostError> {
    serde_json::to_string(options).map_err(|error| {
        SceneHostError::new(
            SceneHostErrorCode::Inspect,
            format!("product options serialization failed: {error}"),
        )
    })
}

fn invalid_input(message: String) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message)
}
