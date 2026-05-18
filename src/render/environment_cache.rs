use crate::assets::{EnvironmentDesc, EnvironmentHandle};

use super::{Renderer, prepare};

#[derive(Debug, Clone)]
pub(super) struct EnvironmentLightingCache {
    pub(super) environment: Option<EnvironmentHandle>,
    pub(super) revision: u64,
    pub(super) profile: prepare::EnvironmentLightingProfile,
    pub(super) lighting: prepare::PreparedEnvironmentLighting,
}

impl Renderer {
    pub(super) fn environment_lighting_for_prepare(
        &mut self,
        environment_desc: Option<&EnvironmentDesc>,
    ) -> prepare::PreparedEnvironmentLighting {
        let profile = prepare::EnvironmentLightingProfile::for_backend(self.target.backend);
        if let Some(cache) = &self.environment_lighting_cache
            && cache.environment == self.environment
            && cache.revision == self.environment_revision
            && cache.profile == profile
        {
            return cache.lighting.clone();
        }
        let lighting = prepare::collect_environment_lighting(environment_desc, self.target.backend);
        self.environment_lighting_cache = Some(EnvironmentLightingCache {
            environment: self.environment,
            revision: self.environment_revision,
            profile,
            lighting: lighting.clone(),
        });
        lighting
    }
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
        renderer.environment_lighting_cache = Some(EnvironmentLightingCache {
            environment: Some(environment),
            revision: 7,
            profile: prepare::EnvironmentLightingProfile::for_backend(renderer.target.backend),
            lighting: prepare::PreparedEnvironmentLighting::default(),
        });

        let lighting = renderer.environment_lighting_for_prepare(Some(&environment_desc));

        assert!(
            lighting.cubemap().is_none(),
            "unchanged environment revision must return the cached lighting value instead of recomputing the expensive prefiltered IBL cubemap"
        );

        renderer.clear_environment();
        assert!(
            renderer.environment_lighting_cache.is_none(),
            "changing the active environment must invalidate cached IBL data"
        );
    }
}
