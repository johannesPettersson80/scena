use super::handles::{HandleKind, handle_kind};
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, NodeKey, SceneImport};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub(super) fn resolve_parent(&self, parent: Option<u64>) -> Result<NodeKey, SceneHostError> {
        parent.map_or(Ok(self.scene.root()), |parent| self.resolve_node(parent))
    }

    pub(super) fn resolve_node(&self, handle: u64) -> Result<NodeKey, SceneHostError> {
        self.node_handles
            .get(
                handle,
                SceneHostErrorCode::NodeHandleNotFound,
                SceneHostErrorCode::StaleNodeHandle,
            )
            .copied()
    }

    pub(super) fn resolve_import(&self, handle: u64) -> Result<&SceneImport, SceneHostError> {
        self.import_handles.get(
            handle,
            SceneHostErrorCode::ImportHandleNotFound,
            SceneHostErrorCode::StaleImportHandle,
        )
    }

    pub(super) fn register_node(&mut self, node: NodeKey) -> u64 {
        if let Some(handle) = self.node_handle_map.get(&node).copied() {
            return handle;
        }
        let handle = self.node_handles.insert(node);
        self.node_handle_map.insert(node, handle);
        handle
    }

    pub(super) fn register_subtree(&mut self, node: NodeKey) {
        self.register_node(node);
        let children = self
            .scene
            .node(node)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();
        for child in children {
            self.register_subtree(child);
        }
    }

    pub(super) fn invalidate_node_handles(&mut self, nodes: &[NodeKey]) {
        for node in nodes {
            let Some(handle) = self.node_handle_map.remove(node) else {
                continue;
            };
            let _ = self.node_handles.remove(
                handle,
                SceneHostErrorCode::NodeHandleNotFound,
                SceneHostErrorCode::StaleNodeHandle,
            );
        }
    }

    pub(super) fn is_instance_root_handle(&self, handle: u64) -> bool {
        handle_kind(handle) == Some(HandleKind::InstanceRoot)
    }

    pub(super) fn ensure_active_camera(&self) -> Result<(), SceneHostError> {
        if self.scene.camera(self.active_camera).is_some() {
            Ok(())
        } else {
            Err(SceneHostError::new(
                SceneHostErrorCode::NoActiveCamera,
                "SceneHost active camera is missing; create or select a camera before this operation",
            ))
        }
    }
}
