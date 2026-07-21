use super::super::RasterTarget;
use super::GpuDeviceState;

impl GpuDeviceState {
    pub(super) fn configure_surface(&mut self, target: RasterTarget) {
        if let Some(surface) = &mut self.surface {
            super::surface_frame::refresh_surface_configuration(
                surface,
                &self.adapter,
                &self.device,
                target,
            );
        }
        #[cfg(target_arch = "wasm32")]
        self.refresh_browser_canvas_output_color_space(target.backend);
    }
}
