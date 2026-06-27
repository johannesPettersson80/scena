use super::SceneHostCore;
use crate::geometry::GeometryTopology;
use crate::material::Color;
use crate::scene::LabelBillboard;
use crate::{
    AssetFetcher, CaptureScreenRegion, RenderQualityRegion, Transform, Vec3,
    screen_region_from_center_size, screen_region_from_points,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelQualityTarget {
    pub region: RenderQualityRegion,
    pub background_srgb8: Option<[u8; 3]>,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn label_quality_regions(&self, width: u32, height: u32) -> Vec<RenderQualityRegion> {
        self.label_quality_targets(width, height)
            .into_iter()
            .map(|target| target.region)
            .collect()
    }

    pub fn label_quality_targets(&self, width: u32, height: u32) -> Vec<LabelQualityTarget> {
        if width == 0 || height == 0 {
            return Vec::new();
        }
        let mut targets = Vec::new();
        for (node, _label_key, label, transform) in self.scene.label_nodes() {
            if !self.scene.visible_for_active_camera(node) {
                continue;
            }
            let LabelBillboard::ScreenAligned = label.billboard();
            let Ok(Some(projected)) = self.scene.project_world_point(
                self.active_camera,
                transform.translation,
                width,
                height,
            ) else {
                continue;
            };
            let metrics = label.metrics();
            let padding = (label.size() * 0.25).ceil().max(2.0);
            let Some(region) = screen_region_from_center_size(
                projected.x,
                projected.y,
                metrics.width_px,
                metrics.height_px,
                padding,
                width,
                height,
            ) else {
                continue;
            };
            targets.push(LabelQualityTarget {
                region: render_quality_region(
                    "label",
                    self.node_handle_map.get(&node).copied(),
                    region,
                ),
                background_srgb8: label.background().map(linear_color_to_srgb8),
            });
        }
        targets
    }

    pub fn line_quality_regions(&self, width: u32, height: u32) -> Vec<RenderQualityRegion> {
        if width == 0 || height == 0 {
            return Vec::new();
        }
        let mut regions = Vec::new();
        for (node, mesh, transform) in self.scene.mesh_nodes() {
            if !self.scene.visible_for_active_camera(node) {
                continue;
            }
            let Some(geometry) = self.assets.geometry(mesh.geometry()) else {
                continue;
            };
            if geometry.topology() != GeometryTopology::Lines {
                continue;
            }
            let world = self.scene.world_transform(node).unwrap_or(transform);
            let vertices = geometry.vertices();
            let stroke_padding = self
                .assets
                .material(mesh.material())
                .and_then(|material| material.stroke_width_px())
                .unwrap_or(1.0)
                .ceil()
                .max(2.0);
            for segment in geometry.indices().chunks_exact(2) {
                let Some(start) = vertices.get(segment[0] as usize) else {
                    continue;
                };
                let Some(end) = vertices.get(segment[1] as usize) else {
                    continue;
                };
                let start = transform_point(world, start.position);
                let end = transform_point(world, end.position);
                let Ok(Some(start)) =
                    self.scene
                        .project_world_point(self.active_camera, start, width, height)
                else {
                    continue;
                };
                let Ok(Some(end)) =
                    self.scene
                        .project_world_point(self.active_camera, end, width, height)
                else {
                    continue;
                };
                let Some(region) = screen_region_from_points(
                    &[(start.x, start.y), (end.x, end.y)],
                    stroke_padding,
                    width,
                    height,
                ) else {
                    continue;
                };
                regions.push(render_quality_region(
                    "line",
                    self.node_handle_map.get(&node).copied(),
                    region,
                ));
            }
        }
        regions
    }
}

fn transform_point(transform: Transform, point: Vec3) -> Vec3 {
    transform.translation + transform.rotation * (point * transform.scale)
}

fn render_quality_region(
    kind: &'static str,
    handle: Option<u64>,
    region: CaptureScreenRegion,
) -> RenderQualityRegion {
    RenderQualityRegion {
        kind,
        handle,
        x: region.x,
        y: region.y,
        width: region.width,
        height: region.height,
    }
}

fn linear_color_to_srgb8(color: Color) -> [u8; 3] {
    [
        linear_channel_to_srgb8(color.r),
        linear_channel_to_srgb8(color.g),
        linear_channel_to_srgb8(color.b),
    ]
}

fn linear_channel_to_srgb8(value: f32) -> u8 {
    (linear_channel_to_srgb(value) * 255.0).round() as u8
}

fn linear_channel_to_srgb(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}
