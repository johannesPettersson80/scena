use super::super::RasterTarget;
use super::GpuDeviceState;

impl GpuDeviceState {
    pub(super) fn configure_surface(&mut self, target: RasterTarget) {
        let size = self.clamp_surface_size_to_device_limits(crate::platform::SurfaceSize {
            width: target.width,
            height: target.height,
        });
        if let Some(surface) = &mut self.surface {
            if surface.config.width != size.width || surface.config.height != size.height {
                surface.config.width = size.width;
                surface.config.height = size.height;
            }
            surface.surface.configure(&self.device, &surface.config);
        }
        #[cfg(target_arch = "wasm32")]
        self.refresh_browser_canvas_output_color_space(target.backend);
    }
}
