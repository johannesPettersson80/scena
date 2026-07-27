struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct PostUniform {
    viewport: vec4<f32>,
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

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(source_texture));
    let coord = clamp(vec2<i32>(in.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    let center = textureLoad(source_texture, coord, 0);
    let bloom = bloom_energy(coord, dims);
    if coord.x <= 0 || coord.y <= 0 || coord.x >= dims.x - 1 || coord.y >= dims.y - 1 {
        return finalize_post_output(vec4<f32>(center.rgb + bloom, center.a));
    }

    let offsets = array<vec2<i32>, 5>(
        vec2<i32>(0, -1),
        vec2<i32>(-1, 0),
        vec2<i32>(0, 0),
        vec2<i32>(1, 0),
        vec2<i32>(0, 1),
    );
    let center_luma = luma(center.rgb);
    var min_luma = 1.0e9;
    var max_luma = -1.0e9;
    var bright_neighbors = 0u;
    var dark_neighbors = 0u;
    var sum = vec4<f32>(0.0);
    for (var index = 0u; index < 5u; index = index + 1u) {
        let sample = textureLoad(source_texture, coord + offsets[index], 0);
        let sample_luma = luma(sample.rgb);
        min_luma = min(min_luma, sample_luma);
        max_luma = max(max_luma, sample_luma);
        if sample_luma - center_luma >= 16.0 / 255.0 {
            bright_neighbors = bright_neighbors + 1u;
        }
        if center_luma - sample_luma >= 16.0 / 255.0 {
            dark_neighbors = dark_neighbors + 1u;
        }
        sum = sum + sample;
    }

    var base = center;
    if max_luma - min_luma >= 16.0 / 255.0 {
        let dark_edge = center_luma - min_luma <= 1.0 / 255.0 && bright_neighbors >= 2u;
        let light_edge = max_luma - center_luma <= 1.0 / 255.0 && dark_neighbors >= 2u;
        if dark_edge || light_edge {
            base = sum / 5.0;
        }
    }
    return finalize_post_output(vec4<f32>(base.rgb + bloom, base.a));
}

fn bloom_energy(coord: vec2<i32>, dims: vec2<i32>) -> vec3<f32> {
    let threshold = post.viewport.z;
    let intensity = clamp(post.viewport.w, 0.0, 1.0);
    let radius = i32(clamp(post.config.x, 0.0, 12.0));
    if radius <= 0 || intensity <= 0.0 {
        return vec3<f32>(0.0);
    }

    let near_radius = max(1, radius / 2);
    let offsets = array<vec2<i32>, 9>(
        vec2<i32>(0, 0),
        vec2<i32>(-near_radius, 0),
        vec2<i32>(near_radius, 0),
        vec2<i32>(0, -near_radius),
        vec2<i32>(0, near_radius),
        vec2<i32>(-radius, 0),
        vec2<i32>(radius, 0),
        vec2<i32>(0, -radius),
        vec2<i32>(0, radius),
    );

    var sum = vec3<f32>(0.0);
    var sample_count = 0.0;
    for (var index = 0u; index < 9u; index = index + 1u) {
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
        return vec3<f32>(0.0);
    }
    return (sum / sample_count) * intensity;
}

fn luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}
