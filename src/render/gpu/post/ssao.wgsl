struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct PostUniform {
    viewport: vec4<f32>,
    config: vec4<f32>,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var<uniform> post: PostUniform;

@group(0) @binding(2)
var depth_texture: texture_2d<f32>;

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

fn sample_scene_depth(coord: vec2<i32>) -> f32 {
    let encoded = textureLoad(depth_texture, coord, 0).rg;
    let high = encoded.r * 255.0;
    let low = encoded.g * 255.0;
    return (high * 256.0 + low) / 65535.0;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(source_texture));
    let coord = clamp(vec2<i32>(in.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    let source = textureLoad(source_texture, coord, 0);
    let radius = i32(clamp(post.viewport.z, 0.0, 12.0));
    let intensity = clamp(post.viewport.w, 0.0, 1.0);
    let threshold = max(post.config.x, 0.0);
    let reversed_z = post.config.y > 0.5;
    let clear_depth = post.config.z;
    if radius <= 0 || intensity <= 0.0 {
        return source;
    }

    let center_depth = sample_scene_depth(coord);
    if abs(center_depth - clear_depth) <= 0.000001 {
        return source;
    }

    let near_radius = max(1, radius / 2);
    let offsets = array<vec2<i32>, 16>(
        vec2<i32>(-near_radius, 0),
        vec2<i32>(near_radius, 0),
        vec2<i32>(0, -near_radius),
        vec2<i32>(0, near_radius),
        vec2<i32>(-near_radius, -near_radius),
        vec2<i32>(near_radius, -near_radius),
        vec2<i32>(-near_radius, near_radius),
        vec2<i32>(near_radius, near_radius),
        vec2<i32>(-radius, 0),
        vec2<i32>(radius, 0),
        vec2<i32>(0, -radius),
        vec2<i32>(0, radius),
        vec2<i32>(-radius, -radius),
        vec2<i32>(radius, -radius),
        vec2<i32>(-radius, radius),
        vec2<i32>(radius, radius),
    );

    var finite_samples = 0.0;
    var occluders = 0.0;
    for (var index = 0u; index < 16u; index = index + 1u) {
        let sample_coord = coord + offsets[index];
        if sample_coord.x < 0 || sample_coord.y < 0 || sample_coord.x >= dims.x || sample_coord.y >= dims.y {
            continue;
        }
        let sample_depth = sample_scene_depth(sample_coord);
        if abs(sample_depth - clear_depth) <= 0.000001 {
            continue;
        }
        finite_samples = finite_samples + 1.0;
        if (!reversed_z && sample_depth + threshold < center_depth)
            || (reversed_z && sample_depth > center_depth + threshold) {
            occluders = occluders + 1.0;
        }
    }

    if finite_samples <= 0.0 || occluders <= 0.0 {
        return source;
    }
    let coverage = occluders / finite_samples;
    let darkening = clamp(coverage * intensity, 0.0, 0.65);
    return vec4<f32>(source.rgb * (1.0 - darkening), source.a);
}
