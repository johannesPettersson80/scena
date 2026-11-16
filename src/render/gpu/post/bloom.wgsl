// scena.bloom.wgsl
//
// Bloom as a separable Gaussian. `fs_main` keeps what is brighter than the
// threshold and blurs it along x; `fs_composite` blurs that along y and is
// blended additively over the frame. Two one-dimensional Gaussian passes are
// an exactly round two-dimensional blur, at a cost that grows with the radius
// instead of its square. The threshold and width live in bloom_kernel.wgsl.
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct PostUniform {
    // .z = linear luminance threshold, .w = intensity.
    viewport: vec4<f32>,
    // .x = radius in target pixels, .z = knee as a share of the threshold.
    config: vec4<f32>,
    white_balance: vec4<f32>,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var<uniform> post: PostUniform;

// Taps each side of the centre at the largest radius: three standard
// deviations of the widest blur.
const BLOOM_MAX_REACH: i32 = 22;

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

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(source_texture));
    let coord = clamp(vec2<i32>(in.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    return vec4<f32>(bloom_blur(coord, dims, vec2<i32>(1, 0), true), 1.0);
}

// Blended additively: the colour is added to the frame, whose alpha is kept.
@fragment
fn fs_composite(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(source_texture));
    let coord = clamp(vec2<i32>(in.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    let intensity = clamp(post.viewport.w, 0.0, 1.0);
    return vec4<f32>(bloom_blur(coord, dims, vec2<i32>(0, 1), false) * intensity, 0.0);
}

// One-dimensional Gaussian along `axis`. Taps that fall outside the frame are
// left out and the rest renormalized, so a uniformly bright frame stays
// uniform up to its border.
fn bloom_blur(coord: vec2<i32>, dims: vec2<i32>, axis: vec2<i32>, bright_pass: bool) -> vec3<f32> {
    let sigma = bloom_sigma();
    if sigma <= 0.0 || post.viewport.w <= 0.0 {
        return vec3<f32>(0.0);
    }
    let reach = min(i32(ceil(3.0 * sigma)), BLOOM_MAX_REACH);
    var sum = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var tap = -reach; tap <= reach; tap = tap + 1) {
        let sample_coord = coord + axis * tap;
        if any(sample_coord < vec2<i32>(0)) || any(sample_coord >= dims) {
            continue;
        }
        var sample = textureLoad(source_texture, sample_coord, 0).rgb;
        if bright_pass {
            sample = bloom_bright(sample);
        }
        let weight = bloom_weight(f32(tap), sigma);
        sum = sum + sample * weight;
        weight_sum = weight_sum + weight;
    }
    return sum / max(weight_sum, 0.000001);
}
