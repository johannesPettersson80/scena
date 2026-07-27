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

/// Compact identity for frame-bound composition/subject observations.
///
/// The key is derived from the exact rendered/readback frame state and can be
/// compared before consuming an observation that was computed for that frame.
/// It intentionally reuses `RenderedFrameState` and `SceneDirtyState` instead
/// of creating a parallel revision system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CompositionFrameKey {
    dirty_state: SceneDirtyState,
    camera: CameraKey,
    viewport_width: u32,
    viewport_height: u32,
    target_width: u32,
    target_height: u32,
    backend: Backend,
    render_generation: u64,
    target_revision: u64,
    output_resources_revision: u64,
    output_color_space: OutputColorSpace,
    exposure_ev_bits: u32,
    tonemapper: &'static str,
    anti_aliasing: &'static str,
    supersample_factor: u32,
    bloom: bool,
    screen_space_ambient_occlusion: bool,
    screen_space_reflections: bool,
    depth_of_field: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompositionFrameStaleReason {
    Camera,
    Viewport,
    Transform,
    Visibility,
    Appearance,
    Structure,
    Interaction,
    Backend,
    RenderGeneration,
    RenderSettings,
    Target,
    OutputResources,
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

impl CompositionFrameKey {
    pub(crate) fn from_rendered_frame(frame: RenderedFrameState) -> Self {
        Self {
            dirty_state: frame.dirty_state,
            camera: frame.camera,
            viewport_width: frame.width,
            viewport_height: frame.height,
            target_width: frame.width,
            target_height: frame.height,
            backend: frame.backend(),
            render_generation: frame.render_generation,
            target_revision: frame.target_revision,
            output_resources_revision: frame.output_resources_revision,
            output_color_space: frame.output_color_space,
            exposure_ev_bits: frame.exposure_ev.to_bits(),
            tonemapper: frame.tonemapper,
            anti_aliasing: frame.anti_aliasing,
            supersample_factor: frame.supersample_factor,
            bloom: frame.bloom,
            screen_space_ambient_occlusion: frame.screen_space_ambient_occlusion,
            screen_space_reflections: frame.screen_space_reflections,
            depth_of_field: frame.depth_of_field,
        }
    }

    pub(crate) fn staleness_against_rendered_frame(
        self,
        frame: RenderedFrameState,
    ) -> Option<CompositionFrameStaleReason> {
        if self.camera != frame.camera
            || self.dirty_state.camera_revision != frame.dirty_state.camera_revision
        {
            return Some(CompositionFrameStaleReason::Camera);
        }
        if self.viewport_width != frame.width
            || self.viewport_height != frame.height
            || self.target_width != frame.width
            || self.target_height != frame.height
        {
            return Some(CompositionFrameStaleReason::Viewport);
        }
        if self.dirty_state.transform_revision != frame.dirty_state.transform_revision {
            return Some(CompositionFrameStaleReason::Transform);
        }
        if self.dirty_state.visibility_revision != frame.dirty_state.visibility_revision {
            return Some(CompositionFrameStaleReason::Visibility);
        }
        if self.dirty_state.appearance_revision != frame.dirty_state.appearance_revision {
            return Some(CompositionFrameStaleReason::Appearance);
        }
        if self.dirty_state.structure_revision != frame.dirty_state.structure_revision {
            return Some(CompositionFrameStaleReason::Structure);
        }
        if self.dirty_state.interaction_revision != frame.dirty_state.interaction_revision {
            return Some(CompositionFrameStaleReason::Interaction);
        }
        if self.backend != frame.backend() {
            return Some(CompositionFrameStaleReason::Backend);
        }
        if self.render_generation != frame.render_generation {
            return Some(CompositionFrameStaleReason::RenderGeneration);
        }
        if self.output_color_space != frame.output_color_space
            || self.exposure_ev_bits != frame.exposure_ev.to_bits()
            || self.tonemapper != frame.tonemapper
            || self.anti_aliasing != frame.anti_aliasing
            || self.supersample_factor != frame.supersample_factor
            || self.bloom != frame.bloom
            || self.screen_space_ambient_occlusion != frame.screen_space_ambient_occlusion
            || self.screen_space_reflections != frame.screen_space_reflections
            || self.depth_of_field != frame.depth_of_field
        {
            return Some(CompositionFrameStaleReason::RenderSettings);
        }
        if self.target_revision != frame.target_revision {
            return Some(CompositionFrameStaleReason::Target);
        }
        if self.output_resources_revision != frame.output_resources_revision {
            return Some(CompositionFrameStaleReason::OutputResources);
        }
        None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Capabilities, OutputColorSpace};
    use crate::scene::Scene;

    fn dirty_state() -> SceneDirtyState {
        SceneDirtyState {
            structure_revision: 1,
            transform_revision: 2,
            camera_revision: 3,
            appearance_revision: 4,
            visibility_revision: 5,
            interaction_revision: 6,
        }
    }

    fn rendered_frame(camera: CameraKey) -> RenderedFrameState {
        RenderedFrameState {
            dirty_state: dirty_state(),
            camera,
            width: 320,
            height: 240,
            capabilities: Capabilities::headless(),
            render_generation: 7,
            target_revision: 8,
            output_resources_revision: 9,
            output_color_space: OutputColorSpace::Srgb,
            exposure_ev: 1.25,
            tonemapper: "pbr_neutral",
            anti_aliasing: "fxaa",
            supersample_factor: 2,
            bloom: true,
            screen_space_ambient_occlusion: true,
            screen_space_reflections: false,
            depth_of_field: true,
            readback_completed_unix_ms: Some(10),
        }
    }

    #[test]
    fn composition_frame_key_reports_specific_stale_reasons() {
        let mut scene = Scene::new();
        let camera = scene.add_default_camera().expect("camera inserts");
        let other_camera = scene.add_default_camera().expect("second camera inserts");
        let frame = rendered_frame(camera);
        let key = CompositionFrameKey::from_rendered_frame(frame);
        assert_eq!(key.staleness_against_rendered_frame(frame), None);

        let mut changed = frame;
        changed.camera = other_camera;
        assert_eq!(
            key.staleness_against_rendered_frame(changed),
            Some(CompositionFrameStaleReason::Camera)
        );

        let mut changed = frame;
        changed.width = 400;
        assert_eq!(
            key.staleness_against_rendered_frame(changed),
            Some(CompositionFrameStaleReason::Viewport)
        );

        let mut changed = frame;
        changed.dirty_state.transform_revision += 1;
        assert_eq!(
            key.staleness_against_rendered_frame(changed),
            Some(CompositionFrameStaleReason::Transform)
        );

        let mut changed = frame;
        changed.dirty_state.visibility_revision += 1;
        assert_eq!(
            key.staleness_against_rendered_frame(changed),
            Some(CompositionFrameStaleReason::Visibility)
        );

        let mut changed = frame;
        changed.dirty_state.appearance_revision += 1;
        assert_eq!(
            key.staleness_against_rendered_frame(changed),
            Some(CompositionFrameStaleReason::Appearance)
        );

        let mut changed = frame;
        changed.render_generation += 1;
        assert_eq!(
            key.staleness_against_rendered_frame(changed),
            Some(CompositionFrameStaleReason::RenderGeneration)
        );
    }
}
