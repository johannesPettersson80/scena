#[cfg(not(target_arch = "wasm32"))]
pub(super) fn create_culling_pipeline(device: &wgpu::Device) -> wgpu::ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scena.m4.frustum_culling_compute"),
        source: wgpu::ShaderSource::Wgsl(CULLING_SHADER.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("scena.m4.frustum_culling_pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn encode_culling_dispatch(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::ComputePipeline,
    workgroups: u32,
) {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("scena.m4.frustum_culling_dispatch"),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(workgroups.max(1), 1, 1);
}

#[cfg(not(target_arch = "wasm32"))]
const CULLING_SHADER: &str = r#"
@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) _id: vec3<u32>) {
}
"#;
