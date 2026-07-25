use crate::material::Color;
use crate::scene::{ClippingPlane, SectionBox, Vec3};

use super::camera::CameraProjection;
use super::cpu::{CpuFrame, write_label_overlay_pixel};
use super::prepare::{PreparedLabelAtlas, PreparedStrokeSegment};
use super::target::RasterTarget;

const STROKE_AA_RADIUS_PX: f32 = 3.25;

pub(super) fn draw_overlay_layers_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    strokes: &[PreparedStrokeSegment],
    labels: &PreparedLabelAtlas,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
    camera: &CameraProjection,
) {
    draw_strokes_cpu(cpu_frame, strokes, clipping_planes, section_box, camera);
    super::cpu_labels::draw_label_atlas_cpu(
        cpu_frame,
        labels,
        clipping_planes,
        section_box,
        camera,
    );
}

pub(super) fn draw_strokes_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    strokes: &[PreparedStrokeSegment],
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
    camera: &CameraProjection,
) {
    for stroke in strokes {
        // G01: leader and dimension lines are generated annotation geometry.
        // A section box sections the model; it must not delete the annotation
        // describing the section.
        let (stroke_planes, stroke_section_box) = if stroke.clips_with_scene() {
            (clipping_planes, section_box)
        } else {
            (&[][..], None)
        };
        draw_stroke_cpu(cpu_frame, stroke, stroke_planes, stroke_section_box, camera);
    }
}

fn draw_stroke_cpu(
    cpu_frame: &mut CpuFrame<'_>,
    stroke: &PreparedStrokeSegment,
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
    camera: &CameraProjection,
) {
    let start = transform_point(stroke.world_from_model(), stroke.start());
    let end = transform_point(stroke.world_from_model(), stroke.end());
    let Some(start_screen) = ScreenStrokePoint::project(start, cpu_frame.target, camera) else {
        return;
    };
    let Some(end_screen) = ScreenStrokePoint::project(end, cpu_frame.target, camera) else {
        return;
    };
    let dx = end_screen.x - start_screen.x;
    let dy = end_screen.y - start_screen.y;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f32::EPSILON {
        return;
    }

    let half_width = (stroke.width_px().max(0.001) * 0.5).max(0.5);
    let aa_radius = STROKE_AA_RADIUS_PX;
    let radius = half_width + aa_radius;
    let min_x = (start_screen.x.min(end_screen.x) - radius).floor().max(0.0) as u32;
    let max_x = (start_screen.x.max(end_screen.x) + radius)
        .ceil()
        .min(cpu_frame.target.width.saturating_sub(1) as f32) as u32;
    let min_y = (start_screen.y.min(end_screen.y) - radius).floor().max(0.0) as u32;
    let max_y = (start_screen.y.max(end_screen.y) + radius)
        .ceil()
        .min(cpu_frame.target.height.saturating_sub(1) as f32) as u32;
    let color = multiply_color(stroke.color(), stroke.tint());

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = (((px - start_screen.x) * dx + (py - start_screen.y) * dy) / length_squared)
                .clamp(0.0, 1.0);
            let closest_x = start_screen.x + dx * t;
            let closest_y = start_screen.y + dy * t;
            let distance = ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt();
            let coverage = stroke_coverage(distance, half_width);
            if coverage <= 0.0 {
                continue;
            }
            let position = start + (end - start) * t;
            if is_clipped(position, clipping_planes, section_box) {
                continue;
            }
            let depth = start_screen.depth + (end_screen.depth - start_screen.depth) * t;
            write_label_overlay_pixel(cpu_frame, x, y, color, coverage, depth);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenStrokePoint {
    x: f32,
    y: f32,
    depth: f32,
}

impl ScreenStrokePoint {
    fn project(position: Vec3, target: RasterTarget, camera: &CameraProjection) -> Option<Self> {
        let projected = camera.project(position)?;
        let width = target.width.saturating_sub(1) as f32;
        let height = target.height.saturating_sub(1) as f32;
        Some(Self {
            x: (projected.ndc_x * 0.5 + 0.5) * width,
            y: (1.0 - (projected.ndc_y * 0.5 + 0.5)) * height,
            depth: projected.depth,
        })
    }
}

fn stroke_coverage(distance_px: f32, half_width_px: f32) -> f32 {
    if distance_px <= half_width_px {
        1.0
    } else {
        let t = ((distance_px - half_width_px) / STROKE_AA_RADIUS_PX).clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    }
}

fn transform_point(matrix: [f32; 16], point: Vec3) -> Vec3 {
    Vec3::new(
        matrix[0] * point.x + matrix[4] * point.y + matrix[8] * point.z + matrix[12],
        matrix[1] * point.x + matrix[5] * point.y + matrix[9] * point.z + matrix[13],
        matrix[2] * point.x + matrix[6] * point.y + matrix[10] * point.z + matrix[14],
    )
}

fn multiply_color(color: Color, tint: Color) -> Color {
    Color::from_linear_rgba(
        color.r * tint.r,
        color.g * tint.g,
        color.b * tint.b,
        color.a * tint.a,
    )
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
