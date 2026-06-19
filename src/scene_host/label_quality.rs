use super::SceneHostCore;
use crate::geometry::GeometryTopology;
use crate::scene::LabelBillboard;
use crate::{AssetFetcher, RenderQualityRegion, Transform, Vec3};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn label_quality_regions(&self, width: u32, height: u32) -> Vec<RenderQualityRegion> {
        if width == 0 || height == 0 {
            return Vec::new();
        }
        let mut regions = Vec::new();
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
            let x0 = (projected.x - metrics.width_px * 0.5 - padding)
                .floor()
                .max(0.0);
            let y0 = (projected.y - metrics.height_px * 0.5 - padding)
                .floor()
                .max(0.0);
            let x1 = (projected.x + metrics.width_px * 0.5 + padding)
                .ceil()
                .min(width as f32);
            let y1 = (projected.y + metrics.height_px * 0.5 + padding)
                .ceil()
                .min(height as f32);
            let x = x0 as u32;
            let y = y0 as u32;
            let max_x = x1.max(x0) as u32;
            let max_y = y1.max(y0) as u32;
            regions.push(RenderQualityRegion {
                kind: "label",
                handle: self.node_handle_map.get(&node).copied(),
                x: x.min(width),
                y: y.min(height),
                width: max_x.saturating_sub(x).max(1),
                height: max_y.saturating_sub(y).max(1),
            });
        }
        regions
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
                let x0 = (start.x.min(end.x) - stroke_padding).floor().max(0.0);
                let y0 = (start.y.min(end.y) - stroke_padding).floor().max(0.0);
                let x1 = (start.x.max(end.x) + stroke_padding)
                    .ceil()
                    .min(width as f32);
                let y1 = (start.y.max(end.y) + stroke_padding)
                    .ceil()
                    .min(height as f32);
                let x = x0 as u32;
                let y = y0 as u32;
                let max_x = x1.max(x0) as u32;
                let max_y = y1.max(y0) as u32;
                regions.push(RenderQualityRegion {
                    kind: "line",
                    handle: self.node_handle_map.get(&node).copied(),
                    x: x.min(width),
                    y: y.min(height),
                    width: max_x.saturating_sub(x).max(1),
                    height: max_y.saturating_sub(y).max(1),
                });
            }
        }
        regions
    }
}

fn transform_point(transform: Transform, point: Vec3) -> Vec3 {
    transform.translation + transform.rotation * (point * transform.scale)
}
