use std::sync::{Arc, Weak};

use super::prepare;
use crate::diagnostics::{Backend, Capabilities, OutputColorSpace};
use crate::scene::{CameraKey, ClippingPlane, SceneDirtyState, SectionBox};

#[derive(Debug, Clone)]
pub(super) struct PreparedSceneState {
    pub(super) scene: Weak<()>,
    pub(super) structure_revision: u64,
    pub(super) transform_revision: u64,
    pub(super) camera_revision: u64,
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
    pub(super) clipping_planes: Arc<[ClippingPlane]>,
    pub(super) section_box: Option<SectionBox>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RenderedFrameState {
    pub(super) dirty_state: SceneDirtyState,
    pub(super) camera: CameraKey,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) capabilities: Capabilities,
    pub(super) render_generation: u64,
    pub(super) target_revision: u64,
    pub(super) output_resources_revision: u64,
    pub(super) output_color_space: OutputColorSpace,
    pub(super) exposure_ev: f32,
    pub(super) tonemapper: &'static str,
    pub(super) anti_aliasing: &'static str,
    pub(super) supersample_factor: u32,
    pub(super) bloom: bool,
    pub(super) screen_space_ambient_occlusion: bool,
    pub(super) screen_space_reflections: bool,
    pub(super) depth_of_field: bool,
    pub(super) readback_completed_unix_ms: Option<u64>,
}

impl RenderedFrameState {
    pub(crate) const fn dirty_state(self) -> SceneDirtyState {
        self.dirty_state
    }

    pub(crate) const fn camera(self) -> CameraKey {
        self.camera
    }

    pub(crate) const fn width(self) -> u32 {
        self.width
    }

    pub(crate) const fn height(self) -> u32 {
        self.height
    }

    pub(crate) const fn capabilities(self) -> Capabilities {
        self.capabilities
    }

    pub(crate) const fn backend(self) -> Backend {
        self.capabilities.backend
    }

    pub(crate) const fn render_generation(self) -> u64 {
        self.render_generation
    }

    pub(crate) const fn target_revision(self) -> u64 {
        self.target_revision
    }

    pub(crate) const fn output_resources_revision(self) -> u64 {
        self.output_resources_revision
    }

    pub(crate) const fn output_color_space(self) -> OutputColorSpace {
        self.output_color_space
    }

    pub(crate) const fn exposure_ev(self) -> f32 {
        self.exposure_ev
    }

    pub(crate) const fn tonemapper(self) -> &'static str {
        self.tonemapper
    }

    pub(crate) const fn anti_aliasing(self) -> &'static str {
        self.anti_aliasing
    }

    pub(crate) const fn supersample_factor(self) -> u32 {
        self.supersample_factor
    }

    pub(crate) const fn bloom(self) -> bool {
        self.bloom
    }

    pub(crate) const fn screen_space_ambient_occlusion(self) -> bool {
        self.screen_space_ambient_occlusion
    }

    pub(crate) const fn screen_space_reflections(self) -> bool {
        self.screen_space_reflections
    }

    pub(crate) const fn depth_of_field(self) -> bool {
        self.depth_of_field
    }

    pub(crate) const fn readback_completed_unix_ms(self) -> Option<u64> {
        self.readback_completed_unix_ms
    }

    pub(super) fn with_readback_completed_now(mut self) -> Self {
        self.readback_completed_unix_ms = Some(readback_completed_unix_ms());
        self
    }

    pub(crate) fn describes_same_render(mut self, mut other: Self) -> bool {
        self.readback_completed_unix_ms = None;
        other.readback_completed_unix_ms = None;
        self == other
    }

    pub(super) fn matches(self, dirty_state: SceneDirtyState, camera: CameraKey) -> bool {
        self.dirty_state.structure_revision == dirty_state.structure_revision
            && self.dirty_state.transform_revision == dirty_state.transform_revision
            && self.dirty_state.camera_revision == dirty_state.camera_revision
            && self.dirty_state.appearance_revision == dirty_state.appearance_revision
            && self.dirty_state.visibility_revision == dirty_state.visibility_revision
            && self.dirty_state.interaction_revision == dirty_state.interaction_revision
            && self.camera == camera
    }
}

#[cfg(target_arch = "wasm32")]
fn readback_completed_unix_ms() -> u64 {
    let now = js_sys::Date::now();
    if now.is_finite() && now >= 0.0 {
        now.min(u64::MAX as f64) as u64
    } else {
        0
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn readback_completed_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}
