use crate::diagnostics::LookupError;

use super::{MeshBuilder, MeshNode, ModelBuilder, ModelNode, NodeKey, NodeKind, Transform};

impl MeshBuilder<'_> {
    /// Overrides the parent node. The parent is validated when [`Self::add`] is called.
    pub fn parent(mut self, parent: NodeKey) -> Self {
        self.parent = parent;
        self
    }

    /// Overrides the local transform. The default is [`Transform::IDENTITY`].
    ///
    /// Mesh geometry is transformed during render preparation, including the active scene
    /// origin shift used for large-scene precision.
    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Inserts the mesh node and returns its typed node key.
    pub fn add(self) -> Result<NodeKey, LookupError> {
        self.scene.insert_node(
            self.parent,
            NodeKind::Mesh(MeshNode {
                geometry: self.geometry,
                material: self.material,
            }),
            self.transform,
        )
    }
}

impl ModelBuilder<'_> {
    /// Overrides the parent node. The parent is validated when [`Self::add`] is called.
    pub fn parent(mut self, parent: NodeKey) -> Self {
        self.parent = parent;
        self
    }

    /// Overrides the local transform. The default is [`Transform::IDENTITY`].
    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Inserts the model node and returns its typed node key.
    pub fn add(self) -> Result<NodeKey, LookupError> {
        self.scene.insert_node(
            self.parent,
            NodeKind::Model(ModelNode { model: self.model }),
            self.transform,
        )
    }
}
