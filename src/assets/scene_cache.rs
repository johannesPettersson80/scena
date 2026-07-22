use super::load::AssetLoadTelemetry;
use super::{AssetLoadOptions, AssetLoadWarning, AssetPath, AssetStorage, SceneAsset};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SceneCacheKey {
    path: AssetPath,
    options: AssetLoadOptions,
}

impl SceneCacheKey {
    pub(super) fn new(path: AssetPath, options: AssetLoadOptions) -> Self {
        Self { path, options }
    }

    pub(super) fn path(&self) -> &AssetPath {
        &self.path
    }

    pub(super) const fn options(&self) -> AssetLoadOptions {
        self.options
    }
}

impl AssetLoadTelemetry {
    fn satisfies(&self, requested: AssetLoadOptions) -> bool {
        if requested.strict_textures()
            && self
                .warnings
                .iter()
                .any(|warning| matches!(warning, AssetLoadWarning::ExternalImageMissing { .. }))
        {
            return false;
        }
        if requested.strict_external_resources()
            && self
                .warnings
                .iter()
                .any(|warning| matches!(warning, AssetLoadWarning::ExternalBufferMissing { .. }))
        {
            return false;
        }
        requested
            .fetch_byte_limit()
            .is_none_or(|limit| self.fetched_bytes <= limit)
    }
}

impl AssetStorage {
    pub(super) fn cached_scene(
        &self,
        path: &AssetPath,
        requested: AssetLoadOptions,
    ) -> Option<(SceneAsset, AssetLoadTelemetry, AssetLoadOptions)> {
        let exact_key = SceneCacheKey::new(path.clone(), requested);
        if let Some(scene) = self.scene_lookup.get(&exact_key) {
            let telemetry = self
                .scene_load_telemetry
                .get(&exact_key)
                .cloned()
                .unwrap_or_default();
            return Some((scene.clone(), telemetry, requested));
        }

        self.scene_lookup.iter().find_map(|(key, scene)| {
            if key.path() != path {
                return None;
            }
            let telemetry = self.scene_load_telemetry.get(key)?.clone();
            telemetry
                .satisfies(requested)
                .then(|| (scene.clone(), telemetry, key.options()))
        })
    }

    pub(super) fn cache_scene(
        &mut self,
        path: AssetPath,
        options: AssetLoadOptions,
        scene: SceneAsset,
        telemetry: AssetLoadTelemetry,
    ) {
        let key = SceneCacheKey::new(path, options);
        self.scene_lookup.insert(key.clone(), scene);
        self.scene_load_telemetry.insert(key, telemetry);
    }

    pub(super) fn replace_cached_scene(
        &mut self,
        path: AssetPath,
        options: AssetLoadOptions,
        scene: SceneAsset,
        telemetry: AssetLoadTelemetry,
    ) {
        self.scene_lookup.retain(|key, _| key.path() != &path);
        self.scene_load_telemetry
            .retain(|key, _| key.path() != &path);
        self.cache_scene(path, options, scene, telemetry);
    }

    pub(super) fn telemetry_for_path(&self, path: &AssetPath) -> AssetLoadTelemetry {
        self.scene_load_telemetry
            .iter()
            .find_map(|(key, telemetry)| (key.path() == path).then(|| telemetry.clone()))
            .unwrap_or_default()
    }
}
