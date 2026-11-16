//! The native occlusion passes and the resources they own.

use super::{VISIBILITY_BYTES_PER_PIXEL, create_visibility_buffer};
use crate::render::camera::CameraProjection;
use crate::render::gpu::depth::DepthPrepassResources;
use crate::render::gpu::shader_manifest::{ShaderVariantId, create_shader_module};
use crate::render::gpu::{GpuPostSettings, GpuResourceStats};
use crate::render::{RasterTarget, ScreenSpaceAmbientOcclusionConfig};

/// `mat4x4` projection followed by two `vec4` parameter rows.
const UNIFORM_FLOAT_COUNT: usize = 24;
const UNIFORM_BYTE_LEN: u64 = (UNIFORM_FLOAT_COUNT * std::mem::size_of::<f32>()) as u64;
/// Spiral taps per pixel. The depth-aware blur removes the remaining noise.
const SAMPLE_COUNT: f32 = 24.0;
/// Must match `@workgroup_size` in the shader.
const WORKGROUP_EDGE: u32 = 8;

/// Visibility bound into the colour pass, plus the passes that fill it.
#[derive(Debug)]
pub(in crate::render::gpu) struct AmbientOcclusionResources {
    /// Blurred visibility, one `f32` per target pixel in row-major order, or
    /// a single `1.0` when the pass is off.
    output: wgpu::Buffer,
    pass: Option<AmbientOcclusionPass>,
}

#[derive(Debug)]
struct AmbientOcclusionPass {
    occlusion_pipeline: wgpu::ComputePipeline,
    blur_pipeline: wgpu::ComputePipeline,
    /// Occlusion, horizontal blur, vertical blur.
    uniforms: [wgpu::Buffer; 3],
    bind_groups: [wgpu::BindGroup; 3],
    workgroups: [u32; 2],
}

/// Per-frame inputs of the occlusion passes.
struct AmbientOcclusionFrame {
    clip_from_view: [f32; 16],
    config: ScreenSpaceAmbientOcclusionConfig,
    clear_depth: f32,
    /// Target pixels per output pixel, so supersampled frames keep the same
    /// apparent radius.
    resolution_scale: f32,
}

impl AmbientOcclusionResources {
    /// Builds the pass when `depth_view` is available, otherwise a placeholder
    /// that leaves environment lighting untouched.
    ///
    /// `depth_view` must be a single-sampled `Depth32Float` view of the depth
    /// prepass for `target`.
    pub(in crate::render::gpu) fn create(
        device: &wgpu::Device,
        target: RasterTarget,
        depth_view: Option<&wgpu::TextureView>,
    ) -> Self {
        let Some(depth_view) = depth_view else {
            return Self {
                output: create_visibility_buffer(device, "scena.ambient_occlusion.placeholder", 1),
                pass: None,
            };
        };
        let pixels = u64::from(target.width.max(1)) * u64::from(target.height.max(1));
        let raw = create_visibility_buffer(device, "scena.ambient_occlusion.raw", pixels);
        let blur = create_visibility_buffer(device, "scena.ambient_occlusion.blur", pixels);
        let output = create_visibility_buffer(device, "scena.ambient_occlusion.output", pixels);
        let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scena.ambient_occlusion.bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(UNIFORM_BYTE_LEN),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                storage(2, true),
                storage(3, false),
            ],
        });
        let uniforms = [(); 3].map(|()| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("scena.ambient_occlusion.uniform"),
                size: UNIFORM_BYTE_LEN,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        // (source, target) per stage. The occlusion stage reads no source, so
        // it borrows a buffer it does not write.
        let stages = [(&blur, &raw), (&raw, &blur), (&blur, &output)];
        let bind_groups = [0, 1, 2].map(|index: usize| {
            let (source, target) = stages[index];
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("scena.ambient_occlusion.bind_group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniforms[index].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: source.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: target.as_entire_binding(),
                    },
                ],
            })
        });
        let shader = create_shader_module(
            device,
            ShaderVariantId::AmbientOcclusion,
            "scena.ambient_occlusion.shader",
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scena.ambient_occlusion.pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label, entry_point| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        Self {
            output,
            pass: Some(AmbientOcclusionPass {
                occlusion_pipeline: pipeline("scena.ambient_occlusion.pipeline", "cs_occlusion"),
                blur_pipeline: pipeline("scena.ambient_occlusion.blur_pipeline", "cs_blur"),
                uniforms,
                bind_groups,
                workgroups: [
                    target.width.max(1).div_ceil(WORKGROUP_EDGE),
                    target.height.max(1).div_ceil(WORKGROUP_EDGE),
                ],
            }),
        }
    }

    /// The buffer the colour pass reads.
    pub(in crate::render::gpu) const fn output(&self) -> &wgpu::Buffer {
        &self.output
    }

    /// True when the colour pass receives real occlusion rather than `1.0`.
    const fn is_active(&self) -> bool {
        self.pass.is_some()
    }

    /// Resources owned by the pass. The placeholder of an inactive pass is
    /// not reported, matching the other one-texel placeholders.
    pub(in crate::render::gpu) fn resource_stats(&self, target: RasterTarget) -> GpuResourceStats {
        if self.pass.is_none() {
            return GpuResourceStats::default();
        }
        let pixels = u64::from(target.width.max(1)) * u64::from(target.height.max(1));
        GpuResourceStats {
            buffers: 6,
            pipelines: 2,
            shader_modules: 1,
            shader_module_creations: 1,
            bind_groups: 3,
            approximate_gpu_memory_bytes: 3 * pixels * VISIBILITY_BYTES_PER_PIXEL
                + 3 * UNIFORM_BYTE_LEN,
            ..GpuResourceStats::default()
        }
    }

    /// Runs the pass for this frame when occlusion is configured, and records
    /// in the returned settings whether the colour pass will apply it. Must be
    /// called after the depth prepass and before the colour pass.
    pub(in crate::render::gpu) fn encode_for_frame(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        post_settings: GpuPostSettings,
        camera_projection: &CameraProjection,
        depth_prepass: Option<&DepthPrepassResources>,
    ) -> GpuPostSettings {
        let applied = match (
            post_settings.ambient_occlusion(),
            depth_prepass,
            camera_projection.clip_from_view_matrix(),
        ) {
            (Some(config), Some(depth_prepass), Some(clip_from_view)) if self.is_active() => {
                self.encode(
                    encoder,
                    queue,
                    &AmbientOcclusionFrame {
                        clip_from_view,
                        config,
                        clear_depth: depth_prepass.clear_depth(),
                        resolution_scale: post_settings.resolution_scale() as f32,
                    },
                );
                true
            }
            _ => false,
        };
        post_settings.with_ambient_occlusion_in_color_pass(applied)
    }

    /// Computes occlusion and its two blur passes.
    fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        frame: &AmbientOcclusionFrame,
    ) {
        let Some(pass) = &self.pass else {
            return;
        };
        let scale = frame.resolution_scale.max(1.0);
        let step = scale.round().max(1.0);
        let radius = f32::from(frame.config.radius_px()) * scale;
        for (uniform, blur_step) in pass
            .uniforms
            .iter()
            .zip([[0.0, 0.0], [step, 0.0], [0.0, step]])
        {
            let mut values = [0.0_f32; UNIFORM_FLOAT_COUNT];
            values[..16].copy_from_slice(&frame.clip_from_view);
            values[16..20].copy_from_slice(&[
                radius,
                frame.config.intensity(),
                frame.clear_depth,
                SAMPLE_COUNT,
            ]);
            values[20..22].copy_from_slice(&blur_step);
            let mut bytes = [0_u8; UNIFORM_BYTE_LEN as usize];
            for (chunk, value) in bytes.chunks_exact_mut(4).zip(values) {
                chunk.copy_from_slice(&value.to_ne_bytes());
            }
            queue.write_buffer(uniform, 0, &bytes);
        }
        let pipelines = [
            &pass.occlusion_pipeline,
            &pass.blur_pipeline,
            &pass.blur_pipeline,
        ];
        // One pass per stage: each stage reads the buffer the previous wrote,
        // and a buffer cannot be both read and written inside one pass.
        for (pipeline, bind_group) in pipelines.into_iter().zip(&pass.bind_groups) {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("scena.ambient_occlusion.pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);
            compute_pass.dispatch_workgroups(pass.workgroups[0], pass.workgroups[1], 1);
        }
    }
}
