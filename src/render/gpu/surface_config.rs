use super::super::RasterTarget;
use super::GpuDeviceState;

impl GpuDeviceState {
    pub(super) fn configure_surface(&mut self, target: RasterTarget) {
        if let Some(surface) = &mut self.surface {
            if surface.config.width != target.width || surface.config.height != target.height {
                surface.config.width = target.width;
                surface.config.height = target.height;
            }
            surface.surface.configure(&self.device, &surface.config);
        }
    }
}
