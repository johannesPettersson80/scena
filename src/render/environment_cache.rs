use std::collections::HashMap;

use crate::assets::{EnvironmentDesc, EnvironmentHandle};

use super::{Renderer, prepare};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EnvironmentLightingCacheKey {
    environment_identity: Option<String>,
    profile: prepare::EnvironmentLightingProfile,
}

#[derive(Debug, Clone, Default)]
pub(super) struct EnvironmentLightingCache {
    active: Option<ActiveEnvironmentLightingCache>,
    entries: HashMap<EnvironmentLightingCacheKey, prepare::PreparedEnvironmentLighting>,
}

#[derive(Debug, Clone)]
struct ActiveEnvironmentLightingCache {
    pub(super) environment: Option<EnvironmentHandle>,
    pub(super) revision: u64,
    key: EnvironmentLightingCacheKey,
    pub(super) lighting: prepare::PreparedEnvironmentLighting,
}

impl EnvironmentLightingCache {
    pub(super) fn clear_active(&mut self) {
        self.active = None;
    }
}

impl Renderer {
    pub(super) fn environment_lighting_for_prepare(
        &mut self,
        environment_desc: Option<&EnvironmentDesc>,
    ) -> prepare::PreparedEnvironmentLighting {
        let profile = prepare::EnvironmentLightingProfile::for_backend(self.target.backend);
        let key = EnvironmentLightingCacheKey::new(environment_desc, profile);
        if let Some(cache) = &self.environment_lighting_cache.active
            && cache.environment == self.environment
            && cache.revision == self.environment_revision
            && cache.key == key
        {
            return cache.lighting.clone();
        }
        if let Some(lighting) = self.environment_lighting_cache.entries.get(&key).cloned() {
            self.environment_lighting_cache.active = Some(ActiveEnvironmentLightingCache {
                environment: self.environment,
                revision: self.environment_revision,
                key,
                lighting: lighting.clone(),
            });
            return lighting;
        }
        let lighting = prepare::collect_environment_lighting(environment_desc, self.target.backend);
        self.environment_lighting_cache
            .entries
            .insert(key.clone(), lighting.clone());
        self.environment_lighting_cache.active = Some(ActiveEnvironmentLightingCache {
            environment: self.environment,
            revision: self.environment_revision,
            key,
            lighting: lighting.clone(),
        });
        lighting
    }
}

impl EnvironmentLightingCacheKey {
    fn new(
        environment_desc: Option<&EnvironmentDesc>,
        profile: prepare::EnvironmentLightingProfile,
    ) -> Self {
        Self {
            environment_identity: environment_desc.map(environment_identity),
            profile,
        }
    }
}

fn environment_identity(environment: &EnvironmentDesc) -> String {
    format!(
        "{}|{}|{:?}|{:?}|{}|{}|{}|{:?}",
        environment.name(),
        environment.source_path().as_str(),
        environment.source_sha256().unwrap_or(""),
        environment.source_dimensions(),
        environment.cubemap_resolution(),
        environment.brdf_lut_size(),
        environment.wasm_delivery() as u8,
        environment.prefilter_sidecar_identity(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;

    #[test]
    fn unchanged_environment_revision_reuses_prepared_environment_lighting_cache() {
        let assets = Assets::new();
        let environment = assets.default_environment();
        let environment_desc = assets
            .environment(environment)
            .expect("default environment descriptor exists");
        let mut renderer = Renderer::headless(16, 16).expect("renderer builds");
        renderer.environment = Some(environment);
        renderer.environment_revision = 7;
        let key = EnvironmentLightingCacheKey::new(
            Some(&environment_desc),
            prepare::EnvironmentLightingProfile::for_backend(renderer.target.backend),
        );
        renderer.environment_lighting_cache.active = Some(ActiveEnvironmentLightingCache {
            environment: Some(environment),
            revision: 7,
            key,
            lighting: prepare::PreparedEnvironmentLighting::default(),
        });

        let lighting = renderer.environment_lighting_for_prepare(Some(&environment_desc));

        assert!(
            lighting.cubemap().is_none(),
            "unchanged environment revision must return the cached lighting value instead of recomputing the expensive prefiltered IBL cubemap"
        );

        renderer.clear_environment();
        assert!(
            renderer.environment_lighting_cache.active.is_none(),
            "changing the active environment must invalidate the active IBL cache entry"
        );
    }

    #[test]
    fn renderer_owned_environment_cache_reuses_matching_environment_identity_after_handle_change() {
        let assets = Assets::new();
        let first_environment = assets.default_environment();
        let first_desc = assets
            .environment(first_environment)
            .expect("default environment descriptor exists");
        let second_assets = Assets::new();
        let second_environment = second_assets.default_environment();
        let second_desc = second_assets
            .environment(second_environment)
            .expect("second default environment descriptor exists");
        let mut renderer = Renderer::headless(16, 16).expect("renderer builds");
        renderer.environment = Some(first_environment);
        renderer.environment_revision = 1;
        let first_key = EnvironmentLightingCacheKey::new(
            Some(&first_desc),
            prepare::EnvironmentLightingProfile::for_backend(renderer.target.backend),
        );
        renderer
            .environment_lighting_cache
            .entries
            .insert(first_key, prepare::PreparedEnvironmentLighting::default());

        renderer.environment = Some(second_environment);
        renderer.environment_revision = 2;
        let lighting = renderer.environment_lighting_for_prepare(Some(&second_desc));

        assert!(
            lighting.cubemap().is_none(),
            "moving a renderer between demo apps must reuse prepared IBL when the new Assets store \
             resolves to the same environment identity"
        );
    }
}
