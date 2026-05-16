const PI: f32 = 3.141592653589793;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) tex_coord0: vec2<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) shadow_visibility: f32,
};

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord0: vec2<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) shadow_visibility: f32,
};

struct LightingUniform {
    directional_light_direction_intensity: vec4<f32>,
    directional_light_color_count: vec4<f32>,
    directional_shadow_control: vec4<f32>,
    point_light_position_intensity: vec4<f32>,
    point_light_color_range: vec4<f32>,
    spot_light_position_intensity: vec4<f32>,
    spot_light_direction_cones: vec4<f32>,
    spot_light_cone_range: vec4<f32>,
    spot_light_color_range: vec4<f32>,
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
};

struct MaterialUniform {
    base_color_uv_offset_scale: vec4<f32>,
    base_color_uv_rotation: vec4<f32>,
    base_color_factor: vec4<f32>,
    emissive_strength: vec4<f32>,
    metallic_roughness_alpha: vec4<f32>,
    // WebGL2 texture_2d shim: keep the same uniform layout as the array shader
    // so draw/material encoding stays shared. This variant ignores the layer
    // index because wgpu 29's GL backend samples material texture arrays as
    // black in Chromium WebGL2.
    material_layer_index: vec4<u32>,
    // Phase 5.1: glTF spec scalar texture strengths.
    // .x = normalTexture.scale   (default 1.0)
    // .y = occlusionTexture.strength (default 1.0)
    // .z, .w = reserved
    texture_strengths: vec4<f32>,
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
// drive diffuse via textureSampleLevel(environment_cubemap, environment_sampler,
// normal, 0). The 1×1 placeholder is never sampled because
// environment_diffuse_intensity.w gates whether IBL contributes at all.
@group(0) @binding(3)
var environment_cubemap: texture_cube<f32>;

@group(0) @binding(4)
var environment_sampler: sampler;

// Phase 1C step 2: split-sum BRDF LUT (RG32Float, NoV × roughness). The
// fragment specular path samples it once per pixel to fold the fresnel
// + geometry terms into the prefiltered cubemap radiance.
@group(0) @binding(5)
var brdf_lut: texture_2d<f32>;

@group(2) @binding(0)
var<uniform> draw: DrawUniform;

@group(1) @binding(0)
var base_color_sampler: sampler;

@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;

@group(1) @binding(2)
var<uniform> material: MaterialUniform;

@group(1) @binding(3)
var normal_sampler: sampler;

@group(1) @binding(4)
var normal_texture: texture_2d<f32>;

@group(1) @binding(5)
var metallic_roughness_sampler: sampler;

@group(1) @binding(6)
var metallic_roughness_texture: texture_2d<f32>;

@group(1) @binding(7)
var occlusion_sampler: sampler;

@group(1) @binding(8)
var occlusion_texture: texture_2d<f32>;

@group(1) @binding(9)
var emissive_sampler: sampler;

@group(1) @binding(10)
var emissive_texture: texture_2d<f32>;

@vertex
fn vs_main(in: VertexIn) -> VertexOut {
    var out: VertexOut;
    let world_position = draw.world_from_model * vec4<f32>(in.position, 1.0);
    out.position = camera.clip_from_view * camera.view_from_world * world_position;
    out.color = in.color;
    out.normal = (draw.normal_from_model * vec4<f32>(in.normal, 0.0)).xyz;
    out.tex_coord0 = in.tex_coord0;
    out.world_position = world_position.xyz;
    out.tangent = vec4<f32>((draw.normal_from_model * vec4<f32>(in.tangent.xyz, 0.0)).xyz, in.tangent.w);
    out.shadow_visibility = clamp(in.shadow_visibility, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let scaled_uv = in.tex_coord0 * material.base_color_uv_offset_scale.zw;
    let transformed_uv = vec2<f32>(
        scaled_uv.x * material.base_color_uv_rotation.y - scaled_uv.y * material.base_color_uv_rotation.x,
        scaled_uv.x * material.base_color_uv_rotation.x + scaled_uv.y * material.base_color_uv_rotation.y,
    ) + material.base_color_uv_offset_scale.xy;
    let base_color_sample = textureSample(base_color_texture, base_color_sampler, transformed_uv);
    let normal_texture_sample = textureSample(normal_texture, normal_sampler, in.tex_coord0).rgb;
    let metallic_roughness_sample = textureSample(metallic_roughness_texture, metallic_roughness_sampler, in.tex_coord0);
    let occlusion_sample = textureSample(occlusion_texture, occlusion_sampler, in.tex_coord0).r;
    let emissive_sample = textureSample(emissive_texture, emissive_sampler, in.tex_coord0).rgb;
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
    let metallic = clamp(material.metallic_roughness_alpha.x * metallic_roughness_sample.b, 0.0, 1.0);
    let roughness = clamp(material.metallic_roughness_alpha.y * metallic_roughness_sample.g, 0.04, 1.0);
    // Phase 5.1: occlusionTexture.strength lerps between 1.0 and the
    // sampled occlusion. strength=0 disables AO; strength=1 applies it
    // at full intensity. glTF spec default = 1.0.
    let occlusion_strength = material.texture_strengths.y;
    let occlusion_applied = mix(1.0, occlusion_sample, occlusion_strength);
    let base = in.color * material.base_color_factor * base_color_sample;
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
            in.world_position,
            in.shadow_visibility,
        );
        let environment = pbr_environment_lighting(base.rgb, metallic, roughness, normal, view);
        if has_punctual_light() || has_environment_light() {
            shaded_rgb = (direct + environment) * occlusion_sample;
        }
    }
    let shaded = vec4<f32>(shaded_rgb + emissive, base.a);
    let color_management_mode = camera.color_management.x;
    return vec4<f32>(
        apply_tonemapper(shaded.rgb * camera.camera_position_exposure.w, color_management_mode),
        shaded.a,
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
    return textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv, light_ndc.z);
}

fn pbr_punctual_lighting(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    world_position: vec3<f32>,
    shadow_visibility: f32,
) -> vec3<f32> {
    var shaded = vec3<f32>(0.0);
    if camera.lighting.directional_light_direction_intensity.w > 0.0 {
        let incoming = normalize(-camera.lighting.directional_light_direction_intensity.xyz);
        // Phase 1B step 2: replace the per-vertex CPU-baked shadow_visibility
        // input with the GPU shadow map sample so the GPU path stops relying
        // on the CPU ray-cast bake (review F7). The argument is kept on the
        // function signature for the WebGL2 fallback that does not yet have
        // a shadow map.
        _ = shadow_visibility;
        let gpu_shadow = select(
            1.0,
            directional_shadow_factor(world_position),
            camera.lighting.directional_shadow_control.x > 0.5,
        );
        let radiance = camera.lighting.directional_light_color_count.rgb *
            camera.lighting.directional_light_direction_intensity.w * gpu_shadow;
        shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    if camera.lighting.point_light_position_intensity.w > 0.0 {
        let to_light = camera.lighting.point_light_position_intensity.xyz - world_position;
        let incoming = normalize(to_light);
        let attenuation = distance_attenuation(to_light, camera.lighting.point_light_color_range.w);
        let radiance = camera.lighting.point_light_color_range.rgb *
            camera.lighting.point_light_position_intensity.w * attenuation;
        shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    if camera.lighting.spot_light_position_intensity.w > 0.0 {
        let to_light = camera.lighting.spot_light_position_intensity.xyz - world_position;
        let incoming = normalize(to_light);
        let to_surface = -incoming;
        let cone = spot_cone_attenuation(
            dot(to_surface, normalize(camera.lighting.spot_light_direction_cones.xyz)),
            camera.lighting.spot_light_cone_range.x,
            camera.lighting.spot_light_cone_range.y,
        );
        let attenuation = distance_attenuation(to_light, camera.lighting.spot_light_cone_range.z);
        let radiance = camera.lighting.spot_light_color_range.rgb *
            camera.lighting.spot_light_position_intensity.w * attenuation * cone;
        shaded += pbr_light_contribution(base, metallic, roughness, normal, view, incoming, radiance);
    }
    return shaded;
}

fn has_punctual_light() -> bool {
    return camera.lighting.directional_light_direction_intensity.w > 0.0 ||
        camera.lighting.point_light_position_intensity.w > 0.0 ||
        camera.lighting.spot_light_position_intensity.w > 0.0;
}

fn has_environment_light() -> bool {
    return camera.lighting.environment_diffuse_intensity.w > 0.0 ||
        camera.lighting.environment_specular_intensity.w > 0.0;
}

fn pbr_environment_lighting(
    base: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
) -> vec3<f32> {
    if !has_environment_light() {
        return vec3<f32>(0.0);
    }
    let n_dot_v = max(dot(normal, view), 0.001);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(n_dot_v, f0);
    let diffuse_energy = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    // Phase 1C step 2: real diffuse + specular IBL.
    //   - Diffuse: cubemap mip 0 sampled in the surface-normal direction.
    //   - Specular: GGX-prefiltered cubemap sampled in the reflection
    //     direction at a roughness-driven mip, then composited with the
    //     split-sum BRDF LUT into `prefiltered * (F0 * lut.x + lut.y)`.
    let environment_radiance = textureSampleLevel(environment_cubemap, environment_sampler, normal, 0.0).rgb;
    let diffuse = diffuse_energy * base * environment_radiance * camera.lighting.environment_diffuse_intensity.w;
    let reflection = reflect(-view, normal);
    let prefilter_max_mip = 4.0;
    let prefilter_mip = clamp(roughness, 0.0, 1.0) * prefilter_max_mip;
    let prefiltered = textureSampleLevel(environment_cubemap, environment_sampler, reflection, prefilter_mip).rgb;
    let lut_size = f32(textureDimensions(brdf_lut).x);
    let lut_pixel = vec2<f32>(n_dot_v * lut_size, clamp(roughness, 0.0, 1.0) * lut_size);
    let lut_coord = vec2<i32>(
        clamp(i32(floor(lut_pixel.x)), 0, i32(lut_size) - 1),
        clamp(i32(floor(lut_pixel.y)), 0, i32(lut_size) - 1),
    );
    let lut_sample = textureLoad(brdf_lut, lut_coord, 0).rg;
    let specular = prefiltered * (f0 * lut_sample.x + vec3<f32>(lut_sample.y)) * camera.lighting.environment_specular_intensity.w;
    return diffuse + specular;
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
    let distribution = distribution_ggx(n_dot_h, alpha);
    let geometry = geometry_smith(n_dot_v, n_dot_l, roughness);
    let f0 = vec3<f32>(0.04) * (1.0 - metallic) + base * metallic;
    let fresnel = fresnel_schlick(v_dot_h, f0);
    let specular = fresnel * (distribution * geometry / max(4.0 * n_dot_v * n_dot_l, 0.0001));
    let diffuse_energy = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
    let diffuse = diffuse_energy * base / PI;
    return (diffuse + specular) * radiance * n_dot_l;
}

fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let alpha_squared = alpha * alpha;
    let denominator = n_dot_h * n_dot_h * (alpha_squared - 1.0) + 1.0;
    return alpha_squared / max(PI * denominator * denominator, 0.0001);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let k = ((roughness + 1.0) * (roughness + 1.0)) / 8.0;
    return geometry_schlick_ggx(n_dot_v, k) * geometry_schlick_ggx(n_dot_l, k);
}

fn geometry_schlick_ggx(n_dot: f32, k: f32) -> f32 {
    return n_dot / max(n_dot * (1.0 - k) + k, 0.0001);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
}

fn distance_attenuation(to_light: vec3<f32>, range: f32) -> f32 {
    let distance_squared = max(dot(to_light, to_light), 0.0001);
    let inverse_square = 1.0 / distance_squared;
    if range <= 0.0 {
        return inverse_square;
    }
    let distance = sqrt(distance_squared);
    let range_falloff = clamp(1.0 - pow(distance / range, 4.0), 0.0, 1.0);
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

fn apply_tonemapper(color: vec3<f32>, color_management_mode: f32) -> vec3<f32> {
    if color_management_mode < 0.5 {
        return clamp(color, vec3<f32>(0.0), vec3<f32>(1.0));
    }
    if color_management_mode > 1.5 {
        return pbr_neutral_tonemap(color);
    }
    return aces_tonemap(color);
}

fn pbr_neutral_tonemap(color_in: vec3<f32>) -> vec3<f32> {
    let start_compression = 0.8 - 0.04;
    let desaturation = 0.15;
    var color = max(color_in, vec3<f32>(0.0));
    let x = min(color.r, min(color.g, color.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    color -= vec3<f32>(offset);
    let peak = max(color.r, max(color.g, color.b));
    if peak < start_compression {
        return color;
    }
    let d = 1.0 - start_compression;
    let new_peak = 1.0 - d * d / (peak + d - start_compression);
    color *= new_peak / peak;
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(color, new_peak * vec3<f32>(1.0), g);
}

fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    let input = vec3<f32>(
        dot(vec3<f32>(0.59719, 0.35458, 0.04823), color),
        dot(vec3<f32>(0.076, 0.90834, 0.01566), color),
        dot(vec3<f32>(0.0284, 0.13383, 0.83777), color),
    );
    let fitted = vec3<f32>(
        rrt_and_odt_fit(input.r),
        rrt_and_odt_fit(input.g),
        rrt_and_odt_fit(input.b),
    );
    let output = vec3<f32>(
        dot(vec3<f32>(1.60475, -0.53108, -0.07367), fitted),
        dot(vec3<f32>(-0.10208, 1.10813, -0.00605), fitted),
        dot(vec3<f32>(-0.00327, -0.07276, 1.07602), fitted),
    );
    return clamp(output, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn rrt_and_odt_fit(value: f32) -> f32 {
    let numerator = value * (value + 0.0245786) - 0.000090537;
    let denominator = value * (0.983729 * value + 0.432951) + 0.238081;
    return numerator / denominator;
}
