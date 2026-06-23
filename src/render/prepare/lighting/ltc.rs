use crate::scene::{AreaLightShape, Vec3};

use super::super::pbr_contract::{PbrMaterial, inverse_square_range_attenuation};
use super::area::PreparedAreaLight;
use super::math::{add_vec3, dot_vec3, normalize_or, scale_color, subtract_vec3};

const MIN_DENOMINATOR: f32 = 0.0001;
const INV_TWO_PI: f32 = 1.0 / (2.0 * std::f32::consts::PI);

pub(super) fn ltc_area_light_specular_contribution(
    light: PreparedAreaLight,
    position: Vec3,
    normal: Vec3,
    view: Vec3,
    material: PbrMaterial,
    shadow_factor: f32,
) -> Vec3 {
    let shadow_factor = shadow_factor.clamp(0.0, 1.0);
    if shadow_factor <= f32::EPSILON {
        return Vec3::ZERO;
    }
    let normal = normalize_or(normal, Vec3::Y);
    let view = normalize_or(view, normal);
    let n_dot_v = dot_vec3(normal, view).clamp(0.0, 1.0);
    if n_dot_v <= f32::EPSILON {
        return Vec3::ZERO;
    }

    let polygon = ltc_area_light_polygon(light, position, normal);
    let irradiance = ltc_evaluate(polygon, position, normal, view, material.roughness);
    if irradiance <= f32::EPSILON {
        return Vec3::ZERO;
    }

    let to_light = subtract_vec3(light.position, position);
    let radiance = scale_color(
        light.color,
        light.luminous_flux_lumens / (4.0 * std::f32::consts::PI)
            * inverse_square_range_attenuation(to_light, light.range),
    );
    let fresnel = fresnel_schlick(n_dot_v, material.f0());
    let roughness_gain = 0.45 + (1.0 - material.roughness).clamp(0.0, 1.0) * 0.75;
    scale_vec3(
        multiply_vec3(fresnel, radiance),
        irradiance * roughness_gain * shadow_factor,
    )
}

fn ltc_area_light_polygon(light: PreparedAreaLight, position: Vec3, normal: Vec3) -> [Vec3; 4] {
    match light.shape {
        AreaLightShape::Rect { .. } => [
            light.position - light.axis_x - light.axis_y,
            light.position + light.axis_x - light.axis_y,
            light.position + light.axis_x + light.axis_y,
            light.position - light.axis_x + light.axis_y,
        ],
        AreaLightShape::Disc { .. } => [
            light.position - light.axis_x,
            light.position - light.axis_y,
            light.position + light.axis_x,
            light.position + light.axis_y,
        ],
        AreaLightShape::Sphere { .. } => {
            let radius = light.axis_x.length().max(light.axis_y.length()).max(0.001);
            let to_surface = normalize_or(subtract_vec3(position, light.position), -normal);
            let tangent = normalize_or(cross_vec3(normal, to_surface), Vec3::X) * radius;
            let bitangent = normalize_or(cross_vec3(to_surface, tangent), Vec3::Z) * radius;
            [
                light.position - tangent - bitangent,
                light.position + tangent - bitangent,
                light.position + tangent + bitangent,
                light.position - tangent + bitangent,
            ]
        }
    }
}

fn ltc_evaluate(
    polygon: [Vec3; 4],
    position: Vec3,
    normal: Vec3,
    view: Vec3,
    roughness: f32,
) -> f32 {
    let (tangent, bitangent) = ltc_matrix(normal, view, roughness);
    let roughness = roughness.clamp(0.04, 1.0);
    let tangent_scale = (0.42 + roughness * 0.58).max(0.04);
    let bitangent_scale = (0.55 + roughness * 0.45).max(0.04);
    let vertices = polygon.map(|point| {
        let local = subtract_vec3(point, position);
        normalize_or(
            Vec3::new(
                dot_vec3(local, tangent) / tangent_scale,
                dot_vec3(local, bitangent) / bitangent_scale,
                dot_vec3(local, normal).max(0.0),
            ),
            Vec3::Z,
        )
    });

    let mut integral = 0.0;
    for index in 0..vertices.len() {
        integral += ltc_integrate_edge(vertices[index], vertices[(index + 1) % vertices.len()]);
    }
    (integral.abs() * INV_TWO_PI).max(0.0)
}

fn ltc_matrix(normal: Vec3, view: Vec3, roughness: f32) -> (Vec3, Vec3) {
    let tangent = normalize_or(
        subtract_vec3(view, scale_vec3(normal, dot_vec3(view, normal))),
        {
            let fallback_axis = if normal.z.abs() < 0.9 {
                Vec3::Z
            } else {
                Vec3::Y
            };
            normalize_or(cross_vec3(fallback_axis, normal), Vec3::X)
        },
    );
    let bitangent = normalize_or(cross_vec3(normal, tangent), Vec3::Z);
    let skew = (1.0 - roughness.clamp(0.04, 1.0)) * 0.18;
    (
        normalize_or(add_vec3(tangent, scale_vec3(normal, skew)), tangent),
        bitangent,
    )
}

fn ltc_integrate_edge(a: Vec3, b: Vec3) -> f32 {
    let cosine = dot_vec3(a, b).clamp(-0.9999, 0.9999);
    let y = cosine.abs();
    let numerator = 0.854_398_5 + (0.496_515_5 + 0.014_520_6 * y) * y;
    let denominator = 3.417_594 + (4.161_672_6 + y) * y;
    let approximation = numerator / denominator.max(MIN_DENOMINATOR);
    let theta_sin_theta = if cosine > 0.0 {
        approximation
    } else {
        0.5 * (1.0 - cosine * cosine).max(MIN_DENOMINATOR).sqrt().recip() - approximation
    };
    cross_vec3(a, b).z * theta_sin_theta
}

fn fresnel_schlick(cos_theta: f32, f0: Vec3) -> Vec3 {
    let factor = (1.0 - cos_theta.clamp(0.0, 1.0)).powi(5);
    add_vec3(
        f0,
        scale_vec3(subtract_vec3(Vec3::new(1.0, 1.0, 1.0), f0), factor),
    )
}

fn multiply_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x * right.x, left.y * right.y, left.z * right.z)
}

fn scale_vec3(value: Vec3, scale: f32) -> Vec3 {
    Vec3::new(value.x * scale, value.y * scale, value.z * scale)
}

fn cross_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(
        left.y * right.z - left.z * right.y,
        left.z * right.x - left.x * right.z,
        left.x * right.y - left.y * right.x,
    )
}

#[cfg(test)]
mod tests {
    use crate::material::Color;
    use crate::scene::AreaLightShape;

    use super::*;

    #[test]
    fn area_ltc_specular_is_width_and_shape_sensitive() {
        let material = PbrMaterial::new(Vec3::new(0.82, 0.78, 0.72), 1.0, 0.34);
        let position = Vec3::ZERO;
        let normal = Vec3::Y;
        let view = normalize_or(Vec3::new(0.0, 0.8, 1.6), Vec3::Y);
        let base = PreparedAreaLight {
            color: Color::from_linear_rgb(1.0, 0.96, 0.9),
            position: Vec3::new(0.0, 1.35, 0.32),
            axis_x: Vec3::X * 0.05,
            axis_y: Vec3::Z * 0.05,
            luminous_flux_lumens: 900.0,
            range: None,
            shape: AreaLightShape::rect(0.1, 0.1),
        };
        let wide = PreparedAreaLight {
            axis_x: Vec3::X * 0.9,
            axis_y: Vec3::Z * 0.45,
            shape: AreaLightShape::rect(1.8, 0.9),
            ..base
        };
        let disc = PreparedAreaLight {
            axis_x: Vec3::X * 0.55,
            axis_y: Vec3::Z * 0.55,
            shape: AreaLightShape::disc(0.55),
            ..base
        };
        let sphere = PreparedAreaLight {
            axis_x: Vec3::X * 0.42,
            axis_y: Vec3::Z * 0.42,
            shape: AreaLightShape::sphere(0.42),
            ..base
        };

        let narrow_specular =
            ltc_area_light_specular_contribution(base, position, normal, view, material, 1.0);
        let wide_specular =
            ltc_area_light_specular_contribution(wide, position, normal, view, material, 1.0);
        let disc_specular =
            ltc_area_light_specular_contribution(disc, position, normal, view, material, 1.0);
        let sphere_specular =
            ltc_area_light_specular_contribution(sphere, position, normal, view, material, 1.0);

        assert!(
            wide_specular.x > narrow_specular.x * 1.6,
            "wide rectangular LTC area light should produce a materially broader/brighter specular response than a tiny emitter; narrow={narrow_specular:?}, wide={wide_specular:?}"
        );
        assert!(
            disc_specular.x > 0.0 && sphere_specular.x > 0.0,
            "disc and sphere LTC area light paths must not be inert; disc={disc_specular:?}, sphere={sphere_specular:?}"
        );
    }
}
