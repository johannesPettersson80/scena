use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, CaptureRgba8, RenderIntrospectionOptions, RenderIntrospectionReportV1};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn render_introspection(
        &self,
        detail: bool,
    ) -> Result<RenderIntrospectionReportV1, SceneHostError> {
        let capture = self.capture()?;
        Ok(self.render_introspection_from_capture(&capture, detail))
    }

    pub fn render_introspection_from_capture(
        &self,
        capture: &CaptureRgba8,
        detail: bool,
    ) -> RenderIntrospectionReportV1 {
        let inspection = self
            .scene
            .inspect_with_assets(&self.assets)
            .to_schema_report_with_node_handles(&self.node_handle_map);
        let options = if detail {
            RenderIntrospectionOptions::detail()
        } else {
            RenderIntrospectionOptions::summary()
        };
        self.renderer
            .introspect_capture(capture, &inspection, options)
    }

    pub fn render_introspection_json(&self, detail: bool) -> Result<String, SceneHostError> {
        let report = self.render_introspection(detail)?;
        serde_json::to_string(&report).map_err(render_introspection_serialization_error)
    }

    pub fn render_introspection_json_from_capture(
        &self,
        capture: &CaptureRgba8,
        detail: bool,
    ) -> Result<String, SceneHostError> {
        let report = self.render_introspection_from_capture(capture, detail);
        serde_json::to_string(&report).map_err(render_introspection_serialization_error)
    }
}

fn render_introspection_serialization_error(error: serde_json::Error) -> SceneHostError {
    SceneHostError::new(
        SceneHostErrorCode::Inspect,
        format!("render introspection serialization failed: {error}"),
    )
}
