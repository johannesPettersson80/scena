use super::{labels, strokes};

pub(super) struct OverlayPasses<'a> {
    pub(super) view: &'a wgpu::TextureView,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) stroke_resources: Option<&'a strokes::StrokeResources>,
    pub(super) stroke_pipeline: Option<&'a wgpu::RenderPipeline>,
    pub(super) label_resources: Option<&'a labels::LabelResources>,
    pub(super) label_pipeline: Option<&'a wgpu::RenderPipeline>,
    pub(super) label: &'static str,
    pub(super) draw_submissions: &'a mut u64,
}

pub(super) fn encode_overlay_passes(encoder: &mut wgpu::CommandEncoder, inputs: OverlayPasses<'_>) {
    if let (Some(stroke_resources), Some(stroke_pipeline)) =
        (inputs.stroke_resources, inputs.stroke_pipeline)
    {
        strokes::encode_pass(
            encoder,
            strokes::StrokePass {
                view: inputs.view,
                depth_view: inputs.depth_view,
                output_bind_group: inputs.output_bind_group,
                draw_bind_group: inputs.draw_bind_group,
                resources: stroke_resources,
                pipeline: stroke_pipeline,
                label: inputs.label,
                draw_submissions: inputs.draw_submissions,
            },
        );
    }
    if let (Some(label_resources), Some(label_pipeline)) =
        (inputs.label_resources, inputs.label_pipeline)
    {
        labels::encode_pass(
            encoder,
            labels::LabelPass {
                view: inputs.view,
                depth_view: inputs.depth_view,
                output_bind_group: inputs.output_bind_group,
                resources: label_resources,
                pipeline: label_pipeline,
                label: inputs.label,
                draw_submissions: inputs.draw_submissions,
            },
        );
    }
}
