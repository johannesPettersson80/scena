use wasm_bindgen::JsCast;

use super::super::{AutoExposureConfig, AutoExposureResult, estimate_auto_exposure_from_srgb8};
use super::GpuDeviceState;

impl GpuDeviceState {
    pub(in crate::render) fn estimate_browser_canvas_auto_exposure(
        &self,
        config: AutoExposureConfig,
    ) -> Option<AutoExposureResult> {
        const SAMPLE_SIZE: u32 = 32;

        let source = self.browser_canvas.as_ref()?;
        let document = web_sys::window()?.document()?;
        let sample = document
            .create_element("canvas")
            .ok()?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()?;
        sample.set_width(SAMPLE_SIZE);
        sample.set_height(SAMPLE_SIZE);
        let context = sample
            .get_context("2d")
            .ok()??
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()?;
        context
            .draw_image_with_html_canvas_element_and_dw_and_dh(
                source,
                0.0,
                0.0,
                f64::from(SAMPLE_SIZE),
                f64::from(SAMPLE_SIZE),
            )
            .ok()?;
        let image = context
            .get_image_data(0.0, 0.0, f64::from(SAMPLE_SIZE), f64::from(SAMPLE_SIZE))
            .ok()?;
        estimate_auto_exposure_from_srgb8(&image.data().to_vec(), config)
    }
}
