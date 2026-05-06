use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{Light, Quat, Scene, Transform, Vec3};

#[derive(Default)]
pub(super) struct PreparedLights {
    directional: Vec<PreparedDirectionalLight>,
    point: Vec<PreparedPointLight>,
    spot: Vec<PreparedSpotLight>,
}

#[derive(Clone, Copy)]
struct PreparedDirectionalLight {
    color: Color,
    direction: Vec3,
    illuminance_lux: f32,
}

#[derive(Clone, Copy)]
struct PreparedPointLight {
    color: Color,
    position: Vec3,
    intensity_candela: f32,
    range: Option<f32>,
}

#[derive(Clone, Copy)]
struct PreparedSpotLight {
    color: Color,
    position: Vec3,
    direction: Vec3,
    intensity_candela: f32,
    range: Option<f32>,
    inner_cone_cos: f32,
    outer_cone_cos: f32,
}

impl PreparedLights {
    pub(super) fn from_scene(scene: &Scene, origin_shift: Vec3) -> Self {
        let mut lights = Self::default();
        for (_node, _light_key, light, transform) in scene.light_nodes() {
            match light {
                Light::Directional(light) => lights.directional.push(PreparedDirectionalLight {
                    color: light.color(),
                    direction: light_direction(transform),
                    illuminance_lux: light.illuminance_lux(),
                }),
                Light::Point(light) => lights.point.push(PreparedPointLight {
                    color: light.color(),
                    position: subtract_vec3(transform.translation, origin_shift),
                    intensity_candela: light.intensity_candela(),
                    range: light.range(),
                }),
                Light::Spot(light) => lights.spot.push(PreparedSpotLight {
                    color: light.color(),
                    position: subtract_vec3(transform.translation, origin_shift),
                    direction: light_direction(transform),
                    intensity_candela: light.intensity_candela(),
                    range: light.range(),
                    inner_cone_cos: light.inner_cone_angle().radians().cos(),
                    outer_cone_cos: light.outer_cone_angle().radians().cos(),
                }),
            }
        }
        lights
    }

    fn has_direct_lights(&self) -> bool {
        !self.directional.is_empty() || !self.point.is_empty() || !self.spot.is_empty()
    }
}

pub(super) fn material_color(
    material: &MaterialDesc,
    position: Vec3,
    normal: Vec3,
    lights: &PreparedLights,
) -> Color {
    let base = material.base_color();
    let mut color = match material.kind() {
        MaterialKind::Unlit => base,
        MaterialKind::PbrMetallicRoughness if lights.has_direct_lights() => {
            shade_pbr_base_color(base, position, normal, lights)
        }
        MaterialKind::PbrMetallicRoughness => base,
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge => base,
    };
    let emissive = material.emissive();
    let emissive_strength = material.emissive_strength();
    color.r += emissive.r * emissive_strength;
    color.g += emissive.g * emissive_strength;
    color.b += emissive.b * emissive_strength;
    match material.alpha_mode() {
        AlphaMode::Opaque => color.a = 1.0,
        AlphaMode::Blend => {}
        AlphaMode::Mask { .. } => {}
    }
    color
}

fn shade_pbr_base_color(
    base: Color,
    position: Vec3,
    normal: Vec3,
    lights: &PreparedLights,
) -> Color {
    let normal = normalize_or(normal, Vec3::new(0.0, 0.0, 1.0));
    let mut irradiance = Vec3::ZERO;

    for light in &lights.directional {
        let incoming = negate_vec3(light.direction);
        let strength = dot_vec3(normal, incoming).max(0.0)
            * (light.illuminance_lux / 10_000.0).clamp(0.0, 8.0);
        irradiance = add_vec3(irradiance, scale_color(light.color, strength));
    }
    for light in &lights.point {
        let to_light = subtract_vec3(light.position, position);
        let incoming = normalize_or(to_light, Vec3::ZERO);
        let strength = dot_vec3(normal, incoming).max(0.0)
            * (light.intensity_candela / 100.0).clamp(0.0, 8.0)
            * distance_attenuation(to_light, light.range);
        irradiance = add_vec3(irradiance, scale_color(light.color, strength));
    }
    for light in &lights.spot {
        let to_light = subtract_vec3(light.position, position);
        let incoming = normalize_or(to_light, Vec3::ZERO);
        let to_surface = negate_vec3(incoming);
        let cone = spot_cone_attenuation(
            dot_vec3(to_surface, light.direction),
            light.inner_cone_cos,
            light.outer_cone_cos,
        );
        let strength = dot_vec3(normal, incoming).max(0.0)
            * (light.intensity_candela / 100.0).clamp(0.0, 8.0)
            * distance_attenuation(to_light, light.range)
            * cone;
        irradiance = add_vec3(irradiance, scale_color(light.color, strength));
    }

    Color::from_linear_rgba(
        base.r * irradiance.x,
        base.g * irradiance.y,
        base.b * irradiance.z,
        base.a,
    )
}

fn light_direction(transform: Transform) -> Vec3 {
    normalize_or(
        rotate_vec3(transform.rotation, Vec3::new(0.0, 0.0, -1.0)),
        Vec3::new(0.0, 0.0, -1.0),
    )
}

fn rotate_vec3(rotation: Quat, vector: Vec3) -> Vec3 {
    let length_squared = rotation.x * rotation.x
        + rotation.y * rotation.y
        + rotation.z * rotation.z
        + rotation.w * rotation.w;
    if length_squared <= f32::EPSILON || !length_squared.is_finite() {
        return vector;
    }
    let inverse_length = length_squared.sqrt().recip();
    let qx = rotation.x * inverse_length;
    let qy = rotation.y * inverse_length;
    let qz = rotation.z * inverse_length;
    let qw = rotation.w * inverse_length;
    let tx = 2.0 * (qy * vector.z - qz * vector.y);
    let ty = 2.0 * (qz * vector.x - qx * vector.z);
    let tz = 2.0 * (qx * vector.y - qy * vector.x);
    Vec3::new(
        vector.x + qw * tx + (qy * tz - qz * ty),
        vector.y + qw * ty + (qz * tx - qx * tz),
        vector.z + qw * tz + (qx * ty - qy * tx),
    )
}

fn add_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x + right.x, left.y + right.y, left.z + right.z)
}

fn subtract_vec3(left: Vec3, right: Vec3) -> Vec3 {
    Vec3::new(left.x - right.x, left.y - right.y, left.z - right.z)
}

fn negate_vec3(vector: Vec3) -> Vec3 {
    Vec3::new(-vector.x, -vector.y, -vector.z)
}

fn dot_vec3(left: Vec3, right: Vec3) -> f32 {
    left.x * right.x + left.y * right.y + left.z * right.z
}

fn length_vec3(vector: Vec3) -> f32 {
    dot_vec3(vector, vector).sqrt()
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    let length = length_vec3(vector);
    if length <= f32::EPSILON || !length.is_finite() {
        fallback
    } else {
        Vec3::new(vector.x / length, vector.y / length, vector.z / length)
    }
}

fn scale_color(color: Color, scale: f32) -> Vec3 {
    Vec3::new(color.r * scale, color.g * scale, color.b * scale)
}

fn distance_attenuation(to_light: Vec3, range: Option<f32>) -> f32 {
    let Some(range) = range else {
        return 1.0;
    };
    if range <= f32::EPSILON {
        return 0.0;
    }
    let distance = length_vec3(to_light);
    (1.0 - distance / range).clamp(0.0, 1.0).powi(2)
}

fn spot_cone_attenuation(cos_angle: f32, inner_cone_cos: f32, outer_cone_cos: f32) -> f32 {
    if cos_angle >= inner_cone_cos {
        1.0
    } else if cos_angle <= outer_cone_cos {
        0.0
    } else {
        ((cos_angle - outer_cone_cos) / (inner_cone_cos - outer_cone_cos)).clamp(0.0, 1.0)
    }
}
