// scena.surface_lighting.wgsl
//
// Layered glTF surface model shared by the texture-array and texture-2D
// triangle shaders. One `Surface` is built per fragment and every light and
// the environment are evaluated against it, so the layering rules live in one
// place instead of being repeated at each light loop.
//
// Layering follows the Khronos glTF material extensions:
//   - base: Lambert diffuse + GGX specular (KHR_materials_specular, _ior),
//     with anisotropy replacing the specular distribution and iridescence
//     replacing its Fresnel term,
//   - sheen: Charlie lobe on top, base scaled down by the energy it reflects,
//   - clearcoat: GGX lobe on top, everything below attenuated by its Fresnel.
// The Rust reference implementation is src/render/prepare/pbr_contract.rs.

const MIN_SHEEN_ROUGHNESS: f32 = 0.07;
const SHEEN_ENERGY_COMPENSATION: f32 = 0.157;

struct Surface {
    base: vec3<f32>,
    metallic: f32,
    diffuse_color: vec3<f32>,
    f0: vec3<f32>,
    f90: f32,
    roughness: f32,
    alpha: f32,
    normal: vec3<f32>,
    view: vec3<f32>,
    n_dot_v: f32,
    clearcoat: f32,
    clearcoat_roughness: f32,
    clearcoat_normal: vec3<f32>,
    // Fraction of light from the layers below that survives the clearcoat.
    clearcoat_attenuation: f32,
    sheen_color: vec3<f32>,
    sheen_roughness: f32,
    // Fraction of base-layer energy left after the sheen layer reflects.
    sheen_scaling: f32,
    anisotropy: f32,
    anisotropy_t: vec3<f32>,
    anisotropy_b: vec3<f32>,
    iridescence: f32,
    iridescence_fresnel: vec3<f32>,
    dispersion: f32,
    dispersion_f0: vec3<f32>,
};

// Dielectric reflectance at normal incidence for an index of refraction.
fn f0_from_ior(ior: f32) -> f32 {
    let ratio = (ior - 1.0) / max(ior + 1.0, 0.0001);
    return ratio * ratio;
}

// Per-channel reflectance of a dielectric whose index of refraction spreads
// across the visible spectrum (KHR_materials_dispersion).
fn dispersion_f0_from_ior(ior: f32, dispersion: f32) -> vec3<f32> {
    let safe_ior = max(ior, 1.0);
    let half_spread = (safe_ior - 1.0) * 0.025 * max(dispersion, 0.0);
    return vec3<f32>(
        f0_from_ior(max(safe_ior - half_spread, 1.0)),
        f0_from_ior(safe_ior),
        f0_from_ior(safe_ior + half_spread),
    );
}

fn fresnel_schlick_f90(cos_theta: f32, f0: vec3<f32>, f90: f32) -> vec3<f32> {
    return f0 + (vec3<f32>(f90) - f0) * pow5(1.0 - clamp(cos_theta, 0.0, 1.0));
}

// Base-layer Fresnel, with thin-film iridescence replacing the Schlick term.
fn surface_fresnel(s: Surface, v_dot_h: f32) -> vec3<f32> {
    let schlick = fresnel_schlick_f90(v_dot_h, s.f0, s.f90);
    if scena_material_iridescence {
        return mix(schlick, s.iridescence_fresnel, s.iridescence);
    }
    return schlick;
}

fn surface_anisotropic_specular(s: Surface, incoming: vec3<f32>, half_vector: vec3<f32>, n_dot_l: f32, n_dot_h: f32) -> f32 {
    let tangent_alpha = mix(s.alpha, 1.0, s.anisotropy * s.anisotropy);
    let distribution = distribution_ggx_anisotropic(
        n_dot_h,
        dot(s.anisotropy_t, half_vector),
        dot(s.anisotropy_b, half_vector),
        tangent_alpha,
        s.alpha,
    );
    let visibility = visibility_ggx_anisotropic(
        n_dot_l,
        s.n_dot_v,
        dot(s.anisotropy_b, s.view),
        dot(s.anisotropy_t, s.view),
        dot(s.anisotropy_t, incoming),
        dot(s.anisotropy_b, incoming),
        tangent_alpha,
        s.alpha,
    );
    return distribution * visibility;
}

// Radiance reflected toward the viewer from one light.
// `base_specular_weight` is 0 for area-light point samples, whose base
// specular lobe is integrated analytically by the LTC path instead.
fn surface_direct_light(
    s: Surface,
    incoming: vec3<f32>,
    radiance: vec3<f32>,
    base_specular_weight: f32,
) -> vec3<f32> {
    let half_vector = normalize(s.view + incoming);
    let v_dot_h = max(dot(s.view, half_vector), 0.0);
    var lit = vec3<f32>(0.0);
    let n_dot_l = max(dot(s.normal, incoming), 0.0);
    if n_dot_l > 0.0 {
        let n_dot_h = max(dot(s.normal, half_vector), 0.0);
        var specular_brdf = brdf_specular_ggx(s.alpha, n_dot_l, s.n_dot_v, n_dot_h);
        if scena_material_anisotropy {
            if s.anisotropy > 0.0 {
                specular_brdf = surface_anisotropic_specular(s, incoming, half_vector, n_dot_l, n_dot_h);
            }
        }
        let fresnel = surface_fresnel(s, v_dot_h);
        let diffuse = (vec3<f32>(1.0) - fresnel) * s.diffuse_color / PI;
        var layer = diffuse + fresnel * specular_brdf * base_specular_weight;
        if scena_material_sheen {
            let sheen_alpha = s.sheen_roughness * s.sheen_roughness;
            layer = layer * s.sheen_scaling +
                s.sheen_color * distribution_charlie(sheen_alpha, n_dot_h) *
                visibility_neubelt(n_dot_l, s.n_dot_v);
        }
        if scena_material_dispersion {
            if s.dispersion > 0.0 {
                // Wavelength-dependent reflectance of the dispersive dielectric.
                layer += fresnel_schlick(v_dot_h, s.dispersion_f0) *
                    brdf_specular_ggx(s.alpha, n_dot_l, s.n_dot_v, n_dot_h) * s.dispersion;
            }
        }
        lit = layer * radiance * (n_dot_l * s.clearcoat_attenuation);
    }
    if scena_material_clearcoat {
        if s.clearcoat > 0.0 {
            let coat_n_dot_l = max(dot(s.clearcoat_normal, incoming), 0.0);
            if coat_n_dot_l > 0.0 {
                let coat_n_dot_v = max(dot(s.clearcoat_normal, s.view), 0.001);
                let coat_n_dot_h = max(dot(s.clearcoat_normal, half_vector), 0.0);
                let coat_alpha = s.clearcoat_roughness * s.clearcoat_roughness;
                let coat_brdf = brdf_specular_ggx(coat_alpha, coat_n_dot_l, coat_n_dot_v, coat_n_dot_h);
                let coat_fresnel = fresnel_schlick(v_dot_h, vec3<f32>(0.04));
                lit += coat_fresnel * coat_brdf * radiance * (coat_n_dot_l * s.clearcoat);
            }
        }
    }
    return lit;
}

// Estevez and Kulla "Charlie" sheen distribution.
fn distribution_charlie(alpha: f32, n_dot_h: f32) -> f32 {
    let inverse_alpha = 1.0 / max(alpha, 0.0001);
    let sin_squared = max(1.0 - n_dot_h * n_dot_h, 0.0078125);
    return (2.0 + inverse_alpha) * pow(sin_squared, inverse_alpha * 0.5) / (2.0 * PI);
}

// Neubelt and Pettineo cloth visibility term.
fn visibility_neubelt(n_dot_l: f32, n_dot_v: f32) -> f32 {
    return clamp(1.0 / (4.0 * (n_dot_l + n_dot_v - n_dot_l * n_dot_v)), 0.0, 1.0);
}

// Analytic fit of the sheen directional albedo used for image-based lighting.
fn sheen_environment_brdf(n_dot_v: f32, roughness: f32) -> f32 {
    let r2 = roughness * roughness;
    let a = select(
        -8.48 * r2 + 14.3 * roughness - 9.95,
        -339.2 * r2 + 161.4 * roughness - 25.9,
        roughness < 0.25,
    );
    let b = select(
        1.97 * r2 - 3.27 * roughness + 0.72,
        44.0 * r2 - 23.7 * roughness + 3.26,
        roughness < 0.25,
    );
    let dg = exp(a * n_dot_v + b) + select(0.1 * (roughness - 0.25), 0.0, roughness < 0.25);
    return clamp(dg / PI, 0.0, 1.0);
}

// Order-2 spherical-harmonics irradiance divided by pi: the radiance leaving a
// white Lambertian surface that faces `normal`. Coefficient 0 is the mean, so
// a uniform environment evaluates to `environment_diffuse_intensity.rgb`.
fn environment_irradiance(normal: vec3<f32>) -> vec3<f32> {
    let n = rotate_environment_direction(normal);
    let value = camera.environment_sh[0].rgb +
        camera.environment_sh[1].rgb * n.y +
        camera.environment_sh[2].rgb * n.z +
        camera.environment_sh[3].rgb * n.x +
        camera.environment_sh[4].rgb * (n.x * n.y) +
        camera.environment_sh[5].rgb * (n.y * n.z) +
        camera.environment_sh[6].rgb * (3.0 * n.z * n.z - 1.0) +
        camera.environment_sh[7].rgb * (n.x * n.z) +
        camera.environment_sh[8].rgb * (n.x * n.x - n.y * n.y);
    return max(value, vec3<f32>(0.0));
}

fn environment_prefiltered(direction: vec3<f32>, world_position: vec3<f32>, roughness: f32) -> vec3<f32> {
    let reflection = environment_reflection_direction(world_position, direction);
    return textureSampleLevel(
        environment_cubemap,
        environment_sampler,
        reflection,
        environment_prefilter_mip(roughness),
    ).rgb;
}

// Image-based lighting for every layer. The base layer uses the split-sum
// approximation with the Fdez-Aguera multiple-scattering term, so rough
// metals keep the energy single scattering loses.
fn surface_environment_light(s: Surface, world_position: vec3<f32>) -> vec3<f32> {
    if !has_environment_light() {
        return vec3<f32>(0.0);
    }
    let diffuse_intensity = camera.lighting.environment_diffuse_intensity.w;
    let specular_intensity = camera.lighting.environment_specular_intensity.w;
    let irradiance = environment_irradiance(s.normal) * diffuse_intensity;

    var reflection_normal = s.normal;
    if scena_material_anisotropy {
        if s.anisotropy > 0.0 {
            // Bend the reflection toward the anisotropic bitangent so the
            // highlight stretches along the grooves.
            let groove_tangent = cross(s.anisotropy_b, s.view);
            let groove_normal = cross(groove_tangent, s.anisotropy_b);
            let bend = 1.0 - s.anisotropy * (1.0 - s.roughness);
            reflection_normal = normalize(mix(groove_normal, s.normal, pow4(bend)));
        }
    }
    let prefiltered = environment_prefiltered(reflect(-s.view, reflection_normal), world_position, s.roughness);
    let lut = split_sum_brdf_table(s.n_dot_v, s.roughness);
    var f0 = s.f0;
    if scena_material_iridescence {
        // Invert Schlick at the view angle so the film colour drives the
        // integrated reflectance too.
        let fresnel_weight = pow5(1.0 - s.n_dot_v);
        let film_f0 = clamp(
            (s.iridescence_fresnel - vec3<f32>(fresnel_weight)) / max(1.0 - fresnel_weight, 0.0001),
            vec3<f32>(0.0),
            vec3<f32>(1.0),
        );
        f0 = mix(f0, film_f0, s.iridescence);
    }
    let single_scatter = f0 * lut.x + vec3<f32>(s.f90 * lut.y);
    let energy_single = lut.x + lut.y;
    let energy_multiple = 1.0 - energy_single;
    let f_average = f0 + (vec3<f32>(1.0) - f0) / 21.0;
    let multiple_scatter = single_scatter * f_average * energy_multiple /
        (vec3<f32>(1.0) - f_average * energy_multiple);
    let diffuse_energy = s.diffuse_color * (vec3<f32>(1.0) - single_scatter - multiple_scatter);
    var lit = (multiple_scatter + diffuse_energy) * irradiance +
        prefiltered * single_scatter * specular_intensity;
    if scena_material_sheen {
        lit = lit * s.sheen_scaling +
            s.sheen_color * irradiance * sheen_environment_brdf(s.n_dot_v, s.sheen_roughness);
    }
    lit *= s.clearcoat_attenuation;
    if scena_material_clearcoat {
        if s.clearcoat > 0.0 {
            let coat_n_dot_v = max(dot(s.clearcoat_normal, s.view), 0.001);
            let coat_prefiltered = environment_prefiltered(
                reflect(-s.view, s.clearcoat_normal),
                world_position,
                s.clearcoat_roughness,
            );
            let coat_lut = split_sum_brdf_table(coat_n_dot_v, s.clearcoat_roughness);
            lit += coat_prefiltered * (0.04 * coat_lut.x + coat_lut.y) * (specular_intensity * s.clearcoat);
        }
    }
    return lit;
}

// Unit tangent and bitangent of the anisotropic highlight in world space.
fn surface_anisotropy_tangent(
    normal: vec3<f32>,
    tangent: vec3<f32>,
    tangent_handedness: f32,
    direction: vec2<f32>,
    rotation: f32,
) -> vec3<f32> {
    let safe_tangent = normalize(tangent - normal * dot(tangent, normal));
    let bitangent = normalize(cross(normal, safe_tangent) * select(-1.0, 1.0, tangent_handedness >= 0.0));
    let rotated = rotated_anisotropy_direction(direction, rotation);
    return normalize(safe_tangent * rotated.x + bitangent * rotated.y);
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

// Thin-film interference (Belcour and Barla 2017), as specified by
// KHR_materials_iridescence. Returns the reflectance of a film of
// `thickness_nm` and index `film_ior` over a base with reflectance `base_f0`,
// seen at `cos_theta` from air.
fn iridescence_fresnel(film_ior_input: f32, cos_theta: f32, thickness_nm: f32, base_f0: vec3<f32>) -> vec3<f32> {
    // A vanishing film must not leave a hard interface behind.
    let film_ior = mix(1.0, film_ior_input, smoothstep(0.0, 0.03, thickness_nm));
    let sin_theta2_squared = (1.0 / (film_ior * film_ior)) * (1.0 - cos_theta * cos_theta);
    let cos_theta2_squared = 1.0 - sin_theta2_squared;
    if cos_theta2_squared < 0.0 {
        return vec3<f32>(1.0);
    }
    let cos_theta2 = sqrt(cos_theta2_squared);

    // Air to film.
    let r0 = f0_from_ior(film_ior);
    let r12 = r0 + (1.0 - r0) * pow5(1.0 - cos_theta);
    let t121 = 1.0 - r12;
    let phi21 = select(PI, 0.0, film_ior < 1.0);

    // Film to base.
    let sqrt_f0 = sqrt(clamp(base_f0, vec3<f32>(0.0), vec3<f32>(0.9999)));
    let base_ior = (vec3<f32>(1.0) + sqrt_f0) / (vec3<f32>(1.0) - sqrt_f0);
    let base_ratio = (base_ior - vec3<f32>(film_ior)) / (base_ior + vec3<f32>(film_ior));
    let r1 = base_ratio * base_ratio;
    let r23 = r1 + (vec3<f32>(1.0) - r1) * pow5(1.0 - cos_theta2);
    let phi23 = select(vec3<f32>(0.0), vec3<f32>(PI), base_ior < vec3<f32>(film_ior));

    let optical_path_difference = 2.0 * film_ior * thickness_nm * cos_theta2;
    let phi = vec3<f32>(phi21) + phi23;

    let r123 = clamp(r12 * r23, vec3<f32>(0.00001), vec3<f32>(0.9999));
    let r123_sqrt = sqrt(r123);
    let rs = t121 * t121 * r23 / (vec3<f32>(1.0) - r123);

    var reflectance = vec3<f32>(r12) + rs;
    var cm = rs - vec3<f32>(t121);
    cm *= r123_sqrt;
    reflectance += cm * 2.0 * iridescence_sensitivity(optical_path_difference, phi);
    cm *= r123_sqrt;
    reflectance += cm * 2.0 * iridescence_sensitivity(2.0 * optical_path_difference, 2.0 * phi);
    return max(reflectance, vec3<f32>(0.0));
}

// CIE XYZ sensitivity curves evaluated in Fourier space, converted to linear
// Rec.709.
fn iridescence_sensitivity(optical_path_difference: f32, shift: vec3<f32>) -> vec3<f32> {
    let phase = 2.0 * PI * optical_path_difference * 1.0e-9;
    let value = vec3<f32>(5.4856e-13, 4.4201e-13, 5.2481e-13);
    let position = vec3<f32>(1.6810e+06, 1.7953e+06, 2.2084e+06);
    let variance = vec3<f32>(4.3278e+09, 9.3046e+09, 6.6121e+09);
    var xyz = value * sqrt(2.0 * PI * variance) * cos(position * phase + shift) *
        exp(-phase * phase * variance);
    xyz.x += 9.7470e-14 * sqrt(2.0 * PI * 4.5282e+09) * cos(2.2399e+06 * phase + shift.x) *
        exp(-4.5282e+09 * phase * phase);
    xyz /= 1.0685e-7;
    return vec3<f32>(
        dot(vec3<f32>(3.2404542, -1.5371385, -0.4985314), xyz),
        dot(vec3<f32>(-0.9692660, 1.8760108, 0.0415560), xyz),
        dot(vec3<f32>(0.0556434, -0.2040259, 1.0572252), xyz),
    );
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
