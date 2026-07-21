fn finalize_post_output(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(linear_to_srgb(color.rgb), color.a);
}
