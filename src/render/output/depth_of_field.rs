use crate::render::target::RasterTarget;

use super::{pixel_offset, quantize_screen_space_depth};

/// Depth-of-field post-process settings.
///
/// The focus distance is measured from the active camera in scene units. Lower
/// `aperture_f_stop` values and larger `radius_px` values produce a stronger
/// blur away from the focus plane. Recipe validation rejects invalid values;
/// this Rust API clamps to the renderer-supported range so host applications
/// can keep interactive controls bounded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthOfFieldConfig {
    focus_distance: f32,
    aperture_f_stop: f32,
    radius_px: u8,
}

impl DepthOfFieldConfig {
    /// Creates a depth-of-field configuration.
    ///
    /// Supported ranges are `focus_distance >= 0.001`,
    /// `0.7 <= aperture_f_stop <= 32.0`, and `radius_px <= 16`.
    pub fn new(focus_distance: f32, aperture_f_stop: f32, radius_px: u8) -> Self {
        Self {
            focus_distance: if focus_distance.is_finite() {
                focus_distance.max(0.001)
            } else {
                1.0
            },
            aperture_f_stop: if aperture_f_stop.is_finite() {
                aperture_f_stop.clamp(0.7, 32.0)
            } else {
                8.0
            },
            radius_px: radius_px.min(16),
        }
    }

    /// Returns the camera-space focus distance in scene units.
    pub const fn focus_distance(self) -> f32 {
        self.focus_distance
    }

    /// Returns the f-stop controlling blur strength.
    pub const fn aperture_f_stop(self) -> f32 {
        self.aperture_f_stop
    }

    /// Returns the maximum blur radius in output pixels.
    pub const fn radius_px(self) -> u8 {
        self.radius_px
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct DepthOfFieldPostConfig {
    focus_depth: f32,
    aperture_f_stop: f32,
    radius_px: u8,
}

impl DepthOfFieldPostConfig {
    pub(in crate::render) fn new(focus_depth: f32, aperture_f_stop: f32, radius_px: u8) -> Self {
        Self {
            focus_depth: if focus_depth.is_finite() {
                focus_depth.clamp(0.0, 1.0)
            } else {
                0.5
            },
            aperture_f_stop: if aperture_f_stop.is_finite() {
                aperture_f_stop.clamp(0.7, 32.0)
            } else {
                8.0
            },
            radius_px: radius_px.min(16),
        }
    }

    pub(in crate::render) const fn focus_depth(self) -> f32 {
        self.focus_depth
    }

    pub(in crate::render) const fn aperture_f_stop(self) -> f32 {
        self.aperture_f_stop
    }

    pub(in crate::render) const fn radius_px(self) -> u8 {
        self.radius_px
    }
}

pub(in crate::render) fn apply_depth_of_field_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    scratch: &mut [u8],
    depth_frame: &[f32],
    config: DepthOfFieldPostConfig,
) -> u64 {
    let max_radius = u32::from(config.radius_px());
    if target.width < 3 || target.height < 3 || max_radius == 0 {
        return 0;
    }
    debug_assert_eq!(frame.len(), target.byte_len());
    debug_assert_eq!(scratch.len(), target.byte_len());
    debug_assert_eq!(depth_frame.len(), target.pixel_len());

    scratch.copy_from_slice(frame);
    let mut changed = false;
    for y in 0..target.height {
        for x in 0..target.width {
            let pixel_index = target.pixel_index(x, y);
            let center_depth = quantize_screen_space_depth(depth_frame[pixel_index]);
            if !center_depth.is_finite() {
                continue;
            }
            let radius = dof_radius_px(center_depth, config);
            if radius == 0 {
                continue;
            }
            let min_x = x.saturating_sub(radius);
            let max_x = x.saturating_add(radius).min(target.width - 1);
            let min_y = y.saturating_sub(radius);
            let max_y = y.saturating_add(radius).min(target.height - 1);
            let mut sum = [0_u32; 4];
            let mut sample_count = 0_u32;
            for sample_y in min_y..=max_y {
                for sample_x in min_x..=max_x {
                    let sample = pixel_offset(target, sample_x, sample_y);
                    sum[0] = sum[0].saturating_add(u32::from(scratch[sample]));
                    sum[1] = sum[1].saturating_add(u32::from(scratch[sample + 1]));
                    sum[2] = sum[2].saturating_add(u32::from(scratch[sample + 2]));
                    sum[3] = sum[3].saturating_add(u32::from(scratch[sample + 3]));
                    sample_count = sample_count.saturating_add(1);
                }
            }
            if sample_count == 0 {
                continue;
            }
            let output = pixel_offset(target, x, y);
            for channel in 0..4 {
                frame[output + channel] = (sum[channel] / sample_count).min(255) as u8;
            }
            changed = true;
        }
    }

    u64::from(changed)
}

fn dof_radius_px(depth: f32, config: DepthOfFieldPostConfig) -> u32 {
    let focus_delta = (depth - config.focus_depth()).abs();
    let aperture_scale = (8.0 / config.aperture_f_stop()).clamp(0.0, 8.0);
    let radius = focus_delta * aperture_scale * f32::from(config.radius_px()) * 32.0;
    radius.round().clamp(0.0, f32::from(config.radius_px())) as u32
}
