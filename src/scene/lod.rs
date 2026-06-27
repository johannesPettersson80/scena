use crate::assets::GeometryHandle;
use crate::diagnostics::LookupError;

use super::{NodeKey, NodeKind, Scene};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshLodLevel {
    max_screen_fraction: f32,
    geometry: GeometryHandle,
}

impl MeshLodLevel {
    pub fn new(max_screen_fraction: f32, geometry: GeometryHandle) -> Self {
        Self {
            max_screen_fraction,
            geometry,
        }
    }

    pub const fn max_screen_fraction(self) -> f32 {
        self.max_screen_fraction
    }

    pub const fn geometry(self) -> GeometryHandle {
        self.geometry
    }
}

impl Scene {
    pub fn set_mesh_lods(
        &mut self,
        node: NodeKey,
        levels: Vec<MeshLodLevel>,
    ) -> Result<(), LookupError> {
        let Some(node_ref) = self.nodes.get(node) else {
            return Err(LookupError::NodeNotFound(node));
        };
        if !matches!(node_ref.kind, NodeKind::Mesh(_)) {
            return Err(LookupError::InvalidFramingOption {
                field: "node",
                reason: "LOD levels can only be attached to mesh nodes",
            });
        }
        for level in &levels {
            if !level.max_screen_fraction.is_finite()
                || level.max_screen_fraction <= 0.0
                || level.max_screen_fraction > 1.0
            {
                return Err(LookupError::InvalidFramingOption {
                    field: "max_screen_fraction",
                    reason: "LOD thresholds must be finite and in the range (0, 1]",
                });
            }
        }
        let mut levels = levels;
        levels.sort_by(|left, right| {
            left.max_screen_fraction
                .total_cmp(&right.max_screen_fraction)
        });
        if levels.is_empty() {
            self.mesh_lods.remove(&node);
        } else if self.mesh_lods.get(&node) != Some(&levels) {
            self.mesh_lods.insert(node, levels);
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub(crate) fn mesh_lods(&self, node: NodeKey) -> Option<&[MeshLodLevel]> {
        self.mesh_lods.get(&node).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Assets, Color, GeometryDesc, MaterialDesc, Transform};

    #[test]
    fn scene_set_mesh_lods_rejects_invalid_thresholds_without_silent_drop() {
        let assets = Assets::new();
        let base_geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
        let lod_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
        let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
        let mut scene = Scene::new();
        let mesh = scene
            .mesh(base_geometry, material)
            .transform(Transform::IDENTITY)
            .add()
            .expect("mesh inserts");

        let error = scene
            .set_mesh_lods(mesh, vec![MeshLodLevel::new(1.25, lod_geometry)])
            .expect_err("invalid threshold must fail closed");

        assert!(matches!(
            error,
            LookupError::InvalidFramingOption {
                field: "max_screen_fraction",
                ..
            }
        ));
        assert!(
            scene.mesh_lods(mesh).is_none(),
            "invalid LOD levels must not be silently filtered into an empty no-op"
        );
    }

    #[test]
    fn scene_set_mesh_lods_rejects_non_mesh_nodes() {
        let assets = Assets::new();
        let lod_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
        let mut scene = Scene::new();
        let empty = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("empty node inserts");

        let error = scene
            .set_mesh_lods(empty, vec![MeshLodLevel::new(0.5, lod_geometry)])
            .expect_err("non-mesh LOD attachment must fail closed");

        assert!(matches!(
            error,
            LookupError::InvalidFramingOption { field: "node", .. }
        ));
    }
}
