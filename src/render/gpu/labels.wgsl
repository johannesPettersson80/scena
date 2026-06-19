struct QuadVertex {
    @location(0) unit: vec2<f32>,
};

struct LabelInstance {
    @location(1) anchor: vec3<f32>,
    @location(2) right: vec3<f32>,
    @location(3) up: vec3<f32>,
    @location(4) world_units_per_px: f32,
    @location(5) rect_px: vec4<f32>,
    @location(6) uv_rect: vec4<f32>,
    @location(7) color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct LightingUniform {
    directional_light_direction_intensity: array<vec4<f32>, 4>,
    directional_light_color: array<vec4<f32>, 4>,
    directional_shadow_control: array<vec4<f32>, 4>,
    point_light_position_intensity: array<vec4<f32>, 4>,
    point_light_color_range: array<vec4<f32>, 4>,
    spot_light_position_intensity: array<vec4<f32>, 4>,
    spot_light_direction_cones: array<vec4<f32>, 4>,
    spot_light_cone_range: array<vec4<f32>, 4>,
    spot_light_color_range: array<vec4<f32>, 4>,
    light_counts: vec4<f32>,
    environment_diffuse_intensity: vec4<f32>,
    environment_specular_intensity: vec4<f32>,
};

struct CameraUniform {
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    light_from_world: mat4x4<f32>,
    camera_position_exposure: vec4<f32>,
    viewport_near_far: vec4<f32>,
    color_management: vec4<f32>,
    lighting: LightingUniform,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var label_atlas: texture_2d<f32>;

@vertex
fn vs_main(quad: QuadVertex, label: LabelInstance) -> VertexOut {
    let px = mix(label.rect_px.x, label.rect_px.z, quad.unit.x);
    let py = mix(label.rect_px.y, label.rect_px.w, quad.unit.y);
    let world = label.anchor
        + label.right * (px * label.world_units_per_px)
        + label.up * (py * label.world_units_per_px);
    var out: VertexOut;
    out.position = camera.clip_from_world * vec4<f32>(world, 1.0);
    out.uv = vec2<f32>(
        mix(label.uv_rect.x, label.uv_rect.z, quad.unit.x),
        mix(label.uv_rect.w, label.uv_rect.y, quad.unit.y),
    );
    out.color = label.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let coverage = manual_bilinear_coverage(in.uv);
    if coverage <= 0.0 {
        discard;
    }
    let color_management_mode = camera.color_management.x;
    let rgb = apply_tonemapper(in.color.rgb * camera.camera_position_exposure.w, color_management_mode);
    return vec4<f32>(encode_post_target_rgb(rgb), in.color.a * coverage);
}

fn manual_bilinear_coverage(uv: vec2<f32>) -> f32 {
    let dims = textureDimensions(label_atlas);
    let coord = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)) * vec2<f32>(dims) - vec2<f32>(0.5);
    let base = floor(coord);
    let frac = coord - base;
    let max_coord = vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1);
    let p00 = clamp(vec2<i32>(base), vec2<i32>(0, 0), max_coord);
    let p10 = clamp(p00 + vec2<i32>(1, 0), vec2<i32>(0, 0), max_coord);
    let p01 = clamp(p00 + vec2<i32>(0, 1), vec2<i32>(0, 0), max_coord);
    let p11 = clamp(p00 + vec2<i32>(1, 1), vec2<i32>(0, 0), max_coord);
    let c00 = textureLoad(label_atlas, p00, 0).a;
    let c10 = textureLoad(label_atlas, p10, 0).a;
    let c01 = textureLoad(label_atlas, p01, 0).a;
    let c11 = textureLoad(label_atlas, p11, 0).a;
    let top = mix(c00, c10, frac.x);
    let bottom = mix(c01, c11, frac.x);
    return clamp(mix(top, bottom, frac.y), 0.0, 1.0);
}

fn apply_tonemapper(color: vec3<f32>, color_management_mode: f32) -> vec3<f32> {
    if color_management_mode < 0.5 {
        return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return aces_tonemap(color);
}

fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + vec3<f32>(b))) / (color * (c * color + vec3<f32>(d)) + vec3<f32>(e)), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn encode_post_target_rgb(color: vec3<f32>) -> vec3<f32> {
    if camera.color_management.y <= 0.5 {
        return color;
    }
    return vec3<f32>(
        linear_to_srgb_channel(color.r),
        linear_to_srgb_channel(color.g),
        linear_to_srgb_channel(color.b),
    );
}

fn linear_to_srgb_channel(channel: f32) -> f32 {
    let value = clamp(channel, 0.0, 1.0);
    if value <= 0.0031308 {
        return value * 12.92;
    }
    return 1.055 * pow(value, 1.0 / 2.4) - 0.055;
}
