use crate::material::Color;
use crate::scene::{LabelBillboard, LabelDesc, NodeKey, Scene, Transform, Vec3};
use std::sync::Arc;

use super::super::{RasterTarget, camera::CameraProjection};
use super::types::{PreparedLabelAtlas, PreparedLabelQuad};

const ATLAS_WIDTH: u32 = 1024;
const ATLAS_GUTTER: u32 = 1;

pub(super) fn prepare_label_atlas(
    target: RasterTarget,
    scene: &Scene,
    camera_projection: Option<&CameraProjection>,
    origin_shift: Vec3,
) -> PreparedLabelAtlas {
    let mut builder = LabelAtlasBuilder::new(ATLAS_WIDTH);
    let solid = builder.insert(Arc::from([u8::MAX]), 1, 1);
    let mut quads = Vec::new();
    for (node, _label_key, label, transform) in scene.label_nodes() {
        prepare_label_billboard(
            node,
            label,
            transform,
            LabelPrepareInputs {
                target,
                camera_projection,
                origin_shift,
            },
            solid,
            &mut builder,
            &mut quads,
        );
    }
    builder.finish(quads)
}

fn prepare_label_billboard(
    node: NodeKey,
    label: &LabelDesc,
    transform: Transform,
    inputs: LabelPrepareInputs<'_>,
    solid: AtlasRect,
    builder: &mut LabelAtlasBuilder,
    quads: &mut Vec<PreparedLabelQuad>,
) {
    match label.billboard() {
        LabelBillboard::ScreenAligned => {
            let world_anchor = transform.translation;
            let shifted_anchor = Vec3::new(
                transform.translation.x - inputs.origin_shift.x,
                transform.translation.y - inputs.origin_shift.y,
                transform.translation.z - inputs.origin_shift.z,
            );
            if let Some(camera_projection) = inputs.camera_projection {
                prepare_pixel_label_billboard(
                    PixelLabelInputs {
                        node,
                        label,
                        world_anchor,
                        shifted_anchor,
                        camera_projection,
                        solid,
                    },
                    builder,
                    quads,
                );
            } else {
                prepare_fallback_world_label(
                    node,
                    label,
                    inputs.target,
                    shifted_anchor,
                    solid,
                    quads,
                );
            }
        }
    }
}

fn prepare_pixel_label_billboard(
    inputs: PixelLabelInputs<'_>,
    builder: &mut LabelAtlasBuilder,
    quads: &mut Vec<PreparedLabelQuad>,
) {
    let Some(world_units_per_px) = inputs
        .camera_projection
        .world_units_per_pixel_at(inputs.world_anchor)
    else {
        return;
    };
    let (right, up) = inputs.camera_projection.billboard_axes();
    let metrics = inputs.label.metrics();
    let half_width = metrics.width_px * 0.5;
    let half_height = metrics.height_px * 0.5;
    let padding = (inputs.label.size() * 0.25).ceil().max(2.0);
    let frame = LabelFrame {
        node: inputs.node,
        anchor: inputs.shifted_anchor,
        right,
        up,
        world_units_per_px,
    };

    if let Some(background) = inputs.label.background() {
        quads.push(
            PreparedLabelQuad::new(
                Some(inputs.node),
                frame.anchor,
                frame.right,
                frame.up,
                frame.world_units_per_px,
                [
                    -half_width - padding,
                    -half_height - padding,
                    half_width + padding,
                    half_height + padding,
                ],
                inputs.solid.uv_rect(),
                background,
                Color::WHITE,
            )
            .with_solid_coverage(),
        );
    }

    let glyphs = inputs.label.glyph_rasters();
    if let Some(halo) = inputs.label.halo() {
        for glyph in &glyphs {
            let atlas = builder.insert(glyph.alpha.clone(), glyph.alpha_width, glyph.alpha_height);
            quads.push(PreparedLabelQuad::new(
                Some(frame.node),
                frame.anchor,
                frame.right,
                frame.up,
                frame.world_units_per_px,
                [
                    glyph.x_px - half_width - 1.0,
                    half_height - (glyph.y_px + glyph.height_px) - 1.0,
                    glyph.x_px + glyph.width_px - half_width + 1.0,
                    half_height - glyph.y_px + 1.0,
                ],
                atlas.uv_rect(),
                halo,
                Color::WHITE,
            ));
        }
    }

    for glyph in &glyphs {
        let atlas = builder.insert(glyph.alpha.clone(), glyph.alpha_width, glyph.alpha_height);
        quads.push(PreparedLabelQuad::new(
            Some(frame.node),
            frame.anchor,
            frame.right,
            frame.up,
            frame.world_units_per_px,
            [
                glyph.x_px - half_width,
                half_height - (glyph.y_px + glyph.height_px),
                glyph.x_px + glyph.width_px - half_width,
                half_height - glyph.y_px,
            ],
            atlas.uv_rect(),
            inputs.label.color(),
            Color::WHITE,
        ));
    }
}

fn prepare_fallback_world_label(
    node: NodeKey,
    label: &LabelDesc,
    target: RasterTarget,
    center: Vec3,
    solid: AtlasRect,
    quads: &mut Vec<PreparedLabelQuad>,
) {
    let target_scale = target.height.max(1) as f32 / 120.0;
    let metrics = label.metrics();
    let half_width = metrics.width_px * 0.5;
    let half_height = metrics.height_px * 0.5;
    let world_units_per_px = 1.0 / (120.0 * target_scale);
    quads.push(
        PreparedLabelQuad::new(
            Some(node),
            center,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            world_units_per_px,
            [-half_width, -half_height, half_width, half_height],
            solid.uv_rect(),
            label.color(),
            Color::WHITE,
        )
        .with_solid_coverage(),
    );
}

#[derive(Debug, Clone, Copy)]
struct LabelPrepareInputs<'a> {
    target: RasterTarget,
    camera_projection: Option<&'a CameraProjection>,
    origin_shift: Vec3,
}

#[derive(Debug, Clone, Copy)]
struct PixelLabelInputs<'a> {
    node: NodeKey,
    label: &'a LabelDesc,
    world_anchor: Vec3,
    shifted_anchor: Vec3,
    camera_projection: &'a CameraProjection,
    solid: AtlasRect,
}

#[derive(Debug, Clone, Copy)]
struct LabelFrame {
    node: NodeKey,
    anchor: Vec3,
    right: Vec3,
    up: Vec3,
    world_units_per_px: f32,
}

#[derive(Debug, Clone, Copy)]
struct AtlasRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    atlas_width: u32,
    atlas_height: u32,
}

impl AtlasRect {
    fn uv_rect(self) -> [f32; 4] {
        [
            self.x as f32 / self.atlas_width as f32,
            self.y as f32 / self.atlas_height as f32,
            self.x.saturating_add(self.width) as f32 / self.atlas_width as f32,
            self.y.saturating_add(self.height) as f32 / self.atlas_height as f32,
        ]
    }
}

struct PendingAtlasInsert {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    alpha: Arc<[u8]>,
}

struct LabelAtlasBuilder {
    width: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    height: u32,
    pending: Vec<PendingAtlasInsert>,
}

impl LabelAtlasBuilder {
    fn new(width: u32) -> Self {
        Self {
            width,
            cursor_x: ATLAS_GUTTER,
            cursor_y: ATLAS_GUTTER,
            row_height: 0,
            height: ATLAS_GUTTER,
            pending: Vec::new(),
        }
    }

    fn insert(&mut self, alpha: Arc<[u8]>, alpha_width: u32, alpha_height: u32) -> AtlasRect {
        let alpha_width = alpha_width.max(1);
        let alpha_height = alpha_height.max(1);
        let padded_width = alpha_width.saturating_add(ATLAS_GUTTER * 2);
        let padded_height = alpha_height.saturating_add(ATLAS_GUTTER * 2);
        if self.cursor_x.saturating_add(padded_width) > self.width {
            self.cursor_x = ATLAS_GUTTER;
            self.cursor_y = self
                .cursor_y
                .saturating_add(self.row_height)
                .saturating_add(ATLAS_GUTTER);
            self.row_height = 0;
        }
        let x = self.cursor_x.saturating_add(ATLAS_GUTTER);
        let y = self.cursor_y.saturating_add(ATLAS_GUTTER);
        self.pending.push(PendingAtlasInsert {
            x,
            y,
            width: alpha_width,
            height: alpha_height,
            alpha,
        });
        self.cursor_x = self.cursor_x.saturating_add(padded_width);
        self.row_height = self.row_height.max(padded_height);
        self.height = self
            .height
            .max(y.saturating_add(alpha_height).saturating_add(ATLAS_GUTTER));
        AtlasRect {
            x,
            y,
            width: alpha_width,
            height: alpha_height,
            atlas_width: self.width,
            atlas_height: self.width,
        }
    }

    fn finish(self, quads: Vec<PreparedLabelQuad>) -> PreparedLabelAtlas {
        let height = self.width.max(1);
        let mut rgba8 = vec![0; self.width.saturating_mul(height).saturating_mul(4) as usize];
        for insert in self.pending {
            for row in 0..insert.height {
                for col in 0..insert.width {
                    let src = row.saturating_mul(insert.width).saturating_add(col) as usize;
                    let Some(alpha) = insert.alpha.get(src).copied() else {
                        continue;
                    };
                    if alpha == 0 {
                        continue;
                    }
                    let dst = ((insert.y + row)
                        .saturating_mul(self.width)
                        .saturating_add(insert.x + col)
                        .saturating_mul(4)) as usize;
                    if let Some(pixel) = rgba8.get_mut(dst..dst + 4) {
                        pixel[0] = u8::MAX;
                        pixel[1] = u8::MAX;
                        pixel[2] = u8::MAX;
                        pixel[3] = alpha;
                    }
                }
            }
        }
        PreparedLabelAtlas::new(self.width, height, rgba8, quads)
    }
}
