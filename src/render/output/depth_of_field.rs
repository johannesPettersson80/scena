use crate::material::Color;
use crate::render::target::RasterTarget;

#[cfg(all(test, not(target_arch = "wasm32")))]
use super::pixel_offset;
use super::quantize_screen_space_depth;

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
    focal_length_mm: f32,
    sensor_height_mm: f32,
    aperture_f_stop: f32,
    aperture_blades: u8,
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
            focal_length_mm: 50.0,
            sensor_height_mm: 24.0,
            aperture_f_stop: if aperture_f_stop.is_finite() {
                aperture_f_stop.clamp(0.7, 32.0)
            } else {
                8.0
            },
            aperture_blades: 9,
            radius_px: radius_px.min(16),
        }
    }

    /// Creates a physically parameterized thin-lens depth-of-field model.
    pub fn physical(
        focus_distance: f32,
        focal_length_mm: f32,
        sensor_height_mm: f32,
        aperture_f_stop: f32,
        aperture_blades: u8,
        radius_px: u8,
    ) -> Self {
        let mut config = Self::new(focus_distance, aperture_f_stop, radius_px);
        config.focal_length_mm = finite_clamped(focal_length_mm, 50.0, 8.0, 600.0);
        config.sensor_height_mm = finite_clamped(sensor_height_mm, 24.0, 4.0, 100.0);
        config.aperture_blades = aperture_blades.clamp(3, 32);
        config
    }

    /// Returns the camera-space focus distance in scene units.
    pub const fn focus_distance(self) -> f32 {
        self.focus_distance
    }

    /// Returns the f-stop controlling blur strength.
    pub const fn aperture_f_stop(self) -> f32 {
        self.aperture_f_stop
    }

    pub const fn focal_length_mm(self) -> f32 {
        self.focal_length_mm
    }

    pub const fn sensor_height_mm(self) -> f32 {
        self.sensor_height_mm
    }

    pub const fn aperture_blades(self) -> u8 {
        self.aperture_blades
    }

    /// Returns the maximum blur radius in output pixels.
    pub const fn radius_px(self) -> u8 {
        self.radius_px
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct DepthOfFieldPostConfig {
    focus_depth: f32,
    focus_distance: f32,
    focal_length_mm: f32,
    sensor_height_mm: f32,
    aperture_f_stop: f32,
    near: f32,
    far: f32,
    reversed_z: bool,
    radius_px: u8,
}

impl DepthOfFieldPostConfig {
    pub(in crate::render) fn new(
        focus_depth: f32,
        config: DepthOfFieldConfig,
        near_far: [f32; 2],
        reversed_z: bool,
    ) -> Self {
        Self {
            focus_depth: if focus_depth.is_finite() {
                focus_depth.clamp(0.0, 1.0)
            } else {
                0.5
            },
            focus_distance: config.focus_distance(),
            focal_length_mm: config.focal_length_mm(),
            sensor_height_mm: config.sensor_height_mm(),
            aperture_f_stop: config.aperture_f_stop(),
            near: near_far[0],
            far: near_far[1],
            reversed_z,
            radius_px: config.radius_px(),
        }
    }

    pub(in crate::render) const fn focus_depth(self) -> f32 {
        self.focus_depth
    }

    pub(in crate::render) const fn radius_px(self) -> u8 {
        self.radius_px
    }

    pub(in crate::render) const fn physical_parameters(self) -> [f32; 4] {
        [
            self.focus_distance,
            self.focal_length_mm,
            self.sensor_height_mm,
            self.aperture_f_stop,
        ]
    }

    pub(in crate::render) const fn depth_parameters(self) -> [f32; 3] {
        [self.near, self.far, if self.reversed_z { 1.0 } else { 0.0 }]
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
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

pub(in crate::render) fn apply_depth_of_field_linear(
    target: RasterTarget,
    frame: &mut [Color],
    scratch: &mut [Color],
    depth_frame: &[f32],
    config: DepthOfFieldPostConfig,
) -> u64 {
    let max_radius = u32::from(config.radius_px());
    if target.width < 3 || target.height < 3 || max_radius == 0 {
        return 0;
    }
    scratch.copy_from_slice(frame);
    let mut changed = false;
    for y in 0..target.height {
        for x in 0..target.width {
            let index = target.pixel_index(x, y);
            let depth = quantize_screen_space_depth(depth_frame[index]);
            if !depth.is_finite() {
                continue;
            }
            let radius = dof_radius_px(depth, config);
            if radius == 0 {
                continue;
            }
            let mut sum = [0.0_f32; 4];
            let mut count = 0.0;
            for sy in y.saturating_sub(radius)..=y.saturating_add(radius).min(target.height - 1) {
                for sx in x.saturating_sub(radius)..=x.saturating_add(radius).min(target.width - 1)
                {
                    let sample = scratch[target.pixel_index(sx, sy)];
                    sum[0] += sample.r;
                    sum[1] += sample.g;
                    sum[2] += sample.b;
                    sum[3] += sample.a;
                    count += 1.0;
                }
            }
            frame[index] = Color::from_linear_rgba(
                sum[0] / count,
                sum[1] / count,
                sum[2] / count,
                sum[3] / count,
            );
            changed = true;
        }
    }
    u64::from(changed)
}

fn dof_radius_px(depth: f32, config: DepthOfFieldPostConfig) -> u32 {
    let depth = camera_distance_from_depth(depth, config);
    let focal_m = config.focal_length_mm * 0.001;
    let focus = config.focus_distance.max(focal_m + 1.0e-4);
    let image_focus = focal_m * focus / (focus - focal_m);
    let image_depth = focal_m * depth / (depth - focal_m).max(1.0e-4);
    let aperture_diameter = focal_m / config.aperture_f_stop;
    let coc_m = aperture_diameter * (image_depth - image_focus).abs() / image_depth.max(1.0e-5);
    let radius = coc_m / (config.sensor_height_mm * 0.001) * f32::from(config.radius_px()) * 12.0;
    radius.round().clamp(0.0, f32::from(config.radius_px())) as u32
}

fn camera_distance_from_depth(depth: f32, config: DepthOfFieldPostConfig) -> f32 {
    let depth = if config.reversed_z {
        1.0 - depth
    } else {
        depth
    }
    .clamp(0.0, 1.0);
    let denominator = config.far - depth * (config.far - config.near);
    (config.near * config.far / denominator.max(1.0e-6)).max(config.near)
}

fn finite_clamped(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::{DepthOfFieldConfig, DepthOfFieldPostConfig, dof_radius_px};

    #[test]
    fn physical_circle_of_confusion_is_sharp_at_focus_and_grows_with_defocus() {
        let near = 0.1;
        let far = 100.0;
        let encode = |distance: f32| (far - near * far / distance) / (far - near);
        let config = DepthOfFieldConfig::physical(5.0, 70.0, 24.0, 4.0, 9, 16);
        let post = DepthOfFieldPostConfig::new(encode(5.0), config, [near, far], false);

        assert_eq!(dof_radius_px(encode(5.0), post), 0);
        assert!(dof_radius_px(encode(2.0), post) > 0);
        assert!(
            dof_radius_px(encode(1.0), post) >= dof_radius_px(encode(2.0), post),
            "circle of confusion must grow as a foreground surface moves farther from focus"
        );
    }
}
