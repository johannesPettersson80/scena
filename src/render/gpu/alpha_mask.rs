//! Alpha-tested depth and shadow pipelines for glTF MASK materials.
//!
//! The plain depth prepass and shadow caster write depth for every covered
//! fragment. For a masked material that makes each cut-out hide whatever lies
//! behind it and cast a solid shadow. Batches flagged `alpha_masked` are drawn
//! with these pipelines instead, which bind the batch's material and discard
//! below the cutoff exactly as the colour pass does.

use super::instancing::{INSTANCE_ATTRIBUTES, INSTANCE_BYTE_LEN, InstanceDrawBatch};
use super::material_bindings::MaterialTextureBindingMode;
use super::material_uniform::MATERIAL_UNIFORM_ENTRY_STRIDE;
use super::materials::MaterialResources;
use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::pipeline::SCENA_FRONT_FACE;
use super::shader_manifest::{ShaderVariantId, create_shader_module};
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

pub(super) const ALPHA_MASK_SHADER_TEXTURE_2D_ARRAY: &str =
    include_str!("alpha_mask_texture_2d_array.wgsl");
pub(super) const ALPHA_MASK_SHADER_TEXTURE_2D: &str = include_str!("alpha_mask_texture_2d.wgsl");

/// Shader and material layout shared by every alpha-mask pipeline of one
/// prepare. Built only when the prepared scene contains masked geometry, so
/// scenes without cut-outs compile no extra pipelines.
#[derive(Debug)]
pub(super) struct AlphaMaskShader {
    module: wgpu::ShaderModule,
}

impl AlphaMaskShader {
    /// Compiles the shader when the prepared scene contains masked geometry.
    ///
    /// `alpha_masked` describes everything the retained template can draw,
    /// not the draw list of the frame being prepared. The retained prepare
    /// path keeps these pipelines while the camera moves, so masked geometry
    /// that starts outside the view needs them just the same.
    pub(super) fn for_scene(
        device: &wgpu::Device,
        texture_binding_mode: MaterialTextureBindingMode,
        alpha_masked: bool,
    ) -> Option<Self> {
        alpha_masked.then(|| {
            let variant = match texture_binding_mode {
                MaterialTextureBindingMode::Texture2d => ShaderVariantId::AlphaMaskTexture2d,
                MaterialTextureBindingMode::Texture2dArray => {
                    ShaderVariantId::AlphaMaskTexture2dArray
                }
            };
            Self {
                module: create_shader_module(device, variant, "scena.alpha_mask.shader"),
            }
        })
    }
}

/// Pipeline inputs for a frame's shader, or `None` when it has no masked batch.
pub(super) fn pipeline_inputs<'a>(
    shader: &'a Option<AlphaMaskShader>,
    material_bind_group_layout: &'a wgpu::BindGroupLayout,
) -> Option<AlphaMaskPipelineInputs<'a>> {
    shader.as_ref().map(|shader| AlphaMaskPipelineInputs {
        shader,
        material_bind_group_layout,
    })
}

/// Borrowed inputs for building alpha-mask pipelines.
#[derive(Clone, Copy)]
pub(super) struct AlphaMaskPipelineInputs<'a> {
    pub(super) shader: &'a AlphaMaskShader,
    pub(super) material_bind_group_layout: &'a wgpu::BindGroupLayout,
}

#[derive(Debug)]
pub(super) struct AlphaMaskDepthPipelines {
    single_sided: wgpu::RenderPipeline,
    double_sided: wgpu::RenderPipeline,
}

impl AlphaMaskDepthPipelines {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create(
        device: &wgpu::Device,
        inputs: AlphaMaskPipelineInputs<'_>,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        draw_bind_group_layout: &wgpu::BindGroupLayout,
        depth_compare: wgpu::CompareFunction,
        depth_color_format: Option<wgpu::TextureFormat>,
        sample_count: u32,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scena.alpha_mask.depth_pipeline_layout"),
            bind_group_layouts: &[
                Some(camera_bind_group_layout),
                Some(inputs.material_bind_group_layout),
                Some(draw_bind_group_layout),
            ],
            immediate_size: 0,
        });
        let color_targets: Vec<_> = depth_color_format
            .map(|format| {
                Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            })
            .into_iter()
            .collect();
        let create = |double_sided: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if double_sided {
                    "scena.alpha_mask.depth_pipeline.double_sided"
                } else {
                    "scena.alpha_mask.depth_pipeline.single_sided"
                }),
                layout: Some(&layout),
                vertex: vertex_state(&inputs.shader.module, "vs_depth"),
                primitive: wgpu::PrimitiveState {
                    front_face: SCENA_FRONT_FACE,
                    cull_mode: (!double_sided).then_some(wgpu::Face::Back),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(depth_compare),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &inputs.shader.module,
                    entry_point: Some(if depth_color_format.is_some() {
                        "fs_depth_color"
                    } else {
                        "fs_depth_only"
                    }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &color_targets,
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        Self {
            single_sided: create(false),
            double_sided: create(true),
        }
    }

    pub(super) const fn for_double_sided(&self, double_sided: bool) -> &wgpu::RenderPipeline {
        if double_sided {
            &self.double_sided
        } else {
            &self.single_sided
        }
    }
}

/// Shadow caster pipeline for masked batches. Depth bias and culling match the
/// plain caster so masked and opaque occluders agree at their shared edges.
pub(super) fn create_shadow_pipeline(
    device: &wgpu::Device,
    inputs: AlphaMaskPipelineInputs<'_>,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    bias: wgpu::DepthBiasState,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.alpha_mask.shadow_pipeline_layout"),
        bind_group_layouts: &[
            Some(camera_bind_group_layout),
            Some(inputs.material_bind_group_layout),
            Some(draw_bind_group_layout),
        ],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scena.alpha_mask.shadow_pipeline"),
        layout: Some(&layout),
        vertex: vertex_state(&inputs.shader.module, "vs_shadow"),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias,
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &inputs.shader.module,
            entry_point: Some("fs_shadow"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn vertex_state<'a>(module: &'a wgpu::ShaderModule, entry_point: &'a str) -> wgpu::VertexState<'a> {
    const BUFFERS: [wgpu::VertexBufferLayout<'static>; 2] = [
        wgpu::VertexBufferLayout {
            array_stride: VERTEX_BYTE_LEN as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        },
        wgpu::VertexBufferLayout {
            array_stride: INSTANCE_BYTE_LEN as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        },
    ];
    wgpu::VertexState {
        module,
        entry_point: Some(entry_point),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        buffers: &BUFFERS,
    }
}

/// Binds `@group(1)` for `material_slot`, mirroring the colour pass.
fn bind_material(
    pass: &mut wgpu::RenderPass<'_>,
    materials: &MaterialResources,
    material_slot: u32,
) {
    match materials {
        MaterialResources::PerMaterial(slots) => {
            if let Some(material) = slots.get(material_slot as usize).or_else(|| slots.first()) {
                pass.set_bind_group(1, &material.bind_group, &[0]);
            }
        }
        MaterialResources::Batched(batched) => {
            let layer =
                u64::from(material_slot).min(u64::from(batched.layer_count.saturating_sub(1)));
            let offset = layer.saturating_mul(MATERIAL_UNIFORM_ENTRY_STRIDE) as u32;
            pass.set_bind_group(1, &batched.bind_group, &[offset]);
        }
    }
}

/// Buffers and batches a masked depth or shadow pass draws from. Vertex buffer
/// slot 0 must already be bound by the caller.
pub(super) struct MaskedDraws<'a> {
    pub(super) materials: &'a MaterialResources,
    pub(super) instance_buffer: &'a wgpu::Buffer,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) instance_batches: &'a [InstanceDrawBatch],
    pub(super) identity_instance: u32,
}

/// Draws the depth-prepass-eligible masked batches with alpha testing.
pub(super) fn encode_depth_draws(
    pass: &mut wgpu::RenderPass<'_>,
    pipelines: &AlphaMaskDepthPipelines,
    draws: &MaskedDraws<'_>,
    draw_submissions: &mut u64,
) {
    for double_sided in [false, true] {
        pass.set_pipeline(pipelines.for_double_sided(double_sided));
        encode_batches(
            pass,
            draws,
            |batch| batch.depth_prepass_eligible && batch.double_sided == double_sided,
            |batch| batch.depth_prepass_eligible && batch.double_sided == double_sided,
            draw_submissions,
        );
    }
}

/// Draws every masked batch into the bound shadow map region.
pub(super) fn encode_shadow_draws(
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    draws: &MaskedDraws<'_>,
    draw_submissions: &mut u64,
) {
    pass.set_pipeline(pipeline);
    encode_batches(pass, draws, |_| true, |_| true, draw_submissions);
}

fn encode_batches(
    pass: &mut wgpu::RenderPass<'_>,
    draws: &MaskedDraws<'_>,
    include: impl Fn(&PrimitiveDrawBatch) -> bool,
    include_instanced: impl Fn(&InstanceDrawBatch) -> bool,
    draw_submissions: &mut u64,
) {
    let identity_offset =
        u64::from(draws.identity_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
    pass.set_vertex_buffer(1, draws.instance_buffer.slice(identity_offset..));
    for batch in draws
        .draw_batches
        .iter()
        .filter(|batch| batch.alpha_masked && include(batch))
    {
        bind_material(pass, draws.materials, batch.material_slot);
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, draws.draw_bind_group, &[draw_offset]);
        pass.draw(
            batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
            0..1,
        );
        *draw_submissions = draw_submissions.saturating_add(1);
    }
    for batch in draws
        .instance_batches
        .iter()
        .filter(|batch| batch.alpha_masked && include_instanced(batch))
    {
        bind_material(pass, draws.materials, batch.material_slot);
        let draw_offset =
            (batch.draw_uniform_index as u64).saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE) as u32;
        pass.set_bind_group(2, draws.draw_bind_group, &[draw_offset]);
        let instance_offset =
            u64::from(batch.start_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
        pass.set_vertex_buffer(1, draws.instance_buffer.slice(instance_offset..));
        pass.draw(
            batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
            0..batch.instance_count,
        );
        *draw_submissions = draw_submissions.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The WebGL2 variant must stay the mechanical `texture_2d` rewrite of the
    /// array variant, so a fix to one cannot silently miss the other.
    #[test]
    fn texture_2d_variant_is_the_array_variant_with_plain_2d_sampling() {
        let expected = ALPHA_MASK_SHADER_TEXTURE_2D_ARRAY
            .replace(
                "// scena.alpha_mask_texture_2d_array.wgsl",
                "// scena.alpha_mask_texture_2d.wgsl",
            )
            .replace(
                "// `alpha_mask_texture_2d.wgsl` is this file with the material texture bound\n\
                 // as `texture_2d` for WebGL2; a unit test keeps the two in step.",
                "// Generated from `alpha_mask_texture_2d_array.wgsl` with the material texture\n\
                 // bound as `texture_2d` for WebGL2; a unit test keeps the two in step.",
            )
            .replace(
                "var base_color_texture: texture_2d_array<f32>;",
                "var base_color_texture: texture_2d<f32>;",
            )
            .replace(
                "    let material_layer = i32(material.material_layer_index.x);\n",
                "",
            )
            .replace(
                "textureSample(base_color_texture, base_color_sampler, transformed_uv, material_layer)",
                "textureSample(base_color_texture, base_color_sampler, transformed_uv)",
            );
        assert_eq!(ALPHA_MASK_SHADER_TEXTURE_2D, expected);
    }
}
