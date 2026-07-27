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

fn dof_radius(depth: f32) -> i32 {
    let max_radius = clamp(post.viewport.w, 0.0, 16.0);
    let focus_distance = max(post.config.x, 0.001);
    let focal_m = clamp(post.config.y, 8.0, 600.0) * 0.001;
    let sensor_height_m = clamp(post.config.z, 4.0, 100.0) * 0.001;
    let aperture_f_stop = clamp(post.config.w, 0.7, 32.0);
    let near = max(post.white_balance.x, 0.000001);
    let far = max(post.white_balance.y, near + 0.000001);
    let normalized_depth = select(depth, 1.0 - depth, post.white_balance.z > 0.5);
    let camera_depth = near * far / max(far - normalized_depth * (far - near), 0.000001);
    let image_focus = focal_m * focus_distance / max(focus_distance - focal_m, 0.0001);
    let image_depth = focal_m * camera_depth / max(camera_depth - focal_m, 0.0001);
    let aperture_diameter = focal_m / aperture_f_stop;
    let coc_m = aperture_diameter * abs(image_depth - image_focus) / max(image_depth, 0.00001);
    let radius = coc_m / sensor_height_m * max_radius * 12.0;
    return i32(round(clamp(radius, 0.0, max_radius)));
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dims = vec2<i32>(textureDimensions(source_texture));
    let coord = clamp(vec2<i32>(in.uv * vec2<f32>(dims)), vec2<i32>(0), dims - vec2<i32>(1));
    let source = textureLoad(source_texture, coord, 0);
    let clear_depth = post.white_balance.w;
    let center_depth = sample_scene_depth(coord);
    if abs(center_depth - clear_depth) <= 0.000001 {
        return source;
    }
    let radius = dof_radius(center_depth);
    if radius <= 0 {
        return source;
    }

    var sum = vec4<f32>(0.0);
    var count = 0.0;
    for (var oy = -radius; oy <= radius; oy = oy + 1) {
        for (var ox = -radius; ox <= radius; ox = ox + 1) {
            let sample_coord = clamp(coord + vec2<i32>(ox, oy), vec2<i32>(0), dims - vec2<i32>(1));
            sum = sum + textureLoad(source_texture, sample_coord, 0);
            count = count + 1.0;
        }
    }
    return sum / max(count, 1.0);
}
