use std::collections::BTreeSet;

use super::{PhotographicGroundV1, PhotographicSurroundingsReportV1};
use crate::{AssetFetcher, Camera, CaptureRgba8, ScreenSpaceReflectionConfig, Transform, Vec3};

use crate::scene_host::{SceneHostCore, SceneHostError, SceneHostErrorCode};

#[derive(Debug, Clone, PartialEq)]
pub struct PhotographicPlanarReflectionCaptureV1 {
    pub capture: CaptureRgba8,
    pub capture_count: u32,
    pub excluded_floor_nodes: u32,
    pub roughness: f32,
    pub strength: f32,
    pub horizontal_flip: bool,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn capture_photographic_planar_reflection(
        &mut self,
        report: &mut PhotographicSurroundingsReportV1,
    ) -> Result<Option<PhotographicPlanarReflectionCaptureV1>, SceneHostError> {
        if report.ground != PhotographicGroundV1::Reflective {
            return Ok(None);
        }
        if report.planar_reflection_capture_count != 0 {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                "reflective photographic ground permits exactly one planar capture per delivered frame",
            ));
        }
        let floor_y = report.support_height_m.ok_or_else(|| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                "reflective photographic ground requires a resolved support plane",
            )
        })?;
        let original_host_camera = self.active_camera;
        let original_scene_camera = self.scene.active_camera();
        let original_camera_node =
            self.scene
                .camera_node(original_host_camera)
                .ok_or_else(|| {
                    SceneHostError::new(SceneHostErrorCode::Lookup, "active camera node is missing")
                })?;
        let original_transform = self
            .scene
            .world_transform(original_camera_node)
            .ok_or_else(|| {
                SceneHostError::new(
                    SceneHostErrorCode::Lookup,
                    "active camera transform is missing",
                )
            })?;
        let perspective = match self.scene.camera(original_host_camera) {
            Some(Camera::Perspective(camera)) => *camera,
            _ => {
                return Err(SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    "reflective photographic ground requires a perspective camera",
                ));
            }
        };
        let reflected_transform = mirrored_camera_transform(original_transform, floor_y);
        let mirror_camera = self.scene.add_perspective_camera(
            self.scene.root(),
            perspective,
            reflected_transform,
        )?;
        let mirror_camera_node = self.scene.camera_node(mirror_camera).ok_or_else(|| {
            SceneHostError::new(SceneHostErrorCode::Lookup, "mirror camera node is missing")
        })?;

        let original_probe_state = self.scene.reflection_probes_enabled();
        let original_ssr: Option<ScreenSpaceReflectionConfig> =
            self.renderer.screen_space_reflections();
        let mut excluded = BTreeSet::new();
        excluded.extend(report.support_nodes.iter().copied());
        excluded.extend(report.contact_shadow_nodes.iter().copied());
        excluded.extend(report.grid_nodes.iter().copied());
        let mut visibility = Vec::with_capacity(excluded.len());
        for handle in excluded.iter().copied() {
            let node = self.resolve_node(handle)?;
            if let Some(visible) = self.scene.visible(node) {
                visibility.push((node, visible));
                if visible {
                    self.scene.set_visible(node, false)?;
                }
            }
        }

        let capture_result = (|| {
            self.scene.set_reflection_probes_enabled(false);
            self.renderer.set_screen_space_reflections(None);
            self.scene.set_active_camera(mirror_camera)?;
            self.active_camera = mirror_camera;
            self.prepare()?;
            self.render()?;
            self.capture()
        })();

        self.scene
            .set_reflection_probes_enabled(original_probe_state);
        self.renderer.set_screen_space_reflections(original_ssr);
        let _ = self
            .scene
            .set_active_camera(original_scene_camera.unwrap_or(original_host_camera));
        self.active_camera = original_host_camera;
        let _ = self.scene.remove_node(mirror_camera_node);
        for (node, visible) in visibility {
            let _ = self.scene.set_visible(node, visible);
        }

        let mut capture = capture_result?;
        flip_rgba8_horizontally(
            &mut capture.rgba8,
            capture.descriptor.width,
            capture.descriptor.height,
        );
        report.planar_reflection_capture_count = 1;
        Ok(Some(PhotographicPlanarReflectionCaptureV1 {
            capture,
            capture_count: 1,
            excluded_floor_nodes: excluded.len() as u32,
            roughness: report.reflection_roughness,
            strength: report.reflection_strength,
            horizontal_flip: true,
        }))
    }
}

fn mirrored_camera_transform(transform: Transform, plane_y: f32) -> Transform {
    let position = Vec3::new(
        transform.translation.x,
        plane_y.mul_add(2.0, -transform.translation.y),
        transform.translation.z,
    );
    let forward = transform.rotation * Vec3::NEG_Z;
    let up = transform.rotation * Vec3::Y;
    let reflected_forward = Vec3::new(forward.x, -forward.y, forward.z);
    let reflected_up = Vec3::new(up.x, -up.y, up.z);
    Transform::at(position).looking_at(position + reflected_forward, reflected_up)
}

fn flip_rgba8_horizontally(rgba8: &mut [u8], width: u32, height: u32) {
    let width = width as usize;
    for y in 0..height as usize {
        for x in 0..width / 2 {
            let left = (y * width + x) * 4;
            let right = (y * width + (width - 1 - x)) * 4;
            for channel in 0..4 {
                rgba8.swap(left + channel, right + channel);
            }
        }
    }
}
