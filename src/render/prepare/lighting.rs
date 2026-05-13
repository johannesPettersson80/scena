use super::pbr_contract::{
    PbrMaterial, directional_illuminance_lux, inverse_square_range_attenuation,
    punctual_intensity_candela, punctual_light_contribution, roughness_or_min,
    spot_cone_attenuation,
};
use crate::assets::EnvironmentDesc;
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{Light, Quat, Scene, Transform, Vec3};

use super::environment::PreparedEnvironmentLighting;

#[derive(Clone)]
pub(super) struct MaterialShadingInput {
    pub(super) position: Vec3,
    pub(super) normal: Vec3,
    pub(super) camera_position: Option<Vec3>,
    pub(super) base_color_texture: Color,
    pub(super) metallic_roughness_texture: (f32, f32),
    pub(super) occlusion_texture: f32,
    pub(super) emissive_texture: Color,
    pub(super) environment: PreparedEnvironmentLighting,
    pub(super) directional_shadow_factor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedGpuLightUniform {
    pub(in crate::render) directional_light_direction_intensity: [f32; 4],
    pub(in crate::render) directional_light_color_count: [f32; 4],
    pub(in crate::render) point_light_position_intensity: [f32; 4],
    pub(in crate::render) point_light_color_range: [f32; 4],
    pub(in crate::render) spot_light_position_intensity: [f32; 4],
    pub(in crate::render) spot_light_direction_cones: [f32; 4],
    pub(in crate::render) spot_light_cone_range: [f32; 4],
    pub(in crate::render) spot_light_color_range: [f32; 4],
    pub(in crate::render) environment_diffuse_intensity: [f32; 4],
    pub(in crate::render) environment_specular_intensity: [f32; 4],
}

impl Default for PreparedGpuLightUniform {
    fn default() -> Self {
        Self {
            directional_light_direction_intensity: [0.0, 0.0, -1.0, 0.0],
            directional_light_color_count: [1.0, 1.0, 1.0, 0.0],
            point_light_position_intensity: [0.0, 0.0, 0.0, 0.0],
            point_light_color_range: [1.0, 1.0, 1.0, 0.0],
            spot_light_position_intensity: [0.0, 0.0, 0.0, 0.0],
            spot_light_direction_cones: [0.0, 0.0, -1.0, 0.0],
            spot_light_cone_range: [0.0, 0.0, 0.0, 0.0],
            spot_light_color_range: [1.0, 1.0, 1.0, 0.0],
            environment_diffuse_intensity: [0.0, 0.0, 0.0, 0.0],
            environment_specular_intensity: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

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
    casts_shadows: bool,
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
                    casts_shadows: light.casts_shadows(),
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

    pub(super) fn primary_shadow_ray_direction(&self) -> Option<Vec3> {
        self.directional
            .iter()
            .find(|light| light.casts_shadows)
            .map(|light| negate_vec3(light.direction))
    }

    pub(super) fn gpu_uniform(
        &self,
        environment: PreparedEnvironmentLighting,
    ) -> PreparedGpuLightUniform {
        let mut uniform = PreparedGpuLightUniform::default();
        if let Some(light) = self.directional.first() {
            uniform.directional_light_direction_intensity = [
                light.direction.x,
                light.direction.y,
                light.direction.z,
                directional_illuminance_lux(light.illuminance_lux),
            ];
            uniform.directional_light_color_count = [
                light.color.r,
                light.color.g,
                light.color.b,
                self.directional.len() as f32,
            ];
        }
        if let Some(light) = self.point.first() {
            uniform.point_light_position_intensity = [
                light.position.x,
                light.position.y,
                light.position.z,
                punctual_intensity_candela(light.intensity_candela),
            ];
            uniform.point_light_color_range = [
                light.color.r,
                light.color.g,
                light.color.b,
                light.range.unwrap_or(0.0).max(0.0),
            ];
        }
        if let Some(light) = self.spot.first() {
            uniform.spot_light_position_intensity = [
                light.position.x,
                light.position.y,
                light.position.z,
                punctual_intensity_candela(light.intensity_candela),
            ];
            uniform.spot_light_direction_cones =
                [light.direction.x, light.direction.y, light.direction.z, 0.0];
            uniform.spot_light_cone_range = [
                light.inner_cone_cos,
                light.outer_cone_cos,
                light.range.unwrap_or(0.0).max(0.0),
                self.spot.len() as f32,
            ];
            uniform.spot_light_color_range = [light.color.r, light.color.g, light.color.b, 0.0];
        }
        if environment.is_active() {
            uniform.environment_diffuse_intensity = environment.gpu_diffuse_intensity();
            uniform.environment_specular_intensity = environment.gpu_specular_intensity();
        }
        uniform
    }
}

pub(in crate::render) fn collect_gpu_light_uniform(
    scene: &Scene,
    origin_shift: Vec3,
    environment: Option<&EnvironmentDesc>,
) -> PreparedGpuLightUniform {
    PreparedLights::from_scene(scene, origin_shift)
        .gpu_uniform(PreparedEnvironmentLighting::from_environment(environment))
}

pub(super) fn material_color(
    material: &MaterialDesc,
    lights: &PreparedLights,
    input: &MaterialShadingInput,
) -> Color {
    let base = multiply_color(material.base_color(), input.base_color_texture);
    let mut color = match material.kind() {
        MaterialKind::Unlit => base,
        MaterialKind::PbrMetallicRoughness
            if lights.has_direct_lights() || input.environment.is_active() =>
        {
            let mut color = shade_pbr_base_color(material, base, lights, input);
            let occlusion = input.occlusion_texture.clamp(0.0, 1.0);
            color.r *= occlusion;
            color.g *= occlusion;
            color.b *= occlusion;
            color
        }
        MaterialKind::PbrMetallicRoughness => base,
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge => base,
    };
    let emissive = material.emissive();
    let emissive_strength = material.emissive_strength();
    color.r += emissive.r * input.emissive_texture.r * emissive_strength;
    color.g += emissive.g * input.emissive_texture.g * emissive_strength;
    color.b += emissive.b * input.emissive_texture.b * emissive_strength;
    match material.alpha_mode() {
        AlphaMode::Opaque => color.a = 1.0,
        AlphaMode::Blend => {}
        AlphaMode::Mask { .. } => {}
    }
    color
}

fn shade_pbr_base_color(
    material: &MaterialDesc,
    base: Color,
    lights: &PreparedLights,
    input: &MaterialShadingInput,
) -> Color {
    let normal = normalize_or(input.normal, Vec3::new(0.0, 0.0, 1.0));
    let view = input
        .camera_position
        .map(|camera| {
            normalize_or(
                subtract_vec3(camera, input.position),
                Vec3::new(0.0, 0.0, 1.0),
            )
        })
        .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
    let base_rgb = Vec3::new(base.r, base.g, base.b);
    let metallic = clamp_unit(material.metallic_factor() * input.metallic_roughness_texture.0);
    let roughness =
        roughness_or_min(material.roughness_factor() * input.metallic_roughness_texture.1);
    let pbr_material = PbrMaterial::new(base_rgb, metallic, roughness);
    let mut shaded = Vec3::ZERO;

    for light in &lights.directional {
        let incoming = negate_vec3(light.direction);
        let shadow_factor = if light.casts_shadows {
            input.directional_shadow_factor.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let radiance = scale_color(
            light.color,
            directional_illuminance_lux(light.illuminance_lux) * shadow_factor,
        );
        shaded = add_vec3(
            shaded,
            punctual_light_contribution(pbr_material, normal, view, incoming, radiance),
        );
    }
    for light in &lights.point {
        let to_light = subtract_vec3(light.position, input.position);
        let incoming = normalize_or(to_light, Vec3::ZERO);
        let radiance = scale_color(
            light.color,
            punctual_intensity_candela(light.intensity_candela)
                * inverse_square_range_attenuation(to_light, light.range),
        );
        shaded = add_vec3(
            shaded,
            punctual_light_contribution(pbr_material, normal, view, incoming, radiance),
        );
    }
    for light in &lights.spot {
        let to_light = subtract_vec3(light.position, input.position);
        let incoming = normalize_or(to_light, Vec3::ZERO);
        let to_surface = negate_vec3(incoming);
        let cone = spot_cone_attenuation(
            dot_vec3(to_surface, light.direction),
            light.inner_cone_cos,
            light.outer_cone_cos,
        );
        let radiance = scale_color(
            light.color,
            punctual_intensity_candela(light.intensity_candela)
                * inverse_square_range_attenuation(to_light, light.range)
                * cone,
        );
        shaded = add_vec3(
            shaded,
            punctual_light_contribution(pbr_material, normal, view, incoming, radiance),
        );
    }
    shaded = add_vec3(
        shaded,
        input
            .environment
            .pbr_contribution(pbr_material, normal, view),
    );

    Color::from_linear_rgba(shaded.x, shaded.y, shaded.z, base.a)
}

fn multiply_color(left: Color, right: Color) -> Color {
    Color::from_linear_rgba(
        left.r * right.r,
        left.g * right.g,
        left.b * right.b,
        left.a * right.a,
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

fn clamp_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
