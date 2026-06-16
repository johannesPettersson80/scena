#[test]
fn post_chain_uses_encoded_rgba8_and_depth_color_contract() {
    let source = include_str!("mod.rs");
    let depth_source = include_str!("../depth.rs");
    let bloom_shader = include_str!("bloom.wgsl");
    let ssao_shader = include_str!("ssao.wgsl");
    assert!(
        source.contains("TextureFormat::Rgba8Unorm")
            && source.contains("TextureSampleType::Float { filterable: false }")
            && depth_source.contains("DEPTH_COLOR_FORMAT")
            && depth_source.contains("color_view")
            && source.contains("scene_pipelines")
            && source.contains("copy_output_to_buffer"),
        "GPU post chain must render into encoded Rgba8Unorm, expose a depth-color SSAO mechanism, and keep readback on the post output"
    );
    assert!(
        ssao_shader.contains("var depth_texture: texture_2d<f32>")
            && ssao_shader.contains("textureLoad(depth_texture")
            && !ssao_shader.contains("texture_depth_2d"),
        "SSAO must sample the packed depth-color target so the shared depth path compiles on WebGL2"
    );
    assert!(
        bloom_shader.contains("array<vec2<i32>, 9>")
            && ssao_shader.contains("array<vec2<i32>, 8>")
            && !bloom_shader.contains("for (var dy = -12")
            && !ssao_shader.contains("for (var dy = -12"),
        "GPU post kernels must avoid max-radius work for subtle presets"
    );
}
