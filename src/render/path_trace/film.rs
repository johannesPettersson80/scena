//! The film: gathers camera paths pixel by pixel, then develops the sums
//! into the linear image and the display-ready one.

use std::sync::atomic::{AtomicU32, Ordering};

use glam::Vec3;

use crate::material::Color;

use super::super::color_contract::linear_channel_to_srgb;
use super::super::output::OutputTransform;
use super::super::parallel::worker_count;
use super::camera::TraceCamera;
use super::denoise::{Guide, denoise};
use super::integrator::Tracer;
use super::options::PathTraceOptions;
use super::sampling::{Rng, luminance};

/// Called after each finished row with the rows done and the total.
pub(super) type Progress<'a> = &'a (dyn Fn(u32, u32) + Sync);

/// Sums over the camera paths of one pixel that met a surface.
#[derive(Clone, Copy, Default)]
struct PixelSum {
    radiance: Vec3,
    luminance: f32,
    luminance_squared: f32,
    albedo: Vec3,
    normal: Vec3,
    position: Vec3,
    hits: u32,
}

pub(super) struct Film {
    width: usize,
    height: usize,
    samples: u32,
    pixels: Vec<PixelSum>,
}

/// Runs `row` for every image row and returns the results in row order.
///
/// Rows are dealt out to the renderer's bounded worker pool in turn, so
/// every worker gets an even share of the cheap and the costly parts of the
/// image. The result does not depend on the number of workers.
pub(super) fn for_rows<T: Send>(height: usize, row: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let workers = worker_count(height);
    #[cfg(not(target_arch = "wasm32"))]
    if workers > 1 {
        use rayon::prelude::*;
        let row = &row;
        let mut rows: Vec<(usize, T)> = (0..workers)
            .into_par_iter()
            .flat_map_iter(move |worker| {
                (worker..height).step_by(workers).map(move |y| (y, row(y)))
            })
            .collect();
        rows.sort_unstable_by_key(|(y, _)| *y);
        return rows.into_iter().map(|(_, row)| row).collect();
    }
    let _ = workers;
    (0..height).map(row).collect()
}

/// Offset in pixels drawn from a tent of radius one, the reconstruction
/// filter each pixel's paths are spread over.
fn tent(u: f32) -> f32 {
    let doubled = 2.0 * u;
    if doubled < 1.0 {
        doubled.sqrt() - 1.0
    } else {
        1.0 - (2.0 - doubled).sqrt()
    }
}

/// Traces every pixel. `clamp` bounds the scattered light of one path.
pub(super) fn expose(
    tracer: &Tracer<'_>,
    camera: &TraceCamera,
    (width, height): (u32, u32),
    options: &PathTraceOptions,
    clamp: f32,
    progress: Progress<'_>,
) -> Film {
    let (width, height) = (width as usize, height as usize);
    let samples = options.samples_per_pixel();
    let finished = AtomicU32::new(0);
    let rows = for_rows(height, |y| {
        let row: Vec<PixelSum> = (0..width)
            .map(|x| {
                // One stream per pixel keeps the image independent of the
                // order rows are traced in.
                let mut rng = Rng::new(options.seed(), (y * width + x) as u64);
                let mut sum = PixelSum::default();
                for _ in 0..samples {
                    let film = (
                        (x as f32 + 0.5 + tent(rng.next())) / width as f32,
                        (y as f32 + 0.5 + tent(rng.next())) / height as f32,
                    );
                    let path = tracer.trace(camera.ray(film, rng.pair()), &mut rng);
                    let Some(first) = path.first_hit else {
                        continue;
                    };
                    let mut scattered = path.scattered;
                    let brightest = scattered.max_element();
                    if brightest > clamp {
                        scattered *= clamp / brightest;
                    }
                    let radiance = path.emitted + scattered;
                    if !radiance.is_finite() {
                        continue;
                    }
                    let brightness = luminance(radiance);
                    sum.radiance += radiance;
                    sum.luminance += brightness;
                    sum.luminance_squared += brightness * brightness;
                    sum.albedo += first.albedo;
                    sum.normal += first.normal;
                    sum.position += first.position;
                    sum.hits += 1;
                }
                sum
            })
            .collect();
        progress(finished.fetch_add(1, Ordering::Relaxed) + 1, height as u32);
        row
    });
    Film {
        width,
        height,
        samples,
        pixels: rows.into_iter().flatten().collect(),
    }
}

const BAYER: [f32; 16] = [
    0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
];

impl Film {
    /// Mean radiance of the surfaces in each pixel, and what the denoiser
    /// may know about them.
    fn surfaces(&self) -> (Vec<Vec3>, Vec<Option<Guide>>) {
        self.pixels
            .iter()
            .map(|pixel| {
                if pixel.hits == 0 {
                    return (Vec3::ZERO, None);
                }
                let count = pixel.hits as f32;
                let mean = pixel.luminance / count;
                // Variance of the mean, from the spread of the pixel's paths.
                let variance = (pixel.hits >= 4).then(|| {
                    (pixel.luminance_squared / count - mean * mean).max(0.0) / (count - 1.0)
                });
                (
                    pixel.radiance / count,
                    Some(Guide {
                        albedo: pixel.albedo / count,
                        normal: pixel.normal.normalize_or_zero(),
                        position: pixel.position / count,
                        variance,
                    }),
                )
            })
            .unzip()
    }

    /// Produces the linear image and the 8-bit display image.
    ///
    /// Surfaces pass through the renderer's exposure, white balance and tone
    /// mapping; the background is a display colour and is blended in after,
    /// as the rasterizer clears to it.
    pub(super) fn develop(
        self,
        filter: bool,
        output: OutputTransform,
        background: Color,
    ) -> (Vec<[f32; 4]>, Vec<u8>) {
        let (mut surfaces, guides) = self.surfaces();
        if filter {
            denoise(self.width, self.height, &mut surfaces, &guides);
        }
        let linear: Vec<[f32; 4]> = self
            .pixels
            .iter()
            .zip(&surfaces)
            .map(|(pixel, surface)| {
                let coverage = pixel.hits as f32 / self.samples as f32;
                let surface = surface.max(Vec3::ZERO) * coverage;
                [surface.x, surface.y, surface.z, coverage]
            })
            .collect();
        let rgba8 = encode(&linear, self.width, output, background);
        (linear, rgba8)
    }
}

/// Turns linear radiance, weighted by coverage, into the 8-bit display image.
pub(super) fn encode(
    linear: &[[f32; 4]],
    width: usize,
    output: OutputTransform,
    background: Color,
) -> Vec<u8> {
    let backdrop = Vec3::new(background.r, background.g, background.b) * background.a;
    let mut rgba8 = Vec::with_capacity(linear.len() * 4);
    for (index, pixel) in linear.iter().enumerate() {
        let coverage = pixel[3];
        let surface = if coverage > 0.0 {
            Vec3::new(pixel[0], pixel[1], pixel[2]) / coverage
        } else {
            Vec3::ZERO
        };
        let display = output.post_color(Color::from_linear_rgba(
            surface.x, surface.y, surface.z, 1.0,
        ));
        let alpha = coverage + background.a * (1.0 - coverage);
        let blended =
            Vec3::new(display.r, display.g, display.b) * coverage + backdrop * (1.0 - coverage);
        let straight = if alpha > 0.0 {
            blended / alpha
        } else {
            Vec3::ZERO
        };
        // Ordered dither hides the banding 8 bits leave in soft gradients.
        // Bare background keeps its exact colour.
        let (x, y) = (index % width, index / width);
        let dither = if coverage > 0.0 {
            (BAYER[(y % 4) * 4 + x % 4] / 16.0 - 0.5) / 255.0
        } else {
            0.0
        };
        let encode = |channel: f32| {
            ((linear_channel_to_srgb(channel) + dither).clamp(0.0, 1.0) * 255.0).round() as u8
        };
        rgba8.extend([
            encode(straight.x),
            encode(straight.y),
            encode(straight.z),
            (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]);
    }
    rgba8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_come_back_in_order_whatever_the_number_of_workers() {
        let expected: Vec<usize> = (0..37).map(|row| row * row).collect();
        assert_eq!(for_rows(37, |row| row * row), expected);
        // Inside a worker the same call runs serially and must agree.
        let single = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("pool builds");
        assert_eq!(single.install(|| for_rows(37, |row| row * row)), expected);
        assert!(for_rows(0, |row| row).is_empty());
    }

    #[test]
    fn the_pixel_filter_is_a_tent_of_radius_one() {
        let offsets: Vec<f32> = (0..4_000)
            .map(|index| tent((index as f32 + 0.5) / 4_000.0))
            .collect();
        assert!(offsets.iter().all(|offset| (-1.0..=1.0).contains(offset)));
        let mean = offsets.iter().sum::<f32>() / offsets.len() as f32;
        // A tent has variance 1/6; a box filter over the pixel would have 1/12.
        let variance =
            offsets.iter().map(|offset| offset * offset).sum::<f32>() / offsets.len() as f32;
        assert!(
            mean.abs() < 1.0e-3 && (variance - 1.0 / 6.0).abs() < 2.0e-3,
            "mean {mean} variance {variance}"
        );
    }
}
