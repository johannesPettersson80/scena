use std::collections::HashMap;

use crate::assets::{EnvironmentDesc, EnvironmentHandle, EnvironmentSidecarProfile};

use super::{Renderer, prepare};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EnvironmentLightingCacheKey {
    environment_identity: Option<EnvironmentIdentity>,
    profile: prepare::EnvironmentLightingProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EnvironmentIdentity {
    name: String,
    source_path: String,
    source_sha256: Option<String>,
    source_dimensions: Option<(u32, u32)>,
    cubemap_resolution: u32,
    brdf_lut_size: u32,
    wasm_delivery: u8,
    prefilter_sidecar: Option<EnvironmentSidecarIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EnvironmentSidecarIdentity {
    profile: EnvironmentSidecarProfile,
    source_sha256: [u8; 32],
    cubemap_resolution: u32,
    brdf_lut_size: u32,
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
        if let Some(cache) = &self.environment_lighting_cache.active
            && cache.environment == self.environment
            && cache.revision == self.environment_revision
            && cache.key.matches(environment_desc, profile)
        {
            return cache.lighting.clone();
        }
        let key = EnvironmentLightingCacheKey::new(environment_desc, profile);
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

    fn matches(
        &self,
        environment_desc: Option<&EnvironmentDesc>,
        profile: prepare::EnvironmentLightingProfile,
    ) -> bool {
        self.profile == profile
            && match (&self.environment_identity, environment_desc) {
                (None, None) => true,
                (Some(identity), Some(environment)) => identity.matches(environment),
                _ => false,
            }
    }
}

fn environment_identity(environment: &EnvironmentDesc) -> EnvironmentIdentity {
    let prefilter_sidecar = environment
        .prefilter_sidecar_profile()
        .and_then(|profile| environment.prefilter_sidecar(profile))
        .map(|sidecar| EnvironmentSidecarIdentity {
            profile: sidecar.profile(),
            source_sha256: sidecar.header().source_sha256_bytes(),
            cubemap_resolution: sidecar.cubemap_resolution(),
            brdf_lut_size: sidecar.brdf_lut_size(),
        });
    EnvironmentIdentity {
        name: environment.name().to_owned(),
        source_path: environment.source_path().as_str().to_owned(),
        source_sha256: environment.source_sha256().map(str::to_owned),
        source_dimensions: environment.source_dimensions(),
        cubemap_resolution: environment.cubemap_resolution(),
        brdf_lut_size: environment.brdf_lut_size(),
        wasm_delivery: environment.wasm_delivery() as u8,
        prefilter_sidecar,
    }
}

impl EnvironmentIdentity {
    fn matches(&self, environment: &EnvironmentDesc) -> bool {
        self.name == environment.name()
            && self.source_path == environment.source_path().as_str()
            && self.source_sha256.as_deref() == environment.source_sha256()
            && self.source_dimensions == environment.source_dimensions()
            && self.cubemap_resolution == environment.cubemap_resolution()
            && self.brdf_lut_size == environment.brdf_lut_size()
            && self.wasm_delivery == environment.wasm_delivery() as u8
            && self.prefilter_sidecar
                == environment
                    .prefilter_sidecar_profile()
                    .and_then(|profile| environment.prefilter_sidecar(profile))
                    .map(|sidecar| EnvironmentSidecarIdentity {
                        profile: sidecar.profile(),
                        source_sha256: sidecar.header().source_sha256_bytes(),
                        cubemap_resolution: sidecar.cubemap_resolution(),
                        brdf_lut_size: sidecar.brdf_lut_size(),
                    })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;

    #[test]
    fn environment_cache_identity_is_typed_and_field_complete() {
        let assets = Assets::new();
        let environment = assets.default_environment();
        let desc = assets
            .environment(environment)
            .expect("default environment descriptor exists");

        let identity = environment_identity(&desc);

        assert_eq!(identity.name, desc.name());
        assert_eq!(identity.source_path, desc.source_path().as_str());
        assert_eq!(identity.source_sha256.as_deref(), desc.source_sha256());
        assert_eq!(identity.source_dimensions, desc.source_dimensions());
        assert_eq!(identity.cubemap_resolution, desc.cubemap_resolution());
        assert_eq!(identity.brdf_lut_size, desc.brdf_lut_size());
    }

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
