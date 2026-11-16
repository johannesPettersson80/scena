// scena.ambient_occlusion.wgsl
//
// Screen-space ambient occlusion evaluated before the colour pass. The colour
// pass multiplies it into environment lighting only, so direct light and
// highlights stay clean while creases and contacts lose their ambient fill.
//
// View-space positions are rebuilt from the depth prepass with the camera
// projection, which keeps the estimator correct for perspective,
// orthographic and reversed-depth cameras. The radius is a screen distance:
// the same number of pixels of contact shading regardless of asset units.
//
// These are compute passes writing one f32 per target pixel into a storage
// buffer. The colour pass already uses every sampled-texture slot WebGPU
// guarantees per stage, so visibility cannot reach it as another texture.

struct AmbientOcclusionUniform {
    clip_from_view: mat4x4<f32>,
    // x: radius in target pixels, y: intensity, z: clear depth, w: sample count
    params: vec4<f32>,
    // xy: blur step in target pixels, z: unused, w: unused
    blur: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> ao: AmbientOcclusionUniform;

@group(0) @binding(1)
var depth_texture: texture_depth_2d;

// Row-major visibility written by the previous stage.
@group(0) @binding(2)
var<storage, read> source_visibility: array<f32>;

@group(0) @binding(3)
var<storage, read_write> target_visibility: array<f32>;

const GOLDEN_ANGLE: f32 = 2.39996323;
const TAU: f32 = 6.28318531;
// Receivers ignore occluders closer to their own tangent plane than this
// fraction of the radius, which keeps flat surfaces free of self-occlusion.
const PLANE_BIAS: f32 = 0.04;
// Scales the mean horizon cosine so a right-angle crease reaches full strength.
const OCCLUSION_GAIN: f32 = 3.0;
// Radius of the wide spiral relative to the configured contact radius.
const WIDE_RADIUS_SCALE: f32 = 3.0;
const BLUR_TAPS: i32 = 5;

fn is_background(depth: f32) -> bool {
    return abs(depth - ao.params.z) <= 0.0000001;
}

// Clip-space w at a view depth: -z for perspective, 1 for orthographic.
fn clip_w(view_z: f32) -> f32 {
    return ao.clip_from_view[2][3] * view_z + ao.clip_from_view[3][3];
}

fn view_position(pixel: vec2<i32>, depth: f32, size: vec2<f32>) -> vec3<f32> {
    let p = ao.clip_from_view;
    let z = (p[3][2] - depth * p[3][3]) / (depth * p[2][3] - p[2][2]);
    let w = clip_w(z);
    let ndc = vec2<f32>(
        (f32(pixel.x) + 0.5) / size.x * 2.0 - 1.0,
        1.0 - (f32(pixel.y) + 0.5) / size.y * 2.0,
    );
    return vec3<f32>(
        (ndc.x * w - p[2][0] * z - p[3][0]) / p[0][0],
        (ndc.y * w - p[2][1] * z - p[3][1]) / p[1][1],
        z,
    );
}

fn load_depth(pixel: vec2<i32>, dims: vec2<i32>) -> f32 {
    return textureLoad(depth_texture, clamp(pixel, vec2<i32>(0), dims - vec2<i32>(1)), 0);
}

// Geometric normal from depth. Each axis uses the neighbour whose depth is
// closer to the centre, so normals stay clean next to silhouettes.
fn view_normal(pixel: vec2<i32>, center: vec3<f32>, dims: vec2<i32>, size: vec2<f32>) -> vec3<f32> {
    let left = view_position(pixel + vec2<i32>(-1, 0), load_depth(pixel + vec2<i32>(-1, 0), dims), size);
    let right = view_position(pixel + vec2<i32>(1, 0), load_depth(pixel + vec2<i32>(1, 0), dims), size);
    let up = view_position(pixel + vec2<i32>(0, -1), load_depth(pixel + vec2<i32>(0, -1), dims), size);
    let down = view_position(pixel + vec2<i32>(0, 1), load_depth(pixel + vec2<i32>(0, 1), dims), size);
    var along_x = right - center;
    if abs(left.z - center.z) < abs(right.z - center.z) {
        along_x = center - left;
    }
    var along_y = down - center;
    if abs(up.z - center.z) < abs(down.z - center.z) {
        along_y = center - up;
    }
    var normal = cross(along_y, along_x);
    let length_squared = dot(normal, normal);
    if length_squared <= 0.0 {
        return vec3<f32>(0.0, 0.0, 1.0);
    }
    normal = normal * inverseSqrt(length_squared);
    var to_camera = vec3<f32>(0.0, 0.0, 1.0);
    if abs(ao.clip_from_view[2][3]) > 0.5 {
        to_camera = -center;
    }
    if dot(normal, to_camera) < 0.0 {
        normal = -normal;
    }
    return normal;
}

// World size of one target pixel at a view depth.
fn pixel_world_size(view_z: f32, size: vec2<f32>) -> f32 {
    return abs(2.0 * clip_w(view_z) / (ao.clip_from_view[1][1] * size.y));
}

fn interleaved_gradient_noise(pixel: vec2<i32>) -> f32 {
    return fract(52.9829189 * fract(dot(vec2<f32>(pixel), vec2<f32>(0.06711056, 0.00583715))));
}

fn store(pixel: vec2<i32>, dims: vec2<i32>, visibility: f32) {
    target_visibility[u32(pixel.y) * u32(dims.x) + u32(pixel.x)] = visibility;
}

fn load_visibility(pixel: vec2<i32>, dims: vec2<i32>) -> f32 {
    return source_visibility[u32(pixel.y) * u32(dims.x) + u32(pixel.x)];
}

@compute @workgroup_size(8, 8)
fn cs_occlusion(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let dims = vec2<i32>(textureDimensions(depth_texture));
    let size = vec2<f32>(dims);
    let pixel = vec2<i32>(invocation.xy);
    if pixel.x >= dims.x || pixel.y >= dims.y {
        return;
    }
    let depth = load_depth(pixel, dims);
    let radius_px = ao.params.x;
    let sample_count = i32(clamp(ao.params.w, 1.0, 32.0));
    if is_background(depth) || radius_px < 1.0 {
        store(pixel, dims, 1.0);
        return;
    }
    let center = view_position(pixel, depth, size);
    let normal = view_normal(pixel, center, dims, size);
    let pixel_world = pixel_world_size(center.z, size);
    let rotation = interleaved_gradient_noise(pixel) * TAU;
    var occlusion = 0.0;
    for (var index = 0; index < sample_count; index = index + 1) {
        // Two interleaved spirals: a tight one for contact creases and a wide
        // one for the soft ambient shadow a body casts around its base. Radius
        // grows linearly along each, so samples are densest near the receiver.
        var sample_radius_px = radius_px;
        if (index & 1) == 1 {
            sample_radius_px = radius_px * WIDE_RADIUS_SCALE;
        }
        let world_radius = sample_radius_px * pixel_world;
        let inverse_radius_squared = 1.0 / max(world_radius * world_radius, 1.0e-12);
        let alpha = (f32(index) + 0.5) / f32(sample_count);
        let angle = f32(index) * GOLDEN_ANGLE + rotation;
        let offset = vec2<f32>(cos(angle), sin(angle)) * (alpha * sample_radius_px);
        let sample_pixel = pixel + vec2<i32>(round(offset));
        if sample_pixel.x < 0 || sample_pixel.y < 0 || sample_pixel.x >= dims.x || sample_pixel.y >= dims.y {
            continue;
        }
        let sample_depth = textureLoad(depth_texture, sample_pixel, 0);
        if is_background(sample_depth) {
            continue;
        }
        let to_sample = view_position(sample_pixel, sample_depth, size) - center;
        let distance_squared = dot(to_sample, to_sample);
        let above_plane = dot(to_sample, normal) - PLANE_BIAS * world_radius;
        if above_plane <= 0.0 || distance_squared <= 0.0 {
            continue;
        }
        let falloff = clamp(1.0 - distance_squared * inverse_radius_squared, 0.0, 1.0);
        occlusion = occlusion + above_plane * inverseSqrt(distance_squared) * falloff * falloff;
    }
    let mean = occlusion / f32(sample_count);
    let visibility = 1.0 - clamp(mean * OCCLUSION_GAIN, 0.0, 1.0) * clamp(ao.params.y, 0.0, 1.0);
    store(pixel, dims, visibility);
}

// Separable blur that only mixes texels lying on the receiver's own surface:
// the weight falls with distance from the centre's tangent plane, so the
// spiral's noise is removed without bleeding occlusion across silhouettes.
@compute @workgroup_size(8, 8)
fn cs_blur(@builtin(global_invocation_id) invocation: vec3<u32>) {
    let dims = vec2<i32>(textureDimensions(depth_texture));
    let size = vec2<f32>(dims);
    let pixel = vec2<i32>(invocation.xy);
    if pixel.x >= dims.x || pixel.y >= dims.y {
        return;
    }
    let depth = load_depth(pixel, dims);
    let center_value = load_visibility(pixel, dims);
    if is_background(depth) {
        store(pixel, dims, 1.0);
        return;
    }
    let center = view_position(pixel, depth, size);
    let normal = view_normal(pixel, center, dims, size);
    let tolerance = max(ao.params.x, 4.0) * 0.08 * pixel_world_size(center.z, size);
    let step = vec2<i32>(ao.blur.xy);
    var sum = center_value;
    var weight_sum = 1.0;
    for (var tap = -BLUR_TAPS; tap <= BLUR_TAPS; tap = tap + 1) {
        if tap == 0 {
            continue;
        }
        let tap_pixel = pixel + step * tap;
        if tap_pixel.x < 0 || tap_pixel.y < 0 || tap_pixel.x >= dims.x || tap_pixel.y >= dims.y {
            continue;
        }
        let tap_depth = textureLoad(depth_texture, tap_pixel, 0);
        if is_background(tap_depth) {
            continue;
        }
        let plane_distance = abs(dot(view_position(tap_pixel, tap_depth, size) - center, normal));
        let surface_weight = clamp(1.0 - plane_distance / max(tolerance, 1.0e-9), 0.0, 1.0);
        let spatial = f32(tap) / f32(BLUR_TAPS);
        let weight = exp(-2.0 * spatial * spatial) * surface_weight;
        sum = sum + load_visibility(tap_pixel, dims) * weight;
        weight_sum = weight_sum + weight;
    }
    store(pixel, dims, sum / weight_sum);
}
