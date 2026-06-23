// scena.pbr_brdf.wgsl
// Khronos/glTF core metallic-roughness BRDF contract.
// Rust reference implementation: src/render/pbr_brdf.rs.
// Port source: KhronosGroup/glTF-Sample-Renderer bec106e brdf.glsl.

fn brdf_specular_ggx(
    alpha_roughness: f32,
    n_dot_l: f32,
    n_dot_v: f32,
    n_dot_h: f32,
) -> f32 {
    return distribution_ggx(n_dot_h, alpha_roughness) *
        visibility_ggx_correlated(n_dot_l, n_dot_v, alpha_roughness);
}

fn distribution_ggx(n_dot_h: f32, alpha_roughness: f32) -> f32 {
    let alpha_squared = alpha_roughness * alpha_roughness;
    let clamped_n_dot_h = clamp(n_dot_h, 0.0, 1.0);
    let f = clamped_n_dot_h * clamped_n_dot_h * (alpha_squared - 1.0) + 1.0;
    if f <= 0.0 {
        return 0.0;
    }
    return alpha_squared / (PI * f * f);
}

fn visibility_ggx_correlated(n_dot_l: f32, n_dot_v: f32, alpha_roughness: f32) -> f32 {
    let alpha_squared = alpha_roughness * alpha_roughness;
    let clamped_n_dot_l = clamp(n_dot_l, 0.0, 1.0);
    let clamped_n_dot_v = clamp(n_dot_v, 0.0, 1.0);
    let ggx_v = clamped_n_dot_l *
        sqrt(clamped_n_dot_v * clamped_n_dot_v * (1.0 - alpha_squared) + alpha_squared);
    let ggx_l = clamped_n_dot_v *
        sqrt(clamped_n_dot_l * clamped_n_dot_l * (1.0 - alpha_squared) + alpha_squared);
    let ggx = ggx_v + ggx_l;
    if ggx > 0.0 {
        return 0.5 / ggx;
    }
    return 0.0;
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow5(1.0 - clamp(cos_theta, 0.0, 1.0));
}

fn split_sum_brdf_approx(n_dot_v: f32, roughness: f32) -> vec2<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = clamp(roughness, 0.0, 1.0) * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * clamp(n_dot_v, 0.0, 1.0))) * r.x + r.y;
    return vec2<f32>(-1.04, 1.04) * a004 + r.zw;
}

fn pow4(value: f32) -> f32 {
    let squared = value * value;
    return squared * squared;
}

fn pow5(value: f32) -> f32 {
    let squared = value * value;
    return squared * squared * value;
}
