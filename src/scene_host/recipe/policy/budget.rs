use crate::scene::recipe::{RecipeBuildPolicy, SceneRecipeDiagnosticV1};

use super::super::error_diagnostic;

#[derive(Debug, Default)]
pub(in crate::scene_host::recipe) struct RecipeBuildBudget {
    nodes: usize,
    instances: usize,
    vertices: u64,
    indices: u64,
    materials: usize,
    animations: usize,
    animation_channels: usize,
    animation_keyframes: usize,
}

impl RecipeBuildBudget {
    pub(in crate::scene_host::recipe) fn reserve_nodes(
        &mut self,
        policy: &RecipeBuildPolicy,
        diagnostic_path: &str,
        count: usize,
    ) -> Option<SceneRecipeDiagnosticV1> {
        let next = self.nodes.saturating_add(count);
        if next > policy.max_nodes() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next} nodes, exceeding RecipeBuildPolicy max_nodes {}",
                    policy.max_nodes()
                ),
                "reduce imported/authored node count or raise the operator-owned max_nodes policy",
            ));
        }
        self.nodes = next;
        None
    }

    pub(in crate::scene_host::recipe) fn reserve_instances(
        &mut self,
        policy: &RecipeBuildPolicy,
        diagnostic_path: &str,
        count: usize,
    ) -> Option<SceneRecipeDiagnosticV1> {
        let next = self.instances.saturating_add(count);
        if next > policy.max_instances() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next} instances, exceeding RecipeBuildPolicy max_instances {}",
                    policy.max_instances()
                ),
                "reduce imported/authored instance count or raise the operator-owned max_instances policy",
            ));
        }
        self.instances = next;
        None
    }

    pub(in crate::scene_host::recipe) fn reserve_geometry(
        &mut self,
        policy: &RecipeBuildPolicy,
        diagnostic_path: &str,
        vertices: u64,
        indices: u64,
    ) -> Option<SceneRecipeDiagnosticV1> {
        let next_vertices = self.vertices.saturating_add(vertices);
        if next_vertices > policy.max_vertices() as u64 {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next_vertices} vertices, exceeding RecipeBuildPolicy max_vertices {}",
                    policy.max_vertices()
                ),
                "reduce imported/authored geometry detail or raise the operator-owned max_vertices policy",
            ));
        }
        let next_indices = self.indices.saturating_add(indices);
        if next_indices > policy.max_indices() as u64 {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next_indices} indices, exceeding RecipeBuildPolicy max_indices {}",
                    policy.max_indices()
                ),
                "reduce imported/authored geometry detail or raise the operator-owned max_indices policy",
            ));
        }
        self.vertices = next_vertices;
        self.indices = next_indices;
        None
    }

    pub(in crate::scene_host::recipe) fn reserve_materials(
        &mut self,
        policy: &RecipeBuildPolicy,
        diagnostic_path: &str,
        count: usize,
    ) -> Option<SceneRecipeDiagnosticV1> {
        let next = self.materials.saturating_add(count);
        if next > policy.max_materials() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next} materials, exceeding RecipeBuildPolicy max_materials {}",
                    policy.max_materials()
                ),
                "reduce material count or raise the operator-owned max_materials policy",
            ));
        }
        self.materials = next;
        None
    }

    pub(in crate::scene_host::recipe) fn reserve_animation(
        &mut self,
        policy: &RecipeBuildPolicy,
        diagnostic_path: &str,
        channels: usize,
        keyframes: usize,
    ) -> Option<SceneRecipeDiagnosticV1> {
        let next_animations = self.animations.saturating_add(1);
        if next_animations > policy.max_animations() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next_animations} animations, exceeding RecipeBuildPolicy max_animations {}",
                    policy.max_animations()
                ),
                "reduce animation count or raise the operator-owned max_animations policy",
            ));
        }
        let next_channels = self.animation_channels.saturating_add(channels);
        if next_channels > policy.max_animation_channels() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next_channels} animation channels, exceeding RecipeBuildPolicy max_animation_channels {}",
                    policy.max_animation_channels()
                ),
                "reduce animation channel count or raise the operator-owned max_animation_channels policy",
            ));
        }
        let next_keyframes = self.animation_keyframes.saturating_add(keyframes);
        if next_keyframes > policy.max_animation_keyframes() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would contain {next_keyframes} animation keyframes, exceeding RecipeBuildPolicy max_animation_keyframes {}",
                    policy.max_animation_keyframes()
                ),
                "reduce keyframe count or raise the operator-owned max_animation_keyframes policy",
            ));
        }
        self.animations = next_animations;
        self.animation_channels = next_channels;
        self.animation_keyframes = next_keyframes;
        None
    }
}

#[derive(Debug, Default)]
pub(in crate::scene_host::recipe) struct RecipeTextureBudget {
    texture_count: usize,
    decoded_texture_bytes: usize,
}

impl RecipeTextureBudget {
    pub(in crate::scene_host::recipe) fn reserve_texture_uri(
        &mut self,
        policy: &RecipeBuildPolicy,
        recipe_path: &str,
        uri: &str,
        diagnostic_path: impl Into<String>,
    ) -> Result<String, Box<SceneRecipeDiagnosticV1>> {
        let diagnostic_path = diagnostic_path.into();
        self.reserve_texture_slot(policy, &diagnostic_path)?;
        let resolved = policy.resolve_import_uri(recipe_path, uri, diagnostic_path.clone())?;
        self.check_local_texture_budget(policy, &resolved, &diagnostic_path)?;
        Ok(resolved)
    }

    pub(in crate::scene_host::recipe) fn reserve_environment_uri(
        &mut self,
        policy: &RecipeBuildPolicy,
        recipe_path: &str,
        uri: &str,
        diagnostic_path: impl Into<String>,
    ) -> Result<String, Box<SceneRecipeDiagnosticV1>> {
        let diagnostic_path = diagnostic_path.into();
        self.reserve_texture_slot(policy, &diagnostic_path)?;
        let resolved = policy.resolve_import_uri(recipe_path, uri, diagnostic_path.clone())?;
        self.check_local_texture_budget(policy, &resolved, &diagnostic_path)?;
        Ok(resolved)
    }

    pub(in crate::scene_host::recipe) fn reserve_builtin_environment(
        &mut self,
        policy: &RecipeBuildPolicy,
        source_bytes: usize,
        diagnostic_path: impl Into<String>,
    ) -> Result<(), Box<SceneRecipeDiagnosticV1>> {
        let diagnostic_path = diagnostic_path.into();
        self.reserve_texture_slot(policy, &diagnostic_path)?;
        if source_bytes > policy.fetch_byte_limit() {
            return Err(Box::new(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "bundled environment source is {source_bytes} bytes, exceeding RecipeBuildPolicy fetch_byte_limit {}",
                    policy.fetch_byte_limit()
                ),
                "choose a smaller preset or raise the operator-owned fetch_byte_limit policy",
            )));
        }
        if source_bytes > policy.max_texture_bytes() {
            return Err(Box::new(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "bundled environment source is {source_bytes} bytes, exceeding RecipeBuildPolicy max_texture_bytes {}",
                    policy.max_texture_bytes()
                ),
                "choose a smaller preset or raise the operator-owned max_texture_bytes policy",
            )));
        }
        Ok(())
    }

    pub(in crate::scene_host::recipe) fn reserve_loaded_textures(
        &mut self,
        policy: &RecipeBuildPolicy,
        diagnostic_path: &str,
        texture_count: usize,
        decoded_bytes: usize,
    ) -> Option<SceneRecipeDiagnosticV1> {
        let next_count = self.texture_count.saturating_add(texture_count);
        if next_count > policy.max_textures() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe build would reference {next_count} textures, exceeding RecipeBuildPolicy max_textures {}",
                    policy.max_textures()
                ),
                "use fewer textures or raise the operator-owned max_textures policy",
            ));
        }
        let next_bytes = self.decoded_texture_bytes.saturating_add(decoded_bytes);
        if next_bytes > policy.max_texture_bytes() {
            return Some(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "decoded textures would use {next_bytes} bytes, exceeding RecipeBuildPolicy max_texture_bytes {}",
                    policy.max_texture_bytes()
                ),
                "use smaller textures or raise the operator-owned max_texture_bytes policy",
            ));
        }
        self.texture_count = next_count;
        self.decoded_texture_bytes = next_bytes;
        None
    }

    fn reserve_texture_slot(
        &mut self,
        policy: &RecipeBuildPolicy,
        diagnostic_path: &str,
    ) -> Result<(), Box<SceneRecipeDiagnosticV1>> {
        let next_count = self.texture_count.saturating_add(1);
        if next_count > policy.max_textures() {
            return Err(Box::new(error_diagnostic(
                diagnostic_path,
                "policy_violation",
                format!(
                    "recipe references {next_count} textures, exceeding RecipeBuildPolicy max_textures {}",
                    policy.max_textures()
                ),
                "use fewer textures or raise the operator-owned max_textures policy",
            )));
        }
        self.texture_count = next_count;
        Ok(())
    }

    fn check_local_texture_budget(
        &mut self,
        policy: &RecipeBuildPolicy,
        resolved: &str,
        diagnostic_path: &str,
    ) -> Result<(), Box<SceneRecipeDiagnosticV1>> {
        let path = std::path::Path::new(resolved);
        let metadata = match std::fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => return Ok(()),
        };
        if metadata.is_file() {
            let source_bytes = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
            if source_bytes > policy.fetch_byte_limit() {
                return Err(Box::new(error_diagnostic(
                    diagnostic_path,
                    "policy_violation",
                    format!(
                        "texture source is {source_bytes} bytes, exceeding RecipeBuildPolicy fetch_byte_limit {}",
                        policy.fetch_byte_limit()
                    ),
                    "use a smaller texture or raise the operator-owned fetch_byte_limit policy",
                )));
            }
            if source_bytes > policy.max_texture_bytes() {
                return Err(Box::new(error_diagnostic(
                    diagnostic_path,
                    "policy_violation",
                    format!(
                        "texture source is {source_bytes} bytes, exceeding RecipeBuildPolicy max_texture_bytes {}",
                        policy.max_texture_bytes()
                    ),
                    "use a smaller texture or raise the operator-owned max_texture_bytes policy",
                )));
            }
        }

        if let Ok((width, height)) = image::image_dimensions(path) {
            let max_dimension = width.max(height);
            if max_dimension > policy.max_image_dimension() {
                return Err(Box::new(error_diagnostic(
                    diagnostic_path,
                    "policy_violation",
                    format!(
                        "texture dimensions {width}x{height} exceed RecipeBuildPolicy max_image_dimension {}",
                        policy.max_image_dimension()
                    ),
                    "use smaller textures or raise the operator-owned max_image_dimension policy",
                )));
            }
            let decoded_bytes = decoded_rgba8_bytes(width, height)?;
            let next_decoded_bytes = self.decoded_texture_bytes.saturating_add(decoded_bytes);
            if next_decoded_bytes > policy.max_texture_bytes() {
                return Err(Box::new(error_diagnostic(
                    diagnostic_path,
                    "policy_violation",
                    format!(
                        "decoded textures project {next_decoded_bytes} bytes, exceeding RecipeBuildPolicy max_texture_bytes {}",
                        policy.max_texture_bytes()
                    ),
                    "use smaller textures or raise the operator-owned max_texture_bytes policy",
                )));
            }
            self.decoded_texture_bytes = next_decoded_bytes;
        }
        Ok(())
    }
}

fn decoded_rgba8_bytes(width: u32, height: u32) -> Result<usize, Box<SceneRecipeDiagnosticV1>> {
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            Box::new(error_diagnostic(
                "$",
                "policy_violation",
                "texture dimensions overflow decoded-byte projection",
                "use smaller textures before building the recipe",
            ))
        })?;
    usize::try_from(bytes).map_err(|_| {
        Box::new(error_diagnostic(
            "$",
            "policy_violation",
            "texture dimensions exceed this platform's addressable memory",
            "use smaller textures before building the recipe",
        ))
    })
}
