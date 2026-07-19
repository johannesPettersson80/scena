use std::sync::{Arc, Weak};

use super::prepare;
use crate::scene::{CameraKey, ClippingPlane, SceneDirtyState, SectionBox};

#[derive(Debug, Clone)]
pub(super) struct PreparedSceneState {
    pub(super) scene: Weak<()>,
    pub(super) structure_revision: u64,
    pub(super) transform_revision: u64,
    pub(super) appearance_revision: u64,
    pub(super) visibility_revision: u64,
    pub(super) environment_revision: u64,
    pub(super) target_revision: u64,
    pub(super) output_resources_revision: u64,
    pub(super) retained_primitives: Arc<[prepare::PreparedPrimitive]>,
    pub(super) primitives: Arc<[prepare::PreparedPrimitive]>,
    pub(super) retained_strokes: Arc<[prepare::PreparedStrokeSegment]>,
    pub(super) strokes: Arc<[prepare::PreparedStrokeSegment]>,
    pub(super) retained_labels: Arc<prepare::PreparedLabelAtlas>,
    pub(super) labels: Arc<prepare::PreparedLabelAtlas>,
    pub(super) retained_instances: Arc<[prepare::PreparedInstanceSet]>,
    pub(super) instances: Arc<[prepare::PreparedInstanceSet]>,
    pub(super) clipping_planes: Vec<ClippingPlane>,
    pub(super) section_box: Option<SectionBox>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderedFrameState {
    pub(super) dirty_state: SceneDirtyState,
    pub(super) camera: CameraKey,
}

impl RenderedFrameState {
    pub(crate) const fn dirty_state(self) -> SceneDirtyState {
        self.dirty_state
    }

    pub(crate) const fn camera(self) -> CameraKey {
        self.camera
    }

    pub(super) fn matches(self, dirty_state: SceneDirtyState, camera: CameraKey) -> bool {
        self.dirty_state.structure_revision == dirty_state.structure_revision
            && self.dirty_state.transform_revision == dirty_state.transform_revision
            && self.dirty_state.appearance_revision == dirty_state.appearance_revision
            && self.dirty_state.visibility_revision == dirty_state.visibility_revision
            && self.dirty_state.interaction_revision == dirty_state.interaction_revision
            && self.camera == camera
    }
}
