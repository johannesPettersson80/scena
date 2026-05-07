use super::Scene;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneDirtyState {
    pub structure_revision: u64,
    pub transform_revision: u64,
    pub interaction_revision: u64,
}

impl Scene {
    pub fn dirty_state(&self) -> SceneDirtyState {
        SceneDirtyState {
            structure_revision: self.structure_revision,
            transform_revision: self.transform_revision,
            interaction_revision: self.interaction.revision(),
        }
    }
}
