//! Edge-avoiding wavelet filter that removes residual path-tracing noise.
//!
//! The filter works on illumination: the image divided by the surface
//! colour, so texture detail is set aside and restored exactly afterwards.
//! Each pass averages a pixel with neighbours on the same surface: close in
//! normal, lying in its tangent plane, and no further away in brightness
//! than the pixel's own noise level explains. A pixel that has converged is
//! therefore left alone, while a noisy one borrows from its surroundings.
//!
//! After Dammertz et al., "Edge-Avoiding A-Trous Wavelet Transform for fast
//! Global Illumination Filtering", and the variance guidance of Schied et
//! al., "Spatiotemporal Variance-Guided Filtering".

use glam::Vec3;

use super::film::for_rows;
use super::sampling::luminance;

const PASSES: u32 = 5;
/// B3 spline weights for offsets of 0, 1 and 2 steps.
const KERNEL: [f32; 3] = [3.0 / 8.0, 1.0 / 4.0, 1.0 / 16.0];
const NORMAL_POWER: f32 = 64.0;
/// Brightness differences accepted, in standard deviations of the noise.
const LUMINANCE_SIGMA: f32 = 4.0;
/// Distance from a pixel's tangent plane accepted, as a share of the
/// surface's distance from the camera.
const PLANE_TOLERANCE: f32 = 0.005;
/// Added to the surface colour before dividing by it.
const ALBEDO_FLOOR: f32 = 0.02;

/// What the filter knows about the surface in one pixel.
#[derive(Clone, Copy)]
pub(super) struct Guide {
    pub(super) albedo: Vec3,
    pub(super) normal: Vec3,
    /// Position relative to the camera.
    pub(super) position: Vec3,
    /// Variance of the pixel's mean luminance; `None` when too few paths
    /// reached the surface to measure it.
    pub(super) variance: Option<f32>,
}

struct Layer {
    width: usize,
    height: usize,
    illumination: Vec<Vec3>,
    variance: Vec<f32>,
}

/// Filters `color` in place. Pixels without a guide show no surface and are
/// left untouched.
pub(super) fn denoise(width: usize, height: usize, color: &mut [Vec3], guides: &[Option<Guide>]) {
    let modulation = |guide: &Guide| guide.albedo + Vec3::splat(ALBEDO_FLOOR);
    let illumination: Vec<Vec3> = color
        .iter()
        .zip(guides)
        .map(|(color, guide)| {
            guide
                .as_ref()
                .map_or(*color, |guide| *color / modulation(guide))
        })
        .collect();
    let variance = initial_variance(width, height, &illumination, guides);
    let mut layer = Layer {
        width,
        height,
        illumination,
        variance,
    };
    for pass in 0..PASSES {
        let step = 1_isize << pass;
        let smoothed = smooth_variance(&layer);
        let rows = for_rows(height, |y| {
            (0..width)
                .map(|x| layer.filter(x, y, step, guides, &smoothed))
                .collect::<Vec<_>>()
        });
        for (index, (illumination, variance)) in rows.into_iter().flatten().enumerate() {
            layer.illumination[index] = illumination;
            layer.variance[index] = variance;
        }
    }
    for ((color, illumination), guide) in color.iter_mut().zip(&layer.illumination).zip(guides) {
        if let Some(guide) = guide {
            *color = *illumination * modulation(guide);
        }
    }
}

/// Noise level of each pixel's illumination. Measured from the pixel's own
/// paths where enough of them reached the surface, and from how much its
/// neighbourhood varies otherwise.
fn initial_variance(
    width: usize,
    height: usize,
    illumination: &[Vec3],
    guides: &[Option<Guide>],
) -> Vec<f32> {
    (0..width * height)
        .map(|index| {
            let Some(guide) = guides[index] else {
                return 0.0;
            };
            let scale = luminance(guide.albedo + Vec3::splat(ALBEDO_FLOOR));
            if let Some(variance) = guide.variance {
                return variance / (scale * scale);
            }
            let (x, y) = ((index % width) as isize, (index / width) as isize);
            let (mut sum, mut sum_squared, mut count) = (0.0_f32, 0.0_f32, 0.0_f32);
            for dy in -2..=2_isize {
                for dx in -2..=2_isize {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= width as isize || ny >= height as isize {
                        continue;
                    }
                    let neighbour = ny as usize * width + nx as usize;
                    if guides[neighbour].is_some() {
                        let value = luminance(illumination[neighbour]);
                        sum += value;
                        sum_squared += value * value;
                        count += 1.0;
                    }
                }
            }
            let mean = sum / count;
            (sum_squared / count - mean * mean).max(0.0)
        })
        .collect()
}

/// A 3x3 blur of the variance, so one unlucky pixel does not misjudge its
/// own noise.
fn smooth_variance(layer: &Layer) -> Vec<f32> {
    let (width, height) = (layer.width as isize, layer.height as isize);
    (0..layer.variance.len())
        .map(|index| {
            let (x, y) = (index as isize % width, index as isize / width);
            let (mut sum, mut weights) = (0.0_f32, 0.0_f32);
            for dy in -1..=1_isize {
                for dx in -1..=1_isize {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= width || ny >= height {
                        continue;
                    }
                    let weight = (2 - dx.abs()) as f32 * (2 - dy.abs()) as f32;
                    sum += layer.variance[(ny * width + nx) as usize] * weight;
                    weights += weight;
                }
            }
            sum / weights
        })
        .collect()
}

impl Layer {
    /// One wavelet pass at one pixel: the filtered illumination and the
    /// variance that remains.
    fn filter(
        &self,
        x: usize,
        y: usize,
        step: isize,
        guides: &[Option<Guide>],
        smoothed: &[f32],
    ) -> (Vec3, f32) {
        let center = y * self.width + x;
        let Some(guide) = guides[center] else {
            return (self.illumination[center], self.variance[center]);
        };
        let brightness = luminance(self.illumination[center]);
        let noise = LUMINANCE_SIGMA * smoothed[center].max(0.0).sqrt() + 1.0e-6;
        let plane = PLANE_TOLERANCE * guide.position.length().max(1.0e-6);
        let (mut sum, mut variance, mut weights) = (Vec3::ZERO, 0.0_f32, 0.0_f32);
        for dy in -2..=2_isize {
            for dx in -2..=2_isize {
                let (nx, ny) = (x as isize + dx * step, y as isize + dy * step);
                if nx < 0 || ny < 0 || nx >= self.width as isize || ny >= self.height as isize {
                    continue;
                }
                let neighbour = ny as usize * self.width + nx as usize;
                let Some(other) = guides[neighbour] else {
                    continue;
                };
                let weight = if neighbour == center {
                    KERNEL[0] * KERNEL[0]
                } else {
                    let facing = guide.normal.dot(other.normal).max(0.0).powf(NORMAL_POWER);
                    let off_plane = guide.normal.dot(other.position - guide.position).abs() / plane;
                    let contrast =
                        (luminance(self.illumination[neighbour]) - brightness).abs() / noise;
                    KERNEL[dx.unsigned_abs()]
                        * KERNEL[dy.unsigned_abs()]
                        * facing
                        * (-off_plane - contrast).exp()
                };
                sum += self.illumination[neighbour] * weight;
                variance += self.variance[neighbour] * weight * weight;
                weights += weight;
            }
        }
        (sum / weights, variance / (weights * weights))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::path_trace::sampling::Rng;

    const SIZE: usize = 48;

    /// A flat wall facing the camera whose left half is lit ten times more
    /// brightly than its right half, with a checkered surface colour.
    fn scene(noise: f32, measured: bool) -> (Vec<Vec3>, Vec<Vec3>, Vec<Option<Guide>>) {
        let mut rng = Rng::new(9, 1);
        let (mut clean, mut noisy, mut guides) = (Vec::new(), Vec::new(), Vec::new());
        for y in 0..SIZE {
            for x in 0..SIZE {
                let albedo = if (x / 4 + y / 4) % 2 == 0 {
                    Vec3::new(0.8, 0.3, 0.2)
                } else {
                    Vec3::splat(0.6)
                };
                let light = if x < SIZE / 2 { 2.0 } else { 0.2 };
                let deviation = noise * light;
                clean.push(albedo * light);
                noisy
                    .push(albedo * (light + deviation * (rng.next() * 2.0 - 1.0) * 3.0_f32.sqrt()));
                guides.push(Some(Guide {
                    albedo,
                    normal: Vec3::Z,
                    position: Vec3::new(x as f32 * 0.01, y as f32 * 0.01, -5.0),
                    variance: measured.then_some((deviation * luminance(albedo)).powi(2)),
                }));
            }
        }
        (clean, noisy, guides)
    }

    fn error(image: &[Vec3], clean: &[Vec3]) -> f32 {
        let sum: f32 = image
            .iter()
            .zip(clean)
            .map(|(a, b)| (*a - *b).length_squared())
            .sum();
        (sum / image.len() as f32).sqrt()
    }

    #[test]
    fn noise_falls_while_texture_and_shadow_edges_stay_sharp() {
        for measured in [true, false] {
            let (clean, noisy, guides) = scene(0.3, measured);
            let mut filtered = noisy.clone();
            denoise(SIZE, SIZE, &mut filtered, &guides);
            let (before, after) = (error(&noisy, &clean), error(&filtered, &clean));
            assert!(
                after < 0.35 * before,
                "measured {measured}: error {before} only fell to {after}"
            );
            // The lighting step between columns 23 and 24 survives.
            let row = 20 * SIZE;
            let (lit, shaded) = (luminance(filtered[row + 20]), luminance(filtered[row + 27]));
            assert!(
                lit > 5.0 * shaded,
                "measured {measured}: lit {lit} shaded {shaded}"
            );
        }
    }

    #[test]
    fn a_converged_image_passes_through_unchanged() {
        let (clean, _, guides) = scene(0.0, true);
        let mut filtered = clean.clone();
        denoise(SIZE, SIZE, &mut filtered, &guides);
        assert!(error(&filtered, &clean) < 1.0e-4);
    }

    #[test]
    fn surfaces_with_different_normals_or_depths_do_not_mix() {
        let (_, mut image, mut guides) = scene(0.0, false);
        // A second surface on the right: darker, tilted, and far behind.
        for y in 0..SIZE {
            for x in SIZE / 2..SIZE {
                let index = y * SIZE + x;
                let guide = guides[index].as_mut().expect("every pixel is covered");
                guide.normal = Vec3::new(0.6, 0.0, 0.8);
                guide.position.z = -9.0;
            }
        }
        // Left surface noisy enough to be filtered hard.
        let mut rng = Rng::new(3, 3);
        for (index, color) in image.iter_mut().enumerate() {
            if index % SIZE < SIZE / 2 {
                *color *= 0.5 + rng.next();
            }
        }
        let before = image.clone();
        denoise(SIZE, SIZE, &mut image, &guides);
        let column = |image: &[Vec3], x: usize| {
            (0..SIZE)
                .map(|y| luminance(image[y * SIZE + x]))
                .sum::<f32>()
                / SIZE as f32
        };
        // The dark surface next to the edge keeps its level.
        assert!(
            (column(&image, SIZE / 2) - column(&before, SIZE / 2)).abs()
                < 0.02 * column(&before, SIZE / 2)
        );
    }

    #[test]
    fn pixels_without_a_surface_are_left_untouched() {
        let (_, mut image, mut guides) = scene(0.3, true);
        guides[5 * SIZE + 5] = None;
        image[5 * SIZE + 5] = Vec3::new(7.0, 8.0, 9.0);
        denoise(SIZE, SIZE, &mut image, &guides);
        assert_eq!(image[5 * SIZE + 5], Vec3::new(7.0, 8.0, 9.0));
    }
}
