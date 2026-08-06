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
            "encode_retained_vertices(retained_primitives, retained_instances)",
            "encode_draw_resources(",
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
        "src/render/gpu/resource_encoding.rs",
        &[
            "encode_vertices_iter(",
            ".chain(retained_instance_primitives)",
            "vertices::encode_draw_batches_indexed_with_semantics(",
            "let (draw_uniforms, draw_uniform_index_metrics) = interner.finish()",
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
        "src/render/gpu/draw_surface.rs",
        &[
            "pub(in crate::render) fn render_to_surface",
            "surface_frame::acquire_surface_frame",
            "encode_shadow_caster_pass",
            "encode_scene_color_passes",
            "surface_output.present();",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/scene_color.rs",
        &["encode_unlit_pass", "ColorLoad::Load", "TransparentOnly"],
    );
    for path in [
        "src/render/gpu/pipeline.rs",
        "src/render/gpu/depth.rs",
        "src/render/gpu/shadow.rs",
    ] {
        forbid_contains(
            root,
            findings,
            "ARCH-RENDER-TRUTH",
            path,
            &["batch.start_instance..batch.start_instance.saturating_add(batch.instance_count)"],
        );
    }
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
            "environment_prefilter_mip",
            "ENVIRONMENT_PREFILTER_MAX_MIP",
            "fresnel_schlick",
            "distribution_ggx",
            "brdf_specular_ggx",
            // The shader reads the baked split-sum table now, not the analytic
            // fit it used to call. The binding itself is pinned against
            // pbr_brdf.wgsl, which declares it.
            "split_sum_brdf_table",
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
            "instance_normal_0",
            "@location(14) instance_tint",
            "let normal_from_model = draw.normal_from_model * instance_normal_from_model",
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
            "environment_prefilter_mip",
            "ENVIRONMENT_PREFILTER_MAX_MIP",
            "textureSample(base_color_texture, base_color_sampler, transformed_uv)",
            "instance_normal_0",
            "@location(14) instance_tint",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/area_ltc.wgsl",
        &["for (var i = 0u; i < vertex_count; i = i + 1u)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/area_ltc.wgsl",
        &[
            "clipped.vertices[0] = ltc_safe_normalize",
            "clipped.vertices[4] = ltc_safe_normalize",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        &["for (var i = 0u; i < MAX_GPU_AREA_LIGHTS; i = i + 1u)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/output_shader_texture_2d.wgsl",
        &[
            "fn ltc_accumulate_area_light(",
            "ltc_accumulate_area_light(0u,",
            "ltc_accumulate_area_light(1u,",
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
            "create_shader_module(device, variant",
            "MaterialTextureBindingMode::Texture2d => ShaderVariantId::TriangleTexture2d",
            "ShaderVariantId::TriangleTexture2dArray",
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
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/material_bindings.rs",
        &[
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
    for path in [
        "src/render/gpu/output_shader.wgsl",
        "src/render/gpu/output_shader_texture_2d.wgsl",
    ] {
        forbid_contains(
            root,
            findings,
            "ARCH-RENDER-TRUTH",
            path,
            &["transpose(inverse(instance_world_from_model))"],
        );
    }
}
