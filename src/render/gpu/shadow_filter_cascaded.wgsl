// scena.shadow_filter_cascaded.wgsl
//
// Directional shadows for the native and WebGPU triangle shader: up to four
// cascades in one atlas, each a crop of the scene-fitted light frustum around
// a slice of the camera frustum, filtered with percentage-closer soft shadows
// so contact edges stay crisp and distant edges soften.
//
// Bias is expressed in shadow texels so it tracks the cascade resolution:
// receivers are offset along their geometric normal before projection and the
// comparison depth is pulled toward the light in proportion to the filter
// footprint and the surface slope.

const SHADOW_BLOCKER_TAPS: u32 = 12u;
const SHADOW_FILTER_TAPS: u32 = 16u;
const SHADOW_MAX_FILTER_TEXELS: f32 = 24.0;
const SHADOW_MIN_FILTER_TEXELS: f32 = 1.25;

const SHADOW_DISC: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216),
    vec2<f32>(0.94558609, -0.76890725),
    vec2<f32>(-0.09418410, -0.92938870),
    vec2<f32>(0.34495938, 0.29387760),
    vec2<f32>(-0.91588581, 0.45771432),
    vec2<f32>(-0.81544232, -0.87912464),
    vec2<f32>(-0.38277543, 0.27676845),
    vec2<f32>(0.97484398, 0.75648379),
    vec2<f32>(0.44323325, -0.97511554),
    vec2<f32>(0.53742981, -0.47373420),
    vec2<f32>(-0.26496911, -0.41893023),
    vec2<f32>(0.79197514, 0.19090188),
    vec2<f32>(-0.24188840, 0.99706507),
    vec2<f32>(-0.81409955, 0.91437590),
    vec2<f32>(0.19984126, 0.78641367),
    vec2<f32>(0.14383161, -0.14100790),
);

struct ShadowCascadeSample {
    // Shadow-map texel coordinates of the receiver inside the atlas.
    texel: vec2<f32>,
    depth: f32,
    // Inclusive atlas texel bounds of this cascade.
    bounds_min: vec2<f32>,
    bounds_max: vec2<f32>,
    texel_world: f32,
    depth_range_world: f32,
    inside: bool,
};

// Stable per-pixel rotation so the disc pattern turns into fine noise that
// multisampling and supersampling average away instead of visible banding.
fn shadow_rotation(fragment_xy: vec2<f32>) -> vec2<f32> {
    let noise = fract(52.9829189 * fract(dot(fragment_xy, vec2<f32>(0.06711056, 0.00583715))));
    let angle = noise * 6.2831853;
    return vec2<f32>(cos(angle), sin(angle));
}

fn shadow_rotate(offset: vec2<f32>, rotation: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        offset.x * rotation.x - offset.y * rotation.y,
        offset.x * rotation.y + offset.y * rotation.x,
    );
}

fn shadow_cascade_sample(cascade: u32, world_position: vec3<f32>, normal: vec3<f32>) -> ShadowCascadeSample {
    let light_from_world = camera.shadow_cascades[cascade];
    let atlas = vec2<f32>(textureDimensions(shadow_map));
    let atlas_scale = camera.shadow_params.y;
    let cascade_size = atlas * atlas_scale;
    let row_x = vec3<f32>(light_from_world[0].x, light_from_world[1].x, light_from_world[2].x);
    let row_z = vec3<f32>(light_from_world[0].z, light_from_world[1].z, light_from_world[2].z);
    var result: ShadowCascadeSample;
    result.texel_world = 2.0 / max(length(row_x) * cascade_size.x, 0.000001);
    result.depth_range_world = 1.0 / max(length(row_z), 0.000001);
    let offset_position = world_position + normal * (result.texel_world * camera.shadow_params.z);
    let light_clip = light_from_world * vec4<f32>(offset_position, 1.0);
    let light_ndc = light_clip.xyz / max(light_clip.w, 0.000001);
    let uv = vec2<f32>(light_ndc.x * 0.5 + 0.5, light_ndc.y * -0.5 + 0.5);
    let origin = vec2<f32>(f32(cascade % 2u), f32(cascade / 2u)) * cascade_size;
    result.texel = origin + uv * cascade_size;
    result.depth = light_ndc.z;
    result.bounds_min = origin + vec2<f32>(0.5);
    result.bounds_max = origin + cascade_size - vec2<f32>(0.5);
    result.inside = light_clip.w > 0.0 &&
        uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0 &&
        light_ndc.z >= 0.0 && light_ndc.z <= 1.0;
    return result;
}

fn shadow_cascade_visibility(
    cascade: u32,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    incoming: vec3<f32>,
    rotation: vec2<f32>,
) -> f32 {
    let receiver = shadow_cascade_sample(cascade, world_position, normal);
    if !receiver.inside {
        return 1.0;
    }
    let atlas = vec2<f32>(textureDimensions(shadow_map));
    let n_dot_l = clamp(dot(normal, incoming), 0.05, 1.0);
    let slope = min(sqrt(1.0 - n_dot_l * n_dot_l) / n_dot_l, 4.0);
    let texel_depth = receiver.texel_world / receiver.depth_range_world;

    // Blocker search: average depth of occluders in front of the receiver.
    let light_tangent = camera.shadow_params.w;
    let search_texels = clamp(
        light_tangent * receiver.depth * receiver.depth_range_world / receiver.texel_world,
        2.0,
        SHADOW_MAX_FILTER_TEXELS,
    );
    let search_bias = texel_depth * (1.0 + search_texels * slope * 0.5);
    var blocker_depth = 0.0;
    var blocker_count = 0.0;
    for (var i = 0u; i < SHADOW_BLOCKER_TAPS; i = i + 1u) {
        let tap = clamp(
            receiver.texel + shadow_rotate(SHADOW_DISC[i], rotation) * search_texels,
            receiver.bounds_min,
            receiver.bounds_max,
        );
        let occluder = textureLoad(shadow_map, vec2<i32>(tap), 0);
        if occluder < receiver.depth - search_bias {
            blocker_depth += occluder;
            blocker_count += 1.0;
        }
    }
    if blocker_count < 0.5 {
        return 1.0;
    }
    let penumbra_world = (receiver.depth - blocker_depth / blocker_count) *
        receiver.depth_range_world * light_tangent * 2.0;
    let filter_texels = clamp(
        penumbra_world / receiver.texel_world,
        SHADOW_MIN_FILTER_TEXELS,
        SHADOW_MAX_FILTER_TEXELS,
    );
    let compare_depth = receiver.depth - texel_depth * (1.0 + filter_texels * slope * 0.5);
    var visibility = 0.0;
    for (var i = 0u; i < SHADOW_FILTER_TAPS; i = i + 1u) {
        let tap = clamp(
            receiver.texel + shadow_rotate(SHADOW_DISC[i], rotation) * filter_texels,
            receiver.bounds_min,
            receiver.bounds_max,
        );
        visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, tap / atlas, compare_depth);
    }
    return visibility / f32(SHADOW_FILTER_TAPS);
}

// Visibility of the shadow-casting directional light at `world_position`.
// `normal` is the geometric (not normal-mapped) surface normal and
// `incoming` the unit direction toward the light.
fn directional_shadow_factor(
    world_position: vec3<f32>,
    normal: vec3<f32>,
    incoming: vec3<f32>,
    fragment_xy: vec2<f32>,
) -> f32 {
    let cascade_count = u32(clamp(camera.shadow_params.x, 1.0, 4.0));
    let view_depth = -(camera.view_from_world * vec4<f32>(world_position, 1.0)).z;
    var cascade = 0u;
    for (var i = 0u; i < 3u; i = i + 1u) {
        if i + 1u < cascade_count && view_depth > camera.shadow_splits[i] {
            cascade = i + 1u;
        }
    }
    let rotation = shadow_rotation(fragment_xy);
    let visibility = shadow_cascade_visibility(cascade, world_position, normal, incoming, rotation);
    // Cross-fade into the next cascade over the last tenth of this one so the
    // resolution step is not visible as a line.
    if cascade + 1u < cascade_count {
        let split = camera.shadow_splits[cascade];
        let fade = clamp((view_depth - split * 0.9) / max(split * 0.1, 0.000001), 0.0, 1.0);
        if fade > 0.0 {
            let next = shadow_cascade_visibility(cascade + 1u, world_position, normal, incoming, rotation);
            return mix(visibility, next, fade);
        }
    }
    return visibility;
}
