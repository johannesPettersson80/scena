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

    let height = max(1.0, post.viewport.y);
    let horizon_y = clamp(post.config.z * height, 1.0, height - 2.0);
    if f32(coord.y) < horizon_y {
        return source;
    }

    let strength = clamp(post.config.x, 0.0, 1.0);
    if strength <= 0.0 {
        return source;
    }
    let floor_height = max(1.0, height - horizon_y);
    let distance = clamp((f32(coord.y) - horizon_y) / floor_height, 0.0, 1.0);
    let fade = clamp(1.0 - distance * clamp(post.config.w, 0.0, 1.0), 0.0, 1.0);
    let floor_mask = clamp(1.0 - luma(source.rgb), 0.0, 1.0);
    let alpha = clamp(strength * fade * floor_mask, 0.0, 1.0);
    if alpha <= 0.0 {
        return source;
    }

    let mirrored_y = i32(clamp(2.0 * horizon_y - f32(coord.y), 0.0, height - 1.0));
    let radius = i32(clamp(round(post.config.y * 8.0), 0.0, 8.0));
    let reflected = reflected_sample(coord.x, mirrored_y, radius, dims);
    return vec4<f32>(mix(source.rgb, reflected, alpha), source.a);
}

fn reflected_sample(x: i32, y: i32, radius: i32, dims: vec2<i32>) -> vec3<f32> {
    if radius <= 0 {
        return textureLoad(source_texture, vec2<i32>(x, y), 0).rgb;
    }
    var sum = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var dy = -8; dy <= 8; dy = dy + 1) {
        for (var dx = -8; dx <= 8; dx = dx + 1) {
            if abs(dx) > radius || abs(dy) > radius {
                continue;
            }
            let distance = length(vec2<f32>(f32(dx), f32(dy)));
            let weight = max(0.0, 1.0 - distance / (f32(radius) + 1.0));
            if weight <= 0.0 {
                continue;
            }
            let sample_coord = clamp(vec2<i32>(x + dx, y + dy), vec2<i32>(0), dims - vec2<i32>(1));
            sum = sum + textureLoad(source_texture, sample_coord, 0).rgb * weight;
            weight_sum = weight_sum + weight;
        }
    }
    if weight_sum <= 0.0 {
        return textureLoad(source_texture, vec2<i32>(x, y), 0).rgb;
    }
    return sum / weight_sum;
}

fn luma(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}
