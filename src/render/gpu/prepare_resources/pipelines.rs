use super::super::light_assignment::LightAssignmentResources;
use super::super::pipeline::{GPU_COLOR_FORMAT, MeshPipelineSet, create_unlit_pipeline_set};
use super::super::{GpuOutputPlan, output};

pub(super) struct PipelineResources {
    pub(super) surface_output_uniform: Option<wgpu::Buffer>,
    pub(super) surface_output_bind_group: Option<wgpu::BindGroup>,
    pub(super) surface_opaque_output_bind_group: Option<wgpu::BindGroup>,
    pub(super) offscreen_pipelines: MeshPipelineSet,
    pub(super) offscreen_msaa4_pipelines: MeshPipelineSet,
    pub(super) offscreen_msaa8_pipelines: Option<MeshPipelineSet>,
    pub(super) surface_pipeline: Option<MeshPipelineSet>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_pipeline_resources(
    device: &wgpu::Device,
    surface_format: Option<wgpu::TextureFormat>,
    output_plan: GpuOutputPlan,
    sample_count: u32,
    triangle_shader: &wgpu::ShaderModule,
    output_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
    draw_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    shadow_view: &wgpu::TextureView,
    shadow_sampler: &wgpu::Sampler,
    environment_cubemap: &wgpu::Texture,
    environment_sampler: &wgpu::Sampler,
    transmission_view: &wgpu::TextureView,
    transmission_placeholder_view: &wgpu::TextureView,
    transmission_sampler: &wgpu::Sampler,
    light_assignment: &LightAssignmentResources,
) -> PipelineResources {
    let surface_output_resources = surface_format.map(|_| {
        let uniform = super::super::output::create_output_uniform_buffer(device);
        let environment_cubemap_view =
            environment_cubemap.create_view(&wgpu::TextureViewDescriptor {
                label: Some("scena.surface_output.environment_cubemap_view"),
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
        let bind_group = output::create_output_bind_group(
            device,
            output_layout,
            &uniform,
            shadow_view,
            shadow_sampler,
            &environment_cubemap_view,
            environment_sampler,
            transmission_view,
            transmission_sampler,
            Some(light_assignment),
        );
        let opaque_bind_group = output::create_output_bind_group(
            device,
            output_layout,
            &uniform,
            shadow_view,
            shadow_sampler,
            &environment_cubemap_view,
            environment_sampler,
            transmission_placeholder_view,
            transmission_sampler,
            Some(light_assignment),
        );
        (uniform, bind_group, opaque_bind_group)
    });
    let (surface_output_uniform, surface_output_bind_group, surface_opaque_output_bind_group) =
        match surface_output_resources {
            Some((uniform, bind_group, opaque_bind_group)) => {
                (Some(uniform), Some(bind_group), Some(opaque_bind_group))
            }
            None => (None, None, None),
        };
    let create_pipelines = |format, samples| {
        create_unlit_pipeline_set(
            device,
            triangle_shader,
            format,
            output_layout,
            material_layout,
            draw_layout,
            depth_compare,
            samples,
        )
    };
    let offscreen_pipelines = create_pipelines(GPU_COLOR_FORMAT, 1);
    let offscreen_msaa4_pipelines = create_pipelines(GPU_COLOR_FORMAT, 4);
    let offscreen_msaa8_pipelines =
        (sample_count == 8).then(|| create_pipelines(GPU_COLOR_FORMAT, 8));
    let surface_pipeline = surface_format
        .filter(|_| !output_plan.post_enabled())
        .map(|format| create_pipelines(format, sample_count));

    PipelineResources {
        surface_output_uniform,
        surface_output_bind_group,
        surface_opaque_output_bind_group,
        offscreen_pipelines,
        offscreen_msaa4_pipelines,
        offscreen_msaa8_pipelines,
        surface_pipeline,
    }
}
