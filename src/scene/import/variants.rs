//! Phase 2B step 3: KHR_materials_variants runtime flip records.
//!
//! Each entry in `MeshVariantRecord` is captured at instantiate time
//! and walked by `Scene::set_active_variant` to swap an imported
//! `MeshNode::material` between the primitive default and the variant-
//! bound MaterialHandle.

use crate::assets::{MaterialHandle, MaterialVariantBinding};
use crate::diagnostics::LookupError;
use crate::scene::{NodeKey, Scene};

use super::SceneImport;

impl SceneImport {
    /// KHR_materials_variants names declared by the source SceneAsset,
    /// in declaration order. Empty when the asset did not carry the
    /// extension.
    pub fn material_variants(&self) -> &[String] {
        &self.material_variants
    }

    /// Name of the currently active variant, or `None` for the
    /// primitive default.
    pub fn active_variant(&self) -> Option<String> {
        let lock = self
            .active_variant
            .lock()
            .expect("variant lock should not be poisoned");
        let index = (*lock)?;
        self.material_variants.get(index as usize).cloned()
    }

    pub(crate) fn variant_index_for(&self, name: &str) -> Option<u32> {
        self.material_variants
            .iter()
            .position(|candidate| candidate == name)
            .map(|index| index as u32)
    }

    pub(crate) fn write_active_variant(&self, index: Option<u32>) {
        let mut lock = self
            .active_variant
            .lock()
            .expect("variant lock should not be poisoned");
        *lock = index;
    }

    pub(crate) fn variant_records(&self) -> &[MeshVariantRecord] {
        &self.variant_records
    }
}

impl Scene {
    /// Phase 2B step 3: activate a `KHR_materials_variants` variant by
    /// name, updating every imported mesh node's `MeshNode::material`
    /// to the variant's bound MaterialHandle and reverting to the
    /// primitive default for unmapped variants. Pass `None` to clear
    /// the active variant and restore every default material. Returns
    /// `LookupError::VariantNotFound` when `name` is not declared by
    /// the source asset; the active variant slot remains unchanged in
    /// that case.
    pub fn set_active_variant(
        &mut self,
        import: &SceneImport,
        name: Option<&str>,
    ) -> Result<(), LookupError> {
        let new_index = match name {
            Some(name) => Some(import.variant_index_for(name).ok_or_else(|| {
                LookupError::VariantNotFound {
                    name: name.to_string(),
                }
            })?),
            None => None,
        };
        import.write_active_variant(new_index);
        for record in import.variant_records() {
            let new_material = record.material_for(new_index);
            // set_mesh_material bumps `structure_revision` so prepare
            // runs again. Best-effort because a host may remove an
            // imported node after instantiate.
            let _ = self.set_mesh_material(record.node, new_material);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MeshVariantRecord {
    pub(crate) node: NodeKey,
    pub(crate) default_material: MaterialHandle,
    pub(crate) bindings: Vec<MaterialVariantBinding>,
}

impl MeshVariantRecord {
    /// Resolves the MaterialHandle for the active variant, falling back
    /// to the primitive's default material when no binding matches the
    /// active index (or when no variant is active at all).
    pub(crate) fn material_for(&self, active_index: Option<u32>) -> MaterialHandle {
        let Some(index) = active_index else {
            return self.default_material;
        };
        self.bindings
            .iter()
            .find(|binding| binding.variants().contains(&index))
            .map(|binding| binding.material())
            .unwrap_or(self.default_material)
    }
}
