use crate::material::Color;
use crate::scene::{NodeKey, Vec3};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedStrokeSegment {
    source_node: Option<NodeKey>,
    clip_with_scene: bool,
    start: Vec3,
    end: Vec3,
    color: Color,
    width_px: f32,
    world_from_model: [f32; 16],
    tint: Color,
    original_segment_index: u32,
}

impl PreparedStrokeSegment {
    pub(in crate::render) const fn new(
        source_node: Option<NodeKey>,
        start: Vec3,
        end: Vec3,
        color: Color,
        width_px: f32,
        world_from_model: [f32; 16],
        tint: Color,
    ) -> Self {
        Self {
            source_node,
            clip_with_scene: true,
            start,
            end,
            color,
            width_px,
            world_from_model,
            tint,
            original_segment_index: 0,
        }
    }

    /// Opts this stroke out of scene clipping (planes and section box).
    pub(in crate::render) const fn with_scene_clipping(mut self, clip_with_scene: bool) -> Self {
        self.clip_with_scene = clip_with_scene;
        self
    }

    pub(in crate::render) const fn clips_with_scene(&self) -> bool {
        self.clip_with_scene
    }

    pub(in crate::render) const fn with_original_segment_index(
        mut self,
        original_segment_index: u32,
    ) -> Self {
        self.original_segment_index = original_segment_index;
        self
    }

    pub(in crate::render) const fn source_node(&self) -> Option<NodeKey> {
        self.source_node
    }

    pub(in crate::render) const fn start(&self) -> Vec3 {
        self.start
    }

    pub(in crate::render) const fn end(&self) -> Vec3 {
        self.end
    }

    pub(in crate::render) const fn color(&self) -> Color {
        self.color
    }

    pub(in crate::render) const fn width_px(&self) -> f32 {
        self.width_px
    }

    pub(in crate::render) const fn world_from_model(&self) -> [f32; 16] {
        self.world_from_model
    }

    pub(in crate::render) const fn tint(&self) -> Color {
        self.tint
    }

    pub(in crate::render) const fn original_segment_index(&self) -> u32 {
        self.original_segment_index
    }

    pub(in crate::render) fn set_tint(&mut self, tint: Color) {
        self.tint = tint;
    }

    pub(in crate::render) fn set_world_from_model(&mut self, world_from_model: [f32; 16]) {
        self.world_from_model = world_from_model;
    }
}
