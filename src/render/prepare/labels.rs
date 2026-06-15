use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::scene::{LabelBillboard, LabelDesc, NodeKey, Scene, Transform, Vec3};

use super::super::{RasterTarget, camera::CameraProjection};
use super::types::PreparedPrimitive;

pub(super) fn append_label_primitives(
    target: RasterTarget,
    scene: &Scene,
    camera_projection: Option<&CameraProjection>,
    origin_shift: Vec3,
    primitives: &mut Vec<PreparedPrimitive>,
) {
    for (node, _label, label, transform) in scene.label_nodes() {
        append_label_billboard(
            node,
            label,
            transform,
            target,
            camera_projection,
            origin_shift,
            primitives,
        );
    }
}

fn append_label_billboard(
    node: NodeKey,
    label: &LabelDesc,
    transform: Transform,
    target: RasterTarget,
    camera_projection: Option<&CameraProjection>,
    origin_shift: Vec3,
    primitives: &mut Vec<PreparedPrimitive>,
) {
    match label.billboard() {
        LabelBillboard::ScreenAligned => {
            let center = Vec3::new(
                transform.translation.x - origin_shift.x,
                transform.translation.y - origin_shift.y,
                transform.translation.z - origin_shift.z,
            );
            if let Some(camera_projection) = camera_projection {
                append_pixel_label_billboard(
                    node,
                    label,
                    transform.translation,
                    center,
                    camera_projection,
                    primitives,
                );
            } else {
                append_fallback_world_billboard(node, label, target, center, primitives);
            }
        }
    }
}

fn append_pixel_label_billboard(
    node: NodeKey,
    label: &LabelDesc,
    world_anchor: Vec3,
    shifted_anchor: Vec3,
    camera_projection: &CameraProjection,
    primitives: &mut Vec<PreparedPrimitive>,
) {
    let Some(world_units_per_px) = camera_projection.world_units_per_pixel_at(world_anchor) else {
        return;
    };
    let (right, up) = camera_projection.billboard_axes();
    let metrics = label.metrics();
    let half_width = metrics.width_px * 0.5;
    let half_height = metrics.height_px * 0.5;
    let padding = (label.size() * 0.25).ceil().max(2.0);

    if let Some(background) = label.background() {
        push_pixel_quad(
            node,
            primitives,
            BillboardFrame {
                anchor: shifted_anchor,
                right,
                up,
                world_units_per_px,
            },
            PixelRect {
                x0: -half_width - padding,
                y0: -half_height - padding,
                x1: half_width + padding,
                y1: half_height + padding,
            },
            background,
        );
    }

    if let Some(halo) = label.halo() {
        for cell in label.glyph_cells() {
            push_pixel_quad(
                node,
                primitives,
                BillboardFrame {
                    anchor: shifted_anchor,
                    right,
                    up,
                    world_units_per_px,
                },
                PixelRect {
                    x0: cell.x0_px - half_width - 1.0,
                    y0: half_height - cell.y1_px - 1.0,
                    x1: cell.x1_px - half_width + 1.0,
                    y1: half_height - cell.y0_px + 1.0,
                },
                halo,
            );
        }
    }

    for cell in label.glyph_cells() {
        push_pixel_quad(
            node,
            primitives,
            BillboardFrame {
                anchor: shifted_anchor,
                right,
                up,
                world_units_per_px,
            },
            PixelRect {
                x0: cell.x0_px - half_width,
                y0: half_height - cell.y1_px,
                x1: cell.x1_px - half_width,
                y1: half_height - cell.y0_px,
            },
            label.color(),
        );
    }
}

fn append_fallback_world_billboard(
    node: NodeKey,
    label: &LabelDesc,
    target: RasterTarget,
    center: Vec3,
    primitives: &mut Vec<PreparedPrimitive>,
) {
    let target_scale = target.height.max(1) as f32 / 120.0;
    let metrics = label.metrics();
    let half_width = (metrics.width_px / 120.0) / target_scale;
    let half_height = (metrics.height_px / 120.0) / target_scale;
    let z = center.z;
    let min = Vec3::new(center.x - half_width, center.y - half_height, z);
    let max = Vec3::new(center.x + half_width, center.y + half_height, z);
    push_quad(node, primitives, min, max, label.color());
}

#[derive(Debug, Clone, Copy)]
struct BillboardFrame {
    anchor: Vec3,
    right: Vec3,
    up: Vec3,
    world_units_per_px: f32,
}

#[derive(Debug, Clone, Copy)]
struct PixelRect {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

fn push_pixel_quad(
    node: NodeKey,
    primitives: &mut Vec<PreparedPrimitive>,
    frame: BillboardFrame,
    rect: PixelRect,
    color: Color,
) {
    let min = frame.anchor
        + frame.right * (rect.x0 * frame.world_units_per_px)
        + frame.up * (rect.y0 * frame.world_units_per_px);
    let max_x = frame.anchor
        + frame.right * (rect.x1 * frame.world_units_per_px)
        + frame.up * (rect.y0 * frame.world_units_per_px);
    let max = frame.anchor
        + frame.right * (rect.x1 * frame.world_units_per_px)
        + frame.up * (rect.y1 * frame.world_units_per_px);
    let min_x_max_y = frame.anchor
        + frame.right * (rect.x0 * frame.world_units_per_px)
        + frame.up * (rect.y1 * frame.world_units_per_px);
    push_quad_vertices(node, primitives, min, max_x, max, min_x_max_y, color);
}

fn push_quad(
    node: NodeKey,
    primitives: &mut Vec<PreparedPrimitive>,
    min: Vec3,
    max: Vec3,
    color: Color,
) {
    push_quad_vertices(
        node,
        primitives,
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        color,
    );
}

fn push_quad_vertices(
    node: NodeKey,
    primitives: &mut Vec<PreparedPrimitive>,
    bottom_left_position: Vec3,
    bottom_right_position: Vec3,
    top_right_position: Vec3,
    top_left_position: Vec3,
    color: Color,
) {
    let bottom_left = Vertex {
        position: bottom_left_position,
        color,
    };
    let bottom_right = Vertex {
        position: bottom_right_position,
        color,
    };
    let top_right = Vertex {
        position: top_right_position,
        color,
    };
    let top_left = Vertex {
        position: top_left_position,
        color,
    };
    primitives.push(PreparedPrimitive::new(
        Primitive::triangle([bottom_left, bottom_right, top_right]),
        Some(node),
        Color::WHITE,
    ));
    primitives.push(PreparedPrimitive::new(
        Primitive::triangle([bottom_left, top_right, top_left]),
        Some(node),
        Color::WHITE,
    ));
}
