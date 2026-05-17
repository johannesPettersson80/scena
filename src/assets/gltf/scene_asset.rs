use std::sync::Arc;

use crate::animation::{AnimationSourceChannel, AnimationSourceClip};
use crate::geometry::Aabb;
use crate::scene::{Light, Transform};

use super::{
    GltfExtensionDiagnostic, MaterialVariantBinding, SceneAssetAnchor, SceneAssetConnector,
    SceneAssetSkin,
};
use crate::assets::{AssetPath, GeometryHandle, MaterialHandle};

#[derive(Debug, Clone)]
pub struct SceneAsset {
    pub(in crate::assets::gltf) inner: Arc<SceneAssetData>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::assets::gltf) struct SceneAssetData {
    pub(in crate::assets::gltf) path: AssetPath,
    pub(in crate::assets::gltf) node_count: usize,
    pub(in crate::assets::gltf) mesh_count: usize,
    pub(in crate::assets::gltf) nodes: Vec<SceneAssetNode>,
    pub(in crate::assets::gltf) skins: Vec<SceneAssetSkin>,
    pub(in crate::assets::gltf) clips: Vec<SceneAssetClip>,
    pub(in crate::assets::gltf) extensions_used: Vec<String>,
    pub(in crate::assets::gltf) extensions_required: Vec<String>,
    pub(in crate::assets::gltf) extension_diagnostics: Vec<GltfExtensionDiagnostic>,
    pub(in crate::assets::gltf) material_variants: Vec<String>,
    pub(in crate::assets::gltf) retained_source_bytes: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetNode {
    pub(in crate::assets::gltf) name: Option<String>,
    pub(in crate::assets::gltf) children: Vec<usize>,
    pub(in crate::assets::gltf) transform: Transform,
    pub(in crate::assets::gltf) meshes: Vec<SceneAssetMesh>,
    pub(in crate::assets::gltf) skin: Option<usize>,
    pub(in crate::assets::gltf) light: Option<SceneAssetLight>,
    pub(in crate::assets::gltf) anchors: Vec<SceneAssetAnchor>,
    pub(in crate::assets::gltf) connectors: Vec<SceneAssetConnector>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetMesh {
    pub(in crate::assets::gltf) geometry: GeometryHandle,
    pub(in crate::assets::gltf) material: MaterialHandle,
    pub(in crate::assets::gltf) bounds: Aabb,
    pub(in crate::assets::gltf) uses_vertex_colors: bool,
    pub(in crate::assets::gltf) morph_weights: Vec<f32>,
    pub(in crate::assets::gltf) material_variant_bindings: Vec<MaterialVariantBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneAssetLight {
    pub(in crate::assets::gltf) light: Light,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetClip {
    pub(in crate::assets::gltf) clip: AnimationSourceClip,
}

impl SceneAsset {
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(SceneAssetData {
                path: AssetPath::from("memory:empty"),
                node_count: 0,
                mesh_count: 0,
                nodes: Vec::new(),
                skins: Vec::new(),
                clips: Vec::new(),
                extensions_used: Vec::new(),
                extensions_required: Vec::new(),
                extension_diagnostics: Vec::new(),
                material_variants: Vec::new(),
                retained_source_bytes: None,
            }),
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.inner.path
    }

    pub fn node_count(&self) -> usize {
        self.inner.node_count
    }

    pub fn mesh_count(&self) -> usize {
        self.inner.mesh_count
    }

    pub fn nodes(&self) -> &[SceneAssetNode] {
        &self.inner.nodes
    }

    pub fn skins(&self) -> &[SceneAssetSkin] {
        &self.inner.skins
    }

    pub fn clips(&self) -> &[SceneAssetClip] {
        &self.inner.clips
    }

    pub fn extensions_used(&self) -> &[String] {
        &self.inner.extensions_used
    }

    pub fn extensions_required(&self) -> &[String] {
        &self.inner.extensions_required
    }

    pub fn extension_diagnostics(&self) -> &[GltfExtensionDiagnostic] {
        &self.inner.extension_diagnostics
    }

    /// Variant names declared by KHR_materials_variants in declaration
    /// order; empty when the extension is absent (Phase 2B step 1).
    pub fn material_variants(&self) -> &[String] {
        &self.inner.material_variants
    }

    pub fn retained_source_bytes_len(&self) -> Option<usize> {
        self.inner
            .retained_source_bytes
            .as_ref()
            .map(|bytes| bytes.len())
    }

    pub(in crate::assets) fn retained_source_bytes(&self) -> Option<&[u8]> {
        self.inner.retained_source_bytes.as_deref()
    }

    pub(in crate::assets) fn with_retained_source_bytes(mut self, bytes: &[u8]) -> Self {
        Arc::make_mut(&mut self.inner).retained_source_bytes =
            Some(Arc::<[u8]>::from(bytes.to_vec()));
        self
    }
}

impl PartialEq for SceneAsset {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner) || self.inner.path == other.inner.path
    }
}

impl Eq for SceneAsset {}

impl SceneAssetNode {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn children(&self) -> &[usize] {
        &self.children
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn mesh(&self) -> Option<&SceneAssetMesh> {
        self.meshes.first()
    }

    pub fn meshes(&self) -> &[SceneAssetMesh] {
        &self.meshes
    }

    pub const fn skin(&self) -> Option<usize> {
        self.skin
    }

    pub fn light(&self) -> Option<SceneAssetLight> {
        self.light
    }

    pub fn anchors(&self) -> &[SceneAssetAnchor] {
        &self.anchors
    }

    pub fn connectors(&self) -> &[SceneAssetConnector] {
        &self.connectors
    }
}

impl SceneAssetMesh {
    pub const fn geometry(&self) -> GeometryHandle {
        self.geometry
    }

    pub const fn material(&self) -> MaterialHandle {
        self.material
    }

    pub const fn bounds(&self) -> Aabb {
        self.bounds
    }

    pub const fn uses_vertex_colors(&self) -> bool {
        self.uses_vertex_colors
    }

    pub fn morph_weights(&self) -> &[f32] {
        &self.morph_weights
    }

    pub fn material_variant_bindings(&self) -> &[MaterialVariantBinding] {
        &self.material_variant_bindings
    }
}

impl SceneAssetLight {
    pub const fn light(self) -> Light {
        self.light
    }
}

impl SceneAssetClip {
    pub fn name(&self) -> Option<&str> {
        self.clip.name()
    }

    pub fn channels(&self) -> &[AnimationSourceChannel] {
        self.clip.channels()
    }

    pub const fn duration_seconds(&self) -> f32 {
        self.clip.duration_seconds()
    }

    pub(crate) fn clip(&self) -> &AnimationSourceClip {
        &self.clip
    }
}
