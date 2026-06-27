struct QuadVertex {
    @location(0) side_along: vec2<f32>,
};

struct StrokeInstance {
    @location(1) start: vec3<f32>,
    @location(2) end: vec3<f32>,
    @location(3) color: vec4<f32>,
    @location(4) width_px: f32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) distance_px: f32,
    @location(2) half_width_px: f32,
};

struct LightingUniform {
    directional_light_direction_intensity: array<vec4<f32>, 16>,
    directional_light_color: array<vec4<f32>, 16>,
    directional_shadow_control: array<vec4<f32>, 16>,
    point_light_position_intensity: array<vec4<f32>, 16>,
    point_light_color_range: array<vec4<f32>, 16>,
    spot_light_position_intensity: array<vec4<f32>, 16>,
    spot_light_direction_cones: array<vec4<f32>, 16>,
    spot_light_cone_range: array<vec4<f32>, 16>,
    spot_light_color_range: array<vec4<f32>, 16>,
    area_light_position_flux: array<vec4<f32>, 2>,
    area_light_axis_x_shape: array<vec4<f32>, 2>,
    area_light_axis_y_range: array<vec4<f32>, 2>,
    area_light_color: array<vec4<f32>, 2>,
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

struct DrawUniform {
    world_from_model: mat4x4<f32>,
    normal_from_model: mat4x4<f32>,
    tint: vec4<f32>,
};

struct ClippedSegment {
    start: vec4<f32>,
    end: vec4<f32>,
    visible: f32,
};

const STROKE_AA_RADIUS_PX: f32 = 3.25;

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(2) @binding(0)
var<uniform> draw: DrawUniform;

@vertex
fn vs_main(quad: QuadVertex, segment: StrokeInstance) -> VertexOut {
    let start_world = draw.world_from_model * vec4<f32>(segment.start, 1.0);
    let end_world = draw.world_from_model * vec4<f32>(segment.end, 1.0);
    let clipped = clip_segment_to_near(
        camera.clip_from_world * start_world,
        camera.clip_from_world * end_world,
    );
    let start_ndc = clipped.start.xy / max(clipped.start.w, 0.0001);
    let end_ndc = clipped.end.xy / max(clipped.end.w, 0.0001);
    let delta = end_ndc - start_ndc;
    let length_px = length(delta * camera.viewport_near_far.xy * 0.5);
    let direction = select(vec2<f32>(1.0, 0.0), normalize(delta), length_px > 0.001);
    let normal = vec2<f32>(-direction.y, direction.x);
    let half_width = max(segment.width_px, 0.001) * 0.5;
    let aa_radius = STROKE_AA_RADIUS_PX;
    let expanded_half_width = half_width + aa_radius;
    let offset_ndc = normal * quad.side_along.x * expanded_half_width * vec2<f32>(
        2.0 / max(camera.viewport_near_far.x, 1.0),
        2.0 / max(camera.viewport_near_far.y, 1.0),
    );
    var position = select(clipped.start, clipped.end, quad.side_along.y > 0.5);
    position.x = position.x + offset_ndc.x * position.w;
    position.y = position.y + offset_ndc.y * position.w;
    if clipped.visible < 0.5 || length_px <= 0.001 {
        position = vec4<f32>(2.0, 2.0, 0.0, 1.0);
    }
    var out: VertexOut;
    out.position = position;
    out.color = segment.color * draw.tint;
    out.distance_px = quad.side_along.x * expanded_half_width;
    out.half_width_px = half_width;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let coverage = stroke_coverage(abs(in.distance_px), in.half_width_px);
    if coverage <= 0.0 {
        discard;
    }
    return vec4<f32>(linear_to_srgb(in.color.rgb), in.color.a * coverage);
}

fn stroke_coverage(distance_px: f32, half_width_px: f32) -> f32 {
    if distance_px <= half_width_px {
        return 1.0;
    }
    let t = clamp((distance_px - half_width_px) / STROKE_AA_RADIUS_PX, 0.0, 1.0);
    return 1.0 - t * t * (3.0 - 2.0 * t);
}

fn clip_segment_to_near(start: vec4<f32>, end: vec4<f32>) -> ClippedSegment {
    var a = start;
    var b = end;
    if a.z < 0.0 && b.z < 0.0 {
        return ClippedSegment(a, b, 0.0);
    }
    if a.z < 0.0 {
        let t = clamp((0.0 - a.z) / max(b.z - a.z, 0.000001), 0.0, 1.0);
        a = mix(a, b, t);
    }
    if b.z < 0.0 {
        let t = clamp((0.0 - b.z) / max(a.z - b.z, 0.000001), 0.0, 1.0);
        b = mix(b, a, t);
    }
    return ClippedSegment(a, b, 1.0);
}
