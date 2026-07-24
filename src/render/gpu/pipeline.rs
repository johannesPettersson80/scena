use super::instancing::{INSTANCE_ATTRIBUTES, INSTANCE_BYTE_LEN, InstanceDrawBatch};
use super::material_bindings::MaterialTextureBindingMode;
use super::material_uniform::MATERIAL_UNIFORM_ENTRY_STRIDE;
use super::materials::MaterialResources;
use super::output::DRAW_UNIFORM_ENTRY_STRIDE;
use super::shader_manifest::{ShaderVariantId, create_shader_module};
use super::vertices::{PrimitiveDrawBatch, VERTEX_ATTRIBUTES, VERTEX_BYTE_LEN};

#[cfg_attr(
    all(target_arch = "wasm32", not(feature = "browser-probe")),
    allow(dead_code)
)]
pub(super) const BYTES_PER_PIXEL: u32 = 4;
pub(super) const GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// WGPU evaluates front-face winding before viewport conversion; prepared
/// triangle order that is front-facing in the CPU rasterizer maps to CCW here.
pub(super) const SCENA_FRONT_FACE: wgpu::FrontFace = wgpu::FrontFace::Ccw;

/// Device-owned cache for the large triangle shader source. Pipeline layouts,
/// target formats, culling, depth state, and sample counts remain pipeline
/// dependencies; none of them changes the compiled shader module.
#[derive(Debug, Default)]
pub(super) struct TriangleShaderModuleCache {
    texture_2d: Option<wgpu::ShaderModule>,
    texture_2d_array: Option<wgpu::ShaderModule>,
}

pub(super) struct TriangleShaderModuleLookup {
    pub(super) module: wgpu::ShaderModule,
    pub(super) hit: bool,
}

impl TriangleShaderModuleCache {
    pub(super) fn get_or_create(
        &mut self,
        device: &wgpu::Device,
        texture_binding_mode: MaterialTextureBindingMode,
    ) -> TriangleShaderModuleLookup {
        let slot = match texture_binding_mode {
            MaterialTextureBindingMode::Texture2d => &mut self.texture_2d,
            MaterialTextureBindingMode::Texture2dArray => &mut self.texture_2d_array,
        };
        let hit = slot.is_some();
        let module = slot
            .get_or_insert_with(|| {
                let variant = match texture_binding_mode {
                    MaterialTextureBindingMode::Texture2d => ShaderVariantId::TriangleTexture2d,
                    MaterialTextureBindingMode::Texture2dArray => {
                        ShaderVariantId::TriangleTexture2dArray
                    }
                };
                create_shader_module(device, variant, "scena.m0.unlit_triangle")
            })
            .clone();
        TriangleShaderModuleLookup { module, hit }
    }
}

pub(super) struct UnlitPass<'a> {
    pub(super) view: &'a wgpu::TextureView,
    pub(super) resolve_target: Option<&'a wgpu::TextureView>,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) instance_buffer: &'a wgpu::Buffer,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) material_resources: &'a MaterialResources,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) instance_batches: &'a [InstanceDrawBatch],
    pub(super) identity_instance: u32,
    pub(super) pipelines: UnlitPipelines<'a>,
    pub(super) color_load: ColorLoad,
    pub(super) draw_filter: DrawFilter,
    pub(super) label: &'static str,
    pub(super) draw_submissions: &'a mut u64,
}

#[derive(Debug)]
pub(super) struct MeshPipelineSet {
    single_sided: wgpu::RenderPipeline,
    double_sided: wgpu::RenderPipeline,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UnlitPipelines<'a> {
    single_sided: &'a wgpu::RenderPipeline,
    double_sided: &'a wgpu::RenderPipeline,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ColorLoad {
    Clear(wgpu::Color),
    Load,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DrawFilter {
    All,
    OpaqueOnly,
    TransparentOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DrawSideFilter {
    SingleSided,
    DoubleSided,
}

pub(super) fn encode_unlit_pass(encoder: &mut wgpu::CommandEncoder, inputs: UnlitPass<'_>) {
    let color_attachment = Some(wgpu::RenderPassColorAttachment {
        view: inputs.view,
        depth_slice: None,
        resolve_target: inputs.resolve_target,
        ops: wgpu::Operations {
            load: match inputs.color_load {
                ColorLoad::Clear(color) => wgpu::LoadOp::Clear(color),
                ColorLoad::Load => wgpu::LoadOp::Load,
            },
            store: wgpu::StoreOp::Store,
        },
    });
    let depth_stencil_attachment =
        inputs
            .depth_view
            .map(|view| wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(inputs.label),
        color_attachments: &[color_attachment],
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_bind_group(0, inputs.output_bind_group, &[]);
    pass.set_vertex_buffer(0, inputs.vertex_buffer.slice(..));
    let identity_instance_offset =
        u64::from(inputs.identity_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
    pass.set_vertex_buffer(1, inputs.instance_buffer.slice(identity_instance_offset..));
    for side_filter in [DrawSideFilter::SingleSided, DrawSideFilter::DoubleSided] {
        pass.set_pipeline(inputs.pipelines.for_side(side_filter));
        match inputs.material_resources {
            MaterialResources::PerMaterial(slots) => {
                let Some(fallback_material) = slots.first() else {
                    return;
                };
                for batch in inputs.draw_batches.iter().filter(|batch| {
                    inputs.draw_filter.includes(batch) && side_filter.includes(batch)
                }) {
                    let material = slots
                        .get(batch.material_slot as usize)
                        .unwrap_or(fallback_material);
                    // Plan line 778 commit 2: per-material bind groups always
                    // bind their own uniform buffer at offset 0; the layer
                    // index in MaterialUniform stays at 0 because each material
                    // owns a 1-layer array.
                    pass.set_bind_group(1, &material.bind_group, &[0]);
                    let draw_offset = (batch.draw_uniform_index as u64)
                        .saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE)
                        as u32;
                    pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
                    pass.draw(
                        batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
                        0..1,
                    );
                    *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
                }
                for batch in inputs.instance_batches.iter().filter(|batch| {
                    inputs.draw_filter.includes_instance(batch)
                        && side_filter.includes_instance(batch)
                }) {
                    let material = slots
                        .get(batch.material_slot as usize)
                        .unwrap_or(fallback_material);
                    pass.set_bind_group(1, &material.bind_group, &[0]);
                    let draw_offset = (batch.draw_uniform_index as u64)
                        .saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE)
                        as u32;
                    pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
                    let instance_offset =
                        u64::from(batch.start_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
                    pass.set_vertex_buffer(1, inputs.instance_buffer.slice(instance_offset..));
                    pass.draw(
                        batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
                        0..batch.instance_count,
                    );
                    *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
                }
            }
            MaterialResources::Batched(batched) => {
                // Plan line 778 commit 2: a single bind group reused for every
                // draw; per-draw dynamic offset selects the per-material uniform
                // slot, and `material_layer_index` (encoded in the uniform)
                // selects the array layer for sampling.
                for batch in inputs.draw_batches.iter().filter(|batch| {
                    inputs.draw_filter.includes(batch) && side_filter.includes(batch)
                }) {
                    let layer_index = (batch.material_slot as u64)
                        .min(u64::from(batched.layer_count.saturating_sub(1)));
                    let material_offset =
                        layer_index.saturating_mul(MATERIAL_UNIFORM_ENTRY_STRIDE) as u32;
                    pass.set_bind_group(1, &batched.bind_group, &[material_offset]);
                    let draw_offset = (batch.draw_uniform_index as u64)
                        .saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE)
                        as u32;
                    pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
                    pass.draw(
                        batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
                        0..1,
                    );
                    *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
                }
                for batch in inputs.instance_batches.iter().filter(|batch| {
                    inputs.draw_filter.includes_instance(batch)
                        && side_filter.includes_instance(batch)
                }) {
                    let layer_index = (batch.material_slot as u64)
                        .min(u64::from(batched.layer_count.saturating_sub(1)));
                    let material_offset =
                        layer_index.saturating_mul(MATERIAL_UNIFORM_ENTRY_STRIDE) as u32;
                    pass.set_bind_group(1, &batched.bind_group, &[material_offset]);
                    let draw_offset = (batch.draw_uniform_index as u64)
                        .saturating_mul(DRAW_UNIFORM_ENTRY_STRIDE)
                        as u32;
                    pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset]);
                    let instance_offset =
                        u64::from(batch.start_instance).saturating_mul(INSTANCE_BYTE_LEN as u64);
                    pass.set_vertex_buffer(1, inputs.instance_buffer.slice(instance_offset..));
                    pass.draw(
                        batch.start_vertex..batch.start_vertex.saturating_add(batch.vertex_count),
                        0..batch.instance_count,
                    );
                    *inputs.draw_submissions = inputs.draw_submissions.saturating_add(1);
                }
            }
        }
    }
}

impl MeshPipelineSet {
    pub(super) const fn refs(&self) -> UnlitPipelines<'_> {
        UnlitPipelines {
            single_sided: &self.single_sided,
            double_sided: &self.double_sided,
        }
    }
}

impl<'a> UnlitPipelines<'a> {
    const fn for_side(self, side_filter: DrawSideFilter) -> &'a wgpu::RenderPipeline {
        match side_filter {
            DrawSideFilter::SingleSided => self.single_sided,
            DrawSideFilter::DoubleSided => self.double_sided,
        }
    }
}

impl DrawFilter {
    fn includes(self, batch: &PrimitiveDrawBatch) -> bool {
        match self {
            DrawFilter::All => true,
            DrawFilter::OpaqueOnly => batch.depth_prepass_eligible,
            DrawFilter::TransparentOnly => !batch.depth_prepass_eligible,
        }
    }

    fn includes_instance(self, batch: &InstanceDrawBatch) -> bool {
        match self {
            DrawFilter::All => true,
            DrawFilter::OpaqueOnly => batch.depth_prepass_eligible,
            DrawFilter::TransparentOnly => !batch.depth_prepass_eligible,
        }
    }
}

impl DrawSideFilter {
    pub(super) const fn includes(self, batch: &PrimitiveDrawBatch) -> bool {
        self.includes_double_sided(batch.double_sided)
    }

    pub(super) const fn includes_instance(self, batch: &InstanceDrawBatch) -> bool {
        self.includes_double_sided(batch.double_sided)
    }

    const fn includes_double_sided(self, double_sided: bool) -> bool {
        match self {
            Self::SingleSided => !double_sided,
            Self::DoubleSided => double_sided,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_unlit_pipeline_set(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    sample_count: u32,
) -> MeshPipelineSet {
    MeshPipelineSet {
        single_sided: create_unlit_pipeline(
            device,
            shader,
            format,
            output_bind_group_layout,
            material_bind_group_layout,
            draw_bind_group_layout,
            depth_compare,
            false,
            sample_count,
        ),
        double_sided: create_unlit_pipeline(
            device,
            shader,
            format,
            output_bind_group_layout,
            material_bind_group_layout,
            draw_bind_group_layout,
            depth_compare,
            true,
            sample_count,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn create_unlit_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    output_bind_group_layout: &wgpu::BindGroupLayout,
    material_bind_group_layout: &wgpu::BindGroupLayout,
    draw_bind_group_layout: &wgpu::BindGroupLayout,
    depth_compare: Option<wgpu::CompareFunction>,
    double_sided: bool,
    sample_count: u32,
) -> wgpu::RenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scena.m0.pipeline_layout"),
        bind_group_layouts: &[
            Some(output_bind_group_layout),
            Some(material_bind_group_layout),
            Some(draw_bind_group_layout),
        ],
        immediate_size: 0,
    });
    let vertex_buffer = wgpu::VertexBufferLayout {
        array_stride: VERTEX_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &VERTEX_ATTRIBUTES,
    };
    let instance_buffer = wgpu::VertexBufferLayout {
        array_stride: INSTANCE_BYTE_LEN as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &INSTANCE_ATTRIBUTES,
    };
    let label = if double_sided {
        "scena.m0.unlit_triangle_pipeline.double_sided"
    } else {
        "scena.m0.unlit_triangle_pipeline.single_sided"
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_buffer, instance_buffer],
        },
        primitive: wgpu::PrimitiveState {
            front_face: SCENA_FRONT_FACE,
            cull_mode: (!double_sided).then_some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: depth_compare.map(|depth_compare| wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(depth_compare),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn unlit_pipeline_source_wires_depth_state_into_visible_color_pass() {
        let source = include_str!("pipeline.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("pipeline implementation precedes tests");
        assert!(
            implementation.contains("RenderPassDepthStencilAttachment")
                && implementation.contains("depth_stencil: depth_compare.map"),
            "visible GPU color pass must use the prepared depth buffer when one exists"
        );
    }

    #[test]
    fn depth_prepass_and_color_pass_use_identical_clip_space_transform() {
        // Pi 5 V3D WebGL2 runs the fragment stage at lower-than-highp
        // precision by default. If the depth pre-pass computes clip-space
        // depth via a different matrix multiplication path than the color
        // pass, the two ULP-diverge and the LessEqual depth test rejects
        // most color-pass fragments, producing a mostly-black render on
        // V3D-class hardware while Lavapipe/desktop GL is unaffected.
        // Both shaders must use `clip_from_world * world_position`.
        let depth = include_str!("depth.rs");
        let color = include_str!("output_shader.wgsl");
        let color_tex2d = include_str!("output_shader_texture_2d.wgsl");
        for (label, source) in [
            ("depth.rs", depth),
            ("output_shader.wgsl", color),
            ("output_shader_texture_2d.wgsl", color_tex2d),
        ] {
            assert!(
                source.contains("camera.clip_from_world * world_position"),
                "{label} vs_main must use `camera.clip_from_world * world_position` so depth values match the other passes bit-for-bit",
            );
            assert!(
                !source.contains("camera.clip_from_view * camera.view_from_world * world_position")
                    && !source.contains(
                        "camera.clip_from_view * camera.view_from_world * draw.world_from_model",
                    ),
                "{label} must not reintroduce a divergent clip-space matrix path",
            );
        }
    }

    #[test]
    fn unlit_pipeline_binds_material_group_for_fragment_sampling() {
        let source = include_str!("pipeline.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("pipeline implementation precedes tests");
        assert!(
            implementation.contains("material_bind_group_layout")
                && implementation.contains("material_resources")
                && implementation.contains("pass.set_bind_group(1, &material.bind_group"),
            "visible GPU color pass must bind material resources, not only camera uniforms"
        );
    }

    #[test]
    fn unlit_pipeline_can_split_opaque_and_transparent_draws_for_transmission() {
        let source = include_str!("pipeline.rs");
        let implementation = source
            .split("#[cfg(test)]")
            .next()
            .expect("pipeline implementation precedes tests");
        assert!(
            implementation.contains("enum DrawFilter")
                && implementation.contains("DrawFilter::OpaqueOnly")
                && implementation.contains("DrawFilter::TransparentOnly")
                && implementation.contains("LoadOp::Load")
                && implementation.contains("depth_prepass_eligible"),
            "physical glass needs an opaque scene-color pass followed by a transparent \
             transmission pass; one all-material alpha-blended pass can still ship fake glass"
        );
    }
}
