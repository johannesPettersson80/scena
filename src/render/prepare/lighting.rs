use super::pbr_contract::{
    PbrMaterial, Surface, directional_illuminance_lux, inverse_square_range_attenuation,
    punctual_intensity_candela, roughness_or_min, spot_cone_attenuation, surface_direct_light,
};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind};
use crate::render::physical_transmission::transmitted_share;
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
use lobes::VolumeLobes;
use ltc::ltc_area_light_specular_contribution;
use math::*;
pub(in crate::render) use tiled::{
    TiledLightAssignment, assignment_required, collect_tiled_light_assignment,
};

pub(in crate::render) const MAX_GPU_LIGHTS_PER_TYPE: usize = 16;
pub(in crate::render) const MAX_GPU_AREA_LIGHTS: usize = 4;

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
    pub(super) environment: PreparedEnvironmentLighting,
    pub(super) directional_shadow_factor: f32,
    pub(super) area_shadow_factor: f32,
    pub(super) ambient_visibility: f32,
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
        for (_node, _light_key, light, world) in scene.light_nodes() {
            match light {
                Light::Directional(light) => lights.directional.push(PreparedDirectionalLight {
                    color: light.color(),
                    direction: light_direction(world),
                    illuminance_lux: light.illuminance_lux(),
                    casts_shadows: light.casts_shadows(),
                }),
                Light::Point(light) => lights.point.push(PreparedPointLight {
                    color: light.color(),
                    position: subtract_vec3(world.translation(), origin_shift),
                    intensity_candela: light.intensity_candela(),
                    range: light.range(),
                }),
                Light::Spot(light) => lights.spot.push(PreparedSpotLight {
                    color: light.color(),
                    position: subtract_vec3(world.translation(), origin_shift),
                    direction: light_direction(world),
                    intensity_candela: light.intensity_candela(),
                    range: light.range(),
                    inner_cone_cos: light.inner_cone_angle().radians().cos(),
                    outer_cone_cos: light.outer_cone_angle().radians().cos(),
                }),
                Light::Area(light) => {
                    if let Some(prepared) = prepared_area_light(light, world, origin_shift) {
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

    pub(super) fn area_shadow_samples(
        &self,
        receiver_position: Vec3,
    ) -> impl Iterator<Item = (Vec3, f32)> + '_ {
        self.area.iter().flat_map(move |light| {
            let light = *light;
            area_light_sample_positions(light)
                .into_iter()
                .map(move |sample_position| {
                    let to_light = subtract_vec3(sample_position, receiver_position);
                    let incident_weight = light.luminous_flux_lumens
                        / AREA_LIGHT_SAMPLE_COUNT as f32
                        * inverse_square_range_attenuation(to_light, light.range);
                    (sample_position, incident_weight.max(0.0))
                })
        })
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

/// Whether [`material_color`] shades a material from the lights, the
/// environment and the view vector, rather than returning its base colour.
pub(super) fn material_is_lit(
    material: &MaterialDesc,
    lights: &PreparedLights,
    environment: &PreparedEnvironmentLighting,
) -> bool {
    matches!(material.kind(), MaterialKind::PbrMetallicRoughness)
        && (lights.has_direct_lights() || environment.is_active())
}

pub(super) fn material_color(
    material: &MaterialDesc,
    lights: &PreparedLights,
    input: &MaterialShadingInput,
) -> Color {
    let base = multiply_color(material.base_color(), input.base_color_texture);
    // Share of the light emitted beneath a clearcoat that leaves through it.
    let mut coat_attenuation = 1.0;
    let mut color = match material.kind() {
        MaterialKind::Unlit => base,
        MaterialKind::PbrMetallicRoughness => {
            let (surface, lobes) = material_surface(material, base, input);
            coat_attenuation = surface.clearcoat_attenuation;
            if material_is_lit(material, lights, &input.environment) {
                let direct = surface_punctual_lighting(&surface, &lobes, lights, input);
                let indirect = surface_environment_lighting(&surface, &lobes, input);
                let authored_occlusion = 1.0
                    + (input.occlusion_texture.clamp(0.0, 1.0) - 1.0)
                        * material.occlusion_strength().clamp(0.0, 1.0);
                let indirect_occlusion =
                    authored_occlusion * input.ambient_visibility.clamp(0.0, 1.0);
                Color::from_linear_rgba(
                    direct.x + indirect.x * indirect_occlusion,
                    direct.y + indirect.y * indirect_occlusion,
                    direct.z + indirect.z * indirect_occlusion,
                    base.a,
                )
            } else {
                // Without lights the albedo stands in for the diffuse lobe,
                // so a transmissive material shows only its opaque share.
                let opaque_share = 1.0
                    - transmitted_share(
                        material.transmission_factor(),
                        input.transmission_texture,
                        material.metallic_factor() * input.metallic_roughness_texture.0,
                    );
                Color::from_linear_rgba(
                    base.r * opaque_share,
                    base.g * opaque_share,
                    base.b * opaque_share,
                    base.a,
                )
            }
        }
        MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge => base,
    };
    let emissive = material.emissive();
    let emissive_strength = material.emissive_strength();
    color.r += emissive.r * input.emissive_texture.r * emissive_strength * coat_attenuation;
    color.g += emissive.g * input.emissive_texture.g * emissive_strength * coat_attenuation;
    color.b += emissive.b * input.emissive_texture.b * emissive_strength * coat_attenuation;
    match material.alpha_mode() {
        AlphaMode::Opaque => color.a = 1.0,
        AlphaMode::Blend => {}
        AlphaMode::Mask { .. } => {}
    }
    color
}

/// Builds the layered surface of one shaded vertex, together with the
/// dispersion lobe the surface model does not own.
fn material_surface(
    material: &MaterialDesc,
    base: Color,
    input: &MaterialShadingInput,
) -> (Surface, VolumeLobes) {
    let (normal, micro_roughness) =
        photographic_micro_surface(material, input.position, input.normal, input.tangent);
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
    let roughness = roughness_or_min(
        material.roughness_factor() * input.metallic_roughness_texture.1 + micro_roughness,
    );
    let pbr_material = PbrMaterial::new(base_rgb, metallic, roughness);
    // Light that a transmissive dielectric passes on is not scattered back as
    // diffuse, so the surface keeps only the opaque share of the base colour.
    // Without transmission the share is exactly one.
    let opaque_share = 1.0
        - transmitted_share(
            material.transmission_factor(),
            input.transmission_texture,
            metallic,
        );
    let opaque_material = PbrMaterial::new(base_rgb * opaque_share, metallic, roughness);
    let sheen_color_factor = material.sheen_color_factor();
    let sheen_color = Vec3::new(
        sheen_color_factor.r * input.sheen_color_texture.r,
        sheen_color_factor.g * input.sheen_color_texture.g,
        sheen_color_factor.b * input.sheen_color_texture.b,
    );
    let iridescence_thickness_mix = clamp_unit(input.iridescence_thickness_texture);
    let iridescence_thickness_nm = material.iridescence_thickness_minimum_nm()
        * (1.0 - iridescence_thickness_mix)
        + material.iridescence_thickness_maximum_nm() * iridescence_thickness_mix;
    // The index of refraction comes first: the iridescence layer is built from
    // the reflectance it sets.
    let surface = Surface::new(opaque_material, normal, view)
        .with_ior(material.ior())
        .with_anisotropy(
            material.anisotropy_strength_factor() * input.anisotropy_texture.z,
            (input.anisotropy_texture.x, input.anisotropy_texture.y),
            material.anisotropy_rotation_radians(),
            input.tangent,
            input.tangent_handedness,
        )
        .with_iridescence(
            material.iridescence_factor() * input.iridescence_texture,
            material.iridescence_ior(),
            iridescence_thickness_nm,
        )
        .with_sheen(
            sheen_color,
            material.sheen_roughness_factor() * input.sheen_roughness_texture,
        )
        .with_clearcoat(
            material.clearcoat_factor() * input.clearcoat_texture,
            material.clearcoat_roughness_factor() * input.clearcoat_roughness_texture,
            normalize_or(input.clearcoat_normal, normal),
        );
    let lobes = VolumeLobes {
        pbr_material,
        normal,
        view,
        opaque_material,
        dispersion_factor: material.dispersion_factor(),
        ior: material.ior(),
    };
    (surface, lobes)
}

/// Direct lighting from every analytic light in the scene.
fn surface_punctual_lighting(
    s: &Surface,
    lobes: &VolumeLobes,
    lights: &PreparedLights,
    input: &MaterialShadingInput,
) -> Vec3 {
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
        shaded = add_vec3(shaded, surface_direct_light(s, incoming, radiance, 1.0));
        shaded = add_vec3(shaded, lobes.contribution(incoming, radiance));
    }
    for light in &lights.point {
        let to_light = subtract_vec3(light.position, input.position);
        let incoming = normalize_or(to_light, Vec3::ZERO);
        let radiance = scale_color(
            light.color,
            punctual_intensity_candela(light.intensity_candela)
                * inverse_square_range_attenuation(to_light, light.range),
        );
        shaded = add_vec3(shaded, surface_direct_light(s, incoming, radiance, 1.0));
        shaded = add_vec3(shaded, lobes.contribution(incoming, radiance));
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
        shaded = add_vec3(shaded, surface_direct_light(s, incoming, radiance, 1.0));
        shaded = add_vec3(shaded, lobes.contribution(incoming, radiance));
    }
    for light in &lights.area {
        shaded = surface_area_light(shaded, *light, s, lobes, input);
    }
    shaded
}

/// Adds one area light to `shaded`: analytic LTC specular plus point-sampled
/// diffuse and coating lobes. The point samples skip the base specular lobe,
/// which the LTC integral already covers.
///
/// The running total is threaded through rather than summed per light, so
/// every term joins it in the order the approved CPU references were rendered
/// with.
fn surface_area_light(
    mut shaded: Vec3,
    light: PreparedAreaLight,
    s: &Surface,
    lobes: &VolumeLobes,
    input: &MaterialShadingInput,
) -> Vec3 {
    let intensity_per_sample =
        light.luminous_flux_lumens / (4.0 * std::f32::consts::PI) / AREA_LIGHT_SAMPLE_COUNT as f32;
    let shadow_factor = input.area_shadow_factor.clamp(0.0, 1.0);
    let specular = ltc_area_light_specular_contribution(
        light,
        input.position,
        s.normal,
        s.view,
        lobes.opaque_material,
        shadow_factor,
    );
    // The coat sits above the LTC lobe too. Without a coat the factor is
    // exactly 1, which leaves every bit of the lobe as it was.
    shaded = add_vec3(
        shaded,
        Vec3::new(
            specular.x * s.clearcoat_attenuation,
            specular.y * s.clearcoat_attenuation,
            specular.z * s.clearcoat_attenuation,
        ),
    );
    for sample_position in area_light_sample_positions(light) {
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
        shaded = add_vec3(shaded, surface_direct_light(s, incoming, radiance, 0.0));
        shaded = add_vec3(shaded, lobes.contribution(incoming, radiance));
    }
    shaded
}

/// Image-based lighting of the surface, plus the environment response of the
/// lobes the surface model does not own.
fn surface_environment_lighting(
    s: &Surface,
    lobes: &VolumeLobes,
    input: &MaterialShadingInput,
) -> Vec3 {
    let mut indirect = input.environment.surface_environment_light(s);
    if input.environment.is_active() {
        let environment_specular = input.environment.gpu_specular_intensity();
        let environment_radiance = Vec3::new(
            environment_specular[0] * environment_specular[3],
            environment_specular[1] * environment_specular[3],
            environment_specular[2] * environment_specular[3],
        );
        indirect = add_vec3(indirect, lobes.contribution(s.normal, environment_radiance));
    }
    indirect
}

fn photographic_micro_surface(
    material: &MaterialDesc,
    _position: Vec3,
    authored_normal: Vec3,
    _authored_tangent: Vec3,
) -> (Vec3, f32) {
    let normal = normalize_or(authored_normal, Vec3::new(0.0, 0.0, 1.0));
    let Some(surface) = material.photographic_micro_surface() else {
        return (normal, 0.0);
    };
    // The generated detail scale is below a product-still pixel footprint for
    // the normal camera range. Resolving it as two world-space sine waves
    // creates coherent contour bands and aliases under perspective. Preserve
    // the old mean roughness contribution while folding the unresolved normal
    // variance into the microfacet distribution.
    (normal, surface.strength() * 0.175)
}

#[cfg(test)]
mod exact_placement_tests;
#[cfg(test)]
mod tests;
