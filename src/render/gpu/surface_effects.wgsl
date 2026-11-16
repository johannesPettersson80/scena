// scena.surface_effects.wgsl
//
// Screen-space material effects and area-light sampling shared by the
// texture-array and texture-2D triangle shaders.

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
