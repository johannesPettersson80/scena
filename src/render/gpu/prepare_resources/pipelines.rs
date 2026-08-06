use super::super::light_assignment::LightAssignmentResources;
use super::super::material_uniform::MaterialShaderFeatures;
use super::super::pipeline::{GPU_COLOR_FORMAT, MeshPipelineSet, create_unlit_pipeline_set};
use super::super::{GpuOutputPlan, output};

pub(super) struct PipelineResources {
    pub(super) surface_output_uniform: Option<wgpu::Buffer>,
    pub(super) surface_output_bind_group: Option<wgpu::BindGroup>,
    pub(super) surface_opaque_output_bind_group: Option<wgpu::BindGroup>,
    pub(super) surface_reflection_probe_output_bind_groups: Vec<wgpu::BindGroup>,
    pub(super) surface_reflection_probe_opaque_output_bind_groups: Vec<wgpu::BindGroup>,
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
    semantic_aov_capture_enabled: bool,
    triangle_shader: &wgpu::ShaderModule,
    output_layout: &wgpu::BindGroupLayout,
    material_layout: &wgpu::BindGroupLayout,
    draw_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    shadow_view: &wgpu::TextureView,
    shadow_sampler: &wgpu::Sampler,
    environment_cubemap: &wgpu::Texture,
    reflection_probe_cubemaps: &[wgpu::Texture],
    environment_sampler: &wgpu::Sampler,
    transmission_view: &wgpu::TextureView,
    transmission_placeholder_view: &wgpu::TextureView,
    transmission_sampler: &wgpu::Sampler,
    ltc_tables: &wgpu::Buffer,
    brdf_table: &wgpu::Buffer,
    light_assignment: &LightAssignmentResources,
    material_features: MaterialShaderFeatures,
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
            ltc_tables,
            brdf_table,
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
            ltc_tables,
            brdf_table,
            Some(light_assignment),
        );
        let mut probe_bind_groups = Vec::with_capacity(reflection_probe_cubemaps.len());
        let mut probe_opaque_bind_groups = Vec::with_capacity(reflection_probe_cubemaps.len());
        for cubemap in reflection_probe_cubemaps {
            let view = cubemap.create_view(&wgpu::TextureViewDescriptor {
                label: Some("scena.surface_output.reflection_probe_cubemap_view"),
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
            probe_bind_groups.push(output::create_output_bind_group(
                device,
                output_layout,
                &uniform,
                shadow_view,
                shadow_sampler,
                &view,
                environment_sampler,
                transmission_view,
                transmission_sampler,
                ltc_tables,
                brdf_table,
                Some(light_assignment),
            ));
            probe_opaque_bind_groups.push(output::create_output_bind_group(
                device,
                output_layout,
                &uniform,
                shadow_view,
                shadow_sampler,
                &view,
                environment_sampler,
                transmission_placeholder_view,
                transmission_sampler,
                ltc_tables,
                brdf_table,
                Some(light_assignment),
            ));
        }
        (
            uniform,
            bind_group,
            opaque_bind_group,
            probe_bind_groups,
            probe_opaque_bind_groups,
        )
    });
    let (
        surface_output_uniform,
        surface_output_bind_group,
        surface_opaque_output_bind_group,
        surface_reflection_probe_output_bind_groups,
        surface_reflection_probe_opaque_output_bind_groups,
    ) = match surface_output_resources {
        Some((uniform, bind_group, opaque_bind_group, probes, opaque_probes)) => (
            Some(uniform),
            Some(bind_group),
            Some(opaque_bind_group),
            probes,
            opaque_probes,
        ),
        None => (None, None, None, Vec::new(), Vec::new()),
    };
    let create_pipelines = |format, samples, semantic_target_format| {
        create_unlit_pipeline_set(
            device,
            triangle_shader,
            format,
            output_layout,
            material_layout,
            draw_layout,
            depth_compare,
            samples,
            semantic_target_format,
            material_features,
        )
    };
    let semantic_target_format =
        semantic_aov_capture_enabled.then_some(super::super::semantic_aov::FORMAT);
    let offscreen_pipelines = create_pipelines(GPU_COLOR_FORMAT, 1, semantic_target_format);
    let offscreen_msaa4_pipelines = create_pipelines(GPU_COLOR_FORMAT, 4, semantic_target_format);
    let offscreen_msaa8_pipelines =
        (sample_count == 8).then(|| create_pipelines(GPU_COLOR_FORMAT, 8, semantic_target_format));
    let surface_pipeline = surface_format
        .filter(|_| !output_plan.post_enabled())
        .map(|format| create_pipelines(format, sample_count, None));

    PipelineResources {
        surface_output_uniform,
        surface_output_bind_group,
        surface_opaque_output_bind_group,
        surface_reflection_probe_output_bind_groups,
        surface_reflection_probe_opaque_output_bind_groups,
        offscreen_pipelines,
        offscreen_msaa4_pipelines,
        offscreen_msaa8_pipelines,
        surface_pipeline,
    }
}
