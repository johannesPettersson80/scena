use crate::{Color, Vec3};

use super::{
    PhotographicEnvironmentProfileV1, PhotographicGeometryProfileV1, PhotographicMaterialProfileV1,
};

pub(super) struct SolvedLight {
    pub(super) role: &'static str,
    pub(super) position: Vec3,
    pub(super) target: Vec3,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) flux: f32,
    pub(super) kelvin: f32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LightingViewBasis {
    right: Vec3,
    up: Vec3,
    pub(super) front: Vec3,
}

impl LightingViewBasis {
    pub(super) fn from_camera(subject: Vec3, camera: Vec3) -> Self {
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

pub(super) fn estimated_illuminant_kelvin(lights: &[SolvedLight]) -> f32 {
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

pub(super) fn automatic_white_balance(
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
    let environment_weight = environment_chroma
        .is_some()
        .then_some(environment_intensity.clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let light_weight = f32::from(light_chroma.is_some());
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

#[allow(clippy::too_many_arguments)]
pub(super) fn solve_lights(
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
    let key_flux = 480.0 * area_scale * (1.0 + material.dark_fraction * 0.22);
    let dark_coverage_response = material.dark_fraction.sqrt();
    let upward_weight = geometry.normal_axis_weights[1];
    let lateral_span = extent.x.max(extent.z).max(radius * 0.5);
    let vertical_span = extent.y.max(radius * 0.5);
    let key_elevation = 1.25 + upward_weight * 0.75;
    let fill_ratio = ((0.26 + material.dark_fraction * 0.40 + dark_coverage_response * 0.45
        - material.reflective_fraction * 0.06)
        * if environment.present { 0.88 } else { 1.0 })
    .clamp(0.22, 0.68);
    let background_luminance =
        0.2126 * background.r + 0.7152 * background.g + 0.0722 * background.b;
    let separation = (material.mean_base_luminance - background_luminance).abs();
    let rim_need = ((0.22 - separation) / 0.22)
        .clamp(0.0, 1.0)
        .max(material.transmission_fraction)
        .max(material.dark_fraction * 0.42)
        .max(dark_coverage_response * 0.32);
    let rim_ratio =
        (0.16 + material.reflective_fraction * 0.48 + material.transmission_fraction * 0.12)
            .clamp(0.16, 0.72)
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
