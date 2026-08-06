use super::SceneHostCore;
use crate::assets::AssetFetcher;
use crate::scene_host::events::HostEventV1;
use crate::scene_host::{SceneHostError, SceneHostErrorCode};
use crate::{SurfaceEvent, SurfaceViewport};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn resize(
        &mut self,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<(), SceneHostError> {
        let viewport = SurfaceViewport::new(logical_width, logical_height, device_pixel_ratio)
            .ok_or_else(|| {
                SceneHostError::new(
                    SceneHostErrorCode::InvalidViewport,
                    format!(
                        "invalid viewport {logical_width}x{logical_height} at DPR {device_pixel_ratio}"
                    ),
                )
            })?;
        self.viewport = viewport;
        self.sync_active_camera_aspect(viewport)?;
        self.renderer
            .handle_surface_event(SurfaceEvent::ViewportChanged(viewport))?;
        self.emit_surface_resized_event(viewport);
        Ok(())
    }

    pub fn handle_surface_event(&mut self, event: SurfaceEvent) -> Result<(), SceneHostError> {
        match event {
            SurfaceEvent::Resize { width, height } if width > 0 && height > 0 => {
                let dpr = self.viewport.device_pixel_ratio();
                let viewport = SurfaceViewport::new(width as f32 / dpr, height as f32 / dpr, dpr)
                    .ok_or_else(|| {
                    SceneHostError::new(
                        SceneHostErrorCode::InvalidViewport,
                        format!("invalid physical viewport {width}x{height} at DPR {dpr}"),
                    )
                })?;
                self.viewport = viewport;
                self.sync_active_camera_aspect(viewport)?;
            }
            SurfaceEvent::ViewportChanged(viewport) => {
                self.viewport = viewport;
                self.sync_active_camera_aspect(viewport)?;
            }
            SurfaceEvent::ScaleFactorChanged { scale_factor } => {
                let dpr = scale_factor as f32;
                let physical = self.viewport.physical_size();
                let viewport = SurfaceViewport::new(
                    physical.width as f32 / dpr,
                    physical.height as f32 / dpr,
                    dpr,
                )
                .ok_or_else(|| {
                    SceneHostError::new(
                        SceneHostErrorCode::InvalidViewport,
                        format!("invalid device-pixel ratio {scale_factor}"),
                    )
                })?;
                self.viewport = viewport;
                self.sync_active_camera_aspect(viewport)?;
            }
            _ => {}
        }
        self.renderer.handle_surface_event(event)?;
        match event {
            SurfaceEvent::Resize { width, height } => {
                let dpr = self.viewport.device_pixel_ratio();
                self.emit_event(HostEventV1::SurfaceResized {
                    width_css_px: width as f32 / dpr,
                    height_css_px: height as f32 / dpr,
                    width_physical_px: width,
                    height_physical_px: height,
                    device_pixel_ratio: dpr,
                });
            }
            SurfaceEvent::ViewportChanged(viewport) => self.emit_surface_resized_event(viewport),
            SurfaceEvent::ContextLost { recoverable } => {
                self.emit_event(HostEventV1::ContextLost { recoverable });
            }
            SurfaceEvent::ContextRestored => {
                self.emit_event(HostEventV1::ContextRestored);
                self.emit_event(HostEventV1::capability_changed(self.backend()));
            }
            SurfaceEvent::DeviceLost { recoverable } => {
                self.emit_event(HostEventV1::DeviceLost { recoverable });
            }
            SurfaceEvent::ScaleFactorChanged { .. } => {
                self.emit_surface_resized_event(self.viewport);
            }
            SurfaceEvent::Occluded { .. }
            | SurfaceEvent::Hidden
            | SurfaceEvent::Shown
            | SurfaceEvent::Lost => {}
        }
        Ok(())
    }

    fn sync_active_camera_aspect(
        &mut self,
        viewport: SurfaceViewport,
    ) -> Result<(), SceneHostError> {
        let aspect = viewport.logical_width() / viewport.logical_height();
        match self.scene.camera(self.active_camera).cloned() {
            Some(crate::Camera::Perspective(mut camera)) => {
                if camera.aspect != aspect {
                    camera.aspect = aspect;
                    self.scene
                        .set_camera(self.active_camera, crate::Camera::Perspective(camera))?;
                }
            }
            Some(crate::Camera::Orthographic(camera)) => {
                let center_x = (camera.left + camera.right) * 0.5;
                let half_height = ((camera.top - camera.bottom).abs() * 0.5).max(0.0001);
                let half_width = half_height * aspect;
                let left = center_x - half_width;
                let right = center_x + half_width;
                if camera.left != left || camera.right != right {
                    self.scene.set_camera(
                        self.active_camera,
                        crate::Camera::Orthographic(crate::OrthographicCamera {
                            left,
                            right,
                            ..camera
                        }),
                    )?;
                }
            }
            None => {}
        }
        Ok(())
    }
}
