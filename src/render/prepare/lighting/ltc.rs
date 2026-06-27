use crate::scene::{AreaLightShape, Vec3};

use super::super::super::area_ltc;
use super::super::pbr_contract::{PbrMaterial, inverse_square_range_attenuation};
use super::area::PreparedAreaLight;
use super::math::{normalize_or, scale_color, subtract_vec3};

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
    let polygon = ltc_area_light_polygon(light, position, normal);
    let probe = area_ltc::evaluate_specular_polygon(
        polygon,
        position,
        normal,
        view,
        material.roughness,
        material.f0(),
    );
    if probe.irradiance <= f32::EPSILON {
        return Vec3::ZERO;
    }

    let to_light = subtract_vec3(light.position, position);
    let radiance = scale_color(
        light.color,
        light.luminous_flux_lumens / (4.0 * std::f32::consts::PI)
            * inverse_square_range_attenuation(to_light, light.range),
    );
    scale_vec3(
        multiply_vec3(probe.fresnel_scale, radiance),
        probe.irradiance * shadow_factor,
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

    #[test]
    fn area_ltc_specular_matches_selfshadow_reference_probe() {
        let material = PbrMaterial::new(Vec3::new(0.82, 0.78, 0.72), 1.0, 0.34);
        let position = Vec3::ZERO;
        let normal = Vec3::Y;
        let view = normalize_or(Vec3::new(0.0, 0.8, 1.6), Vec3::Y);
        let wide = PreparedAreaLight {
            color: Color::from_linear_rgb(1.0, 0.96, 0.9),
            position: Vec3::new(0.0, 1.35, 0.32),
            axis_x: Vec3::X * 0.9,
            axis_y: Vec3::Z * 0.45,
            luminous_flux_lumens: 900.0,
            range: None,
            shape: AreaLightShape::rect(1.8, 0.9),
        };

        let actual =
            ltc_area_light_specular_contribution(wide, position, normal, view, material, 1.0);
        let expected = Vec3::new(0.172_101_13, 0.157_660_25, 0.137_179_69);
        let delta = (actual - expected).abs();

        assert!(
            delta.max_element() <= 0.000_75,
            "LTC specular must match the selfshadow/ltc_code fitted-table probe; actual={actual:?}, expected={expected:?}, delta={delta:?}"
        );
    }
}
