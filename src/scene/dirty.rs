use super::Scene;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneDirtyState {
    pub structure_revision: u64,
    pub transform_revision: u64,
    pub camera_revision: u64,
    pub appearance_revision: u64,
    pub visibility_revision: u64,
    pub interaction_revision: u64,
}

impl Scene {
    pub fn dirty_state(&self) -> SceneDirtyState {
        SceneDirtyState {
            structure_revision: self.structure_revision,
            transform_revision: self.transform_revision,
            camera_revision: self.camera_revision,
            appearance_revision: self.appearance_revision,
            visibility_revision: self.visibility_revision,
            interaction_revision: self.interaction.revision(),
        }
    }
}
