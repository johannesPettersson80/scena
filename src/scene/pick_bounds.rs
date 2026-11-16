//! Local bounds the scene records for asset-backed nodes, so picks that take
//! no `Assets` can still see mesh and instance-set nodes.
//!
//! A scene holds handles, not geometry. Whenever something that does hold the
//! geometry passes by (glTF import, `Renderer::prepare_with_assets`), the
//! scene keeps the local bounds of each node's pickable surface. A plain pick
//! intersects those bounds through the node's world matrix, and reports a
//! structured error for any visible node it has no usable record for.

use slotmap::SlotMap;

use crate::assets::{Assets, GeometryHandle, MaterialHandle, SceneAssetMesh};
use crate::diagnostics::LookupError;
use crate::geometry::{Aabb, GeometryTopology};
use crate::material::MaterialKind;

use super::{InstanceSet, InstanceSetKey, NodeKey, NodeKind, Scene};

/// What the scene last learned about the pickable surface of one node.
///
/// The record names the geometry and material it was taken from. Both are
/// immutable behind their handles, so a record is valid exactly as long as
/// the node still references the same pair; no mutation has to invalidate it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct RecordedPickBounds {
    geometry: GeometryHandle,
    material: MaterialHandle,
    /// Local bounds of pickable triangle geometry. `None` for line topology
    /// and stroke materials, which picking ignores.
    surface: Option<Aabb>,
}

impl Scene {
    /// Records the local pick bounds of every mesh and instance-set node
    /// whose geometry and material resolve in `assets`.
    ///
    /// Nodes that already hold a record for their current geometry and
    /// material are skipped without touching `assets`, so repeating this on
    /// an unchanged scene costs one map lookup per node.
    pub(crate) fn record_pick_bounds<F>(&mut self, assets: &Assets<F>) {
        for (node, node_ref) in &self.nodes {
            let Some((geometry, material)) = asset_handles(&node_ref.kind, &self.instance_sets)
            else {
                continue;
            };
            if self
                .pick_bounds
                .get(&node)
                .is_some_and(|record| record.describes(geometry, material))
            {
                continue;
            }
            let resolved = assets
                .geometry_snapshot(geometry)
                .zip(assets.material_snapshot(material));
            match resolved {
                Some((geometry_desc, material_desc)) => {
                    let pickable = geometry_desc.topology() == GeometryTopology::Triangles
                        && !is_stroke_material(material_desc.kind());
                    self.pick_bounds.insert(
                        node,
                        RecordedPickBounds {
                            geometry,
                            material,
                            surface: pickable.then(|| geometry_desc.bounds()),
                        },
                    );
                }
                None => {
                    self.pick_bounds.remove(&node);
                }
            }
        }
    }

    /// Records pick bounds for a node instantiated from an imported mesh,
    /// whose topology and bounds the asset already carries.
    pub(super) fn record_imported_pick_bounds(&mut self, node: NodeKey, mesh: &SceneAssetMesh) {
        self.pick_bounds.insert(
            node,
            RecordedPickBounds {
                geometry: mesh.geometry(),
                material: mesh.material(),
                surface: (mesh.topology() == GeometryTopology::Triangles).then(|| mesh.bounds()),
            },
        );
    }

    pub(super) fn forget_pick_bounds(&mut self, node: NodeKey) {
        self.pick_bounds.remove(&node);
    }

    /// Local bounds of the pickable surface of a visible mesh or
    /// instance-set node, for a pick that has no `Assets`.
    ///
    /// `Ok(None)` means the node is known line or stroke geometry, which
    /// picking ignores.
    ///
    /// # Errors
    ///
    /// Returns [`LookupError::PickRequiresAssets`] when the scene holds no
    /// record for the node's current geometry and material, or when the node
    /// is morphed or skinned: recorded bounds describe the undeformed mesh,
    /// which a deformed pose can leave.
    pub(crate) fn recorded_pick_surface(
        &self,
        node: NodeKey,
        geometry: GeometryHandle,
        material: MaterialHandle,
    ) -> Result<Option<Aabb>, LookupError> {
        let record = self
            .pick_bounds
            .get(&node)
            .filter(|record| record.describes(geometry, material))
            .ok_or(LookupError::PickRequiresAssets { node })?;
        if record.surface.is_some() && self.is_deformed(node) {
            return Err(LookupError::PickRequiresAssets { node });
        }
        Ok(record.surface)
    }

    fn is_deformed(&self, node: NodeKey) -> bool {
        self.skin_bindings.contains_key(&node)
            || self
                .morph_weights
                .get(&node)
                .is_some_and(|weights| weights.iter().any(|weight| *weight != 0.0))
    }
}

impl RecordedPickBounds {
    fn describes(&self, geometry: GeometryHandle, material: MaterialHandle) -> bool {
        self.geometry == geometry && self.material == material
    }
}

fn asset_handles(
    kind: &NodeKind,
    instance_sets: &SlotMap<InstanceSetKey, InstanceSet>,
) -> Option<(GeometryHandle, MaterialHandle)> {
    match kind {
        NodeKind::Mesh(mesh) => Some((mesh.geometry(), mesh.material())),
        NodeKind::InstanceSet(instance_set) => instance_sets
            .get(*instance_set)
            .map(|instance_set| (instance_set.geometry(), instance_set.material())),
        NodeKind::Empty
        | NodeKind::Renderable(_)
        | NodeKind::Model(_)
        | NodeKind::ParticleSet(_)
        | NodeKind::Label(_)
        | NodeKind::Camera(_)
        | NodeKind::Light(_) => None,
    }
}

const fn is_stroke_material(kind: MaterialKind) -> bool {
    matches!(
        kind,
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge
    )
}
