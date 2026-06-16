use super::super::prepare::PreparedGpuLightUniform;
use crate::scene::{ClippingPlane, SectionBox};

/// Phase 5.4 follow-up: the WGSL fragment shader source moved into
/// a sibling `.wgsl` file so this Rust module stays under doctor's
/// per-module significant-lines budget. The shader still compiles
/// the same — `include_str!` produces a static `&'static str`.
/// Instanced shader contract: `draw.world_from_model * instance_world_from_model * vec4<f32>(in.position, 1.0)`
/// and `let normal_from_model = draw.normal_from_model * instance_normal_from_model`.
pub(super) const GPU_TRIANGLE_SHADER: &str = include_str!("output_shader.wgsl");
pub(super) const GPU_TRIANGLE_SHADER_TEXTURE_2D: &str =
    include_str!("output_shader_texture_2d.wgsl");

pub(super) const MAX_OUTPUT_CLIPPING_PLANES: usize = 6;
pub(super) const OUTPUT_UNIFORM_BYTE_LEN: u64 = 1040;

pub(super) use super::draw_uniform::{
    DRAW_UNIFORM_ENTRY_STRIDE, create_draw_bind_group, create_draw_bind_group_layout,
    create_draw_uniform_buffer, encode_draw_uniform_bytes,
};

pub(super) fn create_output_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.output.bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
            // Phase 1C step 1: env cubemap. Placeholder when unset — gated
            // on environment_diffuse_intensity.w in the fragment shader.
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // Opaque scene color sampled by the transparent transmission pass.
            // This is the minimum real renderer capability for glass proof:
            // transparent fragments can refract/blur the already-rendered
            // background instead of relying on alpha blend alone. Binding 5
            // stays unused so the shader can remain below WebGL2's 16 sampled
            // texture limit while retaining stable group-0 binding numbers.
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

pub(super) fn create_output_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.output.uniform"),
        size: OUTPUT_UNIFORM_BYTE_LEN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn create_output_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    shadow_view: &wgpu::TextureView,
    shadow_sampler: &wgpu::Sampler,
    environment_cubemap_view: &wgpu::TextureView,
    environment_sampler: &wgpu::Sampler,
    transmission_color_view: &wgpu::TextureView,
    transmission_color_sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.output.bind_group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(shadow_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(environment_cubemap_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(environment_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(transmission_color_view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::Sampler(transmission_color_sampler),
            },
        ],
    })
}

pub(super) struct OutputUniformUpload {
    pub(super) exposure_ev: f32,
    pub(super) view_from_world: [f32; 16],
    pub(super) clip_from_view: [f32; 16],
    pub(super) clip_from_world: [f32; 16],
    pub(super) light_from_world: [f32; 16],
    pub(super) camera_position: [f32; 3],
    pub(super) viewport: [f32; 2],
    pub(super) near_far: [f32; 2],
    pub(super) color_management: [f32; 4],
    pub(super) lighting: PreparedGpuLightUniform,
    pub(super) clipping_planes: [[f32; 4]; MAX_OUTPUT_CLIPPING_PLANES],
    pub(super) clipping_control: [f32; 4],
}

pub(super) fn encode_clipping_uniform(
    clipping_planes: &[ClippingPlane],
    section_box: Option<SectionBox>,
) -> ([[f32; 4]; MAX_OUTPUT_CLIPPING_PLANES], [f32; 4]) {
    let mut encoded = [[0.0; 4]; MAX_OUTPUT_CLIPPING_PLANES];
    if let Some(section_box) = section_box {
        for (index, plane) in section_box.planes().into_iter().enumerate() {
            encoded[index] = plane_uniform(plane);
        }
        return (
            encoded,
            [
                MAX_OUTPUT_CLIPPING_PLANES as f32,
                1.0,
                if section_box.inverted() { 1.0 } else { 0.0 },
                0.0,
            ],
        );
    }

    for (index, plane) in clipping_planes
        .iter()
        .take(MAX_OUTPUT_CLIPPING_PLANES)
        .copied()
        .enumerate()
    {
        encoded[index] = plane_uniform(plane);
    }
    (
        encoded,
        [
            clipping_planes.len().min(MAX_OUTPUT_CLIPPING_PLANES) as f32,
            0.0,
            0.0,
            0.0,
        ],
    )
}

pub(super) fn encode_output_uniform(
    upload: OutputUniformUpload,
) -> [u8; OUTPUT_UNIFORM_BYTE_LEN as usize] {
    let exposure_ev = if upload.exposure_ev.is_finite() {
        upload.exposure_ev
    } else {
        0.0
    };
    let mut values = [0.0; 260];
    values[0..16].copy_from_slice(&upload.view_from_world);
    values[16..32].copy_from_slice(&upload.clip_from_view);
    values[32..48].copy_from_slice(&upload.clip_from_world);
    values[48..64].copy_from_slice(&upload.light_from_world);
    values[64] = upload.camera_position[0];
    values[65] = upload.camera_position[1];
    values[66] = upload.camera_position[2];
    values[67] = 2.0_f32.powf(exposure_ev);
    values[68] = upload.viewport[0];
    values[69] = upload.viewport[1];
    values[70] = upload.near_far[0];
    values[71] = upload.near_far[1];
    values[72..76].copy_from_slice(&upload.color_management);
    let mut offset = 76;
    offset = encode_vec4_array(
        &mut values,
        offset,
        &upload.lighting.directional_light_direction_intensity,
    );
    offset = encode_vec4_array(
        &mut values,
        offset,
        &upload.lighting.directional_light_color,
    );
    offset = encode_vec4_array(
        &mut values,
        offset,
        &upload.lighting.directional_shadow_control,
    );
    offset = encode_vec4_array(
        &mut values,
        offset,
        &upload.lighting.point_light_position_intensity,
    );
    offset = encode_vec4_array(
        &mut values,
        offset,
        &upload.lighting.point_light_color_range,
    );
    offset = encode_vec4_array(
        &mut values,
        offset,
        &upload.lighting.spot_light_position_intensity,
    );
    offset = encode_vec4_array(
        &mut values,
        offset,
        &upload.lighting.spot_light_direction_cones,
    );
    offset = encode_vec4_array(&mut values, offset, &upload.lighting.spot_light_cone_range);
    offset = encode_vec4_array(&mut values, offset, &upload.lighting.spot_light_color_range);
    values[offset..offset + 4].copy_from_slice(&upload.lighting.light_counts);
    offset += 4;
    values[offset..offset + 4].copy_from_slice(&upload.lighting.environment_diffuse_intensity);
    offset += 4;
    values[offset..offset + 4].copy_from_slice(&upload.lighting.environment_specular_intensity);
    offset += 4;
    for (index, plane) in upload.clipping_planes.into_iter().enumerate() {
        let plane_offset = offset + index * 4;
        values[plane_offset..plane_offset + 4].copy_from_slice(&plane);
    }
    values[offset + 24..offset + 28].copy_from_slice(&upload.clipping_control);
    let mut bytes = [0; OUTPUT_UNIFORM_BYTE_LEN as usize];
    for (index, value) in values.into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

fn encode_vec4_array<const N: usize>(
    values: &mut [f32],
    offset: usize,
    array: &[[f32; 4]; N],
) -> usize {
    let mut offset = offset;
    for value in array {
        values[offset..offset + 4].copy_from_slice(value);
        offset += 4;
    }
    offset
}

fn plane_uniform(plane: ClippingPlane) -> [f32; 4] {
    [
        plane.normal().x,
        plane.normal().y,
        plane.normal().z,
        plane.distance(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_uniform_buffer_matches_wgsl_uniform_layout() {
        assert_eq!(
            OUTPUT_UNIFORM_BYTE_LEN, 1040,
            "CameraUniform stores view, projection, and view-projection matrices plus \
             camera/exposure, viewport/depth, color-management, punctual-light arrays, \
             directional-shadow-control, environment, and six clipping-plane uniforms — per-draw model + normal matrices live on the new \
             DrawUniform bind group at @group(2)"
        );
        let (clipping_planes, clipping_control) = encode_clipping_uniform(&[], None);
        assert_eq!(
            encode_output_uniform(OutputUniformUpload {
                exposure_ev: 0.0,
                view_from_world: identity_clip_from_world(),
                clip_from_view: identity_clip_from_world(),
                clip_from_world: identity_clip_from_world(),
                light_from_world: identity_clip_from_world(),
                camera_position: [0.0, 0.0, 2.0],
                viewport: [128.0, 64.0],
                near_far: [0.1, 1000.0],
                color_management: [1.0, 0.0, 0.0, 0.0],
                lighting: PreparedGpuLightUniform::default(),
                clipping_planes,
                clipping_control,
            })
            .len(),
            OUTPUT_UNIFORM_BYTE_LEN as usize
        );
    }

    #[test]
    fn triangle_shader_contains_khronos_pbr_neutral_tonemapper() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("pbr_neutral_tonemap")
                && GPU_TRIANGLE_SHADER.contains("start_compression")
                && GPU_TRIANGLE_SHADER.contains("desaturation")
                && GPU_TRIANGLE_SHADER.contains("color_management_mode > 1.5"),
            "native/WebGPU shader must expose the Khronos PBR Neutral tone-mapping branch; \
             WaterBottle screenshots must not be tuned through private color constants"
        );
    }

    #[test]
    fn triangle_shader_uses_camera_projection_uniform() {
        let raw_clip_space_assignment =
            ["out.position = vec4<f32>(in.position", ", 1.0);"].join("");
        assert!(
            GPU_TRIANGLE_SHADER.contains("clip_from_world")
                && GPU_TRIANGLE_SHADER.contains("world_from_model")
                && GPU_TRIANGLE_SHADER.contains("normal_from_model")
                && GPU_TRIANGLE_SHADER.contains("view_from_world")
                && GPU_TRIANGLE_SHADER.contains("clip_from_view")
                && GPU_TRIANGLE_SHADER.contains("camera_position_exposure")
                && GPU_TRIANGLE_SHADER.contains("viewport_near_far")
                && GPU_TRIANGLE_SHADER.contains("color_management"),
            "GPU shader uniform must expose model, normal, view, projection, view-projection, \
             camera position, viewport/depth, and color-management metadata"
        );
        assert!(
            !GPU_TRIANGLE_SHADER.contains(&raw_clip_space_assignment),
            "GPU vertex shader must not treat world-space positions as clip-space coordinates"
        );
        assert!(
            GPU_TRIANGLE_SHADER.contains("@location(2) normal: vec3<f32>")
                && GPU_TRIANGLE_SHADER.contains("@location(3) tex_coord0: vec2<f32>")
                && GPU_TRIANGLE_SHADER.contains("base_color_uv_offset_scale")
                && GPU_TRIANGLE_SHADER.contains("base_color_uv_rotation")
                && GPU_TRIANGLE_SHADER.contains(
                    "textureSample(base_color_texture, base_color_sampler, transformed_uv, material_layer)"
                ),
            "GPU shader must receive normals + TEXCOORD_0 from prepared vertex data and \
             route base-color sampling through the material layer index for array batching"
        );
    }

    #[test]
    fn triangle_shader_declares_material_texture_bindings() {
        // Plan line 778 / RFC 866 commit 2: every material texture role binds
        // a `texture_2d_array<f32>` so the same WGSL pipeline serves the
        // per-material 1-layer fall-back and the batched N-layer path. The
        // MaterialUniform carries `material_layer_index` so sampling can
        // route into the correct layer.
        assert!(
            GPU_TRIANGLE_SHADER.contains("@group(1) @binding(0)")
                && GPU_TRIANGLE_SHADER.contains("var base_color_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(1)")
                && GPU_TRIANGLE_SHADER.contains("var base_color_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(2)")
                && GPU_TRIANGLE_SHADER.contains("var<uniform> material: MaterialUniform")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(3)")
                && GPU_TRIANGLE_SHADER.contains("var normal_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(4)")
                && GPU_TRIANGLE_SHADER.contains("var normal_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(5)")
                && GPU_TRIANGLE_SHADER.contains("var metallic_roughness_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(6)")
                && GPU_TRIANGLE_SHADER
                    .contains("var metallic_roughness_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(7)")
                && GPU_TRIANGLE_SHADER.contains("var occlusion_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(8)")
                && GPU_TRIANGLE_SHADER.contains("var occlusion_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(9)")
                && GPU_TRIANGLE_SHADER.contains("var emissive_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(10)")
                && GPU_TRIANGLE_SHADER.contains("var emissive_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(11)")
                && GPU_TRIANGLE_SHADER.contains("var clearcoat_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(12)")
                && GPU_TRIANGLE_SHADER.contains("var clearcoat_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(13)")
                && GPU_TRIANGLE_SHADER.contains("var clearcoat_roughness_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(14)")
                && GPU_TRIANGLE_SHADER
                    .contains("var clearcoat_roughness_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(15)")
                && GPU_TRIANGLE_SHADER.contains("var clearcoat_normal_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(16)")
                && GPU_TRIANGLE_SHADER
                    .contains("var clearcoat_normal_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(17)")
                && GPU_TRIANGLE_SHADER.contains("var sheen_color_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(18)")
                && GPU_TRIANGLE_SHADER.contains("var sheen_color_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(19)")
                && GPU_TRIANGLE_SHADER.contains("var sheen_roughness_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(20)")
                && GPU_TRIANGLE_SHADER
                    .contains("var sheen_roughness_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(21)")
                && GPU_TRIANGLE_SHADER.contains("var anisotropy_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(22)")
                && GPU_TRIANGLE_SHADER.contains("var anisotropy_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(23)")
                && GPU_TRIANGLE_SHADER.contains("var iridescence_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(24)")
                && GPU_TRIANGLE_SHADER.contains("var iridescence_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(25)")
                && GPU_TRIANGLE_SHADER.contains("var iridescence_thickness_sampler: sampler")
                && GPU_TRIANGLE_SHADER.contains("@group(1) @binding(26)")
                && GPU_TRIANGLE_SHADER
                    .contains("var iridescence_thickness_texture: texture_2d_array<f32>")
                && GPU_TRIANGLE_SHADER.contains("material_layer_index: vec4<u32>")
                && GPU_TRIANGLE_SHADER.contains("textureSample(base_color_texture"),
            "GPU fragment shader must expose material texture bindings as 2D-array views \
             with material_layer_index so per-material and array-batched paths share one shader"
        );
    }

    #[test]
    fn triangle_shader_texture_2d_variant_declares_webgl2_material_bindings() {
        assert!(
            GPU_TRIANGLE_SHADER_TEXTURE_2D.contains(
                "var base_color_texture: texture_2d<f32>"
            ) && GPU_TRIANGLE_SHADER_TEXTURE_2D.contains("var normal_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var metallic_roughness_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var clearcoat_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var clearcoat_roughness_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var clearcoat_normal_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var sheen_color_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var sheen_roughness_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var anisotropy_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var iridescence_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("var iridescence_thickness_texture: texture_2d<f32>")
                && GPU_TRIANGLE_SHADER_TEXTURE_2D.contains(
                    "let base_color_sample = textureSample(base_color_texture, base_color_sampler, transformed_uv)"
                )
                && !GPU_TRIANGLE_SHADER_TEXTURE_2D
                    .contains("textureSample(base_color_texture, base_color_sampler, transformed_uv, material_layer)"),
            "WebGL2 uses a texture_2d material shader variant because wgpu 29's GL backend \
             samples material texture arrays as black in Chromium WebGL2"
        );
    }

    #[test]
    fn triangle_shader_samples_all_material_texture_roles() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("textureSample(base_color_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(normal_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(metallic_roughness_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(occlusion_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(emissive_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(clearcoat_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(clearcoat_roughness_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(clearcoat_normal_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(sheen_color_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(sheen_roughness_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(anisotropy_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(iridescence_texture")
                && GPU_TRIANGLE_SHADER.contains("textureSample(iridescence_thickness_texture")
                && GPU_TRIANGLE_SHADER.contains("base_color_factor")
                && GPU_TRIANGLE_SHADER.contains("emissive_strength")
                && GPU_TRIANGLE_SHADER.contains("clearcoat_factors")
                && GPU_TRIANGLE_SHADER.contains("sheen_factors")
                && GPU_TRIANGLE_SHADER.contains("anisotropy_factors")
                && GPU_TRIANGLE_SHADER.contains("iridescence_factors")
                && GPU_TRIANGLE_SHADER.contains("metallic_roughness_alpha"),
            "GPU material shader must sample every prepared glTF material texture role and \
             consume material factor uniforms before backend material parity can be claimed"
        );
    }

    #[test]
    fn triangle_shader_applies_clearcoat_lobe_in_native_and_webgl2_variants() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("clearcoat_light_contribution")
                    && shader.contains("let clearcoat_factor = clamp(material.clearcoat_factors.x * clearcoat_sample.r, 0.0, 1.0);")
                    && shader.contains("let clearcoat_roughness = clamp(material.clearcoat_factors.y * clearcoat_roughness_sample.g, 0.04, 1.0);")
                    && shader.contains("let clearcoat_normal_scale = material.clearcoat_factors.z;")
                    && shader.contains("shaded += clearcoat_light_contribution(clearcoat_normal, view, incoming, radiance, clearcoat_factor, clearcoat_roughness);"),
                "{name} shader must apply KHR_materials_clearcoat factors plus clearcoat, roughness, and normal texture channels instead of silently dropping them"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_sheen_lobe_in_native_and_webgl2_variants() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("sheen_light_contribution")
                    && shader.contains("let sheen_color = material.sheen_factors.rgb * sheen_color_sample.rgb;")
                    && shader.contains("let sheen_roughness = clamp(material.sheen_factors.a * sheen_roughness_sample.a, 0.04, 1.0);")
                    && shader.contains("shaded += sheen_light_contribution(normal, view, incoming, radiance, sheen_color, sheen_roughness);"),
                "{name} shader must apply KHR_materials_sheen color and roughness texture channels instead of silently dropping them"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_sheen_environment_lobe_in_native_and_webgl2_variants() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("sheen_environment_lighting")
                    && shader.contains("environment += sheen_environment_lighting(normal, view, sheen_color, sheen_roughness);"),
                "{name} shader must apply KHR_materials_sheen under environment lighting; \
                 direct-light-only sheen cannot prove satin/fabric material presets"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_scene_color_transmission_in_native_and_webgl2_variants() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("var transmission_color_texture: texture_2d<f32>")
                    && shader.contains("var transmission_color_sampler: sampler")
                    && shader.contains("transmission_factors: vec4<f32>")
                    && shader.contains("physical_transmission_color(")
                    && shader.contains("let ior = max(material.transmission_factors.y, 1.01);")
                    && shader.contains("let thickness = max(material.transmission_factors.z, 0.0);")
                    && shader.contains("let view_dir = normalize(view);")
                    && shader.contains("let normal_dir = normalize(normal);")
                    && shader.contains("let refracted = refract(-view_dir, normal_dir, 1.0 / ior);")
                    && shader.contains("let thickness_scale = 0.004 + min(thickness, 1.0) * 0.028;")
                    && shader.contains("let blur_px = roughness * roughness * 12.0;")
                    && shader.contains("let refracted_blurred =")
                    && shader.contains("let rim_fresnel = pow(1.0 - n_dot_v, 5.0);")
                    && shader.contains("let reflection_weight = clamp(0.08 + rim_fresnel * 0.42 + (1.0 - transmission) * 0.10, 0.08, 0.50);")
                    && shader.contains("let tint_strength = clamp(transmission * 0.035, 0.0, 0.035);")
                    && shader.contains("return vec4<f32>(mix(transmitted, reflected, reflection_weight), 1.0);")
                    && shader.contains(
                        "textureSample(transmission_color_texture, transmission_color_sampler"
                    ),
                "{name} shader must sample opaque scene color with IOR/thickness refraction \
                 and roughness blur; alpha-blend-only glass is not enough for Round E material proof"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_anisotropy_lobe_in_native_and_webgl2_variants() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("anisotropy_light_contribution")
                    && shader.contains("let anisotropy_direction = anisotropy_sample.rg * 2.0 - vec2<f32>(1.0, 1.0);")
                    && shader.contains("let anisotropy_strength = clamp(material.anisotropy_factors.x * anisotropy_sample.b, 0.0, 1.0);")
                    && shader.contains("material.anisotropy_factors.y")
                    && shader.contains("world_tangent")
                    && shader.contains("tangent_handedness")
                    && shader.contains("shaded += anisotropy_light_contribution(base, metallic, roughness, normal, world_tangent, tangent_handedness, view, incoming, radiance, anisotropy_strength, anisotropy_rotation, anisotropy_direction);"),
                "{name} shader must apply KHR_materials_anisotropy direction, strength, and rotation instead of silently dropping them"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_iridescence_lobe_in_native_and_webgl2_variants() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("iridescence_light_contribution")
                    && shader.contains("let iridescence_factor = clamp(material.iridescence_factors.x * iridescence_sample.r, 0.0, 1.0);")
                    && shader.contains("let iridescence_thickness = mix(material.iridescence_factors.z, material.iridescence_factors.w, clamp(iridescence_thickness_sample.g, 0.0, 1.0));")
                    && shader.contains("material.iridescence_factors.y")
                    && shader.contains("shaded += iridescence_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, iridescence_factor, iridescence_ior, iridescence_thickness);"),
                "{name} shader must apply KHR_materials_iridescence factor, IOR, and thickness texture channels instead of silently dropping them"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_dispersion_lobe_in_native_and_webgl2_variants() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("dispersion_light_contribution")
                    && shader.contains("dispersion_factors: vec4<f32>")
                    && shader.contains("let dispersion_factor = max(material.dispersion_factors.x, 0.0);")
                    && shader.contains("material.dispersion_factors.y")
                    && shader.contains("shaded += dispersion_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, dispersion_factor, dispersion_ior);"),
                "{name} shader must apply KHR_materials_dispersion factor and IOR spread instead of silently dropping them"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_occlusion_strength_to_lit_pbr_output() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains(
                    "let occlusion_applied = mix(1.0, occlusion_sample, occlusion_strength)"
                ) && shader.contains("shaded_rgb = (direct + environment) * occlusion_applied;")
                    && !shader.contains("shaded_rgb = (direct + environment) * occlusion_sample;"),
                "{name} shader must apply glTF occlusionTexture.strength in the lit PBR branch"
            );
        }
    }

    #[test]
    fn triangle_shader_discards_alpha_masked_fragments() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("material.metallic_roughness_alpha.z > 0.0")
                && GPU_TRIANGLE_SHADER.contains("base.a < material.metallic_roughness_alpha.z")
                && GPU_TRIANGLE_SHADER.contains("discard;"),
            "GPU material shader must apply alpha-mask cutoff after base-color texture sampling"
        );
    }

    #[test]
    fn triangle_shader_consumes_gpu_punctual_light_uniforms() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("struct LightingUniform")
                && GPU_TRIANGLE_SHADER.contains("directional_light_direction_intensity")
                && GPU_TRIANGLE_SHADER.contains("point_light_position_intensity")
                && GPU_TRIANGLE_SHADER.contains("spot_light_direction_cones")
                && GPU_TRIANGLE_SHADER.contains("pbr_light_contribution")
                && GPU_TRIANGLE_SHADER.contains("fresnel_schlick")
                && GPU_TRIANGLE_SHADER.contains("distribution_ggx")
                && GPU_TRIANGLE_SHADER.contains("geometry_smith"),
            "GPU PBR shader must consume prepared directional, point, and spot light uniforms \
             through a GGX/Smith/Schlick BRDF before backend PBR lighting can be claimed"
        );
    }

    #[test]
    fn triangle_shader_avoids_pow_in_hot_pbr_fragment_paths() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("fn pow4(value: f32) -> f32")
                    && shader.contains("fn pow5(value: f32) -> f32")
                    && shader.contains("pow5(1.0 - clamp(cos_theta")
                    && shader.contains("pow4(distance / range)")
                    && !shader.contains("pow(1.0 - clamp(cos_theta")
                    && !shader.contains("pow(distance / range"),
                "{name} shader must use multiply-chain pow4/pow5 helpers in the PBR hot path \
                 because WebGL2 drivers can lower generic pow() expensively"
            );
        }
    }

    #[test]
    fn triangle_shader_consumes_gpu_environment_light_uniforms() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("environment_diffuse_intensity")
                && GPU_TRIANGLE_SHADER.contains("environment_specular_intensity")
                && GPU_TRIANGLE_SHADER.contains("has_environment_light")
                && GPU_TRIANGLE_SHADER.contains("pbr_environment_lighting"),
            "GPU PBR shader must consume prepared environment irradiance/specular uniforms \
             before backend IBL lighting can be claimed"
        );
    }

    #[test]
    fn triangle_shader_applies_clearcoat_lobe_to_environment_ibl() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("fn clearcoat_environment_lighting(")
                    && shader.contains(
                        "environment += clearcoat_environment_lighting(clearcoat_normal, view, clearcoat_factor, clearcoat_roughness);"
                    ),
                "{name} shader must add a separate clearcoat specular IBL lobe; \
                 direct-light clearcoat alone makes coated materials read like ordinary glossy plastic under HDR"
            );
        }
    }

    #[test]
    fn triangle_shader_applies_anisotropy_lobe_to_environment_ibl() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains("fn anisotropy_environment_lighting(")
                    && shader.contains(
                        "environment += anisotropy_environment_lighting(base.rgb, metallic, roughness, normal, world_tangent, in.tangent.w, view, anisotropy_strength, material.anisotropy_factors.y, anisotropy_direction);"
                    ),
                "{name} shader must route anisotropy into environment IBL; \
                 direct-light-only anisotropy leaves brushed metal with round HDR highlights"
            );
        }
    }

    #[test]
    fn triangle_shader_uses_prepared_irradiance_for_diffuse_ibl() {
        for (name, shader) in [
            ("texture_2d_array", GPU_TRIANGLE_SHADER),
            ("texture_2d", GPU_TRIANGLE_SHADER_TEXTURE_2D),
        ] {
            assert!(
                shader.contains(
                    "let diffuse_irradiance = camera.lighting.environment_diffuse_intensity.rgb"
                ) && shader.contains(
                    "diffuse_energy * base * diffuse_irradiance * camera.lighting.environment_diffuse_intensity.w"
                ) && !shader.contains(
                    "textureSampleLevel(environment_cubemap, environment_sampler, normal, 0.0).rgb"
                ),
                "{name} shader must use prepared diffuse irradiance for diffuse IBL; \
                 sampling raw HDR radiance in the surface-normal direction makes real HDR \
                 environments dark and high-contrast"
            );
        }
    }

    #[test]
    fn triangle_shader_samples_directional_shadow_map_in_fragment() {
        // Phase 1B step 2: the GPU pipeline sources shadow attenuation from a
        // hardware depth-comparison sample of the directional light's depth
        // map (`shadow_map` + `shadow_sampler` bindings, projected through
        // `camera.light_from_world`), not from a CPU-baked per-vertex
        // attribute. The fragment shader multiplies directional radiance by
        // the per-fragment GPU shadow factor.
        assert!(
            GPU_TRIANGLE_SHADER.contains("textureSampleCompareLevel(shadow_map, shadow_sampler"),
            "GPU PBR lighting must sample the hardware-comparison shadow_map texture \
             with shadow_sampler so opt-in shadowed directional lights project real depth"
        );
        assert!(
            GPU_TRIANGLE_SHADER.contains("camera.light_from_world * vec4<f32>(world_position"),
            "GPU shadow path must reproject world position through camera.light_from_world \
             so the shadow lookup is in light-clip space, not world space"
        );
        assert!(
            GPU_TRIANGLE_SHADER.contains("* gpu_shadow"),
            "GPU PBR fragment must scale directional radiance by the GPU-sampled shadow factor \
             instead of multiplying by the (now retired) CPU shadow_visibility attribute"
        );
        assert!(
            GPU_TRIANGLE_SHADER.contains("directional_shadow_control[i].x > 0.5")
                && GPU_TRIANGLE_SHADER.contains("let gpu_shadow = select(")
                && GPU_TRIANGLE_SHADER.contains("directional_shadow_factor(world_position)")
                && GPU_TRIANGLE_SHADER.contains("* gpu_shadow"),
            "GPU PBR fragment must sample the directional shadow map only when a \
             shadow-casting directional light was prepared; non-shadowed lights must not \
             be multiplied by a placeholder shadow texture"
        );
    }

    #[test]
    fn triangle_shader_builds_tangent_space_normal_from_normal_map() {
        assert!(
            GPU_TRIANGLE_SHADER.contains("@location(4) tangent: vec4<f32>")
                && GPU_TRIANGLE_SHADER.contains("let normal_texture_sample = textureSample(normal_texture")
                && !GPU_TRIANGLE_SHADER.contains("let normal_sample = textureSample(normal_texture")
                && GPU_TRIANGLE_SHADER.contains(
                    "let bitangent = normalize(cross(world_normal, world_tangent) * in.tangent.w);"
                )
                && GPU_TRIANGLE_SHADER.contains(
                    "normal_sample.x * world_tangent + normal_sample.y * bitangent + normal_sample.z * world_normal",
                ),
            "GPU normal mapping must use a prepared tangent basis instead of treating the \
             normal texture as a scalar visibility multiplier"
        );
    }

    fn identity_clip_from_world() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ]
    }
}
