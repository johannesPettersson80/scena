use crate::material::Color;

use super::RasterTarget;
use super::color_contract::{
    aces_tonemap, apply_exposure, linear_channel_to_srgb, linear_rgba_to_srgb8, pbr_neutral_tonemap,
};

mod depth_of_field;
mod legacy_ldr;

pub use depth_of_field::DepthOfFieldConfig;
#[cfg(test)]
pub(in crate::render) use depth_of_field::apply_depth_of_field_rgba8;
pub(in crate::render) use depth_of_field::{DepthOfFieldPostConfig, apply_depth_of_field_linear};
pub(super) use legacy_ldr::apply_fxaa_rgba8;
#[cfg(test)]
use legacy_ldr::{apply_bloom_rgba8_profiled, luma_from_srgb8, pixel_offset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct OutputTransform {
    exposure_ev: f32,
    tonemapper: Tonemapper,
    white_balance: WhiteBalance,
}

impl OutputTransform {
    pub(super) fn post_color(self, color: Color) -> Color {
        let color = self.white_balance.apply(color);
        match self.tonemapper {
            Tonemapper::Aces => aces_tonemap(color, self.exposure_ev),
            Tonemapper::PbrNeutral => pbr_neutral_tonemap(color, self.exposure_ev),
            Tonemapper::Standard => apply_exposure(color, self.exposure_ev),
        }
    }

    pub(super) fn encode_rgba8(self, color: Color) -> [u8; 4] {
        linear_rgba_to_srgb8(self.post_color(color))
    }

    pub(super) fn encode_rgba8_dithered(self, color: Color, x: u32, y: u32) -> [u8; 4] {
        let color = self.post_color(color);
        let bayer = [
            0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
        ];
        let offset = (bayer[((y % 4) * 4 + x % 4) as usize] / 16.0 - 0.5) / 255.0;
        let encode = |channel: f32| {
            ((linear_channel_to_srgb(channel) + offset).clamp(0.0, 1.0) * 255.0).round() as u8
        };
        [
            encode(color.r),
            encode(color.g),
            encode(color.b),
            (color.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    pub(super) fn encode_clear_rgba8(self, color: Color) -> [u8; 4] {
        linear_rgba_to_srgb8(color)
    }

    pub(super) const fn exposure_ev(self) -> f32 {
        self.exposure_ev
    }

    pub(super) fn set_exposure_ev(&mut self, exposure_ev: f32) {
        self.exposure_ev = if exposure_ev.is_finite() {
            exposure_ev
        } else {
            0.0
        };
    }

    pub(super) const fn tonemapper(self) -> Tonemapper {
        self.tonemapper
    }

    pub(super) const fn set_tonemapper(&mut self, tonemapper: Tonemapper) {
        self.tonemapper = tonemapper;
    }

    pub(super) const fn color_management_uniform(self) -> [f32; 4] {
        match self.tonemapper {
            Tonemapper::Standard => [0.0, 0.0, 0.0, 0.0],
            Tonemapper::Aces => [1.0, 0.0, 0.0, 0.0],
            Tonemapper::PbrNeutral => [2.0, 0.0, 0.0, 0.0],
        }
    }

    pub(super) const fn white_balance_uniform(self) -> [f32; 4] {
        let [red, green, blue] = self.white_balance.linear_multipliers();
        [red, green, blue, 0.0]
    }

    pub(super) const fn white_balance(self) -> WhiteBalance {
        self.white_balance
    }

    pub(super) const fn set_white_balance(&mut self, white_balance: WhiteBalance) {
        self.white_balance = white_balance;
    }
}

impl Default for OutputTransform {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            tonemapper: Tonemapper::PbrNeutral,
            white_balance: WhiteBalance::neutral(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhiteBalance {
    illuminant_kelvin: f32,
    tint: f32,
    linear_multipliers: [f32; 3],
}

impl WhiteBalance {
    pub const fn neutral() -> Self {
        Self {
            illuminant_kelvin: 6_500.0,
            tint: 0.0,
            linear_multipliers: [1.0, 1.0, 1.0],
        }
    }

    pub fn from_illuminant_kelvin(illuminant_kelvin: f32) -> Self {
        Self::from_illuminant_kelvin_with_tint(illuminant_kelvin, 0.0)
    }

    pub fn from_illuminant_kelvin_with_tint(illuminant_kelvin: f32, tint: f32) -> Self {
        let illuminant_kelvin = if illuminant_kelvin.is_finite() {
            illuminant_kelvin.clamp(1_000.0, 20_000.0)
        } else {
            6_500.0
        };
        let tint = if tint.is_finite() {
            tint.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        let reference = Color::from_kelvin(6_500.0);
        let illuminant = Color::from_kelvin(illuminant_kelvin);
        let tint_green = 2.0_f32.powf(-tint * 0.25);
        let mut multipliers = [
            safe_channel_ratio(reference.r, illuminant.r),
            safe_channel_ratio(reference.g, illuminant.g) * tint_green,
            safe_channel_ratio(reference.b, illuminant.b),
        ];
        let normalization = multipliers[1].max(1.0e-4);
        for channel in &mut multipliers {
            *channel = (*channel / normalization).clamp(0.25, 4.0);
        }
        Self {
            illuminant_kelvin,
            tint,
            linear_multipliers: multipliers,
        }
    }

    pub const fn illuminant_kelvin(self) -> f32 {
        self.illuminant_kelvin
    }

    pub const fn tint(self) -> f32 {
        self.tint
    }

    pub const fn linear_multipliers(self) -> [f32; 3] {
        self.linear_multipliers
    }

    fn apply(self, color: Color) -> Color {
        Color::from_linear_rgba(
            color.r * self.linear_multipliers[0],
            color.g * self.linear_multipliers[1],
            color.b * self.linear_multipliers[2],
            color.a,
        )
    }
}

impl Default for WhiteBalance {
    fn default() -> Self {
        Self::neutral()
    }
}

fn safe_channel_ratio(reference: f32, illuminant: f32) -> f32 {
    if reference.is_finite() && illuminant.is_finite() && illuminant > 1.0e-4 {
        reference / illuminant
    } else {
        1.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tonemapper {
    Aces,
    Standard,
    #[default]
    PbrNeutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AntiAliasing {
    None,
    #[default]
    Fxaa,
    Msaa4,
    Msaa8,
}

impl AntiAliasing {
    pub const fn gpu_sample_count(self) -> u32 {
        match self {
            Self::None | Self::Fxaa => 1,
            Self::Msaa4 => 4,
            Self::Msaa8 => 8,
        }
    }

    pub const fn cpu_supersample_scale(self) -> u32 {
        match self {
            Self::None | Self::Fxaa => 1,
            Self::Msaa4 => 2,
            Self::Msaa8 => 3,
        }
    }

    pub const fn uses_post_fxaa(self) -> bool {
        matches!(self, Self::Fxaa)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReconstructionFilter {
    #[default]
    Box,
    Tent,
    Gaussian,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderIndependentTransparencyConfig {
    coverage_boost: f32,
}

impl OrderIndependentTransparencyConfig {
    pub const fn weighted_blended() -> Self {
        Self {
            coverage_boost: 1.0,
        }
    }

    pub fn new(coverage_boost: f32) -> Self {
        Self {
            coverage_boost: if coverage_boost.is_finite() {
                coverage_boost.clamp(0.25, 4.0)
            } else {
                1.0
            },
        }
    }

    pub const fn coverage_boost(self) -> f32 {
        self.coverage_boost
    }
}

impl Default for OrderIndependentTransparencyConfig {
    fn default() -> Self {
        Self::weighted_blended()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostBloomConfig {
    threshold_srgb: u8,
    intensity: f32,
    radius_px: u8,
}

impl PostBloomConfig {
    pub const fn subtle() -> Self {
        Self {
            threshold_srgb: 208,
            intensity: 0.28,
            radius_px: 3,
        }
    }

    pub fn new(threshold_srgb: u8, intensity: f32, radius_px: u8) -> Self {
        Self {
            threshold_srgb,
            intensity: if intensity.is_finite() {
                intensity.clamp(0.0, 1.0)
            } else {
                0.0
            },
            radius_px: radius_px.min(12),
        }
    }

    pub const fn threshold_srgb(self) -> u8 {
        self.threshold_srgb
    }

    pub const fn intensity(self) -> f32 {
        self.intensity
    }

    pub const fn radius_px(self) -> u8 {
        self.radius_px
    }
}

impl Default for PostBloomConfig {
    fn default() -> Self {
        Self::subtle()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSpaceAmbientOcclusionConfig {
    radius_px: u8,
    intensity: f32,
    depth_threshold: f32,
}

impl ScreenSpaceAmbientOcclusionConfig {
    pub const fn subtle() -> Self {
        Self {
            radius_px: 3,
            intensity: 0.45,
            depth_threshold: 0.025,
        }
    }

    pub fn new(radius_px: u8, intensity: f32, depth_threshold: f32) -> Self {
        Self {
            radius_px: radius_px.min(12),
            intensity: if intensity.is_finite() {
                intensity.clamp(0.0, 1.0)
            } else {
                0.0
            },
            depth_threshold: if depth_threshold.is_finite() {
                depth_threshold.max(0.0)
            } else {
                0.0
            },
        }
    }

    pub const fn radius_px(self) -> u8 {
        self.radius_px
    }

    pub const fn intensity(self) -> f32 {
        self.intensity
    }

    pub const fn depth_threshold(self) -> f32 {
        self.depth_threshold
    }
}

impl Default for ScreenSpaceAmbientOcclusionConfig {
    fn default() -> Self {
        Self::subtle()
    }
}

fn quantize_screen_space_depth(depth: f32) -> f32 {
    if !depth.is_finite() {
        return depth;
    }
    (depth.clamp(0.0, 1.0) * 65_535.0).round() / 65_535.0
}

/// Applies bloom to scene-linear HDR radiance.
///
/// `threshold_srgb` remains the public UI control, but is converted once to a
/// linear luminance threshold. The working buffers and additive composite
/// never clamp to display white, so genuine highlights retain their energy
/// until the final output transform.
pub(super) fn apply_bloom_linear(
    target: RasterTarget,
    frame: &mut [Color],
    threshold_scratch: &mut [Color],
    horizontal_scratch: &mut [Color],
    config: PostBloomConfig,
) -> u64 {
    let radius = u32::from(config.radius_px());
    let intensity = config.intensity().clamp(0.0, 1.0);
    if target.width < 3 || target.height < 3 || radius == 0 || intensity <= 0.0 {
        return 0;
    }
    debug_assert_eq!(frame.len(), target.pixel_len());
    debug_assert_eq!(threshold_scratch.len(), target.pixel_len());
    debug_assert_eq!(horizontal_scratch.len(), target.pixel_len());
    threshold_scratch.fill(Color::BLACK);
    horizontal_scratch.fill(Color::BLACK);
    let encoded = f32::from(config.threshold_srgb()) / 255.0;
    let threshold = if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    };

    for (output, source) in threshold_scratch.iter_mut().zip(frame.iter().copied()) {
        let luminance = source.r * 0.2126 + source.g * 0.7152 + source.b * 0.0722;
        if luminance >= threshold {
            *output = source;
        }
    }
    for y in 0..target.height {
        for x in 0..target.width {
            let min_x = x.saturating_sub(radius);
            let max_x = x.saturating_add(radius).min(target.width - 1);
            let mut sum = [0.0_f32; 4];
            let count = (max_x - min_x + 1) as f32;
            for sample_x in min_x..=max_x {
                let sample = threshold_scratch[target.pixel_index(sample_x, y)];
                sum[0] += sample.r;
                sum[1] += sample.g;
                sum[2] += sample.b;
                sum[3] += sample.a;
            }
            horizontal_scratch[target.pixel_index(x, y)] = Color::from_linear_rgba(
                sum[0] / count,
                sum[1] / count,
                sum[2] / count,
                sum[3] / count,
            );
        }
    }
    for y in 0..target.height {
        for x in 0..target.width {
            let min_y = y.saturating_sub(radius);
            let max_y = y.saturating_add(radius).min(target.height - 1);
            let mut sum = [0.0_f32; 3];
            let count = (max_y - min_y + 1) as f32;
            for sample_y in min_y..=max_y {
                let sample = horizontal_scratch[target.pixel_index(x, sample_y)];
                sum[0] += sample.r;
                sum[1] += sample.g;
                sum[2] += sample.b;
            }
            let index = target.pixel_index(x, y);
            let base = frame[index];
            frame[index] = Color::from_linear_rgba(
                base.r + sum[0] / count * intensity,
                base.g + sum[1] / count * intensity,
                base.b + sum[2] / count * intensity,
                base.a,
            );
        }
    }
    1
}

pub(super) fn apply_screen_space_ambient_occlusion_linear(
    target: RasterTarget,
    frame: &mut [Color],
    scratch: &mut [Color],
    depth_frame: &[f32],
    config: ScreenSpaceAmbientOcclusionConfig,
) -> u64 {
    let radius = u32::from(config.radius_px());
    let intensity = config.intensity().clamp(0.0, 1.0);
    if target.width < 3 || target.height < 3 || radius == 0 || intensity <= 0.0 {
        return 0;
    }
    scratch.fill(Color::BLACK);
    let threshold = config.depth_threshold().max(0.0);
    for y in 0..target.height {
        for x in 0..target.width {
            let index = target.pixel_index(x, y);
            let center = quantize_screen_space_depth(depth_frame[index]);
            if !center.is_finite() {
                continue;
            }
            let near = (radius / 2).max(1);
            let offsets = [
                (-(near as i32), 0),
                (near as i32, 0),
                (0, -(near as i32)),
                (0, near as i32),
                (-(radius as i32), 0),
                (radius as i32, 0),
                (0, -(radius as i32)),
                (0, radius as i32),
            ];
            let mut finite = 0_u32;
            let mut occluders = 0_u32;
            for (dx, dy) in offsets {
                let sx = x as i32 + dx;
                let sy = y as i32 + dy;
                if sx < 0 || sy < 0 || sx >= target.width as i32 || sy >= target.height as i32 {
                    continue;
                }
                let sample = quantize_screen_space_depth(
                    depth_frame[target.pixel_index(sx as u32, sy as u32)],
                );
                if sample.is_finite() {
                    finite += 1;
                    occluders += u32::from(sample + threshold < center);
                }
            }
            if finite > 0 {
                let darkening = (occluders as f32 / finite as f32 * intensity).clamp(0.0, 0.65);
                scratch[index] = Color::from_linear_rgb(darkening, darkening, darkening);
            }
        }
    }
    let source = frame.to_vec();
    for y in 0..target.height {
        for x in 0..target.width {
            let index = target.pixel_index(x, y);
            if !depth_frame[index].is_finite() {
                continue;
            }
            let mut sum = 0.0;
            let mut count = 0.0;
            for sy in y.saturating_sub(1)..=y.saturating_add(1).min(target.height - 1) {
                for sx in x.saturating_sub(1)..=x.saturating_add(1).min(target.width - 1) {
                    let sample = target.pixel_index(sx, sy);
                    if depth_frame[sample].is_finite() {
                        sum += scratch[sample].r;
                        count += 1.0;
                    }
                }
            }
            let factor = 1.0 - if count > 0.0 { sum / count } else { 0.0 };
            let base = source[index];
            frame[index] =
                Color::from_linear_rgba(base.r * factor, base.g * factor, base.b * factor, base.a);
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;

    #[test]
    fn separable_bloom_is_repeatable_at_edges_and_has_linear_radius_work() {
        let target = RasterTarget {
            width: 32,
            height: 24,
            backend: Backend::Headless,
        };
        let mut source = vec![0_u8; target.byte_len()];
        for pixel in source.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        for (x, y, rgb) in [(0, 0, [255, 240, 220]), (16, 12, [255, 255, 255])] {
            let offset = pixel_offset(target, x, y);
            source[offset..offset + 3].copy_from_slice(&rgb);
        }
        let config = PostBloomConfig::new(128, 0.75, 12);
        let mut first = source.clone();
        let mut threshold = vec![0; target.byte_len()];
        let mut horizontal = vec![0; target.byte_len()];
        let work =
            apply_bloom_rgba8_profiled(target, &mut first, &mut threshold, &mut horizontal, config);
        let mut second = source;
        let repeat_work = apply_bloom_rgba8_profiled(
            target,
            &mut second,
            &mut threshold,
            &mut horizontal,
            config,
        );

        assert_eq!(first, second, "bloom output must be repeatable");
        assert_eq!(work, repeat_work, "bloom work must be deterministic");
        assert_eq!(work.passes, 1);
        assert!(
            first[pixel_offset(target, 1, 0)] > 0,
            "an edge highlight must bloom inward without out-of-bounds sampling",
        );
        let pixels = target.pixel_len() as u64;
        let kernel_width = u64::from(config.radius_px()) * 2 + 1;
        assert!(
            work.blur_sample_reads <= pixels * kernel_width * 2,
            "separable bloom work must grow linearly with radius: {work:?}",
        );
        assert!(
            work.blur_sample_reads < pixels * kernel_width * kernel_width,
            "maximum-radius bloom must do less work than the old 2D kernel: {work:?}",
        );
    }

    #[test]
    fn separable_bloom_matches_legacy_box_contract_across_public_controls() {
        let target = RasterTarget {
            width: 40,
            height: 32,
            backend: Backend::Headless,
        };
        let mut source = vec![0_u8; target.byte_len()];
        for pixel in source.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        for y in 12..20 {
            for x in 15..25 {
                let offset = pixel_offset(target, x, y);
                source[offset..offset + 3].copy_from_slice(&[240, 224, 208]);
            }
        }
        for (x, y, rgb) in [
            (0, 0, [255, 255, 255]),
            (39, 31, [255, 210, 180]),
            (20, 16, [255, 255, 255]),
        ] {
            let offset = pixel_offset(target, x, y);
            source[offset..offset + 3].copy_from_slice(&rgb);
        }

        let mut reports = Vec::new();
        for (threshold, intensity, radius) in
            [(0, 0.25, 1), (128, 0.75, 4), (208, 0.28, 3), (250, 1.0, 12)]
        {
            let config = PostBloomConfig::new(threshold, intensity, radius);
            let mut actual = source.clone();
            let mut threshold_scratch = vec![0; target.byte_len()];
            let mut horizontal_scratch = vec![0; target.byte_len()];
            let work = apply_bloom_rgba8_profiled(
                target,
                &mut actual,
                &mut threshold_scratch,
                &mut horizontal_scratch,
                config,
            );
            let expected = legacy_box_bloom_reference(target, &source, config);
            let metrics = bloom_diff_metrics(&actual, &expected, target.width);

            assert!(
                metrics.max_channel_delta <= 1,
                "separable rounding must stay within one byte of the legacy 2D box contract: {metrics:?} config={config:?}",
            );
            assert!(
                metrics.rmse <= 0.35,
                "separable bloom must preserve the full-frame legacy contract: {metrics:?} config={config:?}",
            );
            assert!(
                metrics.ssim >= 0.999_99,
                "separable bloom must preserve legacy structure: {metrics:?} config={config:?}",
            );
            assert!(
                work.blur_sample_reads
                    <= target.pixel_len() as u64 * (u64::from(radius) * 2 + 1) * 2,
                "separable work must remain linear in radius: {work:?}",
            );
            reports.push(serde_json::json!({
                "threshold_srgb": threshold,
                "intensity": intensity,
                "radius_px": radius,
                "max_channel_delta": metrics.max_channel_delta,
                "rmse": metrics.rmse,
                "ssim": metrics.ssim,
                "worst_region_bbox": metrics.worst_region_bbox,
                "blur_sample_reads": work.blur_sample_reads,
            }));
        }

        let artifact_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/gate-artifacts/p05-cpu-bloom");
        std::fs::create_dir_all(&artifact_dir).expect("P05 artifact directory creates");
        std::fs::write(
            artifact_dir.join("comparison.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema": "scena.p05.cpu_bloom_comparison.v1",
                "status": "passed",
                "reference": "legacy_nonseparable_box_kernel",
                "kernel_contract": "implementation_detail_with_one_lsb_rounding_tolerance",
                "profiles": reports,
            }))
            .expect("P05 comparison serializes"),
        )
        .expect("P05 comparison writes");
    }

    fn legacy_box_bloom_reference(
        target: RasterTarget,
        source: &[u8],
        config: PostBloomConfig,
    ) -> Vec<u8> {
        let radius = u32::from(config.radius_px());
        let intensity = config.intensity().clamp(0.0, 1.0);
        let mut output = source.to_vec();
        if target.width < 3 || target.height < 3 || radius == 0 || intensity <= 0.0 {
            return output;
        }
        let mut threshold = vec![0_u8; target.byte_len()];
        for y in 0..target.height {
            for x in 0..target.width {
                let offset = pixel_offset(target, x, y);
                if luma_from_srgb8(&source[offset..offset + 4])
                    >= f32::from(config.threshold_srgb())
                {
                    threshold[offset..offset + 3].copy_from_slice(&source[offset..offset + 3]);
                }
            }
        }
        for y in 0..target.height {
            for x in 0..target.width {
                let min_x = x.saturating_sub(radius);
                let max_x = x.saturating_add(radius).min(target.width - 1);
                let min_y = y.saturating_sub(radius);
                let max_y = y.saturating_add(radius).min(target.height - 1);
                let mut sum = [0_u32; 3];
                let mut count = 0_u32;
                for sample_y in min_y..=max_y {
                    for sample_x in min_x..=max_x {
                        let sample = pixel_offset(target, sample_x, sample_y);
                        for channel in 0..3 {
                            sum[channel] += u32::from(threshold[sample + channel]);
                        }
                        count += 1;
                    }
                }
                let offset = pixel_offset(target, x, y);
                for channel in 0..3 {
                    let bloom = sum[channel] as f32 / count as f32 * intensity;
                    output[offset + channel] = (f32::from(output[offset + channel]) + bloom)
                        .round()
                        .min(255.0) as u8;
                }
            }
        }
        output
    }

    #[derive(Debug)]
    struct BloomDiffMetrics {
        max_channel_delta: u8,
        rmse: f64,
        ssim: f64,
        worst_region_bbox: Option<[u32; 4]>,
    }

    fn bloom_diff_metrics(actual: &[u8], expected: &[u8], width: u32) -> BloomDiffMetrics {
        let mut max_channel_delta = 0_u8;
        let mut squared_error = 0.0_f64;
        let mut sample_count = 0_u64;
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0_u32;
        let mut max_y = 0_u32;
        let mut actual_luma = Vec::with_capacity(actual.len() / 4);
        let mut expected_luma = Vec::with_capacity(expected.len() / 4);
        for (pixel_index, (left, right)) in actual
            .chunks_exact(4)
            .zip(expected.chunks_exact(4))
            .enumerate()
        {
            actual_luma.push(luma_from_srgb8(left) as f64);
            expected_luma.push(luma_from_srgb8(right) as f64);
            let mut differs = false;
            for channel in 0..3 {
                let delta = left[channel].abs_diff(right[channel]);
                max_channel_delta = max_channel_delta.max(delta);
                squared_error += f64::from(delta).powi(2);
                sample_count += 1;
                differs |= delta != 0;
            }
            if differs {
                let x = pixel_index as u32 % width;
                let y = pixel_index as u32 / width;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        let rmse = (squared_error / sample_count.max(1) as f64).sqrt();
        let mean_actual = actual_luma.iter().sum::<f64>() / actual_luma.len().max(1) as f64;
        let mean_expected = expected_luma.iter().sum::<f64>() / expected_luma.len().max(1) as f64;
        let mut variance_actual = 0.0;
        let mut variance_expected = 0.0;
        let mut covariance = 0.0;
        for (left, right) in actual_luma.iter().zip(&expected_luma) {
            variance_actual += (left - mean_actual).powi(2);
            variance_expected += (right - mean_expected).powi(2);
            covariance += (left - mean_actual) * (right - mean_expected);
        }
        let denominator = actual_luma.len().saturating_sub(1).max(1) as f64;
        variance_actual /= denominator;
        variance_expected /= denominator;
        covariance /= denominator;
        let c1 = (0.01 * 255.0_f64).powi(2);
        let c2 = (0.03 * 255.0_f64).powi(2);
        let ssim = ((2.0 * mean_actual * mean_expected + c1) * (2.0 * covariance + c2))
            / ((mean_actual.powi(2) + mean_expected.powi(2) + c1)
                * (variance_actual + variance_expected + c2));
        BloomDiffMetrics {
            max_channel_delta,
            rmse,
            ssim,
            worst_region_bbox: (min_x != u32::MAX).then_some([min_x, min_y, max_x, max_y]),
        }
    }

    #[test]
    fn pbr_neutral_uses_dedicated_shader_branch_marker() {
        let mut output = OutputTransform::default();
        output.set_tonemapper(Tonemapper::PbrNeutral);

        assert_eq!(output.color_management_uniform(), [2.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn photographic_white_balance_neutralizes_the_estimated_illuminant_before_tonemapping() {
        let daylight = WhiteBalance::from_illuminant_kelvin(6_500.0);
        assert_eq!(daylight.linear_multipliers(), [1.0, 1.0, 1.0]);

        let tungsten = WhiteBalance::from_illuminant_kelvin(3_200.0);
        let correction = tungsten.linear_multipliers();
        assert!(
            correction[2] > correction[0],
            "a warm illuminant must receive more blue than red compensation: {correction:?}"
        );

        let mut output = OutputTransform::default();
        output.set_tonemapper(Tonemapper::Standard);
        output.set_white_balance(tungsten);
        let corrected = output.post_color(Color::from_linear_rgb(0.5, 0.5, 0.5));
        assert!(
            corrected.b > corrected.r,
            "white balance must be applied in scene-linear space before the display transform"
        );
    }

    #[test]
    fn cpu_bloom_uses_scene_linear_hdr_energy_before_final_encoding() {
        let target = RasterTarget {
            width: 5,
            height: 5,
            backend: Backend::Headless,
        };
        let mut frame = vec![Color::BLACK; target.pixel_len()];
        frame[target.pixel_index(2, 2)] = Color::from_linear_rgb(8.0, 4.0, 2.0);
        let mut threshold = vec![Color::BLACK; target.pixel_len()];
        let mut horizontal = vec![Color::BLACK; target.pixel_len()];

        let passes = apply_bloom_linear(
            target,
            &mut frame,
            &mut threshold,
            &mut horizontal,
            PostBloomConfig::new(200, 0.5, 1),
        );

        assert_eq!(passes, 1);
        assert!(
            frame[target.pixel_index(2, 2)].r > 8.0,
            "bloom must preserve and spread radiance above display white"
        );
        assert!(
            frame[target.pixel_index(2, 1)].r > 0.0,
            "HDR energy must reach neighboring pixels before tonemapping"
        );
    }
}
