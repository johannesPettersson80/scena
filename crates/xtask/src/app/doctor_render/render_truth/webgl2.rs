use crate::app::prelude::*;

pub(crate) fn check_renderer_truth_webgl2_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/build.rs",
        &[
            "Backend::WebGl2 => wgpu::Backends::GL",
            "wgpu::Limits::downlevel_webgl2_defaults()",
            "wgpu::SurfaceTarget::Canvas",
            "raw_window_handle::WebDisplayHandle::new()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/prepare_resources.rs",
        &[
            "let vertex_bytes = encode_vertices(primitives)",
            "encode_draw_batches(primitives)",
            "create_output_bind_group_layout",
            "create_material_bind_group_layout",
            "create_unlit_pipeline",
            "self.release_prepared_resources();",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu.rs",
        &[
            "MaterialTextureBindingMode::Texture2d",
            "MaterialTextureBindingMode::Texture2dArray",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/draw.rs",
        &[
            "pub(in crate::render) fn render_to_surface",
            "surface.surface.get_current_texture()",
            "encode_shadow_caster_pass",
            "encode_unlit_pass",
            "surface_output.present();",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output_shader.wgsl",
        &[
            "world_from_model",
            "normal_from_model",
            "view_from_world",
            "clip_from_view",
            "clip_from_world",
            "camera_position_exposure",
            "viewport_near_far",
            "color_management",
            "directional_light_direction_intensity",
            "point_light_position_intensity",
            "spot_light_direction_cones",
            "environment_diffuse_intensity",
            "environment_specular_intensity",
            "pbr_light_contribution",
            "pbr_environment_lighting",
            "fresnel_schlick",
            "distribution_ggx",
            "geometry_smith",
            "base_color_uv_offset_scale",
            "base_color_uv_rotation",
            "var base_color_texture: texture_2d_array<f32>",
            "@location(2) normal: vec3<f32>",
            "@location(4) tangent: vec4<f32>",
            "@location(3) tex_coord0: vec2<f32>",
            "in.tangent.w",
            "normal_sample.x * world_tangent + normal_sample.y * bitangent + normal_sample.z * world_normal",
            "textureSample(base_color_texture, base_color_sampler, transformed_uv, material_layer)",
            "base.a < material.metallic_roughness_alpha.z",
            "discard;",
            "camera.clip_from_world * world_position",
            "draw.normal_from_model * vec4<f32>(in.normal, 0.0)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        &[
            "var base_color_texture: texture_2d<f32>",
            "var normal_texture: texture_2d<f32>",
            "var metallic_roughness_texture: texture_2d<f32>",
            "var occlusion_texture: texture_2d<f32>",
            "var emissive_texture: texture_2d<f32>",
            "textureSample(base_color_texture, base_color_sampler, transformed_uv)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output.rs",
        &[
            "GPU_TRIANGLE_SHADER_TEXTURE_2D",
            "triangle_shader_texture_2d_variant_declares_webgl2_material_bindings",
            "triangle_shader_discards_alpha_masked_fragments",
            "triangle_shader_consumes_gpu_punctual_light_uniforms",
            "triangle_shader_consumes_gpu_environment_light_uniforms",
            "triangle_shader_builds_tangent_space_normal_from_normal_map",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/pipeline.rs",
        &[
            "device.create_shader_module",
            "MaterialTextureBindingMode::Texture2d => GPU_TRIANGLE_SHADER_TEXTURE_2D",
            "MaterialTextureBindingMode::Texture2dArray => GPU_TRIANGLE_SHADER",
            "wgpu::ShaderSource::Wgsl(shader_source.into())",
            "pass.set_bind_group(0, inputs.output_bind_group, &[])",
            "pass.set_bind_group(1, &material.bind_group, &[0])",
            "pass.set_bind_group(2, inputs.draw_bind_group, &[draw_offset])",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/materials.rs",
        &[
            "create_material_resources",
            "create_material_bind_group",
            "MaterialTextureUpload::from_base_color_texture",
            "upload.sampler.wrap_s()",
            "upload.sampler.wrap_t()",
            "address_mode(upload.sampler.wrap_s())",
            "filter_mode(upload.sampler.min_filter())",
            "queue.write_texture",
            "Self::Texture2d => wgpu::TextureViewDimension::D2",
            "TextureViewDimension::D2Array",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output.rs",
        &["out.position = vec4<f32>(in.position, 1.0);"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/depth.rs",
        &["return vec4<f32>(in.position, 1.0);"],
    );
}
