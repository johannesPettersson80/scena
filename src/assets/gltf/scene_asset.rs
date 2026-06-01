use std::collections::BTreeSet;
use std::sync::Arc;

use crate::animation::{AnimationSourceChannel, AnimationSourceClip};
use crate::geometry::Aabb;
use crate::scene::view_math::{transform_aabb, union_aabb};
use crate::scene::{Light, SourceCoordinateSystem, SourceUnits, Transform};
use serde::{Deserialize, Serialize};

use super::{
    GltfExtensionDiagnostic, MaterialVariantBinding, SceneAssetAnchor, SceneAssetConnector,
    SceneAssetSkin,
};
use crate::assets::{AssetPath, AssetProvenance, GeometryHandle, MaterialHandle};

pub const ASSET_GEOMETRY_SUMMARY_SCHEMA_V1: &str = "scena.asset_geometry_summary.v1";

#[derive(Debug, Clone)]
pub struct SceneAsset {
    pub(in crate::assets::gltf) inner: Arc<SceneAssetData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAssetGeometrySummary {
    pub schema: String,
    pub node_count: usize,
    pub mesh_count: usize,
    pub primitive_count: usize,
    pub bounds: Option<Aabb>,
    pub provenance: AssetProvenance,
    pub source_units: Vec<SourceUnits>,
    pub source_coordinate_systems: Vec<SourceCoordinateSystem>,
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
    pub(in crate::assets::gltf) provenance: AssetProvenance,
    pub(in crate::assets::gltf) retained_source_bytes: Option<Arc<[u8]>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAssetNode {
    pub(in crate::assets::gltf) name: Option<String>,
    pub(in crate::assets::gltf) children: Vec<usize>,
    pub(in crate::assets::gltf) transform: Transform,
    pub(in crate::assets::gltf) meshes: Vec<SceneAssetMesh>,
    pub(in crate::assets::gltf) instance_transforms: Vec<Transform>,
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
                provenance: AssetProvenance::new("memory:empty"),
                retained_source_bytes: None,
            }),
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.inner.path
    }

    pub fn provenance(&self) -> &AssetProvenance {
        &self.inner.provenance
    }

    pub fn node_count(&self) -> usize {
        self.inner.node_count
    }

    pub fn mesh_count(&self) -> usize {
        self.inner.mesh_count
    }

    pub fn primitive_count(&self) -> usize {
        self.inner.nodes.iter().map(|node| node.meshes.len()).sum()
    }

    pub fn bounds(&self) -> Option<Aabb> {
        let roots = self.root_indices();
        roots
            .into_iter()
            .filter_map(|root| self.node_bounds_in_asset_space(root, Transform::IDENTITY))
            .reduce(union_aabb)
    }

    pub fn geometry_summary(&self) -> SceneAssetGeometrySummary {
        let source_units = self.source_units_summary();
        SceneAssetGeometrySummary {
            schema: ASSET_GEOMETRY_SUMMARY_SCHEMA_V1.to_owned(),
            node_count: self.node_count(),
            mesh_count: self.mesh_count(),
            primitive_count: self.primitive_count(),
            bounds: self.bounds(),
            provenance: self.provenance().clone(),
            source_units,
            source_coordinate_systems: Vec::new(),
        }
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

    fn root_indices(&self) -> Vec<usize> {
        let mut child_indices = BTreeSet::new();
        for node in &self.inner.nodes {
            child_indices.extend(node.children.iter().copied());
        }
        (0..self.inner.nodes.len())
            .filter(|index| !child_indices.contains(index))
            .collect()
    }

    fn node_bounds_in_asset_space(
        &self,
        node_index: usize,
        asset_from_parent: Transform,
    ) -> Option<Aabb> {
        let node = self.inner.nodes.get(node_index)?;
        let asset_from_node = Transform::compose(asset_from_parent, node.transform);
        let mut bounds = node
            .meshes
            .iter()
            .filter_map(|mesh| mesh_bounds_in_node_space(mesh, &node.instance_transforms))
            .map(|bounds| transform_aabb(bounds, asset_from_node))
            .reduce(union_aabb);

        for child in &node.children {
            if let Some(child_bounds) = self.node_bounds_in_asset_space(*child, asset_from_node) {
                bounds =
                    Some(bounds.map_or(child_bounds, |bounds| union_aabb(bounds, child_bounds)));
            }
        }
        bounds
    }

    fn source_units_summary(&self) -> Vec<SourceUnits> {
        let mut units = BTreeSet::new();
        for node in &self.inner.nodes {
            for anchor in &node.anchors {
                if let Some(source_units) = anchor.source_units() {
                    units.insert(source_units);
                }
            }
        }
        units.into_iter().collect()
    }
}

fn mesh_bounds_in_node_space(mesh: &SceneAssetMesh, instances: &[Transform]) -> Option<Aabb> {
    if instances.is_empty() {
        return Some(mesh.bounds);
    }
    instances
        .iter()
        .map(|instance| transform_aabb(mesh.bounds, *instance))
        .reduce(union_aabb)
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

    pub fn instance_transforms(&self) -> &[Transform] {
        &self.instance_transforms
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
