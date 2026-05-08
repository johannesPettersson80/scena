use crate::geometry::Aabb;
use crate::scene::{MeshNode, NodeKey, NodeKind};

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::assets::{SceneAsset, SceneAssetMesh};
use crate::diagnostics::ImportDiagnosticOverlay;

use super::{ImportAnchor, ImportConnector, ImportOptions};

pub(super) struct ImportBuild<'a> {
    pub(super) scene_asset: &'a SceneAsset,
    pub(super) options: ImportOptions,
    pub(super) import_live: &'a Arc<AtomicBool>,
    pub(super) records: &'a mut Vec<ImportedNode>,
    pub(super) anchors: &'a mut Vec<ImportAnchor>,
    pub(super) connectors: &'a mut Vec<ImportConnector>,
    pub(super) diagnostic_overlays: &'a mut Vec<ImportDiagnosticOverlay>,
    pub(super) pending_skin_bindings: &'a mut Vec<PendingSkinBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportedNode {
    pub(super) source_index: usize,
    pub(super) node: NodeKey,
    pub(super) parent: Option<NodeKey>,
    pub(super) name: Option<String>,
    pub(super) bounds: Option<Aabb>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PendingSkinBinding {
    pub(super) node: NodeKey,
    pub(super) source_node: usize,
    pub(super) skin: usize,
}

impl PartialEq for ImportAnchor {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.node == other.node
            && self.placement_node == other.placement_node
            && self.transform == other.transform
            && self.placement_transform == other.placement_transform
            && self.tags == other.tags
            && self.label == other.label
    }
}

impl PartialEq for ImportConnector {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.kind == other.kind
            && self.allowed_mates == other.allowed_mates
            && self.tags == other.tags
            && self.snap_tolerance == other.snap_tolerance
            && self.clearance_hint == other.clearance_hint
            && self.roll_policy == other.roll_policy
            && self.polarity == other.polarity
            && self.metadata == other.metadata
            && self.node == other.node
            && self.placement_node == other.placement_node
            && self.transform == other.transform
            && self.placement_transform == other.placement_transform
    }
}

pub(super) fn mesh_node_kind(mesh: &SceneAssetMesh) -> NodeKind {
    NodeKind::Mesh(MeshNode {
        geometry: mesh.geometry(),
        material: mesh.material(),
    })
}
