use crate::capture::{CaptureRgba8, CaptureScreenRect};
use crate::diagnostics::RendererStats;
use crate::scene::SceneInspectionReportV1;

use super::Renderer;
use super::color_contract::linear_rgba_to_srgb8;

mod types;
pub use types::{
    RenderIntrospectionArtifactsV1, RenderIntrospectionCapabilitiesV1,
    RenderIntrospectionCaptureSummaryV1, RenderIntrospectionFixV1, RenderIntrospectionFramingV1,
    RenderIntrospectionLuminanceV1, RenderIntrospectionNodeDetailV1,
    RenderIntrospectionNodesSummaryV1, RenderIntrospectionOptions, RenderIntrospectionReasonV1,
    RenderIntrospectionRectV1, RenderIntrospectionReportV1,
};

pub const RENDER_INTROSPECTION_SCHEMA_V1: &str = "scena.render_introspection.v1";

impl RenderIntrospectionReportV1 {
    pub fn from_capture(
        capture: &CaptureRgba8,
        inspection: &SceneInspectionReportV1,
        stats: RendererStats,
        options: RenderIntrospectionOptions,
    ) -> Self {
        let width = capture.descriptor.width;
        let height = capture.descriptor.height;
        let pixel_count = u64::from(width).saturating_mul(u64::from(height));
        let content = summarize_content_against_background(
            width,
            height,
            capture.rgba8.as_slice(),
            options.background_rgba8,
            options.content_tolerance_rgba8,
        );
        let visible_pixels = content.pixels;
        let visible_pixel_fraction = fraction(visible_pixels, pixel_count);
        let content_bbox_css_px = content
            .bbox
            .map(RenderIntrospectionRectV1::from_pixel_bounds);
        let content_bbox_fraction = content_bbox_css_px
            .map(|rect| rect.to_fraction(width.max(1) as f32, height.max(1) as f32));
        let luminance = summarize_luminance(capture.rgba8.as_slice());
        let framing = framing_summary(
            content_bbox_css_px,
            width,
            height,
            visible_pixel_fraction,
            inspection.active_camera,
        );
        let nodes_summary = nodes_summary(inspection, stats);
        let mut reasons = Vec::new();
        let mut fixes = Vec::new();

        if visible_pixels == 0 {
            push_reason(
                &mut reasons,
                "empty_frame",
                "error",
                "rendered frame has no non-background pixels",
            );
            push_fix(
                &mut fixes,
                "frame_bounds",
                "frame the scene or target bounds before rendering again",
            );
        }
        if inspection.counts.visible_drawable == 0 {
            push_reason(
                &mut reasons,
                "no_visible_drawables",
                "error",
                "inspection reports no visible drawable nodes",
            );
            push_fix(
                &mut fixes,
                "set_visible",
                "make at least one drawable node and its parents visible",
            );
        }
        if stats.culled_objects > 0 && visible_pixels == 0 {
            push_reason(
                &mut reasons,
                "all_culled",
                "error",
                "renderer stats report culled objects and no visible pixels",
            );
            push_fix(
                &mut fixes,
                "frame_bounds",
                "move the camera or object so culled content falls inside the frustum",
            );
        }
        if framing.tiny_in_frame {
            push_reason(
                &mut reasons,
                "tiny_in_frame",
                "warning",
                "visible content occupies too little of the frame to verify",
            );
            push_fix(
                &mut fixes,
                "frame_bounds",
                "increase framing fill or move the camera closer to the target bounds",
            );
        }
        if framing.cropped {
            push_reason(
                &mut reasons,
                "cropped",
                "warning",
                "visible content touches a viewport edge",
            );
            push_fix(
                &mut fixes,
                "frame_bounds",
                "decrease framing fill or add margin before rendering again",
            );
        }

        let nodes_detail = if options.detail {
            inspection
                .nodes
                .iter()
                .map(|node| RenderIntrospectionNodeDetailV1 {
                    handle: node.handle,
                    kind: node.kind.clone(),
                    visible: node.visible,
                    coverage: if node.visible { "unknown" } else { "hidden" }.to_owned(),
                    reason_codes: if node.visible {
                        Vec::new()
                    } else {
                        vec!["node_hidden".to_owned()]
                    },
                })
                .collect()
        } else {
            Vec::new()
        };

        Self {
            schema: RENDER_INTROSPECTION_SCHEMA_V1.to_owned(),
            ok: !has_error_reasons(&reasons),
            reasons,
            fixes,
            content_bbox_css_px,
            content_bbox_fraction,
            visible_pixel_fraction: round3(visible_pixel_fraction),
            luminance,
            framing,
            nodes_summary,
            nodes_detail,
            artifacts: RenderIntrospectionArtifactsV1 {
                capture_png_path: options.capture_png_path,
                capture_descriptor_path: options.capture_descriptor_path,
                contact_sheet_path: options.contact_sheet_path,
                capture: RenderIntrospectionCaptureSummaryV1 {
                    schema: capture.descriptor.schema.clone(),
                    width,
                    height,
                    payload_fnv1a64: capture.descriptor.payload.fnv1a64.clone(),
                },
            },
            capabilities: RenderIntrospectionCapabilitiesV1 {
                backend: capture.descriptor.capabilities.backend,
                gpu_device: capture.descriptor.capabilities.gpu_device,
                surface_attached: capture.descriptor.capabilities.surface_attached,
                hardware_tier: capture.descriptor.capabilities.hardware_tier,
                forward_pbr: capture.descriptor.capabilities.forward_pbr,
                readback_headless_screenshots: capture
                    .descriptor
                    .capabilities
                    .readback_headless_screenshots,
            },
        }
    }

    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("render introspection report is serializable")
    }
}

impl Renderer {
    pub fn introspect_capture(
        &self,
        capture: &CaptureRgba8,
        inspection: &SceneInspectionReportV1,
        options: RenderIntrospectionOptions,
    ) -> RenderIntrospectionReportV1 {
        let options = options.with_background_rgba8(linear_rgba_to_srgb8(self.background_color()));
        RenderIntrospectionReportV1::from_capture(capture, inspection, self.stats(), options)
    }
}

impl RenderIntrospectionRectV1 {
    fn from_pixel_bounds(bounds: crate::CapturePixelBounds) -> Self {
        Self {
            min_x: bounds.min_x as f32,
            min_y: bounds.min_y as f32,
            max_x: bounds.max_x as f32,
            max_y: bounds.max_y as f32,
            width: bounds.width as f32,
            height: bounds.height as f32,
        }
    }

    fn to_fraction(self, width: f32, height: f32) -> Self {
        Self {
            min_x: round3(self.min_x / width),
            min_y: round3(self.min_y / height),
            max_x: round3(self.max_x / width),
            max_y: round3(self.max_y / height),
            width: round3(self.width / width),
            height: round3(self.height / height),
        }
    }
}

impl From<CaptureScreenRect> for RenderIntrospectionRectV1 {
    fn from(rect: CaptureScreenRect) -> Self {
        Self {
            min_x: round3(rect.min_x),
            min_y: round3(rect.min_y),
            max_x: round3(rect.max_x),
            max_y: round3(rect.max_y),
            width: round3(rect.width),
            height: round3(rect.height),
        }
    }
}

fn framing_summary(
    bbox: Option<RenderIntrospectionRectV1>,
    width: u32,
    height: u32,
    visible_pixel_fraction: f32,
    active_camera: Option<u64>,
) -> RenderIntrospectionFramingV1 {
    let Some(bbox) = bbox else {
        return RenderIntrospectionFramingV1 {
            center_offset_fraction: [0.0, 0.0],
            fit_fraction: 0.0,
            cropped: false,
            tiny_in_frame: false,
            active_camera,
        };
    };
    let width_f = width.max(1) as f32;
    let height_f = height.max(1) as f32;
    let content_center_x = bbox.min_x + bbox.width * 0.5;
    let content_center_y = bbox.min_y + bbox.height * 0.5;
    let center_offset_x = (content_center_x - width_f * 0.5) / width_f;
    let center_offset_y = (content_center_y - height_f * 0.5) / height_f;
    let fit_fraction = (bbox.width / width_f).max(bbox.height / height_f);
    RenderIntrospectionFramingV1 {
        center_offset_fraction: [round3(center_offset_x), round3(center_offset_y)],
        fit_fraction: round3(fit_fraction),
        cropped: bbox.min_x <= 0.0
            || bbox.min_y <= 0.0
            || bbox.max_x >= width.saturating_sub(1) as f32
            || bbox.max_y >= height.saturating_sub(1) as f32,
        tiny_in_frame: visible_pixel_fraction > 0.0
            && (visible_pixel_fraction < 0.005 || fit_fraction < 0.05),
        active_camera,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentSummary {
    pixels: u64,
    bbox: Option<crate::CapturePixelBounds>,
}

fn summarize_content_against_background(
    width: u32,
    height: u32,
    rgba8: &[u8],
    background: [u8; 4],
    tolerance: u8,
) -> ContentSummary {
    let mut pixels = 0_u64;
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;

    for y in 0..height {
        for x in 0..width {
            let offset = ((y as usize) * (width as usize) + (x as usize)) * 4;
            let Some(pixel) = rgba8.get(offset..offset + 4) else {
                continue;
            };
            if pixel_differs_from_background(pixel, background, tolerance) {
                pixels = pixels.saturating_add(1);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    let bbox = (pixels > 0).then_some(crate::CapturePixelBounds {
        min_x,
        min_y,
        max_x,
        max_y,
        width: max_x.saturating_sub(min_x).saturating_add(1),
        height: max_y.saturating_sub(min_y).saturating_add(1),
    });

    ContentSummary { pixels, bbox }
}

fn pixel_differs_from_background(pixel: &[u8], background: [u8; 4], tolerance: u8) -> bool {
    (0..3).any(|channel| pixel[channel].abs_diff(background[channel]) > tolerance)
}

fn nodes_summary(
    inspection: &SceneInspectionReportV1,
    stats: RendererStats,
) -> RenderIntrospectionNodesSummaryV1 {
    let visible = inspection.nodes.iter().filter(|node| node.visible).count();
    let hidden = inspection.nodes.len().saturating_sub(visible);
    RenderIntrospectionNodesSummaryV1 {
        visible,
        hidden,
        drawn: inspection.draw_list.len(),
        culled: stats.culled_objects,
        clipped: 0,
        transparent: 0,
        failed_material: stats.material_textures_missing_decoded_pixels,
        unknown_coverage: visible,
    }
}

fn has_error_reasons(reasons: &[RenderIntrospectionReasonV1]) -> bool {
    reasons.iter().any(|reason| reason.severity == "error")
}

fn summarize_luminance(rgba8: &[u8]) -> RenderIntrospectionLuminanceV1 {
    let mut values = rgba8
        .chunks_exact(4)
        .map(|pixel| {
            0.2126 * f32::from(pixel[0])
                + 0.7152 * f32::from(pixel[1])
                + 0.0722 * f32::from(pixel[2])
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        return RenderIntrospectionLuminanceV1 {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            p05: 0.0,
            p50: 0.0,
            p95: 0.0,
        };
    }
    values.sort_by(f32::total_cmp);
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    RenderIntrospectionLuminanceV1 {
        min: round3(values[0]),
        max: round3(*values.last().expect("non-empty luminance values")),
        mean: round3(mean),
        p05: round3(percentile_sorted(&values, 0.05)),
        p50: round3(percentile_sorted(&values, 0.50)),
        p95: round3(percentile_sorted(&values, 0.95)),
    }
}

fn percentile_sorted(values: &[f32], percentile: f32) -> f32 {
    let index = ((values.len().saturating_sub(1)) as f32 * percentile)
        .round()
        .clamp(0.0, values.len().saturating_sub(1) as f32) as usize;
    values[index]
}

fn fraction(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn round3(value: f32) -> f32 {
    let rounded = (value * 1000.0).round() / 1000.0;
    if rounded == -0.0 { 0.0 } else { rounded }
}

fn push_reason(
    reasons: &mut Vec<RenderIntrospectionReasonV1>,
    code: &str,
    severity: &str,
    message: &str,
) {
    if reasons.iter().any(|reason| reason.code == code) {
        return;
    }
    reasons.push(RenderIntrospectionReasonV1 {
        code: code.to_owned(),
        severity: severity.to_owned(),
        affected_handles: Vec::new(),
        message: message.to_owned(),
    });
}

fn push_fix(fixes: &mut Vec<RenderIntrospectionFixV1>, action: &str, help: &str) {
    if fixes.iter().any(|fix| fix.action == action) {
        return;
    }
    fixes.push(RenderIntrospectionFixV1 {
        action: action.to_owned(),
        target_handle: None,
        patch: None,
        help: help.to_owned(),
    });
}
