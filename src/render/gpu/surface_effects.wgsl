// scena.surface_effects.wgsl
//
// Screen-space material effects and area-light sampling shared by the
// texture-array and texture-2D triangle shaders.

// Taps in the disc that blurs the scene colour behind rough glass. Keep in
// step with `SCENE_BLUR_TAPS` in `physical_transmission.rs`.
const SCENE_BLUR_TAPS: i32 = 16;
// Angle between successive taps of the spiral, in radians.
const SCENE_BLUR_GOLDEN_ANGLE: f32 = 2.39996323;
// Largest blur radius the disc resolves, as a share of the viewport height.
const SCENE_BLUR_MAX_RADIUS: f32 = 0.03;

// Radius in pixels of the blur that a lobe of the given spread needs. This is
// the Khronos glTF Sample Viewer's mapping: it reads the scene colour at
// level of detail `log2(width) * spread`, whose texels span `width^spread`
// pixels.
fn scene_blur_radius_px(spread: f32, viewport: vec2<f32>) -> f32 {
    let footprint = pow(viewport.x, clamp(spread, 0.0, 1.0));
    return min(0.5 * (footprint - 1.0), SCENE_BLUR_MAX_RADIUS * viewport.y);
}

// Scene colour around `uv`, blurred by a Gaussian-weighted disc of the given
// radius. The taps follow a golden-angle spiral, which covers the disc
// evenly, rotated per pixel by interleaved gradient noise so that
// under-sampling shows as fine grain instead of copies of the scene. A zero
// radius collapses every tap onto `uv`.
fn blurred_scene_color(uv: vec2<f32>, radius_px: f32, frag_coord: vec2<f32>, viewport: vec2<f32>) -> vec3<f32> {
    let noise = fract(52.9829189 * fract(frag_coord.x * 0.06711056 + frag_coord.y * 0.00583715));
    let rotation = 6.28318531 * noise;
    var sum = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var tap = 0; tap < SCENE_BLUR_TAPS; tap = tap + 1) {
        let distance = sqrt((f32(tap) + 0.5) / f32(SCENE_BLUR_TAPS));
        let angle = f32(tap) * SCENE_BLUR_GOLDEN_ANGLE + rotation;
        // Standard deviation of half the disc radius.
        let weight = exp(-2.0 * distance * distance);
        let offset = vec2<f32>(cos(angle), sin(angle)) * (distance * radius_px) / viewport;
        sum += textureSampleLevel(transmission_color_texture, transmission_color_sampler, uv + offset, 0.0).rgb * weight;
        weight_sum += weight;
    }
    return sum / weight_sum;
}

// Share of the light entering the surface that passes through it instead of
// scattering back as diffuse. Only the dielectric part of a material
// transmits.
fn transmitted_share(metallic: f32) -> f32 {
    return clamp(material.transmission_factors.x, 0.0, 1.0) * (1.0 - clamp(metallic, 0.0, 1.0));
}

// Colour of a transmissive surface point. Fresnel reflectance from the index
// of refraction decides how much light passes through. That light is the
// scene colour seen along the refracted ray: the view direction is refracted
// at the surface and travels the material thickness to an exit point, and the
// scene colour is read where that point lands on screen, as the Khronos glTF
// Sample Viewer does. Roughness blurs it, the volume absorbs along the path,
// and the base colour tints it. `surface_rgb` is the light the surface itself
// sends to the viewer; its diffuse lobe already excludes the transmitted
// share.
fn physical_transmission_color(
    frag_coord: vec2<f32>,
    world_position: vec3<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    tint: vec3<f32>,
    metallic: f32,
    roughness: f32,
    surface_rgb: vec3<f32>,
) -> vec4<f32> {
    let transmission = transmitted_share(metallic);
    if transmission <= 0.001 {
        return vec4<f32>(0.0);
    }
    let viewport = max(camera.viewport_near_far.xy, vec2<f32>(1.0, 1.0));
    let ior = max(material.transmission_factors.y, 1.01);
    let thickness = max(material.transmission_factors.z, 0.0);
    let view_dir = normalize(view);
    let normal_dir = normalize(normal);
    let n_dot_v = clamp(dot(normal_dir, view_dir), 0.0, 1.0);
    let fresnel = fresnel_schlick(n_dot_v, vec3<f32>(f0_from_ior(ior))).x;

    // The thickness is in model units; the model matrix columns give the
    // world length of each.
    let model_scale = vec3<f32>(
        length(draw.world_from_model[0].xyz),
        length(draw.world_from_model[1].xyz),
        length(draw.world_from_model[2].xyz),
    );
    let ray = refract(-view_dir, normal_dir, 1.0 / ior) * thickness * model_scale;
    let exit_clip = camera.clip_from_world * vec4<f32>(world_position + ray, 1.0);
    var refracted_uv = frag_coord / viewport;
    if exit_clip.w > 0.0 {
        refracted_uv = exit_clip.xy / exit_clip.w * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    }
    let spread = roughness * clamp(ior * 2.0 - 2.0, 0.0, 1.0);
    let scene_color = blurred_scene_color(
        refracted_uv,
        scene_blur_radius_px(spread, viewport),
        frag_coord,
        viewport,
    );
    let absorption = volume_transmittance(length(ray), material.attenuation_color.rgb, material.transmission_factors.w);
    let transmitted = scene_color * absorption * tint;
    return vec4<f32>(surface_rgb + transmitted * ((1.0 - fresnel) * transmission), 1.0);
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
