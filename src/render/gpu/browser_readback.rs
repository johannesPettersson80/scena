#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use super::super::RasterTarget;
use super::instancing::InstanceDrawBatch;
use super::labels::{self, LabelResources};
use super::materials::MaterialResources;
use super::pipeline::MeshPipelineSet;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "browser-probe",
    feature = "scene-host"
))]
use super::pipeline::{BYTES_PER_PIXEL, create_unlit_pipeline_set};
use super::pipeline::{ColorLoad, DrawFilter, UnlitPass, UnlitPipelines, encode_unlit_pass};
use super::stats::GpuResourceStats;
use super::strokes::{self, StrokeResources};
use super::transmission::TransmissionResources;
use super::vertices::PrimitiveDrawBatch;
#[cfg(target_arch = "wasm32")]
pub(super) fn read_webgl2_canvas_rgba8(
    canvas: &web_sys::HtmlCanvasElement,
    target: RasterTarget,
) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast as _;

    let get_context = js_sys::Reflect::get(
        canvas.as_ref(),
        &wasm_bindgen::JsValue::from_str("getContext"),
    )?
    .dyn_into::<js_sys::Function>()?;
    let context = get_context.call1(canvas.as_ref(), &wasm_bindgen::JsValue::from_str("webgl2"))?;
    if context.is_null() || context.is_undefined() {
        return Err(wasm_bindgen::JsValue::from_str(
            "renderer-owned WebGL2 readback could not reacquire the attached context",
        ));
    }
    let rgba = js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("RGBA"))?;
    let unsigned_byte =
        js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("UNSIGNED_BYTE"))?;
    let read_pixels =
        js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("readPixels"))?
            .dyn_into::<js_sys::Function>()?;
    let get_parameter =
        js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("getParameter"))?
            .dyn_into::<js_sys::Function>()?;
    let read_pack_parameter = |name: &str| -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
        let parameter = js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str(name))?;
        get_parameter.call1(&context, &parameter)
    };
    let pack_state_before = format!(
        "alignment={:?},row_length={:?},skip_pixels={:?},skip_rows={:?}",
        read_pack_parameter("PACK_ALIGNMENT")?,
        read_pack_parameter("PACK_ROW_LENGTH")?,
        read_pack_parameter("PACK_SKIP_PIXELS")?,
        read_pack_parameter("PACK_SKIP_ROWS")?,
    );
    // wgpu's WebGL readback implementation may leave its PACK buffer bound.
    // The typed-array `readPixels` overload is invalid while it is bound, and
    // would otherwise report an all-zero frame through a WebGL error.
    let pixel_pack_buffer = js_sys::Reflect::get(
        &context,
        &wasm_bindgen::JsValue::from_str("PIXEL_PACK_BUFFER"),
    )?;
    let bind_buffer =
        js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("bindBuffer"))?
            .dyn_into::<js_sys::Function>()?;
    bind_buffer.call2(&context, &pixel_pack_buffer, &wasm_bindgen::JsValue::NULL)?;
    let pixel_store_i =
        js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("pixelStorei"))?
            .dyn_into::<js_sys::Function>()?;
    for (name, value) in [
        ("PACK_ALIGNMENT", 1_u32),
        ("PACK_ROW_LENGTH", 0),
        ("PACK_SKIP_PIXELS", 0),
        ("PACK_SKIP_ROWS", 0),
    ] {
        let parameter = js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str(name))?;
        pixel_store_i.call2(
            &context,
            &parameter,
            &wasm_bindgen::JsValue::from_f64(f64::from(value)),
        )?;
    }
    let bytes = js_sys::Uint8Array::new_with_length(target.byte_len() as u32);
    let args = js_sys::Array::new();
    args.push(&wasm_bindgen::JsValue::from_f64(0.0));
    args.push(&wasm_bindgen::JsValue::from_f64(0.0));
    args.push(&wasm_bindgen::JsValue::from_f64(f64::from(target.width)));
    args.push(&wasm_bindgen::JsValue::from_f64(f64::from(target.height)));
    args.push(&rgba);
    args.push(&unsigned_byte);
    args.push(bytes.as_ref());
    read_pixels.apply(&context, &args)?;
    let get_error = js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("getError"))?
        .dyn_into::<js_sys::Function>()?;
    let error = get_error.call0(&context)?;
    let no_error = js_sys::Reflect::get(&context, &wasm_bindgen::JsValue::from_str("NO_ERROR"))?;
    if error != no_error {
        return Err(wasm_bindgen::JsValue::from_str(&format!(
            "renderer-owned WebGL2 readPixels failed: error={error:?}, target={}x{}, bytes={}, pack_state_before={pack_state_before}",
            target.width,
            target.height,
            target.byte_len(),
        )));
    }

    let mut bottom_left_rgba8 = vec![0; target.byte_len()];
    bytes.copy_to(&mut bottom_left_rgba8);
    Ok(flip_rgba8_rows_to_top_left(
        bottom_left_rgba8,
        target.width,
        target.height,
    ))
}

#[cfg(target_arch = "wasm32")]
fn flip_rgba8_rows_to_top_left(mut rgba8: Vec<u8>, width: u32, height: u32) -> Vec<u8> {
    let row_bytes = width as usize * 4;
    for top in 0..height as usize / 2 {
        let bottom = height as usize - 1 - top;
        let (prefix, suffix) = rgba8.split_at_mut(bottom * row_bytes);
        prefix[top * row_bytes..(top + 1) * row_bytes].swap_with_slice(&mut suffix[..row_bytes]);
    }
    rgba8
}

#[derive(Debug)]
pub(super) struct BrowserReadbackResources {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) buffer: wgpu::Buffer,
    pub(super) pipelines: Option<MeshPipelineSet>,
    pub(super) format: wgpu::TextureFormat,
    pub(super) padded_bytes_per_row: u32,
    #[allow(dead_code)]
    pub(super) unpadded_bytes_per_row: u32,
}

pub(super) fn resource_stats(
    resources: &BrowserReadbackResources,
    target: RasterTarget,
) -> GpuResourceStats {
    GpuResourceStats {
        buffers: 1,
        textures: 1,
        render_targets: 1,
        pipelines: u64::from(resources.pipelines.is_some()) * 2,
        approximate_gpu_memory_bytes: GpuResourceStats::target_bytes(target, 4, 1)
            + u64::from(resources.padded_bytes_per_row) * u64::from(target.height),
        ..GpuResourceStats::default()
    }
}

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "browser-probe",
    feature = "scene-host"
))]
pub(super) struct BrowserReadbackResourceDescriptor<'a> {
    pub(super) target: RasterTarget,
    pub(super) surface_format: wgpu::TextureFormat,
    pub(super) output_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) material_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) draw_bind_group_layout: &'a wgpu::BindGroupLayout,
    pub(super) triangle_shader: &'a wgpu::ShaderModule,
    pub(super) depth_compare: Option<wgpu::CompareFunction>,
    pub(super) material_features: super::material_uniform::MaterialShaderFeatures,
    pub(super) render_pipeline_required: bool,
    pub(super) surface_pipeline_reusable: bool,
}

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "browser-probe",
    feature = "scene-host"
))]
pub(super) fn create_browser_readback_resources(
    device: &wgpu::Device,
    descriptor: BrowserReadbackResourceDescriptor<'_>,
) -> BrowserReadbackResources {
    let target = descriptor.target;
    let target_format = readback_format_for_surface(descriptor.surface_format);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scena.browser.proof_readback_target"),
        size: wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: target_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let unpadded_bytes_per_row = target.width.saturating_mul(BYTES_PER_PIXEL);
    let padded_bytes_per_row = align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.browser.proof_readback_buffer"),
        size: u64::from(padded_bytes_per_row) * u64::from(target.height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let pipelines = (descriptor.render_pipeline_required && !descriptor.surface_pipeline_reusable)
        .then(|| {
            create_unlit_pipeline_set(
                device,
                descriptor.triangle_shader,
                target_format,
                descriptor.output_bind_group_layout,
                descriptor.material_bind_group_layout,
                descriptor.draw_bind_group_layout,
                descriptor.depth_compare,
                1,
                None,
                descriptor.material_features,
            )
        });
    BrowserReadbackResources {
        texture,
        view,
        buffer,
        pipelines,
        format: target_format,
        padded_bytes_per_row,
        unpadded_bytes_per_row,
    }
}

pub(super) const fn readback_format_for_surface(
    surface_format: wgpu::TextureFormat,
) -> wgpu::TextureFormat {
    match surface_format {
        wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Rgba8UnormSrgb => {
            wgpu::TextureFormat::Rgba8UnormSrgb
        }
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm => {
            wgpu::TextureFormat::Rgba8Unorm
        }
        _ => surface_format,
    }
}

pub(super) struct BrowserReadbackPass<'a> {
    pub(super) target: RasterTarget,
    pub(super) readback: &'a BrowserReadbackResources,
    pub(super) readback_pipelines: UnlitPipelines<'a>,
    pub(super) depth_view: Option<&'a wgpu::TextureView>,
    pub(super) vertex_buffer: &'a wgpu::Buffer,
    pub(super) instance_buffer: &'a wgpu::Buffer,
    pub(super) output_bind_group: &'a wgpu::BindGroup,
    pub(super) opaque_output_bind_group: &'a wgpu::BindGroup,
    pub(super) reflection_probe_output_bind_groups: &'a [wgpu::BindGroup],
    pub(super) reflection_probe_opaque_output_bind_groups: &'a [wgpu::BindGroup],
    pub(super) draw_bind_group: &'a wgpu::BindGroup,
    pub(super) material_resources: &'a MaterialResources,
    pub(super) stroke_resources: Option<&'a StrokeResources>,
    pub(super) label_resources: Option<&'a LabelResources>,
    pub(super) draw_batches: &'a [PrimitiveDrawBatch],
    pub(super) instance_batches: &'a [InstanceDrawBatch],
    pub(super) identity_instance: u32,
    pub(super) transmission: &'a TransmissionResources,
    pub(super) clear_color: wgpu::Color,
    pub(super) draw_submissions: &'a mut u64,
}

pub(super) fn encode_browser_readback_pass(
    encoder: &mut wgpu::CommandEncoder,
    pass: BrowserReadbackPass<'_>,
) {
    let draw_submissions = pass.draw_submissions;
    let readback_pipelines = pass.readback_pipelines;
    if let Some(transmission_pipelines) = pass.transmission.pipelines.as_ref() {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.transmission.view,
                resolve_target: None,
                semantic_view: None,
                semantic_resolve_target: None,
                depth_view: None,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.opaque_output_bind_group,
                reflection_probe_output_bind_groups: pass
                    .reflection_probe_opaque_output_bind_groups,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: transmission_pipelines.refs(),
                color_load: ColorLoad::Clear(pass.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.browser.proof_transmission_scene_color_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    }
    if has_transparent_batches(pass.draw_batches, pass.instance_batches) {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.readback.view,
                resolve_target: None,
                semantic_view: None,
                semantic_resolve_target: None,
                depth_view: pass.depth_view,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.output_bind_group,
                reflection_probe_output_bind_groups: pass.reflection_probe_output_bind_groups,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: readback_pipelines,
                color_load: ColorLoad::Clear(pass.clear_color),
                draw_filter: DrawFilter::OpaqueOnly,
                label: "scena.browser.proof_readback_opaque_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.readback.view,
                resolve_target: None,
                semantic_view: None,
                semantic_resolve_target: None,
                depth_view: pass.depth_view,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.output_bind_group,
                reflection_probe_output_bind_groups: pass.reflection_probe_output_bind_groups,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: readback_pipelines,
                color_load: ColorLoad::Load,
                draw_filter: DrawFilter::TransparentOnly,
                label: "scena.browser.proof_readback_transparent_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    } else {
        encode_unlit_pass(
            encoder,
            UnlitPass {
                view: &pass.readback.view,
                resolve_target: None,
                semantic_view: None,
                semantic_resolve_target: None,
                depth_view: pass.depth_view,
                vertex_buffer: pass.vertex_buffer,
                instance_buffer: pass.instance_buffer,
                output_bind_group: pass.output_bind_group,
                reflection_probe_output_bind_groups: pass.reflection_probe_output_bind_groups,
                draw_bind_group: pass.draw_bind_group,
                material_resources: pass.material_resources,
                draw_batches: pass.draw_batches,
                instance_batches: pass.instance_batches,
                identity_instance: pass.identity_instance,
                pipelines: readback_pipelines,
                color_load: ColorLoad::Clear(pass.clear_color),
                draw_filter: DrawFilter::All,
                label: "scena.browser.proof_readback_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    }
    if let Some(stroke_resources) = pass.stroke_resources {
        strokes::encode_pass(
            encoder,
            strokes::StrokePass {
                view: &pass.readback.view,
                depth_view: None,
                output_bind_group: pass.output_bind_group,
                draw_bind_group: pass.draw_bind_group,
                resources: stroke_resources,
                pipeline: strokes::flat_pipeline(stroke_resources),
                label: "scena.browser.proof_stroke_readback_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    }
    if let Some(label_resources) = pass.label_resources {
        labels::encode_pass(
            encoder,
            labels::LabelPass {
                view: &pass.readback.view,
                depth_view: None,
                output_bind_group: pass.output_bind_group,
                resources: label_resources,
                pipeline: labels::flat_pipeline(label_resources),
                label: "scena.browser.proof_label_readback_pass",
                draw_submissions: &mut *draw_submissions,
            },
        );
    }
    encode_texture_readback_copy(encoder, &pass.readback.texture, pass.readback, pass.target);
}

pub(super) fn encode_texture_readback_copy(
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
    readback: &BrowserReadbackResources,
    target: RasterTarget,
) {
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback.buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(readback.padded_bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: target.width,
            height: target.height,
            depth_or_array_layers: 1,
        },
    );
}

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "browser-probe",
    feature = "scene-host"
))]
fn align_to(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

fn has_transparent_batches(
    draw_batches: &[PrimitiveDrawBatch],
    instance_batches: &[InstanceDrawBatch],
) -> bool {
    draw_batches
        .iter()
        .any(|batch| !batch.depth_prepass_eligible)
        || instance_batches
            .iter()
            .any(|batch| !batch.depth_prepass_eligible)
}

#[cfg(all(
    test,
    any(
        not(target_arch = "wasm32"),
        feature = "browser-probe",
        feature = "scene-host"
    )
))]
#[path = "browser_readback_tests.rs"]
mod tests;

#[cfg(all(
    test,
    any(
        not(target_arch = "wasm32"),
        feature = "browser-probe",
        feature = "scene-host"
    )
))]
#[test]
fn browser_readback_preserves_surface_transfer_with_rgba_byte_order() {
    assert_eq!(
        readback_format_for_surface(wgpu::TextureFormat::Bgra8Unorm),
        wgpu::TextureFormat::Rgba8Unorm
    );
    assert_eq!(
        readback_format_for_surface(wgpu::TextureFormat::Bgra8UnormSrgb),
        wgpu::TextureFormat::Rgba8UnormSrgb
    );
}
