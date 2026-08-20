use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError};
use crate::{AreaLight, AreaLightShape, AssetFetcher, Color, MaterialHandle, Transform, Vec3};

pub const PHOTOGRAPHIC_LIGHTING_REPORT_SCHEMA_V1: &str = "scena.photographic_lighting_report.v1";
pub(super) const GENERATED_LIGHT_TAG: &str = "scena_generated_photographic_light";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicLightingReportV1 {
    pub schema: String,
    pub source: String,
    pub subject: u64,
    pub subject_extent_m: Vec3,
    pub geometry: PhotographicGeometryProfileV1,
    pub material: PhotographicMaterialProfileV1,
    pub environment: PhotographicEnvironmentProfileV1,
    pub white_balance: PhotographicWhiteBalanceV1,
    pub adjustment: PhotographicLightingAdjustmentV1,
    pub lights: Vec<PhotographicLightV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhotographicLightingAdjustmentV1 {
    pub key_scale: f32,
    pub fill_scale: f32,
    pub rim_scale: f32,
    pub overhead_scale: f32,
    pub environment_intensity_scale: f32,
    pub environment_rotation_offset_degrees: f32,
}

impl Default for PhotographicLightingAdjustmentV1 {
    fn default() -> Self {
        Self {
            key_scale: 1.0,
            fill_scale: 1.0,
            rim_scale: 1.0,
            overhead_scale: 1.0,
            environment_intensity_scale: 1.0,
            environment_rotation_offset_degrees: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhotographicGeometryProfileV1 {
    pub vertex_normal_samples: usize,
    pub normal_axis_weights: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicEnvironmentProfileV1 {
    pub present: bool,
    pub synthesized: bool,
    pub name: Option<String>,
    pub equirectangular_hdr: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_dimensions: Option<[u32; 2]>,
    pub preview_luminance: Option<f32>,
    pub cubemap_resolution: Option<u32>,
    pub intensity: f32,
    pub rotation_y_degrees: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhotographicMaterialProfileV1 {
    pub material_count: usize,
    pub mean_base_luminance: f32,
    pub dark_fraction: f32,
    pub reflective_fraction: f32,
    pub transmission_fraction: f32,
    pub mean_roughness: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicLightV1 {
    pub role: String,
    pub node: u64,
    pub position: Vec3,
    pub target: Vec3,
    pub emitter_width_m: f32,
    pub emitter_height_m: f32,
    pub luminous_flux_lumens: f32,
    pub color_temperature_kelvin: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhotographicWhiteBalanceV1 {
    pub illuminant_kelvin: f32,
    pub tint: f32,
    pub linear_multipliers: [f32; 3],
}

struct SolvedLight {
    role: &'static str,
    position: Vec3,
    target: Vec3,
    width: f32,
    height: f32,
    flux: f32,
    kelvin: f32,
}

#[derive(Debug, Clone, Copy)]
struct LightingViewBasis {
    right: Vec3,
    up: Vec3,
    front: Vec3,
}

impl LightingViewBasis {
    fn from_camera(subject: Vec3, camera: Vec3) -> Self {
        let front = (camera - subject).normalize_or_zero();
        let front = if front.length_squared() > 1.0e-8 {
            front
        } else {
            Vec3::Z
        };
        let right = Vec3::Y.cross(front).normalize_or_zero();
        let right = if right.length_squared() > 1.0e-8 {
            right
        } else {
            Vec3::X
        };
        let up = front.cross(right).normalize_or_zero();
        Self { right, up, front }
    }

    fn offset(self, x: f32, y: f32, z: f32) -> Vec3 {
        self.right * x + self.up * y + self.front * z
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_photographic_lighting(
        &mut self,
        subject: u64,
    ) -> Result<PhotographicLightingReportV1, SceneHostError> {
        self.apply_photographic_lighting_adjusted(
            subject,
            PhotographicLightingAdjustmentV1::default(),
        )
    }

    pub fn apply_photographic_lighting_adjusted(
        &mut self,
        subject: u64,
        adjustment: PhotographicLightingAdjustmentV1,
    ) -> Result<PhotographicLightingReportV1, SceneHostError> {
        self.apply_photographic_lighting_adjusted_for_quality(subject, adjustment, false)
    }

    /// Applies the final-still lighting contract using the full bundled HDRI.
    pub fn apply_final_photographic_lighting(
        &mut self,
        subject: u64,
    ) -> Result<PhotographicLightingReportV1, SceneHostError> {
        self.apply_final_photographic_lighting_adjusted(
            subject,
            PhotographicLightingAdjustmentV1::default(),
        )
    }

    /// Applies adjusted final-still lighting using the full bundled HDRI.
    pub fn apply_final_photographic_lighting_adjusted(
        &mut self,
        subject: u64,
        adjustment: PhotographicLightingAdjustmentV1,
    ) -> Result<PhotographicLightingReportV1, SceneHostError> {
        self.apply_photographic_lighting_adjusted_for_quality(subject, adjustment, true)
    }

    fn apply_photographic_lighting_adjusted_for_quality(
        &mut self,
        subject: u64,
        adjustment: PhotographicLightingAdjustmentV1,
        final_quality: bool,
    ) -> Result<PhotographicLightingReportV1, SceneHostError> {
        let subject_node = self.resolve_node(subject)?;
        let bounds = self
            .scene
            .node_world_bounds(subject_node, &self.assets)?
            .ok_or(crate::LookupError::ImportHasNoBounds)?;
        // Record what the solver installs, so a later pass can tell a derived
        // environment from one the caller authored regardless of ordering.
        let active_environment = self.renderer.environment();
        let manages_active_environment =
            active_environment.is_none() || active_environment == self.generated_environment;
        let synthesized_environment = if manages_active_environment {
            // Derive a real captured environment rather than the preview
            // fixture, whose own source declares `not HDR input and not IBL
            // proof`. Reflective materials need structure to reflect; six
            // constant cube faces give them none.
            let active_meets_quality = active_environment
                .and_then(|handle| self.assets.environment(handle))
                .is_some_and(|environment| {
                    !final_quality
                        || (environment
                            .source_dimensions()
                            .is_some_and(|(width, height)| width >= 1024 && height >= 512)
                            && environment.cubemap_resolution() >= 512)
                });
            if !active_meets_quality {
                let derived = if final_quality {
                    self.assets.bundled_final_studio_environment()?
                } else {
                    match self.assets.bundled_studio_environment() {
                        Ok(handle) => handle,
                        Err(_) => {
                            // Not a silent fallback: `active_environment_profile`
                            // reports the fixture that actually landed.
                            self.assets.default_environment()
                        }
                    }
                };
                self.renderer.set_environment(derived);
                self.generated_environment = Some(derived);
            } else {
                self.generated_environment = active_environment;
            };
            true
        } else {
            false
        };
        let geometry = subject_geometry_profile(self, subject_node)?;
        let material = subject_material_profile(self, subject_node)?;
        let mut environment = active_environment_profile(self, synthesized_environment);
        if final_quality {
            validate_final_environment(&environment)?;
        }
        let extent = bounds.half_extent() * 2.0;
        let center = bounds.center();
        let radius = bounds.bounding_sphere_radius().max(0.05);
        let camera_position = self
            .scene
            .camera_node(self.active_camera)
            .and_then(|node| self.scene.world_transform(node))
            .map(|transform| transform.translation)
            .unwrap_or(center + Vec3::Z * radius * 4.0);
        let view = LightingViewBasis::from_camera(center, camera_position);
        if environment.synthesized {
            let base_intensity = (0.86 + material.dark_fraction * 0.22
                - material.reflective_fraction * 0.08)
                .clamp(0.72, 1.18);
            let camera_azimuth = view.front.x.atan2(view.front.z).to_degrees();
            let base_rotation =
                (camera_azimuth + 18.0 + material.reflective_fraction * 54.0).rem_euclid(360.0);
            environment.intensity =
                (base_intensity * adjustment.environment_intensity_scale).clamp(0.0, 16.0);
            environment.rotation_y_degrees =
                (base_rotation + adjustment.environment_rotation_offset_degrees).rem_euclid(360.0);
        } else {
            environment.intensity =
                (environment.intensity * adjustment.environment_intensity_scale).clamp(0.0, 16.0);
            environment.rotation_y_degrees = (environment.rotation_y_degrees
                + adjustment.environment_rotation_offset_degrees)
                .rem_euclid(360.0);
        }
        self.renderer
            .set_environment_intensity(environment.intensity);
        self.renderer
            .set_environment_rotation_y_degrees(environment.rotation_y_degrees);
        let mut solved = solve_lights(
            center,
            extent,
            radius,
            geometry,
            material,
            &environment,
            self.renderer.background_color(),
            view,
        );
        for light in &mut solved {
            let scale = match light.role {
                "key" => adjustment.key_scale,
                "fill" => adjustment.fill_scale,
                "rim" => adjustment.rim_scale,
                "overhead" => adjustment.overhead_scale,
                _ => 1.0,
            };
            light.flux *= scale.clamp(0.0, 4.0);
        }
        let illuminant_kelvin = estimated_illuminant_kelvin(&solved);
        let environment_irradiance_rgb = self
            .renderer
            .environment()
            .and_then(|handle| self.assets.environment(handle))
            .and_then(|environment| environment.preview_irradiance_rgb());
        let white_balance = automatic_white_balance(
            &solved,
            environment_irradiance_rgb,
            environment.intensity,
            illuminant_kelvin,
        );
        self.renderer.set_white_balance(white_balance);
        let white_balance = PhotographicWhiteBalanceV1 {
            illuminant_kelvin,
            tint: white_balance.tint(),
            linear_multipliers: white_balance.linear_multipliers(),
        };
        let mut lights = Vec::with_capacity(solved.len());
        for light in solved {
            let area = AreaLight::default()
                .with_color(Color::from_kelvin(light.kelvin))
                .with_luminous_flux_lumens(light.flux)
                .with_range(radius * 8.0)
                .with_shape(AreaLightShape::rect(light.width, light.height));
            let transform = Transform::at(light.position).looking_at(light.target, Vec3::Y);
            let node = self.scene.area_light(area).transform(transform).add()?;
            self.scene.add_tag(node, GENERATED_LIGHT_TAG)?;
            let node = self.register_node(node);
            lights.push(PhotographicLightV1 {
                role: light.role.to_owned(),
                node,
                position: light.position,
                target: light.target,
                emitter_width_m: light.width,
                emitter_height_m: light.height,
                luminous_flux_lumens: light.flux,
                color_temperature_kelvin: light.kelvin,
            });
        }
        Ok(PhotographicLightingReportV1 {
            schema: PHOTOGRAPHIC_LIGHTING_REPORT_SCHEMA_V1.to_owned(),
            source: "subject_lighting_solver".to_owned(),
            subject,
            subject_extent_m: extent,
            geometry,
            material,
            environment,
            white_balance,
            adjustment,
            lights,
        })
    }
}

fn validate_final_environment(
    environment: &PhotographicEnvironmentProfileV1,
) -> Result<(), SceneHostError> {
    if !environment.present
        || !environment.equirectangular_hdr
        || environment
            .source_dimensions
            .is_none_or(|[width, height]| width < 1024 || height < 512)
        || environment.cubemap_resolution.is_none_or(|size| size < 512)
    {
        return Err(SceneHostError::new(
            super::SceneHostErrorCode::InvalidInput,
            "final photographic lighting requires an equirectangular HDR source of at least 1024x512 and a cubemap resolution of at least 512",
        ));
    }
    Ok(())
}

fn estimated_illuminant_kelvin(lights: &[SolvedLight]) -> f32 {
    let (weighted_sum, weight_sum) = lights.iter().fold((0.0_f32, 0.0_f32), |sum, light| {
        let weight = light.flux.max(0.0);
        (sum.0 + light.kelvin * weight, sum.1 + weight)
    });
    if weight_sum > 1.0e-6 {
        (weighted_sum / weight_sum).clamp(1_000.0, 20_000.0)
    } else {
        6_500.0
    }
}

fn automatic_white_balance(
    lights: &[SolvedLight],
    environment_irradiance_rgb: Option<[f32; 3]>,
    environment_intensity: f32,
    illuminant_kelvin: f32,
) -> crate::WhiteBalance {
    let (light_rgb_sum, light_flux_sum) =
        lights
            .iter()
            .fold(([0.0_f32; 3], 0.0_f32), |(mut rgb, flux_sum), light| {
                let flux = light.flux.max(0.0);
                let color = Color::from_kelvin(light.kelvin);
                rgb[0] += color.r * flux;
                rgb[1] += color.g * flux;
                rgb[2] += color.b * flux;
                (rgb, flux_sum + flux)
            });
    let light_chroma = (light_flux_sum > 1.0e-6)
        .then(|| normalize_illuminant_rgb(light_rgb_sum.map(|channel| channel / light_flux_sum)));
    let environment_chroma = environment_irradiance_rgb
        .filter(|rgb| {
            rgb.iter()
                .all(|channel| channel.is_finite() && *channel >= 0.0)
        })
        .map(normalize_illuminant_rgb);
    let environment_weight = if environment_chroma.is_some() {
        environment_intensity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let light_weight = if light_chroma.is_some() { 1.0 } else { 0.0 };
    let illuminant_rgb = match (light_chroma, environment_chroma) {
        (Some(light), Some(environment)) => {
            let weight_sum = light_weight + environment_weight;
            if weight_sum > 1.0e-6 {
                std::array::from_fn(|channel| {
                    (light[channel] * light_weight + environment[channel] * environment_weight)
                        / weight_sum
                })
            } else {
                [1.0; 3]
            }
        }
        (Some(light), None) => light,
        (None, Some(environment)) => environment,
        (None, None) => [1.0; 3],
    };
    crate::WhiteBalance::from_linear_illuminant_rgb(illuminant_kelvin, 0.0, illuminant_rgb)
}

fn normalize_illuminant_rgb(rgb: [f32; 3]) -> [f32; 3] {
    let luminance = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
    if luminance.is_finite() && luminance > 1.0e-6 {
        rgb.map(|channel| channel / luminance)
    } else {
        [1.0; 3]
    }
}

fn active_environment_profile<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    synthesized: bool,
) -> PhotographicEnvironmentProfileV1 {
    let Some(handle) = host.renderer.environment() else {
        return PhotographicEnvironmentProfileV1 {
            present: false,
            synthesized,
            name: None,
            equirectangular_hdr: false,
            source_dimensions: None,
            preview_luminance: None,
            cubemap_resolution: None,
            intensity: host.renderer.environment_intensity(),
            rotation_y_degrees: host.renderer.environment_rotation_y_degrees(),
        };
    };
    let Some(environment) = host.assets.environment(handle) else {
        return PhotographicEnvironmentProfileV1 {
            present: true,
            synthesized,
            name: None,
            equirectangular_hdr: false,
            source_dimensions: None,
            preview_luminance: None,
            cubemap_resolution: None,
            intensity: host.renderer.environment_intensity(),
            rotation_y_degrees: host.renderer.environment_rotation_y_degrees(),
        };
    };
    let preview_luminance = environment
        .preview_irradiance_rgb()
        .map(|rgb| 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]);
    PhotographicEnvironmentProfileV1 {
        present: true,
        synthesized,
        name: Some(environment.name().to_owned()),
        equirectangular_hdr: environment.is_equirectangular_hdr(),
        source_dimensions: environment
            .source_dimensions()
            .map(|(width, height)| [width, height]),
        preview_luminance,
        cubemap_resolution: Some(environment.cubemap_resolution()),
        intensity: host.renderer.environment_intensity(),
        rotation_y_degrees: host.renderer.environment_rotation_y_degrees(),
    }
}

fn subject_geometry_profile<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    subject: crate::NodeKey,
) -> Result<PhotographicGeometryProfileV1, SceneHostError> {
    let subtree = host.scene.subtree_nodes(subject)?;
    let inspection = host.scene.inspect_with_assets(&host.assets);
    let mut axis_sum = Vec3::ZERO;
    let mut vertex_normal_samples = 0usize;
    for draw in inspection.draw_list() {
        if !subtree.contains(&draw.node()) {
            continue;
        }
        let Some(geometry) = host.assets.geometry(draw.geometry()) else {
            continue;
        };
        let rotation = draw.world_transform().rotation;
        for vertex in geometry.vertices() {
            let normal = (rotation * vertex.normal).normalize_or_zero();
            if !normal.is_finite() || normal.length_squared() <= 1.0e-8 {
                continue;
            }
            axis_sum += normal.abs();
            vertex_normal_samples += 1;
        }
    }
    let total = axis_sum.element_sum();
    let weights = if total > 1.0e-8 {
        axis_sum / total
    } else {
        Vec3::splat(1.0 / 3.0)
    };
    Ok(PhotographicGeometryProfileV1 {
        vertex_normal_samples,
        normal_axis_weights: [weights.x, weights.y, weights.z],
    })
}

fn subject_material_profile<F: AssetFetcher>(
    host: &SceneHostCore<F>,
    subject: crate::NodeKey,
) -> Result<PhotographicMaterialProfileV1, SceneHostError> {
    let subtree = host.scene.subtree_nodes(subject)?;
    let inspection = host.scene.inspect_with_assets(&host.assets);
    let mut materials = Vec::<(MaterialHandle, f32)>::new();
    for draw in inspection.draw_list() {
        if !subtree.contains(&draw.node()) {
            continue;
        }
        let area = host
            .assets
            .geometry(draw.geometry())
            .map_or(0.0, |geometry| {
                geometry_surface_area(&geometry, draw.world_transform())
            });
        if let Some((_, accumulated_area)) = materials
            .iter_mut()
            .find(|(material, _)| *material == draw.material())
        {
            *accumulated_area += area;
        } else {
            materials.push((draw.material(), area));
        }
    }
    if materials.is_empty() {
        return Ok(PhotographicMaterialProfileV1 {
            material_count: 0,
            mean_base_luminance: 0.5,
            dark_fraction: 0.0,
            reflective_fraction: 0.0,
            transmission_fraction: 0.0,
            mean_roughness: 1.0,
        });
    }

    let mut dark = 0.0;
    let mut base_luminance = 0.0;
    let mut reflective = 0.0;
    let mut transmission = 0.0;
    let mut roughness = 0.0;
    let measured_area = materials.iter().map(|(_, area)| area).sum::<f32>();
    let use_area = measured_area.is_finite() && measured_area > 1.0e-8;
    let mut weight_sum = 0.0;
    for (material, area) in &materials {
        let Some(desc) = host.assets.material(*material) else {
            continue;
        };
        let weight = if use_area { *area } else { 1.0 };
        let color = desc.base_color();
        let luminance = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
        let material_dark = ((0.22 - luminance) / 0.22).clamp(0.0, 1.0);
        let material_transmission = desc.transmission_factor();
        let material_reflective = desc
            .metallic_factor()
            .max((1.0 - desc.roughness_factor()) * 0.65)
            .max(material_transmission);
        dark += material_dark * weight;
        base_luminance += luminance * weight;
        reflective += material_reflective * weight;
        transmission += material_transmission * weight;
        roughness += desc.roughness_factor() * weight;
        weight_sum += weight;
    }
    let count = weight_sum.max(1.0e-8);
    Ok(PhotographicMaterialProfileV1 {
        material_count: materials.len(),
        mean_base_luminance: (base_luminance / count).clamp(0.0, 1.0),
        dark_fraction: (dark / count).clamp(0.0, 1.0),
        reflective_fraction: (reflective / count).clamp(0.0, 1.0),
        transmission_fraction: (transmission / count).clamp(0.0, 1.0),
        mean_roughness: (roughness / count).clamp(0.0, 1.0),
    })
}

fn geometry_surface_area(geometry: &crate::GeometryDesc, transform: Transform) -> f32 {
    if geometry.topology() != crate::GeometryTopology::Triangles {
        return 0.0;
    }
    geometry
        .indices()
        .chunks_exact(3)
        .filter_map(|triangle| {
            let a = geometry.vertices().get(triangle[0] as usize)?.position;
            let b = geometry.vertices().get(triangle[1] as usize)?.position;
            let c = geometry.vertices().get(triangle[2] as usize)?.position;
            let transform_point = |point: Vec3| {
                transform.translation
                    + transform.rotation
                        * Vec3::new(
                            point.x * transform.scale.x,
                            point.y * transform.scale.y,
                            point.z * transform.scale.z,
                        )
            };
            let area = (transform_point(b) - transform_point(a))
                .cross(transform_point(c) - transform_point(a))
                .length()
                * 0.5;
            area.is_finite().then_some(area)
        })
        .sum()
}

#[allow(clippy::too_many_arguments)]
fn solve_lights(
    center: Vec3,
    extent: Vec3,
    radius: f32,
    geometry: PhotographicGeometryProfileV1,
    material: PhotographicMaterialProfileV1,
    environment: &PhotographicEnvironmentProfileV1,
    background: Color,
    view: LightingViewBasis,
) -> [SolvedLight; 4] {
    let area_scale = radius * radius;
    // These dimensions describe real product-photo softboxes rather than
    // compact emitters. Keep luminous exitance close to the former smaller rig
    // by scaling flux with the larger panel area.
    let key_flux = 480.0 * area_scale * (1.0 + material.dark_fraction * 0.22);
    let dark_coverage_response = material.dark_fraction.sqrt();
    let upward_weight = geometry.normal_axis_weights[1];
    let lateral_span = extent.x.max(extent.z).max(radius * 0.5);
    let vertical_span = extent.y.max(radius * 0.5);
    let key_elevation = 1.25 + upward_weight * 0.75;
    let fill_ratio = (0.26 + material.dark_fraction * 0.40 + dark_coverage_response * 0.45
        - material.reflective_fraction * 0.06)
        * if environment.present { 0.88 } else { 1.0 };
    let fill_ratio = fill_ratio.clamp(0.22, 0.68);
    let background_luminance =
        0.2126 * background.r + 0.7152 * background.g + 0.0722 * background.b;
    let separation = (material.mean_base_luminance - background_luminance).abs();
    let rim_need = ((0.22 - separation) / 0.22)
        .clamp(0.0, 1.0)
        .max(material.transmission_fraction)
        .max(material.dark_fraction * 0.42)
        .max(dark_coverage_response * 0.32);
    let rim_ratio =
        ((0.16 + material.reflective_fraction * 0.48 + material.transmission_fraction * 0.12)
            .clamp(0.16, 0.72))
            * rim_need;
    let overhead_ratio =
        (0.08 + upward_weight * 0.22 + material.dark_fraction * 0.08).clamp(0.08, 0.36);
    [
        SolvedLight {
            role: "key",
            position: center + view.offset(radius * 1.8, radius * key_elevation, radius * 1.45),
            target: center,
            width: lateral_span * (1.55 + upward_weight * 0.35),
            height: vertical_span * (1.35 + (1.0 - upward_weight) * 0.40),
            flux: key_flux,
            kelvin: 5_500.0,
        },
        SolvedLight {
            role: "fill",
            position: center + view.offset(-radius * 1.55, radius * 0.65, radius * 1.25),
            target: center,
            width: lateral_span * 1.8,
            height: vertical_span * 1.6,
            flux: key_flux * fill_ratio,
            kelvin: 6_100.0 - material.dark_fraction * 350.0,
        },
        SolvedLight {
            role: "rim",
            position: center + view.offset(-radius * 0.35, radius * 1.35, -radius * 1.8),
            target: center,
            width: lateral_span * 0.85,
            height: vertical_span * 2.0,
            flux: key_flux * rim_ratio,
            kelvin: 4_600.0 + material.reflective_fraction * 450.0,
        },
        SolvedLight {
            role: "overhead",
            position: center + view.offset(0.0, radius * 2.35, radius * 0.15),
            target: center,
            width: lateral_span * 1.7,
            height: lateral_span * 1.25,
            flux: key_flux * overhead_ratio,
            kelvin: 5_800.0 - material.dark_fraction * 250.0,
        },
    ]
}
