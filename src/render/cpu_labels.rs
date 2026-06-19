use crate::material::Color;
use crate::scene::{ClippingPlane, SectionBox, Vec3};

use super::camera::CameraProjection;
use super::cpu::{CpuFrame, write_label_overlay_pixel};
use super::prepare::{PreparedLabelAtlas, PreparedLabelQuad};
use super::target::RasterTarget;

pub(super) fn draw_label_atlas_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    labels: &PreparedLabelAtlas,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
    camera: &CameraProjection,
) {
    for quad in labels.quads() {
        draw_label_quad_cpu(
            cpu_frame,
            labels,
            quad,
            clipping_planes,
            section_box,
            camera,
        );
    }
}

fn draw_label_quad_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    atlas: &PreparedLabelAtlas,
    quad: &PreparedLabelQuad,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
    camera: &CameraProjection,
) {
    let [x0, y0, x1, y1] = quad.rect_px();
    let [u0, v0, u1, v1] = quad.uv_rect();
    let bottom_left = label_vertex(quad, x0, y0);
    let bottom_right = label_vertex(quad, x1, y0);
    let top_right = label_vertex(quad, x1, y1);
    let top_left = label_vertex(quad, x0, y1);
    draw_label_triangle_cpu(
        cpu_frame,
        atlas,
        [
            (bottom_left, u0, v1),
            (bottom_right, u1, v1),
            (top_right, u1, v0),
        ],
        quad.final_color(),
        clipping_planes,
        section_box,
        camera,
    );
    draw_label_triangle_cpu(
        cpu_frame,
        atlas,
        [
            (bottom_left, u0, v1),
            (top_right, u1, v0),
            (top_left, u0, v0),
        ],
        quad.final_color(),
        clipping_planes,
        section_box,
        camera,
    );
}

fn draw_label_triangle_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    atlas: &PreparedLabelAtlas,
    vertices: [(Vec3, f32, f32); 3],
    color: Color,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
    camera: &CameraProjection,
) {
    let Some(a) = ScreenLabelVertex::new(vertices[0], cpu_frame.target, camera) else {
        return;
    };
    let Some(b) = ScreenLabelVertex::new(vertices[1], cpu_frame.target, camera) else {
        return;
    };
    let Some(c) = ScreenLabelVertex::new(vertices[2], cpu_frame.target, camera) else {
        return;
    };
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let max_x =
        a.x.max(b.x)
            .max(c.x)
            .ceil()
            .min(cpu_frame.target.width as f32 - 1.0) as u32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let max_y =
        a.y.max(b.y)
            .max(c.y)
            .ceil()
            .min(cpu_frame.target.height as f32 - 1.0) as u32;
    let area = edge(a, b, c.x, c.y);
    if area.abs() <= f32::EPSILON {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = edge(b, c, px, py) / area;
            let w1 = edge(c, a, px, py) / area;
            let w2 = edge(a, b, px, py) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let position = mix_position(a.position, b.position, c.position, w0, w1, w2);
            if is_clipped(position, clipping_planes, section_box) {
                continue;
            }
            let (u, v) = mix_uv(a, b, c, w0, w1, w2);
            let coverage = sample_label_atlas_coverage(atlas, u, v);
            if coverage <= 0.0 {
                continue;
            }
            let depth = mix_depth(a.depth, b.depth, c.depth, w0, w1, w2);
            write_label_overlay_pixel(cpu_frame, x, y, color, coverage, depth);
        }
    }
}

fn label_vertex(quad: &PreparedLabelQuad, x_px: f32, y_px: f32) -> Vec3 {
    quad.anchor()
        + quad.right() * (x_px * quad.world_units_per_px())
        + quad.up() * (y_px * quad.world_units_per_px())
}

#[derive(Debug, Clone, Copy)]
struct ScreenLabelVertex {
    x: f32,
    y: f32,
    depth: f32,
    inv_depth: f32,
    position: Vec3,
    u: f32,
    v: f32,
}

impl ScreenLabelVertex {
    fn new(
        vertex: (Vec3, f32, f32),
        target: RasterTarget,
        camera: &CameraProjection,
    ) -> Option<Self> {
        let projected = camera.project(vertex.0)?;
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Some(Self {
            x: (projected.ndc_x * 0.5 + 0.5) * width,
            y: (1.0 - (projected.ndc_y * 0.5 + 0.5)) * height,
            depth: projected.depth,
            inv_depth: projected.depth.recip(),
            position: vertex.0,
            u: vertex.1,
            v: vertex.2,
        })
    }
}

fn edge(a: ScreenLabelVertex, b: ScreenLabelVertex, x: f32, y: f32) -> f32 {
    (x - a.x) * (b.y - a.y) - (y - a.y) * (b.x - a.x)
}

fn mix_uv(
    a: ScreenLabelVertex,
    b: ScreenLabelVertex,
    c: ScreenLabelVertex,
    w0: f32,
    w1: f32,
    w2: f32,
) -> (f32, f32) {
    let iw0 = w0 * a.inv_depth;
    let iw1 = w1 * b.inv_depth;
    let iw2 = w2 * c.inv_depth;
    let inv_sum = iw0 + iw1 + iw2;
    if inv_sum.abs() <= f32::EPSILON || !inv_sum.is_finite() {
        return (
            a.u * w0 + b.u * w1 + c.u * w2,
            a.v * w0 + b.v * w1 + c.v * w2,
        );
    }
    let w0 = iw0 / inv_sum;
    let w1 = iw1 / inv_sum;
    let w2 = iw2 / inv_sum;
    (
        a.u * w0 + b.u * w1 + c.u * w2,
        a.v * w0 + b.v * w1 + c.v * w2,
    )
}

fn sample_label_atlas_coverage(atlas: &PreparedLabelAtlas, u: f32, v: f32) -> f32 {
    let width = atlas.width().max(1);
    let height = atlas.height().max(1);
    let x = u.clamp(0.0, 1.0) * width as f32 - 0.5;
    let y = v.clamp(0.0, 1.0) * height as f32 - 0.5;
    let x0 = x.floor();
    let y0 = y.floor();
    let tx = x - x0;
    let ty = y - y0;
    let x0 = x0 as i32;
    let y0 = y0 as i32;
    let sample = |sx: i32, sy: i32| -> f32 {
        let sx = sx.clamp(0, width.saturating_sub(1) as i32) as u32;
        let sy = sy.clamp(0, height.saturating_sub(1) as i32) as u32;
        let offset = sy
            .saturating_mul(width)
            .saturating_add(sx)
            .saturating_mul(4)
            .saturating_add(3) as usize;
        atlas
            .rgba8()
            .get(offset)
            .copied()
            .map(|value| f32::from(value) / 255.0)
            .unwrap_or(0.0)
    };
    let c00 = sample(x0, y0);
    let c10 = sample(x0 + 1, y0);
    let c01 = sample(x0, y0 + 1);
    let c11 = sample(x0 + 1, y0 + 1);
    let top = c00 + (c10 - c00) * tx;
    let bottom = c01 + (c11 - c01) * tx;
    (top + (bottom - top) * ty).clamp(0.0, 1.0)
}

fn mix_position(a: Vec3, b: Vec3, c: Vec3, w0: f32, w1: f32, w2: f32) -> Vec3 {
    Vec3::new(
        a.x * w0 + b.x * w1 + c.x * w2,
        a.y * w0 + b.y * w1 + c.y * w2,
        a.z * w0 + b.z * w1 + c.z * w2,
    )
}

fn mix_depth(a: f32, b: f32, c: f32, w0: f32, w1: f32, w2: f32) -> f32 {
    a * w0 + b * w1 + c * w2
}

fn is_clipped(
    position: Vec3,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
) -> bool {
    clipping_planes
        .iter()
        .any(|plane| !plane.contains(position))
        || section_box.is_some_and(|section| section.clips(position))
}
