use serde::{Deserialize, Serialize};

/// Semantic validation and resource-policy inputs for one scene load.
///
/// Every field affects whether source bytes are acceptable, so scene-cache
/// reuse must either match these options or prove that stored load evidence
/// satisfies them. The options are included in [`super::AssetLoadReport`] and
/// its stable schema report so callers can audit compatible cache hits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct AssetLoadOptions {
    strict_textures: bool,
    strict_external_resources: bool,
    fetch_byte_limit: Option<usize>,
    #[serde(default, skip_serializing_if = "GltfSceneSelection::is_default")]
    gltf_scene: GltfSceneSelection,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GltfSceneSelection {
    #[default]
    Default,
    Index {
        index: usize,
    },
    Name {
        name: String,
    },
}

impl GltfSceneSelection {
    fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }
}

impl AssetLoadOptions {
    pub const fn new() -> Self {
        Self {
            strict_textures: false,
            strict_external_resources: false,
            fetch_byte_limit: None,
            gltf_scene: GltfSceneSelection::Default,
        }
    }

    /// Makes a missing referenced external image a hard load error.
    pub const fn with_strict_textures(mut self, strict_textures: bool) -> Self {
        self.strict_textures = strict_textures;
        self
    }

    pub const fn strict_textures(&self) -> bool {
        self.strict_textures
    }

    /// Makes a missing referenced external buffer a hard load error.
    pub const fn with_strict_external_resources(mut self, strict_external_resources: bool) -> Self {
        self.strict_external_resources = strict_external_resources;
        self
    }

    pub const fn strict_external_resources(&self) -> bool {
        self.strict_external_resources
    }

    /// Limits the combined scene and external-resource source bytes fetched.
    pub const fn with_fetch_byte_limit(mut self, fetch_byte_limit: usize) -> Self {
        self.fetch_byte_limit = Some(fetch_byte_limit);
        self
    }

    pub const fn fetch_byte_limit(&self) -> Option<usize> {
        self.fetch_byte_limit
    }

    pub fn with_gltf_scene_index(mut self, index: usize) -> Self {
        self.gltf_scene = GltfSceneSelection::Index { index };
        self
    }

    pub fn with_gltf_scene_name(mut self, name: impl Into<String>) -> Self {
        self.gltf_scene = GltfSceneSelection::Name { name: name.into() };
        self
    }

    pub const fn gltf_scene(&self) -> &GltfSceneSelection {
        &self.gltf_scene
    }
}
