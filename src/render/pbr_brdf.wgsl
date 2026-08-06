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

/// Baked split-sum BRDF table, 32x32 over (n_dot_v, roughness).
///
/// Two texels of `(scale, bias)` share one `vec4` because std140 pads an array
/// of `vec2<f32>` to a 16-byte stride, which would double the block for nothing.
/// 512 * 16 = 8192 bytes, inside WebGL2's 16 KiB uniform-block floor.
const BRDF_LUT_TABLE_SIZE: u32 = 32u;

struct BrdfLutTable {
    pairs: array<vec4<f32>, 512>,
};

@group(0) @binding(11) var<uniform> brdf_lut_table: BrdfLutTable;

fn brdf_lut_texel(index: u32) -> vec2<f32> {
    let packed = brdf_lut_table.pairs[index >> 1u];
    return select(packed.xy, packed.zw, (index & 1u) == 1u);
}

/// Bilinear read matching the baker's texel centres at `(i + 0.5) / size`.
fn split_sum_brdf_table(n_dot_v: f32, roughness: f32) -> vec2<f32> {
    let size = f32(BRDF_LUT_TABLE_SIZE);
    let last = BRDF_LUT_TABLE_SIZE - 1u;
    let u = clamp(clamp(n_dot_v, 0.0, 1.0) * size - 0.5, 0.0, size - 1.0);
    let v = clamp(clamp(roughness, 0.0, 1.0) * size - 0.5, 0.0, size - 1.0);
    let x0 = u32(floor(u));
    let y0 = u32(floor(v));
    let x1 = min(x0 + 1u, last);
    let y1 = min(y0 + 1u, last);
    let fx = u - floor(u);
    let fy = v - floor(v);
    let row0 = y0 * BRDF_LUT_TABLE_SIZE;
    let row1 = y1 * BRDF_LUT_TABLE_SIZE;
    let lower = mix(brdf_lut_texel(row0 + x0), brdf_lut_texel(row0 + x1), fx);
    let upper = mix(brdf_lut_texel(row1 + x0), brdf_lut_texel(row1 + x1), fx);
    return mix(lower, upper, fy);
}

/// Karis's analytic DFG fit. Retained as the reference the baked table is
/// measured against, not as the shading path.
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
