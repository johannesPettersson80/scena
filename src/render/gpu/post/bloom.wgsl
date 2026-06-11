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
    let source = textureLoad(source_texture, coord, 0);
    let threshold = post.viewport.z;
    let intensity = clamp(post.viewport.w, 0.0, 1.0);
    let radius = i32(clamp(post.config.x, 0.0, 12.0));
    if radius <= 0 || intensity <= 0.0 {
        return source;
    }

    let near_radius = max(1, radius / 2);
    let offsets = array<vec2<i32>, 17>(
        vec2<i32>(0, 0),
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

    var sum = vec3<f32>(0.0);
    var sample_count = 0.0;
    for (var index = 0u; index < 17u; index = index + 1u) {
        let sample_coord = coord + offsets[index];
        if sample_coord.x < 0 || sample_coord.y < 0 || sample_coord.x >= dims.x || sample_coord.y >= dims.y {
            continue;
        }
        let sample = textureLoad(source_texture, sample_coord, 0);
        if luma(sample.rgb) >= threshold {
            sum = sum + sample.rgb;
        }
        sample_count = sample_count + 1.0;
    }
    if sample_count <= 0.0 || all(sum == vec3<f32>(0.0)) {
        return source;
    }
    let bloom = (sum / sample_count) * intensity;
    return vec4<f32>(min(source.rgb + bloom, vec3<f32>(1.0)), source.a);
}

fn luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}
