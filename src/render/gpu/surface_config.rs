use super::super::RasterTarget;
use super::GpuDeviceState;

impl GpuDeviceState {
    pub(super) fn configure_surface(&mut self, target: RasterTarget) {
        let previous_format = self.surface.as_ref().map(|surface| surface.config.format);
        if let Some(surface) = &mut self.surface {
            super::surface_frame::refresh_surface_configuration(
                surface,
                &self.adapter,
                &self.device,
                target,
            );
        }
        let current_format = self.surface.as_ref().map(|surface| surface.config.format);
        if current_format != previous_format {
            self.sample_count_capabilities.clear();
        }
        #[cfg(target_arch = "wasm32")]
        self.refresh_browser_canvas_output_color_space(target.backend);
    }
}
