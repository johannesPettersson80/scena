struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct PostUniform {
    viewport: vec4<f32>,
    // .y = display exposure scale used for the perceptual luma.
    config: vec4<f32>,
    white_balance: vec4<f32>,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var<uniform> post: PostUniform;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(3.0, 1.0),
    );
    let position = positions[vertex_index];
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = position * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

// The edge search lives in fxaa_resolve.wgsl, shared with the combined
// bloom + FXAA surface pass.
@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(source_texture));
    let coord = clamp(vec2<i32>(in.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    return finalize_post_output(fxaa_resolve(coord, dims));
}
