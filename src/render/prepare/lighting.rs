use super::pbr_contract::{
    PbrMaterial, directional_illuminance_lux, inverse_square_range_attenuation,
    punctual_intensity_candela, punctual_light_contribution, roughness_or_min,
    spot_cone_attenuation,
};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{Light, Scene, Vec3};

use super::environment::PreparedEnvironmentLighting;

mod lobes;
mod math;
use lobes::LayeredMaterialLobes;
use math::*;

pub(in crate::render) const MAX_GPU_LIGHTS_PER_TYPE: usize = 4;

#[derive(Clone)]
pub(super) struct MaterialShadingInput {
    pub(super) position: Vec3,
    pub(super) normal: Vec3,
    pub(super) tangent: Vec3,
    pub(super) tangent_handedness: f32,
    pub(super) camera_position: Option<Vec3>,
    pub(super) base_color_texture: Color,
    pub(super) metallic_roughness_texture: (f32, f32),
    pub(super) occlusion_texture: f32,
    pub(super) emissive_texture: Color,
    pub(super) clearcoat_texture: f32,
    pub(super) clearcoat_roughness_texture: f32,
    pub(super) clearcoat_normal: Vec3,
    pub(super) sheen_color_texture: Color,
    pub(super) sheen_roughness_texture: f32,
    pub(super) anisotropy_texture: Vec3,
    pub(super) iridescence_texture: f32,
    pub(super) iridescence_thickness_texture: f32,
    pub(super) transmission_texture: f32,
    pub(super) thickness_texture: f32,
    pub(super) environment: PreparedEnvironmentLighting,
    pub(super) directional_shadow_factor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedGpuLightUniform {
    pub(in crate::render) directional_light_direction_intensity:
        [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) directional_light_color: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) directional_shadow_control: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) point_light_position_intensity: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) point_light_color_range: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_position_intensity: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_direction_cones: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_cone_range: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_color_range: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) light_counts: [f32; 4],
    pub(in crate::render) environment_diffuse_intensity: [f32; 4],
    pub(in crate::render) environment_specular_intensity: [f32; 4],
}

impl Default for PreparedGpuLightUniform {
    fn default() -> Self {
        Self {
            directional_light_direction_intensity: [[0.0, 0.0, -1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            directional_light_color: [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            directional_shadow_control: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            point_light_position_intensity: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            point_light_color_range: [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_position_intensity: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_direction_cones: [[0.0, 0.0, -1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_cone_range: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_color_range: [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            light_counts: [0.0, 0.0, 0.0, 0.0],
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
        let mut uniform = PreparedGpuLightUniform {
            light_counts: [
                self.directional.len().min(MAX_GPU_LIGHTS_PER_TYPE) as f32,
                self.point.len().min(MAX_GPU_LIGHTS_PER_TYPE) as f32,
                self.spot.len().min(MAX_GPU_LIGHTS_PER_TYPE) as f32,
                0.0,
            ],
            ..Default::default()
        };
        let mut shadow_slot_used = false;
        for (index, light) in self
            .directional
            .iter()
            .take(MAX_GPU_LIGHTS_PER_TYPE)
            .enumerate()
        {
            uniform.directional_light_direction_intensity[index] = [
                light.direction.x,
                light.direction.y,
                light.direction.z,
                directional_illuminance_lux(light.illuminance_lux),
            ];
            uniform.directional_light_color[index] =
                [light.color.r, light.color.g, light.color.b, 0.0];
            if light.casts_shadows && !shadow_slot_used {
                uniform.directional_shadow_control[index] = [1.0, 0.0, 0.0, 0.0];
                shadow_slot_used = true;
            }
        }
        for (index, light) in self.point.iter().take(MAX_GPU_LIGHTS_PER_TYPE).enumerate() {
            uniform.point_light_position_intensity[index] = [
                light.position.x,
                light.position.y,
                light.position.z,
                punctual_intensity_candela(light.intensity_candela),
            ];
            uniform.point_light_color_range[index] = [
                light.color.r,
                light.color.g,
                light.color.b,
                light.range.unwrap_or(0.0).max(0.0),
            ];
        }
        for (index, light) in self.spot.iter().take(MAX_GPU_LIGHTS_PER_TYPE).enumerate() {
            uniform.spot_light_position_intensity[index] = [
                light.position.x,
                light.position.y,
                light.position.z,
                punctual_intensity_candela(light.intensity_candela),
            ];
            uniform.spot_light_direction_cones[index] =
                [light.direction.x, light.direction.y, light.direction.z, 0.0];
            uniform.spot_light_cone_range[index] = [
                light.inner_cone_cos,
                light.outer_cone_cos,
                light.range.unwrap_or(0.0).max(0.0),
                0.0,
            ];
            uniform.spot_light_color_range[index] =
                [light.color.r, light.color.g, light.color.b, 0.0];
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
    environment: &PreparedEnvironmentLighting,
) -> PreparedGpuLightUniform {
    PreparedLights::from_scene(scene, origin_shift).gpu_uniform(environment.clone())
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
    let clearcoat_factor = clamp_unit(material.clearcoat_factor() * input.clearcoat_texture);
    let clearcoat_roughness =
        roughness_or_min(material.clearcoat_roughness_factor() * input.clearcoat_roughness_texture);
    let clearcoat_normal = normalize_or(input.clearcoat_normal, normal);
    let sheen_color_factor = material.sheen_color_factor();
    let sheen_color = Vec3::new(
        sheen_color_factor.r * input.sheen_color_texture.r,
        sheen_color_factor.g * input.sheen_color_texture.g,
        sheen_color_factor.b * input.sheen_color_texture.b,
    );
    let sheen_roughness =
        roughness_or_min(material.sheen_roughness_factor() * input.sheen_roughness_texture);
    let anisotropy_strength = material.anisotropy_strength_factor();
    let anisotropy_rotation = material.anisotropy_rotation_radians();
    let iridescence_factor = material.iridescence_factor();
    let iridescence_ior = material.iridescence_ior();
    let iridescence_thickness_minimum = material.iridescence_thickness_minimum_nm();
    let iridescence_thickness_maximum = material.iridescence_thickness_maximum_nm();
    let dispersion_factor = material.dispersion_factor();
    let transmission_factor = material.transmission_factor();
    let ior = material.ior();
    let thickness_factor = material.thickness_factor();
    let attenuation_color = material.attenuation_color();
    let attenuation_distance = material.attenuation_distance();
    let layered_lobes = LayeredMaterialLobes {
        pbr_material,
        normal,
        clearcoat_normal,
        view,
        tangent: input.tangent,
        tangent_handedness: input.tangent_handedness,
        clearcoat_factor,
        clearcoat_roughness,
        sheen_color,
        sheen_roughness,
        anisotropy_strength,
        anisotropy_rotation,
        anisotropy_texture: input.anisotropy_texture,
        iridescence_factor,
        iridescence_ior,
        iridescence_thickness_minimum,
        iridescence_thickness_maximum,
        iridescence_texture: input.iridescence_texture,
        iridescence_thickness_texture: input.iridescence_thickness_texture,
        dispersion_factor,
        transmission_factor: transmission_factor * input.transmission_texture,
        ior,
        thickness_factor,
        thickness_texture: input.thickness_texture,
        attenuation_color: Vec3::new(
            attenuation_color.r,
            attenuation_color.g,
            attenuation_color.b,
        ),
        attenuation_distance,
    };
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
        shaded = add_vec3(shaded, layered_lobes.contribution(incoming, radiance));
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
        shaded = add_vec3(shaded, layered_lobes.contribution(incoming, radiance));
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
        shaded = add_vec3(shaded, layered_lobes.contribution(incoming, radiance));
    }
    shaded = add_vec3(
        shaded,
        input
            .environment
            .pbr_contribution(pbr_material, normal, view),
    );
    if input.environment.is_active() {
        let environment_specular = input.environment.gpu_specular_intensity();
        let environment_radiance = Vec3::new(
            environment_specular[0] * environment_specular[3],
            environment_specular[1] * environment_specular[3],
            environment_specular[2] * environment_specular[3],
        );
        shaded = add_vec3(
            shaded,
            layered_lobes.contribution(normal, environment_radiance),
        );
    }

    Color::from_linear_rgba(shaded.x, shaded.y, shaded.z, base.a)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Angle, DirectionalLight, PointLight, SpotLight, Transform};

    #[test]
    fn gpu_light_uniform_consumes_prepared_environment_lighting() {
        let scene = Scene::new();
        let environment = PreparedEnvironmentLighting::default();

        let uniform = collect_gpu_light_uniform(&scene, Vec3::ZERO, &environment);

        assert_eq!(uniform.environment_diffuse_intensity, [0.0; 4]);
        assert_eq!(uniform.environment_specular_intensity, [0.0; 4]);
    }

    #[test]
    fn gpu_light_uniform_encodes_multiple_lights_per_type() {
        let mut scene = Scene::new();
        for index in 0..5 {
            scene
                .directional_light(
                    DirectionalLight::default()
                        .with_color(Color::from_linear_rgb(index as f32 + 0.1, 0.2, 0.3))
                        .with_illuminance_lux(1_000.0 + index as f32)
                        .with_shadows(index == 1),
                )
                .transform(Transform::default())
                .add()
                .expect("directional light inserts");
            scene
                .point_light(
                    PointLight::default()
                        .with_color(Color::from_linear_rgb(0.1, index as f32 + 0.2, 0.3))
                        .with_intensity_candela(100.0 + index as f32)
                        .with_range(10.0 + index as f32),
                )
                .transform(Transform::at(Vec3::new(index as f32, 2.0, 3.0)))
                .add()
                .expect("point light inserts");
            scene
                .spot_light(
                    SpotLight::default()
                        .with_color(Color::from_linear_rgb(0.1, 0.2, index as f32 + 0.3))
                        .with_intensity_candela(200.0 + index as f32)
                        .with_range(20.0 + index as f32)
                        .with_inner_cone_angle(Angle::from_degrees(10.0))
                        .with_outer_cone_angle(Angle::from_degrees(25.0)),
                )
                .transform(Transform::at(Vec3::new(1.0, index as f32, 3.0)))
                .add()
                .expect("spot light inserts");
        }
        let environment = PreparedEnvironmentLighting::default();

        let uniform = collect_gpu_light_uniform(&scene, Vec3::ZERO, &environment);

        assert_eq!(uniform.light_counts, [4.0, 4.0, 4.0, 0.0]);
        assert_eq!(uniform.directional_light_color[1], [1.1, 0.2, 0.3, 0.0]);
        assert_eq!(uniform.directional_shadow_control[1][0], 1.0);
        assert_eq!(uniform.directional_light_color[3], [3.1, 0.2, 0.3, 0.0]);
        assert_eq!(
            uniform.directional_light_color[MAX_GPU_LIGHTS_PER_TYPE - 1],
            [3.1, 0.2, 0.3, 0.0],
            "the fifth directional light is intentionally capped, not encoded over slot 3"
        );
        assert_eq!(
            uniform.point_light_position_intensity[2][0..3],
            [2.0, 2.0, 3.0]
        );
        assert_eq!(uniform.point_light_color_range[2], [0.1, 2.2, 0.3, 12.0]);
        assert_eq!(
            uniform.spot_light_position_intensity[2][0..3],
            [1.0, 2.0, 3.0]
        );
        assert_eq!(uniform.spot_light_color_range[2], [0.1, 0.2, 2.3, 0.0]);
        assert!(uniform.spot_light_cone_range[2][0] > uniform.spot_light_cone_range[2][1]);
    }
}
