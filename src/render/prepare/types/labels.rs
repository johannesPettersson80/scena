use crate::material::Color;
use crate::scene::{NodeKey, Vec3};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render) struct PreparedLabelAtlas {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
    quads: Vec<PreparedLabelQuad>,
}

impl PreparedLabelAtlas {
    pub(in crate::render) fn new(
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
        quads: Vec<PreparedLabelQuad>,
    ) -> Self {
        Self {
            width,
            height,
            rgba8,
            quads,
        }
    }

    pub(in crate::render) const fn width(&self) -> u32 {
        self.width
    }

    pub(in crate::render) const fn height(&self) -> u32 {
        self.height
    }

    pub(in crate::render) fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }

    pub(in crate::render) fn quads(&self) -> &[PreparedLabelQuad] {
        &self.quads
    }

    pub(in crate::render) fn quads_mut(&mut self) -> &mut [PreparedLabelQuad] {
        &mut self.quads
    }

    pub(in crate::render) fn set_quads(&mut self, quads: Vec<PreparedLabelQuad>) {
        self.quads = quads;
    }

    pub(in crate::render) fn is_empty(&self) -> bool {
        self.quads.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedLabelQuad {
    source_node: Option<NodeKey>,
    anchor: Vec3,
    right: Vec3,
    up: Vec3,
    world_units_per_px: f32,
    rect_px: [f32; 4],
    uv_rect: [f32; 4],
    color: Color,
    tint: Color,
    solid_coverage: bool,
    original_quad_index: u32,
}

impl PreparedLabelQuad {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::render) const fn new(
        source_node: Option<NodeKey>,
        anchor: Vec3,
        right: Vec3,
        up: Vec3,
        world_units_per_px: f32,
        rect_px: [f32; 4],
        uv_rect: [f32; 4],
        color: Color,
        tint: Color,
    ) -> Self {
        Self {
            source_node,
            anchor,
            right,
            up,
            world_units_per_px,
            rect_px,
            uv_rect,
            color,
            tint,
            solid_coverage: false,
            original_quad_index: 0,
        }
    }

    pub(in crate::render) const fn with_solid_coverage(mut self) -> Self {
        self.solid_coverage = true;
        self
    }

    pub(in crate::render) const fn with_original_quad_index(
        mut self,
        original_quad_index: u32,
    ) -> Self {
        self.original_quad_index = original_quad_index;
        self
    }

    pub(in crate::render) const fn source_node(&self) -> Option<NodeKey> {
        self.source_node
    }

    pub(in crate::render) const fn anchor(&self) -> Vec3 {
        self.anchor
    }

    pub(in crate::render) const fn right(&self) -> Vec3 {
        self.right
    }

    pub(in crate::render) const fn up(&self) -> Vec3 {
        self.up
    }

    pub(in crate::render) const fn world_units_per_px(&self) -> f32 {
        self.world_units_per_px
    }

    pub(in crate::render) const fn rect_px(&self) -> [f32; 4] {
        self.rect_px
    }

    pub(in crate::render) const fn uv_rect(&self) -> [f32; 4] {
        self.uv_rect
    }

    pub(in crate::render) const fn solid_coverage(&self) -> bool {
        self.solid_coverage
    }

    pub(in crate::render) fn set_tint(&mut self, tint: Color) {
        self.tint = tint;
    }

    pub(in crate::render) fn final_color(&self) -> Color {
        Color::from_linear_rgba(
            self.color.r * self.tint.r,
            self.color.g * self.tint.g,
            self.color.b * self.tint.b,
            self.color.a * self.tint.a,
        )
    }
}
