use super::pbr_contract::{
    PbrMaterial, directional_illuminance_lux, inverse_square_range_attenuation,
    punctual_intensity_candela, punctual_light_contribution, roughness_or_min,
    spot_cone_attenuation,
};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::scene::{Light, Scene, Vec3};

use super::environment::PreparedEnvironmentLighting;

mod area;
mod counts;
mod gpu_uniform;
mod lobes;
mod ltc;
mod math;
mod tiled;
use area::{
    AREA_LIGHT_SAMPLE_COUNT, PreparedAreaLight, area_light_sample_positions, prepared_area_light,
};
pub(in crate::render) use gpu_uniform::PreparedGpuLightUniform;
use lobes::LayeredMaterialLobes;
use ltc::ltc_area_light_specular_contribution;
use math::*;
pub(in crate::render) use tiled::{
    TiledLightAssignment, assignment_required, collect_tiled_light_assignment,
};

pub(in crate::render) const MAX_GPU_LIGHTS_PER_TYPE: usize = 16;
pub(in crate::render) const MAX_GPU_AREA_LIGHTS: usize = 2;

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
    pub(super) area_shadow_factor: f32,
}

#[derive(Default)]
pub(super) struct PreparedLights {
    directional: Vec<PreparedDirectionalLight>,
    point: Vec<PreparedPointLight>,
    spot: Vec<PreparedSpotLight>,
    area: Vec<PreparedAreaLight>,
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
                Light::Area(light) => {
                    if let Some(prepared) = prepared_area_light(light, transform, origin_shift) {
                        lights.area.push(prepared);
                    }
                }
            }
        }
        lights
    }

    fn has_direct_lights(&self) -> bool {
        !self.directional.is_empty()
            || !self.point.is_empty()
            || !self.spot.is_empty()
            || !self.area.is_empty()
    }

    pub(super) fn has_area_lights(&self) -> bool {
        !self.area.is_empty()
    }

    pub(super) fn area_shadow_sample_positions(&self) -> impl Iterator<Item = Vec3> + '_ {
        self.area
            .iter()
            .flat_map(|light| area_light_sample_positions(*light))
    }

    pub(super) fn primary_shadow_ray_direction(&self) -> Option<Vec3> {
        self.directional
            .iter()
            .find(|light| light.casts_shadows)
            .map(|light| negate_vec3(light.direction))
    }

    pub(super) fn shadow_state_signature(&self) -> u64 {
        let mut signature = 0xcbf2_9ce4_8422_2325_u64;
        for light in &self.directional {
            signature = hash_shadow_u32(signature, u32::from(light.casts_shadows));
            signature = hash_shadow_vec3(signature, light.direction);
        }
        for sample in self.area_shadow_sample_positions() {
            signature = hash_shadow_vec3(signature, sample);
        }
        signature
    }
}

fn hash_shadow_vec3(signature: u64, value: Vec3) -> u64 {
    let signature = hash_shadow_u32(signature, value.x.to_bits());
    let signature = hash_shadow_u32(signature, value.y.to_bits());
    hash_shadow_u32(signature, value.z.to_bits())
}

fn hash_shadow_u32(mut signature: u64, value: u32) -> u64 {
    for byte in value.to_le_bytes() {
        signature ^= u64::from(byte);
        signature = signature.wrapping_mul(0x0000_0100_0000_01b3);
    }
    signature
}

pub(in crate::render) fn collect_gpu_light_uniform(
    scene: &Scene,
    origin_shift: Vec3,
    environment: &PreparedEnvironmentLighting,
    tiled_assignment_active: bool,
) -> PreparedGpuLightUniform {
    PreparedLights::from_scene(scene, origin_shift)
        .gpu_uniform(environment.clone(), tiled_assignment_active)
}

pub(in crate::render) fn collect_gpu_tiled_light_assignment(
    scene: &Scene,
    origin_shift: Vec3,
    target: crate::render::RasterTarget,
    camera: Option<&crate::render::camera::CameraProjection>,
) -> Result<TiledLightAssignment, crate::diagnostics::PrepareError> {
    let lights = PreparedLights::from_scene(scene, origin_shift);
    collect_tiled_light_assignment(&lights, target, camera)
}

pub(in crate::render) fn gpu_tiled_light_assignment_required(scene: &Scene) -> bool {
    let lights = PreparedLights::from_scene(scene, Vec3::ZERO);
    assignment_required(&lights)
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
    for light in &lights.area {
        let intensity_per_sample = light.luminous_flux_lumens
            / (4.0 * std::f32::consts::PI)
            / AREA_LIGHT_SAMPLE_COUNT as f32;
        let shadow_factor = input.area_shadow_factor.clamp(0.0, 1.0);
        shaded = add_vec3(
            shaded,
            ltc_area_light_specular_contribution(
                *light,
                input.position,
                normal,
                view,
                pbr_material,
                shadow_factor,
            ),
        );
        for sample_position in area_light_sample_positions(*light) {
            let to_light = subtract_vec3(sample_position, input.position);
            let incoming = normalize_or(to_light, Vec3::ZERO);
            let radiance = scale_color(
                light.color,
                punctual_intensity_candela(intensity_per_sample)
                    * inverse_square_range_attenuation(to_light, light.range),
            );
            let radiance = Vec3::new(
                radiance.x * shadow_factor,
                radiance.y * shadow_factor,
                radiance.z * shadow_factor,
            );
            shaded = add_vec3(
                shaded,
                punctual_light_contribution(pbr_material, normal, view, incoming, radiance),
            );
            shaded = add_vec3(shaded, layered_lobes.contribution(incoming, radiance));
        }
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
mod tests;
