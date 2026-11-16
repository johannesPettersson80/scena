//! Default structural limits for one glTF/GLB import.
//!
//! The defaults are far above what authored assets use and exist so a small
//! hostile file cannot make the importer do an unbounded amount of work. Each
//! one can be changed per load through [`super::AssetLoadOptions`].

/// Longest root-to-leaf node chain, counted in nodes; a root has depth 1.
pub(crate) const DEFAULT_MAX_NODE_DEPTH: usize = 1_024;

/// Total number of nodes in the document.
pub(crate) const DEFAULT_MAX_NODE_COUNT: usize = 1_000_000;

/// Structural limits resolved from [`super::AssetLoadOptions`] for one import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GltfImportLimits {
    pub(crate) max_node_depth: usize,
    pub(crate) max_node_count: usize,
}

impl Default for GltfImportLimits {
    fn default() -> Self {
        Self {
            max_node_depth: DEFAULT_MAX_NODE_DEPTH,
            max_node_count: DEFAULT_MAX_NODE_COUNT,
        }
    }
}

impl GltfImportLimits {
    /// Whether an asset admitted under `self` is also acceptable to a caller
    /// who asked for `requested`: every limit it was checked against must be
    /// at least as strict.
    pub(crate) fn within(&self, requested: &Self) -> bool {
        self.max_node_depth <= requested.max_node_depth
            && self.max_node_count <= requested.max_node_count
    }
}
