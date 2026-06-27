use super::{SceneHostCore, SceneHostError};
use crate::AssetFetcher;

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn material_variants(&self, import: u64) -> Result<Vec<String>, SceneHostError> {
        Ok(self.resolve_import(import)?.material_variants().to_vec())
    }

    pub fn active_material_variant(&self, import: u64) -> Result<Option<String>, SceneHostError> {
        Ok(self.resolve_import(import)?.active_variant())
    }

    pub fn set_active_material_variant(
        &mut self,
        import: u64,
        variant: Option<&str>,
    ) -> Result<(), SceneHostError> {
        let import = self.resolve_import(import)?.clone();
        self.scene.set_active_variant(&import, variant)?;
        Ok(())
    }
}
