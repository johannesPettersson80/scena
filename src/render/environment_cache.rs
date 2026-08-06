use std::collections::HashMap;

use crate::assets::{Assets, EnvironmentDesc, EnvironmentHandle, EnvironmentSidecarProfile};
use crate::{PrepareError, Scene};

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
            return cache.lighting.clone().with_controls(
                self.environment_intensity,
                self.environment_rotation_y_radians,
            );
        }
        let key = EnvironmentLightingCacheKey::new(environment_desc, profile);
        let lighting = self.cached_environment_lighting(environment_desc, profile);
        self.environment_lighting_cache.active = Some(ActiveEnvironmentLightingCache {
            environment: self.environment,
            revision: self.environment_revision,
            key,
            lighting: lighting.clone(),
        });
        lighting.with_controls(
            self.environment_intensity,
            self.environment_rotation_y_radians,
        )
    }

    pub(super) fn reflection_probe_lighting_for_prepare<F>(
        &mut self,
        scene: &Scene,
        assets: Option<&Assets<F>>,
    ) -> Result<Vec<prepare::PreparedReflectionProbe>, PrepareError> {
        if !scene.reflection_probes_enabled() {
            return Ok(Vec::new());
        }
        let profile = prepare::EnvironmentLightingProfile::for_backend(self.target.backend);
        scene
            .reflection_probes()
            .filter_map(|(key, probe)| {
                probe
                    .environment()
                    .map(|environment| (key, probe, environment))
            })
            .enumerate()
            .map(|(slot, (key, probe, environment))| {
                let assets =
                    assets.ok_or(PrepareError::EnvironmentAssetsRequired { environment })?;
                let description = assets
                    .environment(environment)
                    .ok_or(PrepareError::EnvironmentNotFound { environment })?;
                let lighting = self.cached_environment_lighting(Some(&description), profile);
                Ok(prepare::PreparedReflectionProbe::new(
                    key,
                    slot as u32,
                    probe.bounds(),
                    probe.capture_position(),
                    lighting,
                ))
            })
            .collect()
    }

    fn cached_environment_lighting(
        &mut self,
        environment_desc: Option<&EnvironmentDesc>,
        profile: prepare::EnvironmentLightingProfile,
    ) -> prepare::PreparedEnvironmentLighting {
        let key = EnvironmentLightingCacheKey::new(environment_desc, profile);
        if let Some(lighting) = self.environment_lighting_cache.entries.get(&key).cloned() {
            return lighting;
        }
        let lighting = prepare::PreparedEnvironmentLighting::from_environment_with_profile(
            environment_desc,
            profile,
        );
        self.environment_lighting_cache
            .entries
            .insert(key, lighting.clone());
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
    use crate::{Aabb, EnvironmentDesc, ReflectionProbe, Scene, Transform, Vec3};

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

    #[test]
    fn local_reflection_probes_reuse_renderer_owned_environment_cache() {
        let assets = Assets::new();
        let environment = assets.create_environment(
            EnvironmentDesc::from_cubemap_radiance(
                "scena://generated/reflection-probe/cache-test",
                2,
                std::array::from_fn(|_| vec![[2.0, 1.0, 0.5]; 4]),
            )
            .expect("probe environment is valid"),
        );
        let environment_desc = assets
            .environment(environment)
            .expect("probe environment resolves");
        let mut scene = Scene::new();
        let assigned = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("assigned node inserts");
        scene
            .add_reflection_probe(
                ReflectionProbe::new(Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0)))
                    .with_environment(environment)
                    .assign_node(assigned),
            )
            .expect("probe inserts");
        let mut renderer = Renderer::headless(16, 16).expect("renderer builds");
        let key = EnvironmentLightingCacheKey::new(
            Some(&environment_desc),
            prepare::EnvironmentLightingProfile::for_backend(renderer.target.backend),
        );
        renderer
            .environment_lighting_cache
            .entries
            .insert(key, prepare::PreparedEnvironmentLighting::default());

        let probes = renderer
            .reflection_probe_lighting_for_prepare(&scene, Some(&assets))
            .expect("probe preparation succeeds");

        assert_eq!(probes.len(), 1);
        assert!(
            probes[0].lighting().cubemap().is_none(),
            "probe preparation must reuse the cached sentinel instead of rebaking its HDR cubemap",
        );
    }
}
