use crate::diagnostics::OutputColorSpace;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureFrameProvenance {
    pub pixel_source: String,
    pub state_binding: String,
    pub release_evidence: bool,
    pub render_generation: u64,
    pub target_revision: u64,
    pub output_resources_revision: u64,
    pub output_color_space: OutputColorSpace,
    pub exposure_ev: f32,
    pub tonemapper: String,
    pub anti_aliasing: String,
    pub supersample_factor: u32,
    pub bloom: bool,
    pub screen_space_ambient_occlusion: bool,
    pub screen_space_reflections: bool,
    pub depth_of_field: bool,
    pub readback_completed_unix_ms: Option<u64>,
}

impl Default for CaptureFrameProvenance {
    fn default() -> Self {
        Self {
            pixel_source: "legacy_unspecified".to_owned(),
            state_binding: "unverified".to_owned(),
            release_evidence: false,
            render_generation: 0,
            target_revision: 0,
            output_resources_revision: 0,
            output_color_space: OutputColorSpace::Srgb,
            exposure_ev: 0.0,
            tonemapper: "unknown".to_owned(),
            anti_aliasing: "unknown".to_owned(),
            supersample_factor: 1,
            bloom: false,
            screen_space_ambient_occlusion: false,
            screen_space_reflections: false,
            depth_of_field: false,
            readback_completed_unix_ms: None,
        }
    }
}

impl CaptureFrameProvenance {
    pub(super) fn is_legacy_unspecified(&self) -> bool {
        self.pixel_source == "legacy_unspecified"
            && self.state_binding == "unverified"
            && !self.release_evidence
            && self.render_generation == 0
            && self.target_revision == 0
            && self.output_resources_revision == 0
            && self.readback_completed_unix_ms.is_none()
    }
}
