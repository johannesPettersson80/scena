const PI: f32 = 3.141592653589793;
const MAX_GPU_LIGHTS_PER_TYPE: u32 = 16u;
const MAX_GPU_AREA_LIGHTS: u32 = 4u;
const AREA_LIGHT_SAMPLE_COUNT: u32 = 16u;
const MAX_TILED_GPU_LIGHTS_PER_TILE: u32 = 32u;
const ENVIRONMENT_PREFILTER_MAX_MIP: f32 = 4.0;

// Scene-union material feature specialization. Each flag folds one optional
// material system out of the compiled fragment shader when no material in the
// prepared scene can produce a nonzero factor for it. The full shader fails
// Mesa v3d's 4-thread register allocation and walks a fallback ladder whose
// cost exceeds Chromium's 30-second GPU watchdog on Raspberry Pi 5 WebGL2,
// killing the GPU process; with unused features folded off it allocates on
// the first attempt. Defaults keep the full shader so callers that pass no
// pipeline constants are unchanged.
override scena_material_clearcoat: bool = true;
override scena_material_sheen: bool = true;
override scena_material_anisotropy: bool = true;
override scena_material_iridescence: bool = true;
override scena_material_dispersion: bool = true;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tex_coord0: vec2<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) baked_visibility: vec2<f32>,
    @location(6) instance_world_0: vec4<f32>,
    @location(7) instance_world_1: vec4<f32>,
    @location(8) instance_world_2: vec4<f32>,
    @location(9) instance_world_3: vec4<f32>,
    @location(10) instance_normal_0: vec4<f32>,
    @location(11) instance_normal_1: vec4<f32>,
    @location(12) instance_normal_2: vec4<f32>,
    @location(13) instance_normal_3: vec4<f32>,
    @location(14) instance_tint: vec4<f32>,
    @location(15) instance_semantic_id: vec4<f32>,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord0: vec2<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) baked_visibility: vec2<f32>,
    @location(6) instance_tint: vec4<f32>,
    @location(7) semantic_id: vec4<f32>,
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
    // Order-2 spherical-harmonics irradiance; see `environment_irradiance`.
    environment_sh: array<vec4<f32>, 9>,
    // Directional shadow cascades; see shadow_filter_cascaded.wgsl.
    shadow_cascades: array<mat4x4<f32>, 4>,
    // View-space far distance of each cascade.
    shadow_splits: vec4<f32>,
    // .x = cascade count, .y = atlas scale per cascade,
    // .z = normal offset in texels, .w = light angular-radius tangent.
    shadow_params: vec4<f32>,
};

struct TiledLightRecord {
    position_intensity: vec4<f32>,
    direction_kind: vec4<f32>,
    color_range: vec4<f32>,
    cone: vec4<f32>,
};

struct TiledLightTile {
    offset_count: vec4<u32>,
};

struct DrawUniform {
    world_from_model: mat4x4<f32>,
    normal_from_model: mat4x4<f32>,
    tint: vec4<f32>,
    semantic_id: vec4<f32>,
    reflection_probe_bounds_min: vec4<f32>,
    reflection_probe_bounds_max: vec4<f32>,
    reflection_probe_capture: vec4<f32>,
};

struct MaterialUniform {
    base_color_uv_offset_scale: vec4<f32>,
    base_color_uv_rotation: vec4<f32>,
    base_color_factor: vec4<f32>,
    emissive_strength: vec4<f32>,
    metallic_roughness_alpha: vec4<f32>,
    // Plan line 778 / RFC 866 commit 2: layer index in the shared
    // `texture_2d_array<f32>` per role. Per-material fall-back leaves this
    // at 0 (each material owns a 1-layer array). When N materials collapse
    // into one bind group, the dynamic-offset uniform points at the layer's
    // 256-byte slot and `material_layer_index.x` selects the array slice.
    material_layer_index: vec4<u32>,
    // Phase 5.1: glTF spec scalar texture strengths.
    // .x = normalTexture.scale   (default 1.0)
    // .y = occlusionTexture.strength (default 1.0)
    // .z, .w = reserved
    texture_strengths: vec4<f32>,
    // KHR_materials_clearcoat scalar factors.
    // .x = clearcoatFactor
    // .y = clearcoatRoughnessFactor
    // .z = clearcoatNormalTexture.scale
    // .w = reserved
    clearcoat_factors: vec4<f32>,
    // KHR_materials_sheen scalar factors.
    // .rgb = sheenColorFactor
    // .a = sheenRoughnessFactor
    sheen_factors: vec4<f32>,
    // KHR_materials_anisotropy scalar factors.
    // .x = anisotropyStrength
    // .y = anisotropyRotation radians
    // .z, .w = reserved
    anisotropy_factors: vec4<f32>,
    // KHR_materials_iridescence scalar factors.
    // .x = iridescenceFactor
    // .y = iridescenceIor
    // .z = iridescenceThicknessMinimum
    // .w = iridescenceThicknessMaximum
    iridescence_factors: vec4<f32>,
    // KHR_materials_dispersion scalar factors.
    // .x = dispersion
    // .y = KHR_materials_ior.ior for channel spread
    // .z, .w = reserved
    dispersion_factors: vec4<f32>,
    // KHR_materials_transmission / volume scalar factors.
    // .x = transmissionFactor
    // .y = KHR_materials_ior.ior
    // .z = thicknessFactor
    // .w = attenuationDistance
    transmission_factors: vec4<f32>,
    // KHR_materials_volume attenuation color.
    // .rgb = attenuationColor
    // .a = reserved
    attenuation_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Phase 1B step 2: directional shadow map + comparison sampler. The shadow
// caster pass writes the texture; the fragment samples with comparison
// against the receiver depth in light-clip space.
@group(0) @binding(1)
var shadow_map: texture_depth_2d;

@group(0) @binding(2)
var shadow_sampler: sampler_comparison;

// Phase 1C step 1: real environment cubemap. Six faces of decoded radiance
// drive prefiltered specular IBL. Diffuse IBL uses prepared irradiance from
// environment_diffuse_intensity.rgb; the 1×1 placeholder is never sampled
// because environment_diffuse_intensity.w gates whether IBL contributes at all.
@group(0) @binding(3)
var environment_cubemap: texture_cube<f32>;

@group(0) @binding(4)
var environment_sampler: sampler;

@group(0) @binding(5)
var<storage, read> tiled_light_records: array<TiledLightRecord>;

@group(0) @binding(6)
var transmission_color_texture: texture_2d<f32>;

@group(0) @binding(7)
var transmission_color_sampler: sampler;

@group(0) @binding(8)
var<storage, read> light_tile_indices: array<u32>;

@group(0) @binding(9)
var<storage, read> light_tiles: array<TiledLightTile>;

// Screen-space ambient visibility from `ambient_occlusion.wgsl`: one f32 per
// target pixel in row-major order, or a single 1.0 when the pass is off.
@group(0) @binding(12)
var<storage, read> ambient_visibility: array<f32>;

// Share of the screen-space occlusion applied to lights that cast no shadow.
const UNSHADOWED_LIGHT_OCCLUSION: f32 = 0.7;

// Visibility of environment light at this fragment. Only surfaces written by
// the depth prepass own the pixel under them, so blended and transmissive
// fragments keep full ambient light instead of inheriting what lies behind.
fn screen_space_visibility(fragment_position: vec4<f32>, owns_depth: bool) -> f32 {
    let index = u32(fragment_position.y) * u32(camera.viewport_near_far.x)
        + u32(fragment_position.x);
    if !owns_depth || index >= arrayLength(&ambient_visibility) {
        return 1.0;
    }
    return ambient_visibility[index];
}

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

@group(1) @binding(0)
var base_color_sampler: sampler;

@group(1) @binding(1)
var base_color_texture: texture_2d_array<f32>;

@group(1) @binding(2)
var<uniform> material: MaterialUniform;

@group(1) @binding(3)
var normal_sampler: sampler;

@group(1) @binding(4)
var normal_texture: texture_2d_array<f32>;

@group(1) @binding(5)
var metallic_roughness_sampler: sampler;

@group(1) @binding(6)
var metallic_roughness_texture: texture_2d_array<f32>;

@group(1) @binding(7)
var occlusion_sampler: sampler;

@group(1) @binding(8)
var occlusion_texture: texture_2d_array<f32>;

@group(1) @binding(9)
var emissive_sampler: sampler;

@group(1) @binding(10)
var emissive_texture: texture_2d_array<f32>;

@group(1) @binding(11)
var clearcoat_sampler: sampler;

@group(1) @binding(12)
var clearcoat_texture: texture_2d_array<f32>;

@group(1) @binding(13)
var clearcoat_roughness_sampler: sampler;

@group(1) @binding(14)
var clearcoat_roughness_texture: texture_2d_array<f32>;

@group(1) @binding(15)
var clearcoat_normal_sampler: sampler;

@group(1) @binding(16)
var clearcoat_normal_texture: texture_2d_array<f32>;

@group(1) @binding(17)
var sheen_color_sampler: sampler;

@group(1) @binding(18)
var sheen_color_texture: texture_2d_array<f32>;

@group(1) @binding(19)
var sheen_roughness_sampler: sampler;

@group(1) @binding(20)
var sheen_roughness_texture: texture_2d_array<f32>;

@group(1) @binding(21)
var anisotropy_sampler: sampler;

@group(1) @binding(22)
var anisotropy_texture: texture_2d_array<f32>;

@group(1) @binding(23)
var iridescence_sampler: sampler;

@group(1) @binding(24)
var iridescence_texture: texture_2d_array<f32>;

@group(1) @binding(25)
var iridescence_thickness_sampler: sampler;

@group(1) @binding(26)
var iridescence_thickness_texture: texture_2d_array<f32>;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let instance_world_from_model = mat4x4<f32>(
        in.instance_world_0,
        in.instance_world_1,
        in.instance_world_2,
        in.instance_world_3,
    );
    let instance_normal_from_model = mat4x4<f32>(
        in.instance_normal_0,
        in.instance_normal_1,
        in.instance_normal_2,
        in.instance_normal_3,
    );
    let world_position = draw.world_from_model * instance_world_from_model * vec4<f32>(in.position, 1.0);
    let normal_from_model = draw.normal_from_model * instance_normal_from_model;
    out.position = camera.clip_from_world * world_position;
    out.color = in.color;
    out.normal = (normal_from_model * vec4<f32>(in.normal, 0.0)).xyz;
    out.tex_coord0 = in.tex_coord0;
    out.world_position = world_position.xyz;
    out.tangent = vec4<f32>((normal_from_model * vec4<f32>(in.tangent.xyz, 0.0)).xyz, in.tangent.w);
    out.baked_visibility = clamp(in.baked_visibility, vec2<f32>(0.0), vec2<f32>(1.0));
    out.instance_tint = in.instance_tint;
    out.semantic_id = select(draw.semantic_id, in.instance_semantic_id, in.instance_semantic_id.a > 0.5);
    return out;
}

struct SemanticOutput {
    @location(0) id: vec4<f32>,
    @location(1) depth: vec4<f32>,
    @location(2) normal: vec4<f32>,
};

fn encode_semantic_depth(view_depth: f32) -> vec4<f32> {
    let near = camera.viewport_near_far.z;
    let far = camera.viewport_near_far.w;
    let normalized = clamp((view_depth - near) / max(far - near, 0.000001), 0.0, 1.0);
    let code = floor(1.0 + normalized * 16777214.0 + 0.5);
    let red = code - floor(code / 256.0) * 256.0;
    let green_code = floor(code / 256.0);
    let green = green_code - floor(green_code / 256.0) * 256.0;
    let blue = floor(code / 65536.0);
    return vec4<f32>(red, green, blue, 255.0) / 255.0;
}

@fragment
fn fs_semantic(in: VertexOut) -> SemanticOutput {
    if in.semantic_id.a < 0.5 || clipped_by_scene(in.world_position) {
        discard;
    }
    let scaled_uv = in.tex_coord0 * material.base_color_uv_offset_scale.zw;
    let transformed_uv = vec2<f32>(
        scaled_uv.x * material.base_color_uv_rotation.y - scaled_uv.y * material.base_color_uv_rotation.x,
        scaled_uv.x * material.base_color_uv_rotation.x + scaled_uv.y * material.base_color_uv_rotation.y,
    ) + material.base_color_uv_offset_scale.xy;
    let material_layer = i32(material.material_layer_index.x);
    let base_color_sample = textureSample(base_color_texture, base_color_sampler, transformed_uv, material_layer);
    let base = in.color * material.base_color_factor * base_color_sample * draw.tint * in.instance_tint;
    if material.metallic_roughness_alpha.z > 0.0 && base.a < material.metallic_roughness_alpha.z {
        discard;
    }
    let view_position = camera.view_from_world * vec4<f32>(in.world_position, 1.0);
    let view_depth = -view_position.z;
    if view_depth <= 0.0 {
        discard;
    }
    var world_normal = vec3<f32>(0.0);
    let normal_length = length(in.normal);
    if normal_length > 0.000001 {
        world_normal = in.normal / normal_length;
    }
    var output: SemanticOutput;
    output.id = in.semantic_id;
    output.depth = encode_semantic_depth(view_depth);
    output.normal = vec4<f32>(world_normal * 0.5 + vec3<f32>(0.5), 1.0);
    return output;
}

fn shade_fragment(in: VertexOut, front_facing: bool) -> vec4<f32> {
    if clipped_by_scene(in.world_position) {
        discard;
    }
    let scaled_uv = in.tex_coord0 * material.base_color_uv_offset_scale.zw;
    let transformed_uv = vec2<f32>(
        scaled_uv.x * material.base_color_uv_rotation.y - scaled_uv.y * material.base_color_uv_rotation.x,
        scaled_uv.x * material.base_color_uv_rotation.x + scaled_uv.y * material.base_color_uv_rotation.y,
    ) + material.base_color_uv_offset_scale.xy;
    let material_layer = i32(material.material_layer_index.x);
    let base_color_sample = textureSample(base_color_texture, base_color_sampler, transformed_uv, material_layer);
    let normal_texture_sample = textureSample(normal_texture, normal_sampler, in.tex_coord0, material_layer).rgb;
    let metallic_roughness_sample = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.tex_coord0, material_layer);
    let occlusion_sample = textureSample(occlusion_texture, occlusion_sampler, in.tex_coord0, material_layer).r;
    let emissive_sample = textureSample(emissive_texture, emissive_sampler, in.tex_coord0, material_layer).rgb;
    // A folded-off feature must not sample its textures at all. An
    // unconditional sample whose result feeds only disabled code stays in
    // ANGLE's active-sampler list while Mesa v3d dead-codes its uniform
    // location; wgpu can only assign units to locatable samplers, so the
    // leftovers sit on default unit 0 and type-conflict with the shadow
    // sampler bound there — GL then rejects every draw with
    // INVALID_OPERATION and the geometry silently vanishes.
    var clearcoat_sample = vec4<f32>(0.0);
    var clearcoat_roughness_sample = vec4<f32>(0.0);
    var clearcoat_normal_sample = vec3<f32>(0.0, 0.0, 1.0);
    if scena_material_clearcoat {
        clearcoat_sample = textureSample(clearcoat_texture, clearcoat_sampler, in.tex_coord0, material_layer);
        clearcoat_roughness_sample = textureSample(clearcoat_roughness_texture, clearcoat_roughness_sampler, in.tex_coord0, material_layer);
        clearcoat_normal_sample = textureSample(clearcoat_normal_texture, clearcoat_normal_sampler, in.tex_coord0, material_layer).rgb;
    }
    var sheen_color_sample = vec4<f32>(0.0);
    var sheen_roughness_sample = vec4<f32>(0.0);
    if scena_material_sheen {
        sheen_color_sample = textureSample(sheen_color_texture, sheen_color_sampler, in.tex_coord0, material_layer);
        sheen_roughness_sample = textureSample(sheen_roughness_texture, sheen_roughness_sampler, in.tex_coord0, material_layer);
    }
    var anisotropy_sample = vec4<f32>(0.5, 0.5, 0.0, 0.0);
    if scena_material_anisotropy {
        anisotropy_sample = textureSample(anisotropy_texture, anisotropy_sampler, in.tex_coord0, material_layer);
    }
    var iridescence_sample = vec4<f32>(0.0);
    var iridescence_thickness_sample = vec4<f32>(0.0);
    if scena_material_iridescence {
        iridescence_sample = textureSample(iridescence_texture, iridescence_sampler, in.tex_coord0, material_layer);
        iridescence_thickness_sample = textureSample(iridescence_thickness_texture, iridescence_thickness_sampler, in.tex_coord0, material_layer);
    }
    // Phase 5.1: apply normalTexture.scale to the tangent-space X/Y
    // components before TBN reconstruction. Z stays unscaled so the
    // unit-length invariant holds after normalize().
    let raw_normal = normal_texture_sample * 2.0 - vec3<f32>(1.0);
    let normal_scale = material.texture_strengths.x;
    let scaled_tangent_normal = vec3<f32>(
        raw_normal.x * normal_scale,
        raw_normal.y * normal_scale,
        raw_normal.z,
    );
    let normal_sample = normalize(scaled_tangent_normal);
    // glTF: back faces of double-sided surfaces shade with the reversed frame.
    let facing = select(-1.0, 1.0, front_facing);
    let world_normal = normalize(in.normal) * facing;
    let world_tangent = normalize(in.tangent.xyz) * facing;
    let bitangent = normalize(cross(world_normal, world_tangent) * in.tangent.w) * facing;
    let normal = normalize(normal_sample.x * world_tangent + normal_sample.y * bitangent + normal_sample.z * world_normal);
    // Auto-generated micro detail is smaller than a normal product-still pixel.
    // Fold its variance into roughness instead of resolving world-space waves
    // that alias into coherent contour bands under perspective.
    let unresolved_micro_roughness = material.texture_strengths.z * 0.175;
    var clearcoat_factor = 0.0;
    if scena_material_clearcoat {
        clearcoat_factor = clamp(material.clearcoat_factors.x * clearcoat_sample.r, 0.0, 1.0);
    }
    let clearcoat_roughness = clamp(material.clearcoat_factors.y * clearcoat_roughness_sample.g, 0.04, 1.0);
    let clearcoat_normal_scale = material.clearcoat_factors.z;
    let raw_clearcoat_normal = clearcoat_normal_sample * 2.0 - vec3<f32>(1.0);
    let scaled_clearcoat_normal = vec3<f32>(
        raw_clearcoat_normal.x * clearcoat_normal_scale,
        raw_clearcoat_normal.y * clearcoat_normal_scale,
        raw_clearcoat_normal.z,
    );
    let clearcoat_tangent_normal = normalize(scaled_clearcoat_normal);
    let clearcoat_normal = normalize(clearcoat_tangent_normal.x * world_tangent + clearcoat_tangent_normal.y * bitangent + clearcoat_tangent_normal.z * world_normal);
    var sheen_color = vec3<f32>(0.0);
    if scena_material_sheen {
        sheen_color = material.sheen_factors.rgb * sheen_color_sample.rgb;
    }
    let sheen_roughness = clamp(material.sheen_factors.a * sheen_roughness_sample.a, 0.04, 1.0);
    let anisotropy_direction = anisotropy_sample.rg * 2.0 - vec2<f32>(1.0, 1.0);
    var anisotropy_strength = 0.0;
    if scena_material_anisotropy {
        anisotropy_strength = clamp(material.anisotropy_factors.x * anisotropy_sample.b, 0.0, 1.0);
    }
    var iridescence_factor = 0.0;
    if scena_material_iridescence {
        iridescence_factor = clamp(material.iridescence_factors.x * iridescence_sample.r, 0.0, 1.0);
    }
    let iridescence_thickness = mix(material.iridescence_factors.z, material.iridescence_factors.w, clamp(iridescence_thickness_sample.g, 0.0, 1.0));
    var dispersion_factor = 0.0;
    if scena_material_dispersion {
        dispersion_factor = max(material.dispersion_factors.x, 0.0);
    }
    let dispersion_ior = max(material.dispersion_factors.y, 1.0);
    let metallic = clamp(material.metallic_roughness_alpha.x * metallic_roughness_sample.b, 0.0, 1.0);
    let roughness = clamp(
        material.metallic_roughness_alpha.y * metallic_roughness_sample.g
            + unresolved_micro_roughness,
        0.04,
        1.0,
    );
    // Phase 5.1: occlusionTexture.strength lerps between 1.0 and the
    // sampled occlusion. strength=0 disables AO; strength=1 applies it
    // at full intensity. glTF spec default = 1.0.
    let occlusion_strength = material.texture_strengths.y;
    let occlusion_applied = mix(1.0, occlusion_sample, occlusion_strength);
    let indirect_occlusion = occlusion_applied * clamp(in.baked_visibility.y, 0.0, 1.0);
    let base = in.color * material.base_color_factor * base_color_sample * draw.tint * in.instance_tint;
    if material.metallic_roughness_alpha.z > 0.0 && base.a < material.metallic_roughness_alpha.z {
        discard;
    }
    // glTF alpha modes: OPAQUE and MASK surfaces are fully opaque whatever
    // alpha their base colour carries; only a ghosting tint keeps them
    // see-through. BLEND surfaces use the full base alpha.
    var surface_alpha = base.a;
    if material.base_color_uv_rotation.z < 1.5 {
        surface_alpha = draw.tint.a * in.instance_tint.a;
    }
    let emissive = material.emissive_strength.rgb * emissive_sample * material.emissive_strength.w;
    let view = normalize(camera.camera_position_exposure.xyz - in.world_position);
    let ambient_visibility = screen_space_visibility(
        in.position,
        surface_alpha >= 0.999 && material.transmission_factors.x <= 0.0,
    );
    // An unlit colour is all the light the surface has, so occlusion scales
    // it directly; lit surfaces lose only ambient and unshadowed light.
    var shaded_rgb = base.rgb * ambient_visibility;
    var coat_attenuation = 1.0;
    if material.metallic_roughness_alpha.w < 0.5 {
        var surface: Surface;
        surface.base = base.rgb;
        surface.metallic = metallic;
        surface.diffuse_color = base.rgb * (1.0 - metallic);
        surface.f0 = mix(vec3<f32>(f0_from_ior(material.transmission_factors.y)), base.rgb, metallic);
        surface.f90 = 1.0;
        surface.roughness = roughness;
        surface.alpha = roughness * roughness;
        surface.normal = normal;
        surface.view = view;
        surface.n_dot_v = max(dot(normal, view), 0.001);
        surface.clearcoat = clearcoat_factor;
        surface.clearcoat_roughness = clearcoat_roughness;
        surface.clearcoat_normal = clearcoat_normal;
        surface.clearcoat_attenuation = 1.0 -
            clearcoat_factor * fresnel_schlick(max(dot(clearcoat_normal, view), 0.0), vec3<f32>(0.04)).x;
        surface.sheen_color = clamp(sheen_color, vec3<f32>(0.0), vec3<f32>(1.0));
        surface.sheen_roughness = max(sheen_roughness, MIN_SHEEN_ROUGHNESS);
        surface.sheen_scaling = 1.0 - SHEEN_ENERGY_COMPENSATION *
            max(surface.sheen_color.r, max(surface.sheen_color.g, surface.sheen_color.b));
        surface.anisotropy = anisotropy_strength;
        surface.anisotropy_t = world_tangent;
        surface.anisotropy_b = bitangent;
        if scena_material_anisotropy {
            if anisotropy_strength > 0.0 {
                surface.anisotropy_t = surface_anisotropy_tangent(
                    normal,
                    world_tangent,
                    in.tangent.w,
                    anisotropy_direction,
                    material.anisotropy_factors.y,
                );
                surface.anisotropy_b = normalize(cross(normal, surface.anisotropy_t));
            }
        }
        surface.iridescence = 0.0;
        surface.iridescence_fresnel = vec3<f32>(0.0);
        if scena_material_iridescence {
            if iridescence_factor > 0.0 {
                surface.iridescence = iridescence_factor;
                surface.iridescence_fresnel = iridescence_fresnel(
                    material.iridescence_factors.y,
                    surface.n_dot_v,
                    iridescence_thickness,
                    surface.f0,
                );
            }
        }
        surface.dispersion = dispersion_factor;
        surface.dispersion_f0 = dispersion_f0_from_ior(dispersion_ior, dispersion_factor);
        coat_attenuation = surface.clearcoat_attenuation;
        shaded_rgb = base.rgb * (occlusion_applied * ambient_visibility);
        // Lights without a shadow map have no other occlusion: the screen
        // visibility stands in for the soft shadow a fill light would cast.
        let direct = surface_punctual_lighting(
            surface,
            in.world_position,
            world_normal,
            in.position,
            in.baked_visibility.x,
            mix(1.0, ambient_visibility, UNSHADOWED_LIGHT_OCCLUSION),
        );
        let environment = surface_environment_light(surface, in.world_position);
        if has_punctual_light() || has_environment_light() {
            shaded_rgb = direct + environment * (indirect_occlusion * ambient_visibility);
        }
    }
    let shaded = vec4<f32>(shaded_rgb + emissive * coat_attenuation, surface_alpha);
    let color_management_mode = camera.color_management.x;
    var output_rgb = shaded.rgb;
    if camera.color_management.y >= -0.5 {
        output_rgb = apply_tonemapper(
            shaded.rgb * camera.white_balance.rgb * camera.camera_position_exposure.w,
            color_management_mode,
        );
    }
    output_rgb = screen_space_material_reflection(
        in.position.xy,
        normal,
        view,
        metallic,
        roughness,
        output_rgb,
    );
    let transmitted = physical_transmission_color(
        in.position.xy,
        normal,
        view,
        base.rgb,
        roughness,
        output_rgb,
    );
    if transmitted.a > 0.0 {
        return vec4<f32>(encode_post_target_rgb(transmitted.rgb, camera.color_management.y), transmitted.a);
    }
    return vec4<f32>(encode_post_target_rgb(output_rgb, camera.color_management.y), shaded.a);
}

// One area light: analytic LTC specular plus point-sampled diffuse and
// coating lobes. The point samples skip the base specular lobe, which the LTC
// integral already covers.
fn surface_area_light(
    index: u32,
    s: Surface,
    world_position: vec3<f32>,
    area_shadow_visibility: f32,
) -> vec3<f32> {
    var shaded = ltc_area_light_specular_contribution(
        index,
        s.base,
        s.metallic,
        s.roughness,
        s.normal,
        s.view,
        world_position,
        area_shadow_visibility,
    ) * s.clearcoat_attenuation;
    for (var sample = 0u; sample < AREA_LIGHT_SAMPLE_COUNT; sample = sample + 1u) {
        let sample_position = area_light_sample_position(index, sample);
        let incoming = normalize(sample_position - world_position);
        let radiance = area_light_radiance(index, sample_position, world_position) * area_shadow_visibility;
        shaded += surface_direct_light(s, incoming, radiance, 0.0);
    }
    return shaded;
}

// Direct lighting from every analytic light in the scene.
// `geometric_normal` is the interpolated surface normal before normal mapping;
// shadow bias uses it so normal-map detail cannot push samples under the
// surface.
fn surface_punctual_lighting(
    s: Surface,
    world_position: vec3<f32>,
    geometric_normal: vec3<f32>,
    fragment_position: vec4<f32>,
    shadow_visibility: f32,
    unshadowed_visibility: f32,
) -> vec3<f32> {
    var shaded = vec3<f32>(0.0);
    // Directional shadows use the GPU shadow map. The vertex stream's
    // shadow_visibility carries deterministic area-light sample visibility so
    // CPU and GPU agree on the soft penumbra of finite emitters.
    let area_shadow_visibility = clamp(shadow_visibility, 0.0, 1.0);
    if tiled_lighting_active() {
        let tile = tiled_light_tile(fragment_position);
        let tile_count = min(tile.offset_count.y, MAX_TILED_GPU_LIGHTS_PER_TILE);
        for (var i = 0u; i < MAX_TILED_GPU_LIGHTS_PER_TILE; i = i + 1u) {
            if i < tile_count {
                let record_index = light_tile_indices[tile.offset_count.x + i];
                let record = tiled_light_records[record_index];
                let kind = record.direction_kind.w;
                var incoming = vec3<f32>(0.0, 0.0, 1.0);
                var radiance = vec3<f32>(0.0);
                if kind < 0.5 {
                    incoming = normalize(-record.position_intensity.xyz);
                    radiance = record.color_range.rgb * record.position_intensity.w;
                    // cone.z marks the directional light that owns the shadow map.
                    if record.cone.z > 0.5 {
                        radiance *= directional_shadow_factor(world_position, geometric_normal, incoming, fragment_position.xy);
                    } else {
                        radiance *= unshadowed_visibility;
                    }
                } else if kind < 1.5 {
                    let to_light = record.position_intensity.xyz - world_position;
                    incoming = normalize(to_light);
                    let attenuation = distance_attenuation(to_light, record.color_range.w);
                    radiance = record.color_range.rgb * record.position_intensity.w * attenuation
                        * unshadowed_visibility;
                } else {
                    let to_light = record.position_intensity.xyz - world_position;
                    incoming = normalize(to_light);
                    let to_surface = -incoming;
                    let cone = spot_cone_attenuation(
                        dot(to_surface, normalize(record.direction_kind.xyz)),
                        record.cone.x,
                        record.cone.y,
                    );
                    let attenuation = distance_attenuation(to_light, record.color_range.w);
                    radiance = record.color_range.rgb * record.position_intensity.w * attenuation * cone
                        * unshadowed_visibility;
                }
                shaded += surface_direct_light(s, incoming, radiance, 1.0);
            }
        }
    }
    let directional_count = min(u32(camera.lighting.light_counts.x), MAX_GPU_LIGHTS_PER_TYPE);
    for (var i = 0u; i < MAX_GPU_LIGHTS_PER_TYPE; i = i + 1u) {
        if i < directional_count {
            let light_direction = camera.lighting.directional_light_direction_intensity[i];
            let incoming = normalize(-light_direction.xyz);
            var radiance = camera.lighting.directional_light_color[i].rgb * light_direction.w;
            if camera.lighting.directional_shadow_control[i].x > 0.5 {
                radiance *= directional_shadow_factor(world_position, geometric_normal, incoming, fragment_position.xy);
            } else {
                radiance *= unshadowed_visibility;
            }
            shaded += surface_direct_light(s, incoming, radiance, 1.0);
        }
    }
    let point_count = min(u32(camera.lighting.light_counts.y), MAX_GPU_LIGHTS_PER_TYPE);
    for (var i = 0u; i < MAX_GPU_LIGHTS_PER_TYPE; i = i + 1u) {
        if i < point_count {
            let point_light = camera.lighting.point_light_position_intensity[i];
            let point_color = camera.lighting.point_light_color_range[i];
            let to_light = point_light.xyz - world_position;
            let incoming = normalize(to_light);
            let attenuation = distance_attenuation(to_light, point_color.w);
            let radiance = point_color.rgb * point_light.w * attenuation * unshadowed_visibility;
            shaded += surface_direct_light(s, incoming, radiance, 1.0);
        }
    }
    let spot_count = min(u32(camera.lighting.light_counts.z), MAX_GPU_LIGHTS_PER_TYPE);
    for (var i = 0u; i < MAX_GPU_LIGHTS_PER_TYPE; i = i + 1u) {
        if i < spot_count {
            let spot_light = camera.lighting.spot_light_position_intensity[i];
            let spot_direction = camera.lighting.spot_light_direction_cones[i];
            let spot_cones = camera.lighting.spot_light_cone_range[i];
            let to_light = spot_light.xyz - world_position;
            let incoming = normalize(to_light);
            let to_surface = -incoming;
            let cone = spot_cone_attenuation(
                dot(to_surface, normalize(spot_direction.xyz)),
                spot_cones.x,
                spot_cones.y,
            );
            let attenuation = distance_attenuation(to_light, spot_cones.z);
            let radiance = camera.lighting.spot_light_color_range[i].rgb *
                spot_light.w * attenuation * cone * unshadowed_visibility;
            shaded += surface_direct_light(s, incoming, radiance, 1.0);
        }
    }
    let area_count = min(u32(camera.lighting.light_counts.w), MAX_GPU_AREA_LIGHTS);
    for (var i = 0u; i < MAX_GPU_AREA_LIGHTS; i = i + 1u) {
        if i < area_count {
            shaded += surface_area_light(i, s, world_position, area_shadow_visibility);
        }
    }
    return shaded;
}

struct BeautySemanticOutput {
    @location(0) color: vec4<f32>,
    @location(1) semantic_id: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    return shade_fragment(in, front_facing);
}

@fragment
fn fs_beauty_semantic(in: VertexOut, @builtin(front_facing) front_facing: bool) -> BeautySemanticOutput {
    var output: BeautySemanticOutput;
    output.color = shade_fragment(in, front_facing);
    output.semantic_id = in.semantic_id;
    return output;
}

fn has_punctual_light() -> bool {
    return tiled_lighting_active() ||
        camera.lighting.light_counts.x > 0.0 ||
        camera.lighting.light_counts.y > 0.0 ||
        camera.lighting.light_counts.z > 0.0 ||
        camera.lighting.light_counts.w > 0.0;
}

fn tiled_lighting_active() -> bool {
    return light_tiles[0].offset_count.w > 0u;
}

fn tiled_light_tile(fragment_position: vec4<f32>) -> TiledLightTile {
    let metadata = light_tiles[0].offset_count;
    let columns = max(metadata.x, 1u);
    let rows = max(metadata.y, 1u);
    let tile_size = max(metadata.z, 1u);
    let tile_x = min(u32(max(fragment_position.x, 0.0)) / tile_size, columns - 1u);
    let tile_y = min(u32(max(fragment_position.y, 0.0)) / tile_size, rows - 1u);
    return light_tiles[1u + tile_y * columns + tile_x];
}
