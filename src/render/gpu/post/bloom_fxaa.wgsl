// scena.bloom_fxaa.wgsl
//
// Combined bloom + FXAA for a surface path that has one pass to spend. The
// edge search is fxaa_resolve.wgsl, shared with the FXAA pass; the threshold
// and blur width are bloom_kernel.wgsl, shared with the bloom passes.
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct PostUniform {
    // .z = linear luminance threshold, .w = bloom intensity.
    viewport: vec4<f32>,
    // .x = bloom radius in target pixels, .y = display exposure scale,
    // .z = bloom knee as a share of the threshold.
    config: vec4<f32>,
    white_balance: vec4<f32>,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var<uniform> post: PostUniform;

// Taps on the bloom disc, beside the centre tap.
const BLOOM_DISC_TAPS: i32 = 48;
// Angle between successive taps of the spiral, in radians.
const BLOOM_GOLDEN_ANGLE: f32 = 2.39996323;

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
    let base = fxaa_resolve(coord, dims);
    let bloom = bloom_energy(coord, dims);
    return finalize_post_output(vec4<f32>(base.rgb + bloom, base.a));
}

// Single-pass stand-in for the two bloom passes: Gaussian-weighted taps on a
// golden-angle spiral that covers the disc evenly out to three standard
// deviations, so the halo has no preferred axis. The taps are point samples,
// which leaves an emitter smaller than their spacing visible as separate
// copies; the two-pass blur in bloom.wgsl has no such limit.
fn bloom_energy(coord: vec2<i32>, dims: vec2<i32>) -> vec3<f32> {
    let sigma = bloom_sigma();
    let intensity = clamp(post.viewport.w, 0.0, 1.0);
    if sigma <= 0.0 || intensity <= 0.0 {
        return vec3<f32>(0.0);
    }
    var sum = bloom_bright(textureLoad(source_texture, coord, 0).rgb);
    var weight_sum = 1.0;
    for (var tap = 0; tap < BLOOM_DISC_TAPS; tap = tap + 1) {
        let distance = 3.0 * sigma * sqrt((f32(tap) + 0.5) / f32(BLOOM_DISC_TAPS));
        let angle = f32(tap) * BLOOM_GOLDEN_ANGLE;
        let offset = vec2<f32>(cos(angle), sin(angle)) * distance;
        let sample_coord = coord + vec2<i32>(round(offset));
        if any(sample_coord < vec2<i32>(0)) || any(sample_coord >= dims) {
            continue;
        }
        let weight = bloom_weight(distance, sigma);
        sum = sum + bloom_bright(textureLoad(source_texture, sample_coord, 0).rgb) * weight;
        weight_sum = weight_sum + weight;
    }
    return sum / weight_sum * intensity;
}
