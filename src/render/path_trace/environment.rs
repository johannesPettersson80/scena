//! Image-based lighting: the environment as a cube of radiance that can be
//! looked up along a direction and sampled in proportion to its brightness.

use glam::Vec3;

use crate::assets::EnvironmentDesc;

use super::super::prepare::environment_baker::cubemap_face_direction;
use super::sampling::luminance;

/// Share of the sampling density spread evenly over all directions, so dark
/// texels next to bright ones are still reached.
const UNIFORM_SHARE: f32 = 0.05;

pub(super) struct TraceEnvironment {
    resolution: usize,
    faces: [Vec<[f32; 3]>; 6],
    intensity: f32,
    /// Sine and cosine of the rotation about the vertical axis.
    rotation: (f32, f32),
    /// Probability of each row of each face; `6 * resolution` entries.
    row_probability: Vec<f32>,
    row_cdf: Vec<f32>,
    /// Probability of each texel within its row.
    texel_probability: Vec<f32>,
    texel_cdf: Vec<f32>,
}

/// A direction drawn from the environment.
pub(super) struct EnvironmentSample {
    pub(super) direction: Vec3,
    pub(super) radiance: Vec3,
    /// Density of `direction` per unit solid angle.
    pub(super) pdf: f32,
}

/// Solid angle of a unit area of the cube face at `(u, v)`.
fn solid_angle_scale(u: f32, v: f32) -> f32 {
    (1.0 + u * u + v * v).powf(-1.5)
}

impl TraceEnvironment {
    /// Builds the environment at `resolution` texels per cube edge. Sources
    /// that are not equirectangular images keep their own resolution.
    pub(super) fn from_desc(
        desc: &EnvironmentDesc,
        resolution: u32,
        intensity: f32,
        rotation_y_radians: f32,
    ) -> Option<Self> {
        if !(intensity.is_finite() && intensity > 0.0) {
            return None;
        }
        let Some(cubemap) = desc
            .clone()
            .with_cubemap_resolution(resolution)
            .cubemap_faces()
        else {
            // No image to sample: light evenly with the environment's
            // stated irradiance, as the rasterizer does.
            let radiance = desc.preview_irradiance_rgb()?;
            return Some(Self::from_faces(
                1,
                std::array::from_fn(|_| vec![radiance]),
                intensity,
                0.0,
            ));
        };
        let resolution = cubemap.resolution().max(1) as usize;
        let faces: [Vec<[f32; 3]>; 6] = match cubemap.face_pixels.as_ref() {
            Some(faces) => faces.clone(),
            None => cubemap.build_face_pixels_rgba32f().map(|face| {
                face.chunks_exact(4)
                    .map(|texel| [texel[0], texel[1], texel[2]])
                    .collect()
            }),
        };
        if faces
            .iter()
            .any(|face| face.len() != resolution * resolution)
        {
            return None;
        }
        Some(Self::from_faces(
            resolution,
            faces,
            intensity,
            rotation_y_radians,
        ))
    }

    pub(super) fn from_faces(
        resolution: usize,
        mut faces: [Vec<[f32; 3]>; 6],
        intensity: f32,
        rotation_y_radians: f32,
    ) -> Self {
        for texel in faces.iter_mut().flatten() {
            *texel = texel.map(|channel| {
                if channel.is_finite() {
                    channel.max(0.0)
                } else {
                    0.0
                }
            });
        }
        let texel_center = |index: usize| (index as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
        // Weight of a texel: its brightness times the solid angle it covers.
        let weight = |face: usize, x: usize, y: usize| {
            let texel = faces[face][y * resolution + x];
            f64::from(
                luminance(Vec3::from(texel)) * solid_angle_scale(texel_center(x), texel_center(y)),
            )
        };
        let mut total = 0.0_f64;
        let mut solid_angle = 0.0_f64;
        for face in 0..6 {
            for y in 0..resolution {
                for x in 0..resolution {
                    total += weight(face, x, y);
                    solid_angle += f64::from(solid_angle_scale(texel_center(x), texel_center(y)));
                }
            }
        }
        // Mixing in a uniform share keeps every direction reachable.
        let floor = if total > 0.0 {
            total / solid_angle * f64::from(UNIFORM_SHARE / (1.0 - UNIFORM_SHARE))
        } else {
            1.0
        };
        let mixed = |face: usize, x: usize, y: usize| {
            weight(face, x, y)
                + floor * f64::from(solid_angle_scale(texel_center(x), texel_center(y)))
        };
        let rows = 6 * resolution;
        let mut row_probability = vec![0.0_f32; rows];
        let mut texel_probability = vec![0.0_f32; rows * resolution];
        let mut row_weights = vec![0.0_f64; rows];
        for face in 0..6 {
            for y in 0..resolution {
                let row = face * resolution + y;
                let row_total: f64 = (0..resolution).map(|x| mixed(face, x, y)).sum();
                row_weights[row] = row_total;
                for x in 0..resolution {
                    texel_probability[row * resolution + x] =
                        (mixed(face, x, y) / row_total) as f32;
                }
            }
        }
        let all: f64 = row_weights.iter().sum();
        for (probability, weight) in row_probability.iter_mut().zip(&row_weights) {
            *probability = (weight / all) as f32;
        }
        let row_cdf = cumulative(&row_probability);
        let texel_cdf = texel_probability
            .chunks(resolution)
            .flat_map(cumulative)
            .collect();
        let (sin, cos) = rotation_y_radians.sin_cos();
        Self {
            resolution,
            faces,
            intensity,
            rotation: (sin, cos),
            row_probability,
            row_cdf,
            texel_probability,
            texel_cdf,
        }
    }

    /// Texels along each edge of the cube.
    pub(super) fn resolution(&self) -> u32 {
        self.resolution as u32
    }

    /// World direction to the direction the cube is addressed with.
    fn to_cube(&self, direction: Vec3) -> Vec3 {
        let (sin, cos) = self.rotation;
        Vec3::new(
            cos * direction.x + sin * direction.z,
            direction.y,
            -sin * direction.x + cos * direction.z,
        )
    }

    fn to_world(&self, direction: Vec3) -> Vec3 {
        let (sin, cos) = self.rotation;
        Vec3::new(
            cos * direction.x - sin * direction.z,
            direction.y,
            sin * direction.x + cos * direction.z,
        )
    }

    fn texel(&self, face: usize, x: usize, y: usize) -> Vec3 {
        Vec3::from(self.faces[face][y * self.resolution + x])
    }

    /// Radiance arriving from `direction`, filtered bilinearly within a face.
    pub(super) fn radiance(&self, direction: Vec3) -> Vec3 {
        let (face, u, v) = face_coordinates(self.to_cube(direction));
        let last = self.resolution - 1;
        let to_texel = |coordinate: f32| {
            ((coordinate * 0.5 + 0.5) * self.resolution as f32 - 0.5).clamp(0.0, last as f32)
        };
        let (x, y) = (to_texel(u), to_texel(v));
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(last), (y0 + 1).min(last));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let top = self.texel(face, x0, y0).lerp(self.texel(face, x1, y0), fx);
        let bottom = self.texel(face, x0, y1).lerp(self.texel(face, x1, y1), fx);
        top.lerp(bottom, fy) * self.intensity
    }

    /// Density with which [`Self::sample`] produces `direction`.
    pub(super) fn pdf(&self, direction: Vec3) -> f32 {
        let (face, u, v) = face_coordinates(self.to_cube(direction));
        let to_index = |coordinate: f32| {
            (((coordinate * 0.5 + 0.5) * self.resolution as f32) as usize).min(self.resolution - 1)
        };
        self.texel_pdf(face, to_index(u), to_index(v), u, v)
    }

    fn texel_pdf(&self, face: usize, x: usize, y: usize, u: f32, v: f32) -> f32 {
        let row = face * self.resolution + y;
        let probability =
            self.row_probability[row] * self.texel_probability[row * self.resolution + x];
        // A texel spans (2 / resolution)^2 of the face's (u, v) square.
        let texel_area = 4.0 / (self.resolution * self.resolution) as f32;
        probability / (texel_area * solid_angle_scale(u, v))
    }

    pub(super) fn sample(&self, pick: (f32, f32), jitter: (f32, f32)) -> EnvironmentSample {
        let row = search(&self.row_cdf, pick.0);
        let x = search(
            &self.texel_cdf[row * self.resolution..(row + 1) * self.resolution],
            pick.1,
        );
        let (face, y) = (row / self.resolution, row % self.resolution);
        let u = (x as f32 + jitter.0) / self.resolution as f32 * 2.0 - 1.0;
        let v = (y as f32 + jitter.1) / self.resolution as f32 * 2.0 - 1.0;
        let direction = self.to_world(cubemap_face_direction(face, u, v));
        EnvironmentSample {
            direction,
            radiance: self.radiance(direction),
            pdf: self.texel_pdf(face, x, y, u, v),
        }
    }
}

fn cumulative(probabilities: &[f32]) -> Vec<f32> {
    let mut sum = 0.0_f32;
    probabilities
        .iter()
        .map(|probability| {
            sum += probability;
            sum
        })
        .collect()
}

/// First entry of a cumulative distribution that reaches `u`.
fn search(cdf: &[f32], u: f32) -> usize {
    let target = u * cdf.last().copied().unwrap_or(1.0);
    cdf.partition_point(|value| *value <= target)
        .min(cdf.len() - 1)
}

/// Face and `(u, v)` in `[-1, 1]` for a direction; the inverse of
/// `cubemap_face_direction`.
fn face_coordinates(direction: Vec3) -> (usize, f32, f32) {
    let magnitude = direction.abs();
    let (x, y, z) = (direction.x, direction.y, direction.z);
    if magnitude.x >= magnitude.y && magnitude.x >= magnitude.z && magnitude.x > 0.0 {
        if x > 0.0 {
            (0, -z / x, -y / x)
        } else {
            (1, -z / x, y / x)
        }
    } else if magnitude.y >= magnitude.z && magnitude.y > 0.0 {
        if y > 0.0 {
            (2, x / y, z / y)
        } else {
            (3, -x / y, z / y)
        }
    } else if magnitude.z > 0.0 {
        if z > 0.0 {
            (4, x / z, -y / z)
        } else {
            (5, x / z, y / z)
        }
    } else {
        (4, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::path_trace::sampling::Rng;

    fn uniform_direction(rng: &mut Rng) -> Vec3 {
        let (a, b) = rng.pair();
        let z = 1.0 - 2.0 * a;
        let radius = (1.0 - z * z).max(0.0).sqrt();
        let phi = std::f32::consts::TAU * b;
        Vec3::new(radius * phi.cos(), radius * phi.sin(), z)
    }

    /// A dim cube with one bright patch on the `+x` face.
    fn spot_environment(rotation: f32) -> TraceEnvironment {
        let resolution = 16;
        let mut faces: [Vec<[f32; 3]>; 6] =
            std::array::from_fn(|_| vec![[0.05; 3]; resolution * resolution]);
        for y in 6..10 {
            for x in 6..10 {
                faces[0][y * resolution + x] = [40.0, 30.0, 20.0];
            }
        }
        TraceEnvironment::from_faces(resolution, faces, 1.0, rotation)
    }

    #[test]
    fn face_lookup_inverts_the_cubemap_face_directions() {
        for face in 0..6 {
            for (u, v) in [(0.0, 0.0), (0.7, -0.4), (-0.95, 0.9), (0.3, 0.99)] {
                let (found, found_u, found_v) =
                    face_coordinates(cubemap_face_direction(face, u, v));
                assert_eq!(found, face);
                assert!(
                    (found_u - u).abs() < 1.0e-5 && (found_v - v).abs() < 1.0e-5,
                    "face {face} ({u}, {v})"
                );
            }
        }
    }

    #[test]
    fn sampling_density_integrates_to_one_over_the_sphere() {
        let environment = spot_environment(0.4);
        let mut rng = Rng::new(3, 9);
        let samples = 200_000;
        let mass: f32 = (0..samples)
            .map(|_| environment.pdf(uniform_direction(&mut rng)))
            .sum::<f32>()
            * (4.0 * std::f32::consts::PI / samples as f32);
        assert!((mass - 1.0).abs() < 0.03, "density mass {mass}");
    }

    #[test]
    fn samples_report_the_density_the_lookup_reports() {
        let environment = spot_environment(-1.1);
        let mut rng = Rng::new(5, 2);
        let mut bright = 0;
        for _ in 0..2_000 {
            let sample = environment.sample(rng.pair(), rng.pair());
            assert!((sample.direction.length() - 1.0).abs() < 1.0e-4);
            let looked_up = environment.pdf(sample.direction);
            assert!(
                (looked_up - sample.pdf).abs() <= 0.02 * sample.pdf,
                "sampled {} but looked up {looked_up}",
                sample.pdf
            );
            bright += usize::from(sample.radiance.x > 5.0);
        }
        // The patch covers under 1 % of the sphere yet carries most energy.
        assert!(bright > 1_400, "bright patch drew {bright} of 2000 samples");
    }

    #[test]
    fn importance_sampling_and_uniform_sampling_agree_on_irradiance() {
        let environment = spot_environment(0.9);
        let normal = Vec3::new(0.3, 0.8, 0.52).normalize();
        let mut rng = Rng::new(17, 4);
        let samples = 150_000;
        let (mut importance, mut uniform) = (Vec3::ZERO, Vec3::ZERO);
        for _ in 0..samples {
            let sample = environment.sample(rng.pair(), rng.pair());
            importance += sample.radiance * (sample.direction.dot(normal).max(0.0) / sample.pdf);
            let direction = uniform_direction(&mut rng);
            uniform += environment.radiance(direction)
                * (direction.dot(normal).max(0.0) * 4.0 * std::f32::consts::PI);
        }
        let (importance, uniform) = (importance / samples as f32, uniform / samples as f32);
        assert!(
            (importance - uniform).abs().max_element() < 0.06 * uniform.max_element(),
            "importance {importance:?} uniform {uniform:?}"
        );
    }

    #[test]
    fn rotation_turns_the_environment_about_the_vertical_axis() {
        let unrotated = spot_environment(0.0);
        assert!(unrotated.radiance(Vec3::X).x > 30.0);
        let rotated = spot_environment(std::f32::consts::FRAC_PI_2);
        assert!(rotated.radiance(Vec3::X).x < 1.0);
        let bright = rotated.to_world(Vec3::X);
        assert!(
            rotated.radiance(bright).x > 30.0,
            "bright patch moved to {bright:?}"
        );
    }
}
