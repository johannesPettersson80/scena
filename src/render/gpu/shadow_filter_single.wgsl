// scena.shadow_filter_single.wgsl
//
// Directional shadows for the WebGL2 triangle shader: one scene-fitted map and
// a fixed nine-tap 3x3 percentage-closer filter. The shape is deliberately
// small: this variant must compile inside the fragment-shader budget of
// tile-based mobile GPUs (see shader_manifest.rs).
//
// Each tap is a hardware-filtered comparison, so the nine taps blend into a
// smooth 4x4 footprint. The receiver is offset along its geometric normal by
// a fraction of a shadow texel and compared slightly toward the light, which
// removes self-shadowing stripes without detaching contact shadows.

fn directional_shadow_factor(
    world_position: vec3<f32>,
    normal: vec3<f32>,
    incoming: vec3<f32>,
    fragment_xy: vec2<f32>,
) -> f32 {
    let shadow_size = vec2<f32>(textureDimensions(shadow_map));
    let row_x = vec3<f32>(
        camera.light_from_world[0].x,
        camera.light_from_world[1].x,
        camera.light_from_world[2].x,
    );
    let row_z = vec3<f32>(
        camera.light_from_world[0].z,
        camera.light_from_world[1].z,
        camera.light_from_world[2].z,
    );
    let texel_world = 2.0 / max(length(row_x) * shadow_size.x, 0.000001);
    let texel_depth = texel_world * length(row_z);
    let n_dot_l = clamp(dot(normal, incoming), 0.05, 1.0);
    let slope = min(sqrt(1.0 - n_dot_l * n_dot_l) / n_dot_l, 4.0);
    let offset_position = world_position + normal * (texel_world * 1.5);
    let light_clip = camera.light_from_world * vec4<f32>(offset_position, 1.0);
    if light_clip.w <= 0.0 {
        return 1.0;
    }
    let light_ndc = light_clip.xyz / light_clip.w;
    let shadow_uv = vec2<f32>(light_ndc.x * 0.5 + 0.5, light_ndc.y * -0.5 + 0.5);
    // Receivers outside the shadow caster AABB get full radiance: the
    // sampler's ClampToEdge would otherwise read the texture border and
    // produce false self-shadow streaks.
    if shadow_uv.x < 0.0 || shadow_uv.x > 1.0 ||
       shadow_uv.y < 0.0 || shadow_uv.y > 1.0 ||
       light_ndc.z < 0.0 || light_ndc.z > 1.0 {
        return 1.0;
    }
    let shadow_texel_size = 1.0 / shadow_size;
    let compare_depth = light_ndc.z - texel_depth * (1.0 + slope);
    var shadow_visibility = 0.0;
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>(-1.0, -1.0), compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 0.0, -1.0), compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 1.0, -1.0), compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>(-1.0,  0.0), compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv, compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 1.0,  0.0), compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>(-1.0,  1.0), compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 0.0,  1.0), compare_depth);
    shadow_visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, shadow_uv + shadow_texel_size * vec2<f32>( 1.0,  1.0), compare_depth);
    _ = fragment_xy;
    return shadow_visibility / 9.0;
}
