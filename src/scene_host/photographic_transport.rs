use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError};
use crate::material::{AlphaMode, Color, MaterialDesc};
use crate::{AssetFetcher, Camera, CaptureRgba8, Hit, HitTarget, Light, NodeKey, Transform, Vec3};

mod exposure;
mod trace;

use exposure::{display_rgba8, match_final_subject_exposure, raster_background_linear};
use trace::{Guide, Ray};

pub const PHOTOGRAPHIC_TRANSPORT_REPORT_SCHEMA_V1: &str = "scena.photographic_transport_report.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotographicTransportQuality {
    Preview,
    Final,
    Ultra,
}

impl PhotographicTransportQuality {
    const fn samples(self) -> u32 {
        match self {
            Self::Preview => 2,
            Self::Final => 8,
            Self::Ultra => 24,
        }
    }

    const fn bounces(self) -> u32 {
        match self {
            Self::Preview => 2,
            Self::Final => 4,
            Self::Ultra => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicTransportReportV1 {
    pub schema: String,
    pub path: String,
    pub samples_per_pixel: u32,
    pub maximum_bounces: u32,
    pub primary_rays: u64,
    pub secondary_rays: u64,
    pub shadow_rays: u64,
    pub intersections: u64,
    pub mesh_acceleration_structures: usize,
    pub instance_count: usize,
    pub light_kinds: Vec<String>,
    pub emissive_geometry_lights: usize,
    pub multiple_importance_sampling: bool,
    pub edge_aware_denoising: bool,
    pub firefly_suppression: String,
    pub raster_preview_preserved: bool,
    pub final_exposure_scale: f32,
    pub exposure_target_luminance_srgb8: f32,
    pub exposure_measured_luminance_srgb8: f32,
    pub exposure_sample_count: usize,
}

struct CameraRaySample {
    viewport: [u32; 2],
    pixel: [u32; 2],
    jitter: [f32; 2],
    depth_of_field: Option<crate::DepthOfFieldConfig>,
    sample: u32,
}

fn camera_ray(
    scene: &crate::Scene,
    camera_key: crate::CameraKey,
    sample: CameraRaySample,
) -> Option<Ray> {
    let [width, height] = sample.viewport;
    let [x, y] = sample.pixel;
    let camera = scene.camera(camera_key)?;
    let node = scene.camera_node(camera_key)?;
    let transform = scene.world_transform(node)?;
    let ndc_x = ((x as f32 + sample.jitter[0]) / width.max(1) as f32).mul_add(2.0, -1.0);
    let ndc_y = 1.0 - ((y as f32 + sample.jitter[1]) / height.max(1) as f32) * 2.0;
    match camera {
        Camera::Perspective(camera) => {
            let aspect = if camera.aspect > 0.0 {
                camera.aspect
            } else {
                width.max(1) as f32 / height.max(1) as f32
            };
            let tangent = (camera.vertical_fov.radians() * 0.5).tan();
            let local = Vec3::new(ndc_x * aspect * tangent, ndc_y * tangent, -1.0);
            if let Some(depth_of_field) = sample.depth_of_field {
                let lens = aperture_sample(
                    x,
                    y,
                    sample.sample,
                    depth_of_field.aperture_blades(),
                    depth_of_field.focal_length_mm() * 0.001
                        / (2.0 * depth_of_field.aperture_f_stop()),
                );
                let focus_scale = depth_of_field.focus_distance() / -local.z;
                let focus_point = local * focus_scale;
                let local_origin = Vec3::new(lens[0], lens[1], 0.0);
                return Some(Ray {
                    origin: transform_point(local_origin, transform),
                    direction: (transform.rotation * (focus_point - local_origin))
                        .normalize_or_zero(),
                });
            }
            Some(Ray {
                origin: transform.translation,
                direction: (transform.rotation * local).normalize_or_zero(),
            })
        }
        Camera::Orthographic(camera) => Some(Ray {
            origin: transform_point(
                Vec3::new(
                    camera.left + (ndc_x + 1.0) * 0.5 * (camera.right - camera.left),
                    camera.bottom + (ndc_y + 1.0) * 0.5 * (camera.top - camera.bottom),
                    0.0,
                ),
                transform,
            ),
            direction: (transform.rotation * Vec3::NEG_Z).normalize_or_zero(),
        }),
    }
}

fn aperture_sample(x: u32, y: u32, sample: u32, blades: u8, radius: f32) -> [f32; 2] {
    if radius <= 0.0 {
        return [0.0, 0.0];
    }
    let mut rng = seed(x, y, sample).rotate_left(17);
    let radial = random01(&mut rng).sqrt();
    let angle = random01(&mut rng) * std::f32::consts::TAU;
    let blades = u32::from(blades.max(3)) as f32;
    let sector = std::f32::consts::TAU / blades;
    let local_angle = (angle + sector * 0.5).rem_euclid(sector) - sector * 0.5;
    let polygon_radius = (sector * 0.5).cos() / local_angle.cos().max(1.0e-4);
    let r = radius * radial * polygon_radius;
    [angle.cos() * r, angle.sin() * r]
}

fn oriented_normal(hit: Hit, incoming: Vec3) -> Vec3 {
    let normal = hit.normal.unwrap_or(Vec3::Y).normalize_or_zero();
    if normal.dot(incoming) > 0.0 {
        -normal
    } else {
        normal
    }
}

fn hit_node(hit: Hit) -> NodeKey {
    match hit.target {
        HitTarget::Node(node) | HitTarget::Instance { node, .. } => node,
    }
}

fn light_kind(light: Light) -> &'static str {
    match light {
        Light::Directional(_) => "directional",
        Light::Point(_) => "point",
        Light::Spot(_) => "spot",
        Light::Area(_) => "rectangular_area",
    }
}

fn edge_aware_denoise(input: &[Vec3], guides: &[Guide], width: u32, height: u32) -> Vec<Vec3> {
    let mut output = input.to_vec();
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            let guide = guides[index];
            let mut sum = input[index] * 4.0;
            let mut weight = 4.0;
            for oy in -1..=1 {
                for ox in -1..=1 {
                    if ox == 0 && oy == 0 {
                        continue;
                    }
                    let nx = x as i32 + ox;
                    let ny = y as i32 + oy;
                    if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                        continue;
                    }
                    let neighbor = (ny as u32 * width + nx as u32) as usize;
                    let other = guides[neighbor];
                    let same_surface = guide.hit == other.hit
                        && (!guide.hit
                            || (guide.target == other.target
                                && guide.normal.dot(other.normal) > 0.92
                                && (guide.depth - other.depth).abs()
                                    < guide.depth.max(0.01) * 0.04));
                    if same_surface {
                        sum += input[neighbor];
                        weight += 1.0;
                    }
                }
            }
            output[index] = sum / weight;
        }
    }
    output
}

fn suppress_isolated_fireflies(values: &mut [Vec3], guides: &[Guide], width: u32, height: u32) {
    let original = values.to_vec();
    for y in 1..height.saturating_sub(1) {
        for x in 1..width.saturating_sub(1) {
            let index = (y * width + x) as usize;
            let mut neighbors = Vec::with_capacity(8);
            for oy in -1..=1 {
                for ox in -1..=1 {
                    if ox == 0 && oy == 0 {
                        continue;
                    }
                    let other = ((y as i32 + oy) as u32 * width + (x as i32 + ox) as u32) as usize;
                    if guides[index].target == guides[other].target {
                        neighbors.push(original[other].max_element());
                    }
                }
            }
            if neighbors.len() < 4 {
                continue;
            }
            neighbors.sort_by(f32::total_cmp);
            let median = neighbors[neighbors.len() / 2].max(1.0e-4);
            let luminance = original[index].max_element();
            if luminance > median * 8.0 {
                values[index] *= (median * 8.0) / luminance;
            }
        }
    }
}

fn refract_or_reflect(direction: Vec3, normal: Vec3, ior: f32, rough: Vec3) -> Vec3 {
    let front = direction.dot(normal) < 0.0;
    let ratio = if front {
        1.0 / ior.max(1.0)
    } else {
        ior.max(1.0)
    };
    let cos_theta = (-direction).dot(normal).min(1.0);
    let perpendicular = ratio * (direction + cos_theta * normal);
    let parallel_squared = 1.0 - perpendicular.length_squared();
    if parallel_squared <= 0.0 {
        reflect(direction, normal)
    } else {
        (perpendicular - normal * parallel_squared.sqrt() + rough).normalize_or_zero()
    }
}

fn reflect(direction: Vec3, normal: Vec3) -> Vec3 {
    direction - 2.0 * direction.dot(normal) * normal
}

fn cosine_hemisphere(normal: Vec3, rng: &mut u64) -> Vec3 {
    let r1 = random01(rng);
    let r2 = random01(rng);
    let radius = r1.sqrt();
    let theta = std::f32::consts::TAU * r2;
    let local = Vec3::new(
        radius * theta.cos(),
        radius * theta.sin(),
        (1.0 - r1).sqrt(),
    );
    tangent_to_world(local, normal)
}

fn random_in_hemisphere(normal: Vec3, rng: &mut u64) -> Vec3 {
    let sample = random_unit_vector(rng);
    if sample.dot(normal) < 0.0 {
        -sample
    } else {
        sample
    }
}

fn random_unit_vector(rng: &mut u64) -> Vec3 {
    let z = random01(rng).mul_add(2.0, -1.0);
    let theta = random01(rng) * std::f32::consts::TAU;
    let radius = (1.0 - z * z).max(0.0).sqrt();
    Vec3::new(radius * theta.cos(), radius * theta.sin(), z)
}

fn tangent_to_world(local: Vec3, normal: Vec3) -> Vec3 {
    let helper = if normal.z.abs() < 0.999 {
        Vec3::Z
    } else {
        Vec3::X
    };
    let tangent = helper.cross(normal).normalize_or_zero();
    let bitangent = normal.cross(tangent);
    (tangent * local.x + bitangent * local.y + normal * local.z).normalize_or_zero()
}

fn schlick(cosine: f32, ior: f32) -> f32 {
    let r0 = ((1.0 - ior) / (1.0 + ior)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
}

fn scene_epsilon(distance: f32) -> f32 {
    distance.abs().mul_add(1.0e-5, 1.0e-5).clamp(1.0e-5, 0.01)
}

fn sample_jitter(x: u32, y: u32, sample: u32) -> [f32; 2] {
    let mut state = seed(x, y, sample);
    [random01(&mut state), random01(&mut state)]
}

fn seed(x: u32, y: u32, sample: u32) -> u64 {
    (u64::from(x) << 32) ^ (u64::from(y) << 8) ^ u64::from(sample) ^ 0x9E37_79B9_7F4A_7C15
}

fn random01(state: &mut u64) -> f32 {
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    let value = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
    ((value >> 40) as u32) as f32 / 16_777_216.0
}

fn color_vec(color: Color) -> Vec3 {
    Vec3::new(color.r, color.g, color.b)
}

fn transform_point(point: Vec3, transform: Transform) -> Vec3 {
    transform.translation + transform.rotation * (point * transform.scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeometryDesc, MaterialDesc};

    #[test]
    fn photographic_final_path_traces_scene_and_reports_real_work() {
        let mut host = SceneHostCore::headless(32, 24).expect("headless host builds");
        let geometry = host
            .assets
            .create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
        let material = host.assets.create_material(
            MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(110, 125, 145), 0.65, 0.28)
                .with_photographic_micro_surface(0.025, 0.002),
        );
        let node = host
            .scene
            .mesh(geometry, material)
            .add()
            .expect("mesh inserts");
        let subject = host.register_node(node);
        host.scene.add_studio_lighting().expect("lights insert");
        host.frame_all().expect("scene frames");
        host.prepare().expect("raster prepares");
        host.render().expect("raster renders");
        let raster = host.capture().expect("raster captures");

        let (final_capture, report) = host
            .render_photographic_final(
                &raster,
                Some(subject),
                PhotographicTransportQuality::Preview,
            )
            .expect("final transport renders");

        assert_eq!(final_capture.rgba8.len(), raster.rgba8.len());
        assert_ne!(final_capture.rgba8, raster.rgba8);
        assert_eq!(report.path, "cpu_progressive_path_tracer");
        assert!(report.primary_rays >= 32_u64 * 24 * 2);
        assert!(report.intersections > 0);
        assert!(report.shadow_rays > 0);
        assert!(report.mesh_acceleration_structures > 0);
        assert!(report.multiple_importance_sampling);
        assert!(report.edge_aware_denoising);
    }

    #[test]
    fn denoiser_preserves_target_boundaries_and_firefly_filter_is_local() {
        let width = 3;
        let height = 3;
        let mut values = vec![Vec3::splat(0.2); 9];
        values[4] = Vec3::splat(100.0);
        let guides = vec![
            Guide {
                target: None,
                depth: 1.0,
                normal: Vec3::Z,
                hit: true,
            };
            9
        ];
        suppress_isolated_fireflies(&mut values, &guides, width, height);
        assert!(values[4].max_element() <= 1.61);

        let mut boundary_guides = guides;
        boundary_guides[4].target = Some(NodeKey::default());
        let denoised = edge_aware_denoise(&values, &boundary_guides, width, height);
        assert_eq!(denoised[4], values[4]);
    }
}
