#[test]
fn post_chain_preserves_scene_linear_hdr_until_final_display_transform() {
    let resources_source = include_str!("resources.rs");
    let browser_draw_source = include_str!("../draw_surface.rs");
    let scene_shader = include_str!("../output_shader.wgsl");
    let depth_source = include_str!("../depth.rs");
    let bloom_shader = include_str!("bloom.wgsl");
    let blit_shader = include_str!("blit.wgsl");
    let ssao_shader = include_str!("ssao.wgsl");
    assert!(
        resources_source.contains("TextureFormat::Rgba16Float")
            && resources_source.contains("TextureSampleType::Float { filterable: false }")
            && depth_source.contains("DEPTH_COLOR_FORMAT")
            && depth_source.contains("color_view")
            && resources_source.contains("scene_pipelines")
            && resources_source.contains("readback_blit_pipeline")
            && browser_draw_source.contains("encode_blit_to_view")
            && browser_draw_source.contains("encode_texture_readback_copy"),
        "GPU post chain must preserve scene-linear HDR in floating-point attachments, expose a depth-color SSAO mechanism, and tonemap HDR readback into an RGBA8 target before copying bytes"
    );
    assert!(
        ssao_shader.contains("var depth_texture: texture_2d<f32>")
            && ssao_shader.contains("textureLoad(depth_texture")
            && !ssao_shader.contains("texture_depth_2d"),
        "SSAO must sample the packed depth-color target so the shared depth path compiles on WebGL2"
    );
    assert!(
        bloom_shader.contains("array<vec2<i32>, 9>")
            && !bloom_shader.contains("min(source.rgb + bloom, vec3<f32>(1.0))")
            && blit_shader.contains("apply_tonemapper")
            && blit_shader.contains("hdr.rgb * post.config.x")
            && blit_shader.contains("ordered_dither_4x4")
            && blit_shader.contains("apply_srgb8_dither")
            && scene_shader.contains("var output_rgb = shaded.rgb")
            && scene_shader.contains("camera.color_management.y >= -0.5")
            && ssao_shader.contains("array<vec2<i32>, 8>")
            && !bloom_shader.contains("for (var dy = -12")
            && !ssao_shader.contains("for (var dy = -12"),
        "GPU post must preserve highlights above 1.0 until the final tonemapper while keeping bounded kernels"
    );
}

#[test]
fn automatic_exposure_forces_the_scene_linear_hdr_path() {
    use crate::render::AntiAliasing;

    let ordinary =
        super::GpuOutputPlan::new(AntiAliasing::None, false, false, false, false, false, false);
    let metered =
        super::GpuOutputPlan::new(AntiAliasing::None, false, false, false, false, true, false);

    assert!(!ordinary.post_enabled());
    assert!(
        metered.post_enabled(),
        "automatic exposure must render through the unexposed floating-point scene target"
    );
}
