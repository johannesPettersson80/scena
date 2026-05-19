use super::{HeadlessGltfViewer, InteractiveGltfViewer};

impl HeadlessGltfViewer {
    /// Declared `KHR_materials_variants` names for the loaded import.
    pub fn material_variants(&self) -> &[String] {
        self.import.material_variants()
    }

    /// Name of the currently active material variant, or `None` for the default materials.
    pub fn active_material_variant(&self) -> Option<String> {
        self.import.active_variant()
    }

    /// Applies a material variant and prepares the viewer before the next render.
    pub fn set_active_material_variant(&mut self, name: Option<&str>) -> crate::Result<()> {
        self.scene.set_active_variant(&self.import, name)?;
        self.prepare()
    }
}

impl InteractiveGltfViewer {
    /// Declared `KHR_materials_variants` names for the loaded import.
    pub fn material_variants(&self) -> &[String] {
        self.import.material_variants()
    }

    /// Name of the currently active material variant, or `None` for the default materials.
    pub fn active_material_variant(&self) -> Option<String> {
        self.import.active_variant()
    }

    /// Applies a material variant and prepares the viewer before the next render.
    pub fn set_active_material_variant(&mut self, name: Option<&str>) -> crate::Result<()> {
        self.scene.set_active_variant(&self.import, name)?;
        self.prepare()
    }
}
