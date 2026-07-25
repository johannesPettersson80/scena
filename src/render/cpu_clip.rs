use crate::scene::{ClippingPlane, SectionBox};

use super::camera::CameraProjection;
use super::prepare::PreparedPrimitive;

/// Clipping state applied while rasterizing one triangle.
#[derive(Clone, Copy)]
pub(super) struct CpuTriangleClipInputs<'a> {
    pub(super) clipping_planes: &'a [ClippingPlane],
    pub(super) section_box: Option<SectionBox>,
    pub(super) camera: &'a CameraProjection,
}

impl<'a> CpuTriangleClipInputs<'a> {
    /// Narrows this clip context for one primitive.
    ///
    /// G01: generated annotation geometry (stroke quads for leader and
    /// dimension lines) opts out of scene clipping, so a section box cuts the
    /// model without deleting the annotation describing the section.
    pub(super) fn for_primitive(self, primitive: &PreparedPrimitive) -> Self {
        if primitive.clips_with_scene() {
            self
        } else {
            Self {
                clipping_planes: &[],
                section_box: None,
                camera: self.camera,
            }
        }
    }
}
