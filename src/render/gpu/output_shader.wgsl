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

fn shade_fragment(in: VertexOut) -> vec4<f32> {
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
    let world_normal = normalize(in.normal);
    let world_tangent = normalize(in.tangent.xyz);
    let bitangent = normalize(cross(world_normal, world_tangent) * in.tangent.w);
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
    let emissive = material.emissive_strength.rgb * emissive_sample * material.emissive_strength.w;
    let view = normalize(camera.camera_position_exposure.xyz - in.world_position);
    var shaded_rgb = base.rgb;
    if material.metallic_roughness_alpha.w < 0.5 {
        shaded_rgb = base.rgb * occlusion_applied;
        let direct = pbr_punctual_lighting(
            base.rgb,
            metallic,
            roughness,
            normal,
            view,
            clearcoat_normal,
            clearcoat_factor,
            clearcoat_roughness,
            sheen_color,
            sheen_roughness,
            world_tangent,
            in.tangent.w,
            anisotropy_strength,
            material.anisotropy_factors.y,
            anisotropy_direction,
            iridescence_factor,
            material.iridescence_factors.y,
            iridescence_thickness,
            dispersion_factor,
            dispersion_ior,
            in.world_position,
            in.position,
            in.baked_visibility.x,
        );
        var environment = pbr_environment_lighting(base.rgb, metallic, roughness, normal, view, in.world_position);
        environment += clearcoat_environment_lighting(clearcoat_normal, view, clearcoat_factor, clearcoat_roughness, in.world_position);
        environment += sheen_environment_lighting(normal, view, sheen_color, sheen_roughness);
        environment += anisotropy_environment_lighting(base.rgb, metallic, roughness, normal, world_tangent, in.tangent.w, view, anisotropy_strength, material.anisotropy_factors.y, anisotropy_direction, in.world_position);
        if has_punctual_light() || has_environment_light() {
            shaded_rgb = direct + environment * indirect_occlusion;
        }
    }
    let shaded = vec4<f32>(shaded_rgb + emissive, base.a);
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

fn physical_transmission_color(
    frag_coord: vec2<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    tint: vec3<f32>,
    roughness: f32,
    surface_rgb: vec3<f32>,
) -> vec4<f32> {
    let transmission = clamp(material.transmission_factors.x, 0.0, 1.0);
    if transmission <= 0.001 {
        return vec4<f32>(0.0);
    }
    let viewport = max(camera.viewport_near_far.xy, vec2<f32>(1.0, 1.0));
    let uv = clamp(frag_coord / viewport, vec2<f32>(0.001), vec2<f32>(0.999));
    let ior = max(material.transmission_factors.y, 1.01);
    let thickness = max(material.transmission_factors.z, 0.0);
    let view_dir = normalize(view);
    let normal_dir = normalize(normal);
    let n_dot_v = max(dot(normal_dir, view_dir), 0.0);
    let rim_fresnel = pow(1.0 - n_dot_v, 5.0);
    let refracted = refract(-view_dir, normal_dir, 1.0 / ior);
    let thickness_scale = 0.004 + min(thickness, 1.0) * 0.028;
    let refracted_uv = clamp(
        uv + vec2<f32>(refracted.x, -refracted.y) * thickness_scale * transmission,
        vec2<f32>(0.001),
        vec2<f32>(0.999),
    );
    let texel = 1.0 / viewport;
    let blur_px = roughness * roughness * 48.0;
    let blur = texel * blur_px;
    let straight = textureSample(transmission_color_texture, transmission_color_sampler, uv).rgb;
    let refracted_center = textureSample(transmission_color_texture, transmission_color_sampler, refracted_uv).rgb;
    let refracted_blurred =
        refracted_center * 0.36 +
        textureSample(transmission_color_texture, transmission_color_sampler, refracted_uv + vec2<f32>(blur.x, 0.0)).rgb * 0.16 +
        textureSample(transmission_color_texture, transmission_color_sampler, refracted_uv - vec2<f32>(blur.x, 0.0)).rgb * 0.16 +
        textureSample(transmission_color_texture, transmission_color_sampler, refracted_uv + vec2<f32>(0.0, blur.y)).rgb * 0.16 +
        textureSample(transmission_color_texture, transmission_color_sampler, refracted_uv - vec2<f32>(0.0, blur.y)).rgb * 0.16;
    let refraction_mix = clamp(0.58 + roughness * 0.40 + rim_fresnel * 0.10, 0.58, 0.96);
    let scene_color = mix(straight, refracted_blurred, refraction_mix);
    let tint_strength = clamp(transmission * 0.035, 0.0, 0.035);
    let volume_tint = volume_transmittance(thickness, material.attenuation_color.rgb, material.transmission_factors.w);
    let transmitted = scene_color * volume_tint * mix(vec3<f32>(1.0), tint, tint_strength);
    let reflection_weight = clamp(0.08 + rim_fresnel * 0.42 + (1.0 - transmission) * 0.10, 0.08, 0.50);
    let reflected = surface_rgb * volume_tint * (1.08 + rim_fresnel * 0.72);
    return vec4<f32>(mix(transmitted, reflected, reflection_weight), 1.0);
}

fn screen_space_material_reflection(
    frag_coord: vec2<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    metallic: f32,
    roughness: f32,
    surface_rgb: vec3<f32>,
) -> vec3<f32> {
    let strength = clamp(camera.color_management.z, 0.0, 1.0);
    let ssr_active = step(0.001, strength) * step(0.5, metallic);
    let viewport = max(camera.viewport_near_far.xy, vec2<f32>(1.0, 1.0));
    let uv = clamp(frag_coord / viewport, vec2<f32>(0.001), vec2<f32>(0.999));
    let normal_dir = normalize(normal);
    let view_dir = normalize(view);
    let reflection = reflect(-view_dir, normal_dir);
    let reflect_scale = (0.12 + (1.0 - roughness) * 0.24) * strength;
    let raw_reflected_uv = uv + vec2<f32>(reflection.x, -reflection.y) * reflect_scale;
    let edge_distance = min(
        min(raw_reflected_uv.x, raw_reflected_uv.y),
        min(1.0 - raw_reflected_uv.x, 1.0 - raw_reflected_uv.y),
    );
    let edge_fade = smoothstep(0.0, 0.02, edge_distance);
    let reflected_uv = clamp(
        raw_reflected_uv,
        vec2<f32>(0.001),
        vec2<f32>(0.999),
    );
    let texel = 1.0 / viewport;
    let blur_px = clamp(roughness * roughness * 14.0 + camera.color_management.w * 5.0, 0.0, 8.0);
    let blur = texel * blur_px;
    let reflected =
        textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv).rgb * 0.44 +
        textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv + vec2<f32>(blur.x, 0.0)).rgb * 0.14 +
        textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv - vec2<f32>(blur.x, 0.0)).rgb * 0.14 +
        textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv + vec2<f32>(0.0, blur.y)).rgb * 0.14 +
        textureSample(transmission_color_texture, transmission_color_sampler, reflected_uv - vec2<f32>(0.0, blur.y)).rgb * 0.14;
    let n_dot_v = max(dot(normal_dir, view_dir), 0.0);
    let fresnel = pow5(1.0 - n_dot_v);
    let weight = clamp(ssr_active * strength * metallic * (0.38 + fresnel * 0.52) * (1.0 - roughness * 0.55) * edge_fade, 0.0, 0.88);
    return mix(surface_rgb, reflected, weight);
}

fn volume_transmittance(
    thickness: f32,
    attenuation_color: vec3<f32>,
    attenuation_distance: f32,
) -> vec3<f32> {
    if thickness <= 0.000001 || attenuation_distance > 1.0e20 {
        return vec3<f32>(1.0);
    }
    let exponent = thickness / max(attenuation_distance, 0.0001);
    return vec3<f32>(
        pow(clamp(attenuation_color.r, 0.0, 1.0), exponent),
        pow(clamp(attenuation_color.g, 0.0, 1.0), exponent),
        pow(clamp(attenuation_color.b, 0.0, 1.0), exponent),
    );
}

fn directional_shadow_factor(world_position: vec3<f32>) -> f32 {
    // Phase 1B step 2: GPU shadow map sampling. Project the fragment's world
    // position into light-clip space, map clip [-1..1, -1..1, 0..1] to
    // texture [0..1, 0..1] (Y-flip — clip y is up, texture v is down), and
    // sample with depth-comparison.
    let light_clip = camera.light_from_world * vec4<f32>(world_position, 1.0);
    if light_clip.w <= 0.0 {
        return 1.0;
    }
    let light_ndc = light_clip.xyz / light_clip.w;
    let shadow_uv = vec2<f32>(light_ndc.x * 0.5 + 0.5, light_ndc.y * -0.5 + 0.5);
    // Receivers outside the shadow caster AABB get full radiance — the
    // sampler's ClampToEdge would otherwise read the texture border and
    // produce false self-shadow streaks (review F6).
    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 ||
       shadow_uv.y < 0.0 || shadow_uv.y > 1.0 ||
       light_ndc.z < 0.0 || light_ndc.z > 1.0 {
        return 1.0;
    }
    let shadow_texel_size = 1.0 / vec2<f32>(textureDimensions(shadow_map));
    var shadow_visibility = 0.0;
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>(-1.0, -1.0), light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 0.0, -1.0), light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 1.0, -1.0), light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>(-1.0,  0.0), light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv, light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 1.0,  0.0), light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>(-1.0,  1.0), light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 0.0,  1.0), light_ndc.z);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 1.0,  1.0), light_ndc.z);
    return shadow_visibility / 9.0;
}

fn area_light_sample_position(index: u32, sample: u32) -> vec3<f32> {
    let center = camera.lighting.area_light_position_flux[index].xyz;
    let axis_x_shape = camera.lighting.area_light_axis_x_shape[index];
    let axis_x = axis_x_shape.xyz;
    let axis_y = camera.lighting.area_light_axis_y_range[index].xyz;
    if axis_x_shape.w < 0.5 {
        let column = f32(sample % 4u);
        let row = f32(sample / 4u);
        let sx = column * 0.5 - 0.75;
        let sy = row * 0.5 - 0.75;
        return center + axis_x * sx + axis_y * sy;
    }
    if axis_x_shape.w < 1.5 {
        let offsets = array<vec2<f32>, 16>(
            vec2<f32>(0.35, 0.0),
            vec2<f32>(0.247487, 0.247487),
            vec2<f32>(0.0, 0.35),
            vec2<f32>(-0.247487, 0.247487),
            vec2<f32>(-0.35, 0.0),
            vec2<f32>(-0.247487, -0.247487),
            vec2<f32>(0.0, -0.35),
            vec2<f32>(0.247487, -0.247487),
            vec2<f32>(0.69291, 0.287013),
            vec2<f32>(0.287013, 0.69291),
            vec2<f32>(-0.287013, 0.69291),
            vec2<f32>(-0.69291, 0.287013),
            vec2<f32>(-0.69291, -0.287013),
            vec2<f32>(-0.287013, -0.69291),
            vec2<f32>(0.287013, -0.69291),
            vec2<f32>(0.69291, -0.287013),
        );
        let offset = offsets[sample];
        return center + axis_x * offset.x + axis_y * offset.y;
    }
    let radius = max(max(length(axis_x), length(axis_y)), 0.001);
    let raw_forward = cross(axis_x, axis_y);
    let forward_direction = select(
        vec3<f32>(0.0, 0.0, 1.0),
        normalize(raw_forward),
        dot(raw_forward, raw_forward) > 0.00000001,
    );
    let forward = forward_direction * radius;
    let offsets = array<vec3<f32>, 16>(
        vec3<f32>(0.577350, 0.577350, 0.577350),
        vec3<f32>(-0.577350, 0.577350, 0.577350),
        vec3<f32>(0.577350, -0.577350, 0.577350),
        vec3<f32>(-0.577350, -0.577350, 0.577350),
        vec3<f32>(0.577350, 0.577350, -0.577350),
        vec3<f32>(-0.577350, 0.577350, -0.577350),
        vec3<f32>(0.577350, -0.577350, -0.577350),
        vec3<f32>(-0.577350, -0.577350, -0.577350),
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(-1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, -1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
        vec3<f32>(0.0, 0.0, -1.0),
        vec3<f32>(0.0, 0.707107, 0.707107),
        vec3<f32>(0.0, -0.707107, -0.707107),
    );
    let offset = offsets[sample];
    return center + axis_x * offset.x + axis_y * offset.y + forward * offset.z;
}

fn area_light_radiance(index: u32, sample_position: vec3<f32>, world_position: vec3<f32>) -> vec3<f32> {
    let to_light = sample_position - world_position;
    let range = camera.lighting.area_light_axis_y_range[index].w;
    let intensity_per_sample = camera.lighting.area_light_position_flux[index].w / (4.0 * PI) / f32(AREA_LIGHT_SAMPLE_COUNT);
    return camera.lighting.area_light_color[index].rgb *
        intensity_per_sample *
        distance_attenuation(to_light, range);
}

fn pbr_punctual_lighting(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    clearcoat_normal: vec3<f32>,
    clearcoat_factor: f32,
    clearcoat_roughness: f32,
    sheen_color: vec3<f32>,
    sheen_roughness: f32,
    world_tangent: vec3<f32>,
    tangent_handedness: f32,
    anisotropy_strength: f32,
    anisotropy_rotation: f32,
    anisotropy_direction: vec2<f32>,
    iridescence_factor: f32,
    iridescence_ior: f32,
    iridescence_thickness: f32,
    dispersion_factor: f32,
    dispersion_ior: f32,
    world_position: vec3<f32>,
    fragment_position: vec4<f32>,
    shadow_visibility: f32,
) -> vec3<f32> {
    var shaded = vec3<f32>(0.0);
    // Directional shadows use the GPU shadow map below. The vertex stream's
    // shadow_visibility now carries deterministic area-light sample visibility
    // so CPU and GPU agree on soft penumbra approximation for finite emitters.
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
                } else if kind < 1.5 {
                    let to_light = record.position_intensity.xyz - world_position;
                    incoming = normalize(to_light);
                    let attenuation = distance_attenuation(to_light, record.color_range.w);
                    radiance = record.color_range.rgb * record.position_intensity.w * attenuation;
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
                    radiance = record.color_range.rgb * record.position_intensity.w * attenuation * cone;
                }
                shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
                shaded += clearcoat_light_contribution(clearcoat_normal, view, incoming, radiance, clearcoat_factor, clearcoat_roughness);
                shaded += sheen_light_contribution(normal, view, incoming, radiance, sheen_color, sheen_roughness);
                shaded += anisotropy_light_contribution(base, metallic, roughness, normal, world_tangent, tangent_handedness, view, incoming, radiance, anisotropy_strength, anisotropy_rotation, anisotropy_direction);
                shaded += iridescence_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, iridescence_factor, iridescence_ior, iridescence_thickness);
                shaded += dispersion_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, dispersion_factor, dispersion_ior);
            }
        }
    }
    let directional_count = min(u32(camera.lighting.light_counts.x), MAX_GPU_LIGHTS_PER_TYPE);
    for (var i = 0u; i < MAX_GPU_LIGHTS_PER_TYPE; i = i + 1u) {
        if i < directional_count {
            let light_direction = camera.lighting.directional_light_direction_intensity[i];
            let incoming = normalize(-light_direction.xyz);
            let gpu_shadow = select(
                1.0,
                directional_shadow_factor(world_position),
                camera.lighting.directional_shadow_control[i].x > 0.5,
            );
            let radiance = camera.lighting.directional_light_color[i].rgb *
                light_direction.w * gpu_shadow;
            shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
            shaded += clearcoat_light_contribution(clearcoat_normal, view, incoming, radiance, clearcoat_factor, clearcoat_roughness);
            shaded += sheen_light_contribution(normal, view, incoming, radiance, sheen_color, sheen_roughness);
            shaded += anisotropy_light_contribution(base, metallic, roughness, normal, world_tangent, tangent_handedness, view, incoming, radiance, anisotropy_strength, anisotropy_rotation, anisotropy_direction);
            shaded += iridescence_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, iridescence_factor, iridescence_ior, iridescence_thickness);
            shaded += dispersion_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, dispersion_factor, dispersion_ior);
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
            let radiance = point_color.rgb * point_light.w * attenuation;
            shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
            shaded += clearcoat_light_contribution(clearcoat_normal, view, incoming, radiance, clearcoat_factor, clearcoat_roughness);
            shaded += sheen_light_contribution(normal, view, incoming, radiance, sheen_color, sheen_roughness);
            shaded += anisotropy_light_contribution(base, metallic, roughness, normal, world_tangent, tangent_handedness, view, incoming, radiance, anisotropy_strength, anisotropy_rotation, anisotropy_direction);
            shaded += iridescence_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, iridescence_factor, iridescence_ior, iridescence_thickness);
            shaded += dispersion_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, dispersion_factor, dispersion_ior);
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
                spot_light.w * attenuation * cone;
            shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
            shaded += clearcoat_light_contribution(clearcoat_normal, view, incoming, radiance, clearcoat_factor, clearcoat_roughness);
            shaded += sheen_light_contribution(normal, view, incoming, radiance, sheen_color, sheen_roughness);
            shaded += anisotropy_light_contribution(base, metallic, roughness, normal, world_tangent, tangent_handedness, view, incoming, radiance, anisotropy_strength, anisotropy_rotation, anisotropy_direction);
            shaded += iridescence_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, iridescence_factor, iridescence_ior, iridescence_thickness);
            shaded += dispersion_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, dispersion_factor, dispersion_ior);
        }
    }
    let area_count = min(u32(camera.lighting.light_counts.w), MAX_GPU_AREA_LIGHTS);
    for (var i = 0u; i < MAX_GPU_AREA_LIGHTS; i = i + 1u) {
        if i < area_count {
            shaded += ltc_area_light_specular_contribution(
                i,
                base,
                metallic,
                roughness,
                normal,
                view,
                world_position,
                area_shadow_visibility,
            );
            for (var sample = 0u; sample < AREA_LIGHT_SAMPLE_COUNT; sample = sample + 1u) {
                let sample_position = area_light_sample_position(i, sample);
                let to_light = sample_position - world_position;
                let incoming = normalize(to_light);
                let radiance = area_light_radiance(i, sample_position, world_position) * area_shadow_visibility;
                shaded += pbr_area_light_diffuse_contribution(base, metallic, normal, view, incoming, radiance);
                shaded += clearcoat_light_contribution(clearcoat_normal, view, incoming, radiance, clearcoat_factor, clearcoat_roughness);
                shaded += sheen_light_contribution(normal, view, incoming, radiance, sheen_color, sheen_roughness);
                shaded += anisotropy_light_contribution(base, metallic, roughness, normal, world_tangent, tangent_handedness, view, incoming, radiance, anisotropy_strength, anisotropy_rotation, anisotropy_direction);
                shaded += iridescence_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, iridescence_factor, iridescence_ior, iridescence_thickness);
                shaded += dispersion_light_contribution(base, metallic, roughness, normal, view, incoming, radiance, dispersion_factor, dispersion_ior);
            }
        }
    }
    return shaded;
}

struct BeautySemanticOutput {
    @location(0) color: vec4<f32>,
    @location(1) semantic_id: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return shade_fragment(in);
}

@fragment
fn fs_beauty_semantic(in: VertexOut) -> BeautySemanticOutput {
    var output: BeautySemanticOutput;
    output.color = shade_fragment(in);
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

fn has_environment_light() -> bool {
    return camera.lighting.environment_diffuse_intensity.w > 0.0 ||
        camera.lighting.environment_specular_intensity.w > 0.0;
}

fn environment_prefilter_mip(roughness: f32) -> f32 {
    return sqrt(clamp(roughness, 0.0, 1.0)) * ENVIRONMENT_PREFILTER_MAX_MIP;
}

fn rotate_environment_direction(direction: vec3<f32>) -> vec3<f32> {
    let cosine = camera.lighting.environment_transform.x;
    let sine = camera.lighting.environment_transform.y;
    return vec3<f32>(
        cosine * direction.x + sine * direction.z,
        direction.y,
        -sine * direction.x + cosine * direction.z,
    );
}

fn environment_reflection_direction(
    world_position: vec3<f32>,
    direction: vec3<f32>,
) -> vec3<f32> {
    if draw.reflection_probe_bounds_min.w < 0.5 {
        return rotate_environment_direction(direction);
    }
    let epsilon = vec3<f32>(0.000001);
    let safe_direction = select(
        select(-epsilon, epsilon, direction >= vec3<f32>(0.0)),
        direction,
        abs(direction) >= epsilon,
    );
    let exit_plane = select(
        draw.reflection_probe_bounds_min.xyz,
        draw.reflection_probe_bounds_max.xyz,
        safe_direction >= vec3<f32>(0.0),
    );
    let distances = (exit_plane - world_position) / safe_direction;
    let distance = max(min(distances.x, min(distances.y, distances.z)), 0.0);
    let projected = world_position + safe_direction * distance - draw.reflection_probe_capture.xyz;
    let projected_length = length(projected);
    return select(direction, projected / max(projected_length, 0.000001), projected_length > 0.000001);
}

fn pbr_environment_lighting(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    world_position: vec3<f32>,
) -> vec3<f32> {
    if !has_environment_light() {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(n_dot_v, f0);
    let diffuse_energy = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    // Phase 1C step 2: real diffuse + specular IBL.
    //   - Diffuse: prepared irradiance from the active environment.
    //   - Specular: GGX-prefiltered cubemap sampled in the reflection
    //     direction at a roughness-driven mip, then composited with an
    //     baked split-sum BRDF table.
    let diffuse_irradiance = camera.lighting.environment_diffuse_intensity.rgb;
    let diffuse = diffuse_energy * base * diffuse_irradiance * camera.lighting.environment_diffuse_intensity.w;
    let reflection = environment_reflection_direction(world_position, reflect(-view, normal));
    let prefilter_mip = environment_prefilter_mip(roughness);
    let prefiltered = textureSampleLevel(environment_cubemap, environment_sampler, reflection, prefilter_mip).rgb;
    let lut_sample = split_sum_brdf_table(n_dot_v, roughness);
    let specular = prefiltered * (f0 * lut_sample.x + vec3<f32>(lut_sample.y)) * camera.lighting.environment_specular_intensity.w;
    return diffuse + specular;
}

fn clearcoat_environment_lighting(
    normal: vec3<f32>,
    view: vec3<f32>,
    factor: f32,
    roughness: f32,
    world_position: vec3<f32>,
) -> vec3<f32> {
    if !has_environment_light() || factor <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let reflection = environment_reflection_direction(world_position, reflect(-view, normal));
    let prefilter_mip = environment_prefilter_mip(roughness);
    let prefiltered = textureSampleLevel(environment_cubemap, environment_sampler, reflection, prefilter_mip).rgb;
    let lut_sample = split_sum_brdf_table(n_dot_v, roughness);
    let specular = prefiltered * (vec3<f32>(0.04) * lut_sample.x + vec3<f32>(lut_sample.y));
    return specular * camera.lighting.environment_specular_intensity.w * factor;
}

fn sheen_environment_lighting(
    normal: vec3<f32>,
    view: vec3<f32>,
    color: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {
    if !has_environment_light() {
        return vec3<f32>(0.0);
    }
    let sheen_color = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    if max(sheen_color.r, max(sheen_color.g, sheen_color.b)) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.0);
    let grazing = pow(1.0 - n_dot_v, mix(2.2, 0.65, clamp(roughness, 0.0, 1.0)));
    let irradiance = camera.lighting.environment_diffuse_intensity.rgb * camera.lighting.environment_diffuse_intensity.w +
        camera.lighting.environment_specular_intensity.rgb * camera.lighting.environment_specular_intensity.w;
    let broadness = mix(0.26, 1.25, clamp(roughness, 0.0, 1.0));
    return sheen_color * irradiance * grazing * broadness;
}

fn anisotropy_environment_lighting(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    tangent: vec3<f32>,
    tangent_handedness: f32,
    view: vec3<f32>,
    strength: f32,
    rotation: f32,
    direction: vec2<f32>,
    world_position: vec3<f32>,
) -> vec3<f32> {
    if !has_environment_light() || strength <= 0.0 {
        return vec3<f32>(0.0);
    }
    let safe_tangent = normalize(tangent - normal * dot(tangent, normal));
    let bitangent = normalize(cross(normal, safe_tangent) * select(-1.0, 1.0, tangent_handedness >= 0.0));
    let anisotropy_direction = rotated_anisotropy_direction(direction, rotation);
    let anisotropic_t = normalize(safe_tangent * anisotropy_direction.x + bitangent * anisotropy_direction.y);
    let anisotropic_normal = normalize(mix(normal, anisotropic_t, clamp(strength * 0.55, 0.0, 0.55)));
    let n_dot_v = max(dot(normal, view), 0.001);
    let reflection = environment_reflection_direction(world_position, reflect(-view, anisotropic_normal));
    let directional_roughness = clamp(roughness * (1.0 - strength * 0.60), 0.04, 1.0);
    let prefilter_mip = environment_prefilter_mip(directional_roughness);
    let prefiltered = textureSampleLevel(environment_cubemap, environment_sampler, reflection, prefilter_mip).rgb;
    let lut_sample = split_sum_brdf_table(n_dot_v, directional_roughness);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let specular = prefiltered * (f0 * lut_sample.x + vec3<f32>(lut_sample.y));
    return specular * camera.lighting.environment_specular_intensity.w * strength;
}

fn pbr_light_contribution(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
) -> vec3<f32> {
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let half_vector = normalize(view + incoming);
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let v_dot_h = max(dot(view, half_vector), 0.0);
    let alpha = roughness * roughness;
    let specular_brdf = brdf_specular_ggx(alpha, n_dot_l, n_dot_v, n_dot_h);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(v_dot_h, f0);
    let specular = fresnel * specular_brdf;
    let diffuse_energy = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    let diffuse = diffuse_energy * base / PI;
    return (diffuse + specular) * radiance * n_dot_l;
}

fn pbr_area_light_diffuse_contribution(
    base: vec3<f32>,
    metallic: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
) -> vec3<f32> {
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let half_vector = normalize(view + incoming);
    let v_dot_h = max(dot(view, half_vector), 0.0);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(v_dot_h, f0);
    let diffuse_energy = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    return diffuse_energy * base * radiance * n_dot_l / PI;
}

fn clearcoat_light_contribution(
    normal: vec3<f32>,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
    factor: f32,
    roughness: f32,
) -> vec3<f32> {
    if factor <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let half_vector = normalize(view + incoming);
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let v_dot_h = max(dot(view, half_vector), 0.0);
    let alpha = roughness * roughness;
    let specular_brdf = brdf_specular_ggx(alpha, n_dot_l, n_dot_v, n_dot_h);
    let fresnel = fresnel_schlick(v_dot_h, vec3<f32>(0.04));
    let specular = fresnel * specular_brdf;
    return specular * radiance * n_dot_l * factor;
}

fn sheen_light_contribution(
    normal: vec3<f32>,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
    color: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {
    let sheen_color_clamped = clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    if max(sheen_color_clamped.r, max(sheen_color_clamped.g, sheen_color_clamped.b)) <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let half_vector = normalize(view + incoming);
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let alpha = roughness * roughness;
    let sheen = sheen_color_clamped * brdf_specular_ggx(alpha, n_dot_l, n_dot_v, n_dot_h);
    return sheen * radiance * n_dot_l;
}

fn anisotropy_light_contribution(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    tangent: vec3<f32>,
    tangent_handedness: f32,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
    strength: f32,
    rotation: f32,
    direction: vec2<f32>,
) -> vec3<f32> {
    if strength <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let safe_tangent = normalize(tangent - normal * dot(tangent, normal));
    let bitangent = normalize(cross(normal, safe_tangent) * select(-1.0, 1.0, tangent_handedness >= 0.0));
    let anisotropy_direction = rotated_anisotropy_direction(direction, rotation);
    let anisotropic_t = normalize(safe_tangent * anisotropy_direction.x + bitangent * anisotropy_direction.y);
    let anisotropic_b = normalize(cross(normal, anisotropic_t));
    let n_dot_v = max(dot(normal, view), 0.001);
    let half_vector = normalize(view + incoming);
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let v_dot_h = max(dot(view, half_vector), 0.0);
    let t_dot_v = dot(anisotropic_t, view);
    let b_dot_v = dot(anisotropic_b, view);
    let t_dot_l = dot(anisotropic_t, incoming);
    let b_dot_l = dot(anisotropic_b, incoming);
    let t_dot_h = dot(anisotropic_t, half_vector);
    let b_dot_h = dot(anisotropic_b, half_vector);
    let base_alpha = roughness * roughness;
    let tangent_alpha = mix(base_alpha, 1.0, strength * strength);
    let bitangent_alpha = base_alpha;
    let distribution = distribution_ggx_anisotropic(n_dot_h, t_dot_h, b_dot_h, tangent_alpha, bitangent_alpha);
    let visibility = visibility_ggx_anisotropic(n_dot_l, n_dot_v, b_dot_v, t_dot_v, t_dot_l, b_dot_l, tangent_alpha, bitangent_alpha);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(v_dot_h, f0);
    return fresnel * distribution * visibility * radiance * n_dot_l * strength;
}

fn iridescence_light_contribution(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
    factor: f32,
    ior: f32,
    thickness_nm: f32,
) -> vec3<f32> {
    if factor <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let half_vector = normalize(view + incoming);
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let v_dot_h = max(dot(view, half_vector), 0.0);
    let alpha = roughness * roughness;
    let specular_brdf = brdf_specular_ggx(alpha, n_dot_l, n_dot_v, n_dot_h);
    let film_color = iridescence_film_color(thickness_nm, ior);
    let f0 = (vec3<f32>(0.04) * (1.0 - metallic) + base * metallic) * film_color;
    let fresnel = fresnel_schlick(v_dot_h, f0);
    let specular = fresnel * film_color * specular_brdf;
    return specular * radiance * n_dot_l * factor;
}

fn dispersion_light_contribution(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
    factor: f32,
    ior: f32,
) -> vec3<f32> {
    let dispersion = max(factor, 0.0);
    if dispersion <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_l = max(dot(normal, incoming), 0.0);
    if n_dot_l <= 0.0 {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let half_vector = normalize(view + incoming);
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let v_dot_h = max(dot(view, half_vector), 0.0);
    let alpha = roughness * roughness;
    let specular_brdf = brdf_specular_ggx(alpha, n_dot_l, n_dot_v, n_dot_h);
    let f0 = dispersion_f0_from_ior(ior, dispersion);
    let fresnel = fresnel_schlick(v_dot_h, f0);
    let specular = fresnel * specular_brdf;
    _ = base;
    _ = metallic;
    return specular * radiance * n_dot_l * dispersion;
}

fn iridescence_film_color(thickness_nm: f32, ior: f32) -> vec3<f32> {
    let safe_ior = select(1.3, ior, ior > 0.0);
    let phase = max(thickness_nm, 0.0) * safe_ior / 650.0 * PI * 1.25;
    return clamp(vec3<f32>(
        sin(phase) * 0.5 + 0.5,
        sin(phase + 2.0 * PI / 3.0) * 0.5 + 0.5,
        sin(phase + 4.0 * PI / 3.0) * 0.5 + 0.5,
    ), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn dispersion_f0_from_ior(ior: f32, dispersion: f32) -> vec3<f32> {
    let safe_ior = max(ior, 1.0);
    let half_spread = (safe_ior - 1.0) * 0.025 * max(dispersion, 0.0);
    return vec3<f32>(
        f0_from_ior(max(safe_ior - half_spread, 1.0)),
        f0_from_ior(safe_ior),
        f0_from_ior(safe_ior + half_spread),
    );
}

fn f0_from_ior(ior: f32) -> f32 {
    let ratio = (ior - 1.0) / max(ior + 1.0, 0.0001);
    return ratio * ratio;
}

fn distribution_ggx_anisotropic(n_dot_h: f32, t_dot_h: f32, b_dot_h: f32, tangent_alpha: f32, bitangent_alpha: f32) -> f32 {
    let alpha_product = max(tangent_alpha * bitangent_alpha, 0.0001);
    let f = vec3<f32>(
        bitangent_alpha * t_dot_h,
        tangent_alpha * b_dot_h,
        alpha_product * n_dot_h,
    );
    let w2 = alpha_product / max(dot(f, f), 0.0001);
    return alpha_product * w2 * w2 / PI;
}

fn visibility_ggx_anisotropic(
    n_dot_l: f32,
    n_dot_v: f32,
    b_dot_v: f32,
    t_dot_v: f32,
    t_dot_l: f32,
    b_dot_l: f32,
    tangent_alpha: f32,
    bitangent_alpha: f32,
) -> f32 {
    let ggx_v = n_dot_l * length(vec3<f32>(tangent_alpha * t_dot_v, bitangent_alpha * b_dot_v, n_dot_v));
    let ggx_l = n_dot_v * length(vec3<f32>(tangent_alpha * t_dot_l, bitangent_alpha * b_dot_l, n_dot_l));
    return clamp(0.5 / max(ggx_v + ggx_l, 0.0001), 0.0, 1.0);
}

fn rotated_anisotropy_direction(direction: vec2<f32>, rotation: f32) -> vec2<f32> {
    let normalized_direction = select(vec2<f32>(1.0, 0.0), normalize(direction), length(direction) > 0.0001);
    let sin_r = sin(rotation);
    let cos_r = cos(rotation);
    return vec2<f32>(
        normalized_direction.x * cos_r - normalized_direction.y * sin_r,
        normalized_direction.x * sin_r + normalized_direction.y * cos_r,
    );
}

fn distance_attenuation(to_light: vec3<f32>, range: f32) -> f32 {
    let distance_squared = max(dot(to_light, to_light), 0.0001);
    let inverse_square = 1.0 / distance_squared;
    if range <= 0.0 {
        return inverse_square;
    }
    let distance = sqrt(distance_squared);
    let range_falloff = clamp(1.0 - pow4(distance / range), 0.0, 1.0);
    return inverse_square * range_falloff * range_falloff;
}

fn spot_cone_attenuation(cos_angle: f32, inner_cone_cos: f32, outer_cone_cos: f32) -> f32 {
    if cos_angle >= inner_cone_cos {
        return 1.0;
    }
    if cos_angle <= outer_cone_cos {
        return 0.0;
    }
    return clamp((cos_angle - outer_cone_cos) / (inner_cone_cos - outer_cone_cos), 0.0, 1.0);
}
