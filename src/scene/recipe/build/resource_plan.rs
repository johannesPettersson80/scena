use super::*;
use crate::scene::recipe::types::{
    SceneRecipeDiagnosticResourceV1, SceneRecipeMaterialV1, SceneRecipeResourceResolutionV1,
    SceneRecipeResourceStatusV1, SceneRecipeTextureColorSpaceV1, SceneRecipeTextureSlotV1,
    SceneRecipeV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RecipeResourceRole {
    Import(usize),
    Environment,
    Font(usize),
    Texture(SceneRecipeTextureColorSpaceV1),
    BuiltinEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedRecipeResource {
    pub report: SceneRecipeResourceResolutionV1,
    pub role: RecipeResourceRole,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RecipeResourcePlan {
    pub resources: Vec<PlannedRecipeResource>,
    pub diagnostics: Vec<SceneRecipeDiagnosticV1>,
}

impl RecipeResourcePlan {
    #[cfg(feature = "scene-host")]
    pub(crate) fn resolved_uri(&self, path: &str) -> Option<&str> {
        self.resources
            .iter()
            .find(|resource| resource.report.path == path)
            .and_then(|resource| resource.report.normalized_uri.as_deref())
    }

    pub(crate) fn reports(&self) -> Vec<SceneRecipeResourceResolutionV1> {
        self.resources
            .iter()
            .map(|resource| resource.report.clone())
            .collect()
    }
}

impl RecipeBuildPolicy {
    pub(crate) fn recipe_resource_inventory(&self, recipe: &SceneRecipeV1) -> RecipeResourcePlan {
        let mut plan = RecipeResourcePlan::default();
        collect_resources(recipe, &mut plan.resources);
        plan
    }

    pub(crate) fn resolve_recipe_resources(
        &self,
        recipe_path: &str,
        recipe: &SceneRecipeV1,
    ) -> RecipeResourcePlan {
        let mut plan = self.recipe_resource_inventory(recipe);
        let allowed_roots = self
            .to_schema_report()
            .allowed_roots
            .into_iter()
            .map(|root| root.path)
            .collect::<Vec<_>>();
        for resource in &mut plan.resources {
            if matches!(resource.role, RecipeResourceRole::BuiltinEnvironment) {
                resource.report.status = SceneRecipeResourceStatusV1::Builtin;
                continue;
            }
            resource.report.normalized_uri = Some(best_effort_normalized_uri(
                recipe_path,
                &resource.report.authored_uri,
            ));
            match self.resolve_import_uri(
                recipe_path,
                &resource.report.authored_uri,
                resource.report.path.clone(),
            ) {
                Ok(uri) => {
                    resource.report.normalized_uri = Some(uri);
                    resource.report.status = SceneRecipeResourceStatusV1::Resolved;
                }
                Err(mut diagnostic) => {
                    resource.report.status = SceneRecipeResourceStatusV1::ResolutionFailed;
                    diagnostic.resource = Some(SceneRecipeDiagnosticResourceV1 {
                        kind: resource.report.kind.clone(),
                        authored_uri: resource.report.authored_uri.clone(),
                        normalized_uri: resource.report.normalized_uri.clone(),
                        required: resource.report.required,
                        allowed_roots: allowed_roots.clone(),
                    });
                    plan.diagnostics.push(*diagnostic);
                }
            }
        }
        plan
    }
}

fn best_effort_normalized_uri(recipe_path: &str, uri: &str) -> String {
    let resolved = resolve_recipe_asset_uri(recipe_path, uri);
    let path = Path::new(&resolved);
    if path.is_absolute() || resolved.contains("://") || resolved.starts_with("data:") {
        return resolved;
    }
    std::env::current_dir()
        .map(|current| current.join(path).display().to_string())
        .unwrap_or(resolved)
}

fn collect_resources(recipe: &SceneRecipeV1, resources: &mut Vec<PlannedRecipeResource>) {
    for (index, import) in recipe.imports.iter().enumerate() {
        push_uri(
            resources,
            format!("$.imports[{index}].uri"),
            "import",
            &import.uri,
            !import.optional,
            RecipeResourceRole::Import(index),
        );
    }
    for (index, font) in recipe.fonts.iter().enumerate() {
        push_uri(
            resources,
            format!("$.fonts[{index}].uri"),
            "font",
            &font.uri,
            !font.optional,
            RecipeResourceRole::Font(index),
        );
    }
    for (index, material) in recipe.materials.iter().enumerate() {
        collect_material_textures(index, material, resources);
    }
    if let Some(environment) = recipe
        .scene
        .as_ref()
        .and_then(|scene| scene.environment.as_ref())
    {
        if let Some(preset) = environment.preset.as_deref() {
            resources.push(PlannedRecipeResource {
                report: SceneRecipeResourceResolutionV1 {
                    path: "$.scene.environment.preset".to_owned(),
                    kind: "builtin_environment".to_owned(),
                    authored_uri: preset.to_owned(),
                    normalized_uri: Some(format!("builtin://environment/{preset}")),
                    required: true,
                    status: SceneRecipeResourceStatusV1::NotChecked,
                },
                role: RecipeResourceRole::BuiltinEnvironment,
            });
        } else if environment.kind.as_deref() == Some("uri")
            && let Some(uri) = environment.uri.as_deref()
        {
            push_uri(
                resources,
                "$.scene.environment.uri".to_owned(),
                "environment",
                uri,
                !environment.optional,
                RecipeResourceRole::Environment,
            );
        }
    }
}

fn collect_material_textures(
    index: usize,
    material: &SceneRecipeMaterialV1,
    resources: &mut Vec<PlannedRecipeResource>,
) {
    for (field, slot) in [
        ("base_color_texture", material.base_color_texture.as_ref()),
        ("normal_texture", material.normal_texture.as_ref()),
        (
            "metallic_roughness_texture",
            material.metallic_roughness_texture.as_ref(),
        ),
        ("occlusion_texture", material.occlusion_texture.as_ref()),
        ("emissive_texture", material.emissive_texture.as_ref()),
        ("clearcoat_texture", material.clearcoat_texture.as_ref()),
        (
            "clearcoat_roughness_texture",
            material.clearcoat_roughness_texture.as_ref(),
        ),
        (
            "clearcoat_normal_texture",
            material.clearcoat_normal_texture.as_ref(),
        ),
        ("sheen_color_texture", material.sheen_color_texture.as_ref()),
        (
            "sheen_roughness_texture",
            material.sheen_roughness_texture.as_ref(),
        ),
        ("anisotropy_texture", material.anisotropy_texture.as_ref()),
        ("iridescence_texture", material.iridescence_texture.as_ref()),
        (
            "iridescence_thickness_texture",
            material.iridescence_thickness_texture.as_ref(),
        ),
        (
            "transmission_texture",
            material.transmission_texture.as_ref(),
        ),
        ("thickness_texture", material.thickness_texture.as_ref()),
    ] {
        if let Some(slot) = slot {
            let default_color_space = if matches!(
                field,
                "base_color_texture" | "emissive_texture" | "sheen_color_texture"
            ) {
                SceneRecipeTextureColorSpaceV1::Srgb
            } else {
                SceneRecipeTextureColorSpaceV1::Linear
            };
            push_texture(resources, index, field, slot, default_color_space);
        }
    }
}

fn push_texture(
    resources: &mut Vec<PlannedRecipeResource>,
    material_index: usize,
    field: &str,
    slot: &SceneRecipeTextureSlotV1,
    default_color_space: SceneRecipeTextureColorSpaceV1,
) {
    push_uri(
        resources,
        format!("$.materials[{material_index}].{field}.uri"),
        "texture",
        &slot.uri,
        !slot.optional,
        RecipeResourceRole::Texture(slot.color_space.clone().unwrap_or(default_color_space)),
    );
}

fn push_uri(
    resources: &mut Vec<PlannedRecipeResource>,
    path: String,
    kind: &str,
    uri: &str,
    required: bool,
    role: RecipeResourceRole,
) {
    resources.push(PlannedRecipeResource {
        report: SceneRecipeResourceResolutionV1 {
            path,
            kind: kind.to_owned(),
            authored_uri: uri.to_owned(),
            normalized_uri: None,
            required,
            status: SceneRecipeResourceStatusV1::NotChecked,
        },
        role,
    });
}
