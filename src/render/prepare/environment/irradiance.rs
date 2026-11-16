//! Order-2 spherical-harmonics irradiance for diffuse image-based lighting.
//!
//! A single average colour lights every surface direction identically, which
//! removes all shape cues from matte materials. Nine RGB coefficients keep the
//! environment's low-frequency directionality (bright softbox above, darker
//! floor below) at negligible shading cost, following Ramamoorthi and
//! Hanrahan, "An Efficient Representation for Irradiance Environment Maps".

use crate::scene::Vec3;

use super::super::environment_baker::cubemap_face_direction;

pub(in crate::render) const IRRADIANCE_SH_COEFFICIENT_COUNT: usize = 9;

/// Real spherical-harmonics basis constants for bands 0, 1 and 2.
const Y00: f32 = 0.282_094_8;
const Y1: f32 = 0.488_602_5;
const Y2_OFF_DIAGONAL: f32 = 1.092_548_4;
const Y20: f32 = 0.315_391_57;
const Y22: f32 = 0.546_274_2;

/// Cosine-lobe convolution per band divided by pi, so the evaluated value is
/// the radiance leaving a white Lambertian surface rather than irradiance.
const LAMBERT_BAND_0: f32 = 1.0;
const LAMBERT_BAND_1: f32 = 2.0 / 3.0;
const LAMBERT_BAND_2: f32 = 0.25;

/// Mild band window. Small bright emitters otherwise ring into negative lobes
/// on the side of the sphere facing away from them.
const WINDOW_BAND_1: f32 = 0.98;
const WINDOW_BAND_2: f32 = 0.85;

/// Faces larger than this are box-averaged down before projection.
const PROJECTION_FACE_RESOLUTION: usize = 64;

/// Polynomial-form irradiance coefficients.
///
/// [`Self::radiance`] returns `irradiance(normal) / pi`: multiply by a diffuse
/// albedo to get outgoing radiance. Coefficient 0 is the spherical mean.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct DiffuseIrradianceSh {
    coefficients: [Vec3; IRRADIANCE_SH_COEFFICIENT_COUNT],
}

impl DiffuseIrradianceSh {
    /// A direction-independent environment: every normal sees `radiance`.
    pub(in crate::render) const fn uniform(radiance: Vec3) -> Self {
        let mut coefficients = [Vec3::ZERO; IRRADIANCE_SH_COEFFICIENT_COUNT];
        coefficients[0] = radiance;
        Self { coefficients }
    }

    /// Projects six RGBA32F cubemap faces of `resolution` square texels.
    ///
    /// Returns `None` when the buffers do not match `resolution` or hold no
    /// finite energy, so callers keep the uniform fallback.
    pub(in crate::render) fn from_cubemap_faces(
        faces: &[Vec<f32>; 6],
        resolution: u32,
    ) -> Option<Self> {
        let size = resolution as usize;
        if size == 0 || faces.iter().any(|face| face.len() != size * size * 4) {
            return None;
        }
        // Radiance varies slowly against a band-2 basis, so large faces are
        // box-averaged into blocks before the basis is evaluated. Every source
        // texel still contributes its energy.
        let stride = if size > PROJECTION_FACE_RESOLUTION
            && size.is_multiple_of(PROJECTION_FACE_RESOLUTION)
        {
            size / PROJECTION_FACE_RESOLUTION
        } else {
            1
        };
        let blocks = size / stride;
        let block_texel = 2.0 / blocks as f32;
        let block_weight = 1.0 / (stride * stride) as f64;
        let mut sums = [[0.0_f64; 3]; IRRADIANCE_SH_COEFFICIENT_COUNT];
        let mut solid_angle_total = 0.0_f64;
        for (face_index, face) in faces.iter().enumerate() {
            for block_y in 0..blocks {
                let v = (block_y as f32 + 0.5) * block_texel - 1.0;
                for block_x in 0..blocks {
                    let u = (block_x as f32 + 0.5) * block_texel - 1.0;
                    let mut radiance = [0.0_f64; 3];
                    for y in block_y * stride..(block_y + 1) * stride {
                        for x in block_x * stride..(block_x + 1) * stride {
                            let pixel = &face[(y * size + x) * 4..(y * size + x) * 4 + 3];
                            if pixel.iter().all(|channel| channel.is_finite()) {
                                radiance[0] += f64::from(pixel[0].max(0.0));
                                radiance[1] += f64::from(pixel[1].max(0.0));
                                radiance[2] += f64::from(pixel[2].max(0.0));
                            }
                        }
                    }
                    let direction = cubemap_face_direction(face_index, u, v);
                    // Differential solid angle of a cube-face texel.
                    let stretch = 1.0 + u * u + v * v;
                    let solid_angle =
                        f64::from(block_texel * block_texel / (stretch * stretch.sqrt()));
                    solid_angle_total += solid_angle;
                    let basis = basis_polynomials(direction);
                    for (sum, weight) in sums.iter_mut().zip(basis) {
                        let weight = f64::from(weight) * solid_angle * block_weight;
                        sum[0] += radiance[0] * weight;
                        sum[1] += radiance[1] * weight;
                        sum[2] += radiance[2] * weight;
                    }
                }
            }
        }
        if solid_angle_total <= 0.0 {
            return None;
        }
        // Rescale so the sampled sphere integrates to exactly 4*pi regardless
        // of skipped non-finite texels.
        let sphere = 4.0 * std::f64::consts::PI / solid_angle_total;
        let band_scale = band_scales();
        let mut coefficients = [Vec3::ZERO; IRRADIANCE_SH_COEFFICIENT_COUNT];
        for ((coefficient, sum), scale) in coefficients.iter_mut().zip(sums).zip(band_scale) {
            let scale = f64::from(scale) * sphere;
            *coefficient = Vec3::new(
                (sum[0] * scale) as f32,
                (sum[1] * scale) as f32,
                (sum[2] * scale) as f32,
            );
        }
        let mean = coefficients[0];
        (mean.is_finite() && mean.max_element() > f32::EPSILON).then_some(Self { coefficients })
    }

    /// Rescales every channel so the spherical mean equals `mean`.
    ///
    /// Environments that record an authored preview irradiance keep that
    /// calibrated brightness; the cubemap contributes only directionality.
    pub(in crate::render) fn normalized_to_mean(mut self, mean: Vec3) -> Self {
        let current = self.coefficients[0];
        let scale = Vec3::new(
            channel_scale(mean.x, current.x),
            channel_scale(mean.y, current.y),
            channel_scale(mean.z, current.z),
        );
        for coefficient in &mut self.coefficients {
            *coefficient *= scale;
        }
        self.coefficients[0] = mean;
        self
    }

    /// Radiance leaving a white Lambertian surface with unit `normal`.
    pub(in crate::render) fn radiance(&self, normal: Vec3) -> Vec3 {
        let c = &self.coefficients;
        let n = normal;
        let value = c[0]
            + c[1] * n.y
            + c[2] * n.z
            + c[3] * n.x
            + c[4] * (n.x * n.y)
            + c[5] * (n.y * n.z)
            + c[6] * (3.0 * n.z * n.z - 1.0)
            + c[7] * (n.x * n.z)
            + c[8] * (n.x * n.x - n.y * n.y);
        value.max(Vec3::ZERO)
    }

    #[cfg(test)]
    pub(in crate::render) fn mean(&self) -> Vec3 {
        self.coefficients[0]
    }

    /// Coefficients in the order the WGSL `environment_irradiance` reads them.
    pub(in crate::render) fn gpu_coefficients(
        &self,
    ) -> [[f32; 4]; IRRADIANCE_SH_COEFFICIENT_COUNT] {
        self.coefficients
            .map(|coefficient| [coefficient.x, coefficient.y, coefficient.z, 0.0])
    }
}

fn channel_scale(target: f32, current: f32) -> f32 {
    if current > f32::EPSILON && target.is_finite() {
        target / current
    } else {
        0.0
    }
}

/// Unnormalised basis polynomials in evaluation order.
fn basis_polynomials(direction: Vec3) -> [f32; IRRADIANCE_SH_COEFFICIENT_COUNT] {
    let Vec3 { x, y, z } = direction;
    [
        1.0,
        y,
        z,
        x,
        x * y,
        y * z,
        3.0 * z * z - 1.0,
        x * z,
        x * x - y * y,
    ]
}

/// Basis constant squared (projection and reconstruction), Lambert band
/// factor, and ringing window for each coefficient.
fn band_scales() -> [f32; IRRADIANCE_SH_COEFFICIENT_COUNT] {
    let band_1 = Y1 * Y1 * LAMBERT_BAND_1 * WINDOW_BAND_1;
    let band_2 = LAMBERT_BAND_2 * WINDOW_BAND_2;
    [
        Y00 * Y00 * LAMBERT_BAND_0,
        band_1,
        band_1,
        band_1,
        Y2_OFF_DIAGONAL * Y2_OFF_DIAGONAL * band_2,
        Y2_OFF_DIAGONAL * Y2_OFF_DIAGONAL * band_2,
        Y20 * Y20 * band_2,
        Y2_OFF_DIAGONAL * Y2_OFF_DIAGONAL * band_2,
        Y22 * Y22 * band_2,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faces_from(resolution: usize, radiance: impl Fn(Vec3) -> Vec3) -> [Vec<f32>; 6] {
        std::array::from_fn(|face| {
            let mut pixels = Vec::with_capacity(resolution * resolution * 4);
            for y in 0..resolution {
                for x in 0..resolution {
                    let u = (x as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let value = radiance(cubemap_face_direction(face, u, v));
                    pixels.extend_from_slice(&[value.x, value.y, value.z, 1.0]);
                }
            }
            pixels
        })
    }

    fn assert_close(actual: Vec3, expected: Vec3, tolerance: f32, label: &str) {
        assert!(
            (actual - expected).abs().max_element() <= tolerance,
            "{label}: expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn uniform_environment_lights_every_normal_with_its_radiance() {
        let radiance = Vec3::new(0.4, 0.5, 0.6);
        let faces = faces_from(16, |_| radiance);
        let sh =
            DiffuseIrradianceSh::from_cubemap_faces(&faces, 16).expect("uniform faces project");
        for normal in [Vec3::X, Vec3::Y, Vec3::Z, -Vec3::X, -Vec3::Y, -Vec3::Z] {
            assert_close(sh.radiance(normal), radiance, 2.0e-3, "uniform environment");
        }
        assert_close(sh.mean(), radiance, 2.0e-3, "uniform mean");
    }

    #[test]
    fn sky_above_ground_below_makes_upward_normals_brighter_than_downward() {
        // Radiance 1 in the upper hemisphere, 0 below: the exact Lambert
        // response is 1.0 facing up, 0.5 at the horizon and 0.0 facing down.
        let faces = faces_from(64, |direction| {
            if direction.y > 0.0 {
                Vec3::ONE
            } else {
                Vec3::ZERO
            }
        });
        let sh = DiffuseIrradianceSh::from_cubemap_faces(&faces, 64).expect("hemisphere projects");
        let up = sh.radiance(Vec3::Y).x;
        let side = sh.radiance(Vec3::X).x;
        let down = sh.radiance(-Vec3::Y).x;
        assert!((up - 1.0).abs() < 0.08, "up-facing radiance {up}");
        assert!((side - 0.5).abs() < 0.05, "horizon-facing radiance {side}");
        assert!(down < 0.08, "down-facing radiance {down}");
        assert!(up > side && side > down);
    }

    #[test]
    fn directional_lobe_follows_the_bright_side_of_the_environment() {
        let faces = faces_from(32, |direction| Vec3::splat(direction.x.max(0.0)));
        let sh = DiffuseIrradianceSh::from_cubemap_faces(&faces, 32).expect("lobe projects");
        assert!(sh.radiance(Vec3::X).x > 2.0 * sh.radiance(Vec3::Z).x);
        assert!(sh.radiance(Vec3::Z).x > sh.radiance(-Vec3::X).x);
    }

    #[test]
    fn normalizing_to_a_mean_preserves_shape_and_sets_the_average() {
        let faces = faces_from(32, |direction| Vec3::splat(0.2 + direction.y.max(0.0)));
        let sh = DiffuseIrradianceSh::from_cubemap_faces(&faces, 32).expect("projects");
        let ratio_before = sh.radiance(Vec3::Y).x / sh.radiance(-Vec3::Y).x;
        let target = Vec3::new(0.25, 0.5, 1.0);
        let normalized = sh.normalized_to_mean(target);
        assert_close(normalized.mean(), target, 1.0e-6, "normalized mean");
        let ratio_after = normalized.radiance(Vec3::Y).x / normalized.radiance(-Vec3::Y).x;
        assert!((ratio_before - ratio_after).abs() < 1.0e-3);
        assert!(
            (normalized.radiance(Vec3::Y).z / normalized.radiance(Vec3::Y).x - 4.0).abs() < 1.0e-3
        );
    }

    #[test]
    fn mismatched_or_empty_faces_fall_back_to_none() {
        let faces = faces_from(8, |_| Vec3::ONE);
        assert!(DiffuseIrradianceSh::from_cubemap_faces(&faces, 4).is_none());
        let black = faces_from(8, |_| Vec3::ZERO);
        assert!(DiffuseIrradianceSh::from_cubemap_faces(&black, 8).is_none());
    }

    #[test]
    fn uniform_constructor_is_direction_independent() {
        let sh = DiffuseIrradianceSh::uniform(Vec3::new(0.1, 0.2, 0.3));
        assert_eq!(sh.radiance(Vec3::Y), sh.radiance(-Vec3::Z));
        assert_eq!(sh.gpu_coefficients()[0], [0.1, 0.2, 0.3, 0.0]);
        assert!(sh.gpu_coefficients()[1..].iter().all(|c| *c == [0.0; 4]));
    }
}
