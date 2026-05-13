use crate::app::prelude::*;

pub(crate) fn check_renderer_truth_material_depth_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/geometry.rs",
        &[
            "pub(crate) struct PrimitiveVertexAttributes",
            "pub(crate) normal: Vec3",
            "pub(crate) tex_coord0: [f32; 2]",
            "attributes: [PrimitiveVertexAttributes; 3]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/geometry/primitive.rs",
        &[
            "triangle_with_attributes",
            "attributes: [PrimitiveVertexAttributes; 3]",
            "vertex_attributes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/prepare.rs",
        &["accumulate_vertex_tangents", "authored_vertex_tangents"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/prepare/tangents.rs",
        &[
            "accumulate_vertex_tangents",
            "authored_vertex_tangents",
            "triangle_tangent",
            "raw_triangle_tangent_frame",
            "TangentFrame",
            "handedness",
            "fallback_tangent",
            "accumulated_vertex_tangents_resolve_shared_triangle_through_mikktspace",
            "accumulated_vertex_tangents_preserve_mirrored_uv_handedness",
            "authored_vertex_tangents_preserve_handedness_and_orthogonalize",
            "generated_triangle_tangent_follows_texcoord_u_axis",
            "generated_triangle_tangent_falls_back_for_degenerate_uvs",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/vertices.rs",
        &[
            "PrimitiveDrawBatch",
            "encode_draw_batches",
            "primitive.render_material_slot()",
            "VERTEX_BYTE_LEN: usize = 17",
            "shader_location: 2",
            "shader_location: 3",
            "shader_location: 4",
            "shader_location: 5",
            "Float32x4",
            "primitive.vertex_attributes()",
            "attributes.normal.x",
            "attributes.tex_coord0[0]",
            "attributes.tangent_handedness",
            "attributes.tangent.x",
            "attributes.shadow_visibility",
            "gpu_vertex_stream_carries_normals_and_texcoord0",
            "gpu_draw_batches_preserve_prepared_material_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/materials.rs",
        &[
            "MaterialTextureResources",
            "MaterialTextureUpload",
            "MaterialUniformUpload",
            "MATERIAL_UNIFORM_BYTE_LEN",
            "create_material_bind_group_layout",
            "create_material_resources",
            "material_texture_byte_len",
            "Vec<MaterialTextureResources>",
            "binding: 2",
            "NORMAL_BINDINGS",
            "METALLIC_ROUGHNESS_BINDINGS",
            "OCCLUSION_BINDINGS",
            "EMISSIVE_BINDINGS",
            "SamplerBindingType::Filtering",
            "TextureSampleType::Float { filterable: true }",
            "scena.material.base_color",
            "scena.material.normal",
            "scena.material.metallic_roughness",
            "scena.material.occlusion",
            "scena.material.emissive",
            "scena.material.fallback_base_color",
            "scena.material.fallback_bind_group",
            "texture_byte_len",
        ],
    );
    // Plan line 778 commit 2: per-role uploads + sampler/filter helpers live
    // in `material_upload.rs` after the array-batching split.
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/material_upload.rs",
        &[
            "MaterialTextureUpload",
            "from_base_color_texture",
            "from_normal_texture",
            "from_metallic_roughness_texture",
            "from_occlusion_texture",
            "from_emissive_texture",
            "from_linear_texture",
            "decoded_base_color_texture_becomes_backend_upload",
        ],
    );
    // Plan line 778 commit 2: shared `texture_2d_array<f32>` allocation +
    // dynamic-offset bind group lives in `material_batched.rs`.
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/material_batched.rs",
        &[
            "MaterialBatchedResources",
            "create_batched_material_resources",
            "TextureViewDimension::D2Array",
            "scena.material.batched_uniform",
            "scena.material.batched_base_color",
            "depth_or_array_layers: layer_count",
            "with_layer_index",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/material_uniform.rs",
        &[
            "MaterialUniformUpload",
            "MATERIAL_UNIFORM_BYTE_LEN",
            "from_material",
            "from_transform",
            "base_color_factor",
            "emissive_strength",
            "metallic_roughness_alpha",
            "material_uniform_upload_encodes_base_color_texture_transform",
            "material_uniform_upload_encodes_material_factors",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render.rs",
        &[
            "collect_backend_material_slots(scene, assets)",
            "backend_material_handles",
            "backend_sampled_base_color_textures",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/pipeline.rs",
        &[
            "RenderPassDepthStencilAttachment",
            "depth_stencil: depth_compare.map",
            "depth_write_enabled: Some(false)",
            "material_bind_group_layout",
            "material_resources",
            "pass.set_bind_group(1, &material.bind_group",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/render/gpu/depth.rs",
        &[
            "camera.clip_from_view * camera.view_from_world * draw.world_from_model",
            "pub(super) color_compare",
        ],
    );
}
