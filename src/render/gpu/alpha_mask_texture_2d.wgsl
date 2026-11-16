// scena.alpha_mask_texture_2d.wgsl
//
// Alpha-tested depth for glTF MASK materials. The plain depth prepass and
// shadow caster write depth for every covered fragment, so a cut-out would
// hide whatever lies behind it and cast a solid shadow. Batches whose
// material is alpha-masked use these entry points instead: they evaluate the
// same base-colour alpha as the colour pass and discard below the cutoff.
//
// Generated from `alpha_mask_texture_2d_array.wgsl` with the material texture
// bound as `texture_2d` for WebGL2; a unit test keeps the two in step.

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(3) tex_coord0: vec2<f32>,
    @location(6) instance_world_0: vec4<f32>,
    @location(7) instance_world_1: vec4<f32>,
    @location(8) instance_world_2: vec4<f32>,
    @location(9) instance_world_3: vec4<f32>,
    @location(14) instance_tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) tex_coord0: vec2<f32>,
    @location(2) alpha: f32,
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
    area_light_position_flux: array<vec4<f32>, 4>,
    area_light_axis_x_shape: array<vec4<f32>, 4>,
    area_light_axis_y_range: array<vec4<f32>, 4>,
    area_light_color: array<vec4<f32>, 4>,
    light_counts: vec4<f32>,
    environment_diffuse_intensity: vec4<f32>,
    environment_specular_intensity: vec4<f32>,
    environment_transform: vec4<f32>,
};

struct CameraUniform {
    view_from_world: mat4x4<f32>,
    clip_from_view: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    light_from_world: mat4x4<f32>,
    camera_position_exposure: vec4<f32>,
    viewport_near_far: vec4<f32>,
    color_management: vec4<f32>,
    white_balance: vec4<f32>,
    lighting: LightingUniform,
    clipping_planes: array<vec4<f32>, 16>,
    clipping_control: vec4<f32>,
};

struct DrawUniform {
    world_from_model: mat4x4<f32>,
    normal_from_model: mat4x4<f32>,
    tint: vec4<f32>,
};

struct MaterialUniform {
    base_color_uv_offset_scale: vec4<f32>,
    base_color_uv_rotation: vec4<f32>,
    base_color_factor: vec4<f32>,
    emissive_strength: vec4<f32>,
    metallic_roughness_alpha: vec4<f32>,
    material_layer_index: vec4<u32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var base_color_sampler: sampler;

@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;

@group(1) @binding(2)
var<uniform> material: MaterialUniform;

@group(2) @binding(0)
var<uniform> draw: DrawUniform;

fn clipped_by_scene(world_position: vec3<f32>) -> bool {
    let plane_count = i32(clamp(camera.clipping_control.x, 0.0, 16.0));
    if plane_count <= 0 {
        return false;
    }
    var rejected = false;
    var section_inside_all = true;
    var section_seen = false;
    let section_start = i32(clamp(camera.clipping_control.y, 0.0, 16.0));
    let has_section = camera.clipping_control.w > 0.5;
    for (var index = 0; index < 16; index = index + 1) {
        if index < plane_count {
            let plane = camera.clipping_planes[index];
            let inside = dot(plane.xyz, world_position) + plane.w >= 0.0;
            if has_section && index >= section_start {
                section_seen = true;
                section_inside_all = section_inside_all && inside;
            } else {
                rejected = rejected || !inside;
            }
        }
    }
    if has_section && section_seen {
        let inverted_section = camera.clipping_control.z > 0.5;
        if inverted_section {
            rejected = rejected || section_inside_all;
        } else {
            rejected = rejected || !section_inside_all;
        }
    }
    return rejected;
}

fn world_position_of(in: VertexIn) -> vec4<f32> {
    let instance_world_from_model = mat4x4<f32>(
        in.instance_world_0,
        in.instance_world_1,
        in.instance_world_2,
        in.instance_world_3,
    );
    return draw.world_from_model * instance_world_from_model * vec4<f32>(in.position, 1.0);
}

// Camera-space depth. The clip position uses the same multiplication order as
// the colour pass so depth values stay bit-identical on low-precision drivers.
@vertex
fn vs_depth(in: VertexIn) -> VertexOut {
    let world_position = world_position_of(in);
    var out: VertexOut;
    out.position = camera.clip_from_world * world_position;
    out.world_position = world_position.xyz;
    out.tex_coord0 = in.tex_coord0;
    out.alpha = in.color.a * in.instance_tint.a;
    return out;
}

// Light-space depth for the directional shadow map.
@vertex
fn vs_shadow(in: VertexIn) -> VertexOut {
    let instance_world_from_model = mat4x4<f32>(
        in.instance_world_0,
        in.instance_world_1,
        in.instance_world_2,
        in.instance_world_3,
    );
    var out: VertexOut;
    out.position = camera.light_from_world * draw.world_from_model * instance_world_from_model * vec4<f32>(in.position, 1.0);
    out.world_position = vec3<f32>(0.0);
    out.tex_coord0 = in.tex_coord0;
    out.alpha = in.color.a * in.instance_tint.a;
    return out;
}

// True when the colour pass would discard this fragment.
fn alpha_masked_out(in: VertexOut) -> bool {
    let scaled_uv = in.tex_coord0 * material.base_color_uv_offset_scale.zw;
    let transformed_uv = vec2<f32>(
        scaled_uv.x * material.base_color_uv_rotation.y - scaled_uv.y * material.base_color_uv_rotation.x,
        scaled_uv.x * material.base_color_uv_rotation.x + scaled_uv.y * material.base_color_uv_rotation.y,
    ) + material.base_color_uv_offset_scale.xy;
    let base_alpha = textureSample(base_color_texture, base_color_sampler, transformed_uv).a *
        material.base_color_factor.a * in.alpha * draw.tint.a;
    return base_alpha < material.metallic_roughness_alpha.z;
}

fn pack_depth(depth: f32) -> vec4<f32> {
    let scaled = clamp(depth, 0.0, 1.0) * 65535.0;
    let high = floor(scaled / 256.0);
    let low = scaled - high * 256.0;
    return vec4<f32>(high / 255.0, low / 255.0, 0.0, 1.0);
}

@fragment
fn fs_depth_color(in: VertexOut) -> @location(0) vec4<f32> {
    if alpha_masked_out(in) || clipped_by_scene(in.world_position) {
        discard;
    }
    return pack_depth(in.position.z);
}

@fragment
fn fs_depth_only(in: VertexOut) {
    if alpha_masked_out(in) || clipped_by_scene(in.world_position) {
        discard;
    }
}

@fragment
fn fs_shadow(in: VertexOut) {
    if alpha_masked_out(in) {
        discard;
    }
}
