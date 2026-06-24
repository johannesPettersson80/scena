use crate::{diagnostics::Backend, material::Color};

use super::Renderer;
use super::color_contract::linear_rgba_to_srgb8;

const DEFAULT_TARGET_LUMINANCE: f32 = 0.18;
const DEFAULT_MIN_EV: f32 = -4.0;
const DEFAULT_MAX_EV: f32 = 4.0;
const DEFAULT_HIGHLIGHT_PERCENTILE: f32 = 0.95;
const DEFAULT_HIGHLIGHT_TARGET_LUMINANCE: f32 = 0.85;
const LUMINANCE_EPSILON: f32 = 1.0e-4;
const AUTO_EXPOSURE_BACKGROUND_TOLERANCE_RGBA8: u8 = 2;
const MIN_FOREGROUND_AUTO_EXPOSURE_SAMPLES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoExposureConfig {
    target_luminance: f32,
    min_ev: f32,
    max_ev: f32,
    highlight_percentile: f32,
    highlight_target_luminance: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoExposureResult {
    measured_luminance: f32,
    target_luminance: f32,
    exposure_ev: f32,
    sample_count: u32,
    clamped: bool,
}

impl AutoExposureConfig {
    pub const PRESET_NAMES: &'static [&'static str] =
        &["product_studio", "indoor", "outdoor", "mixed"];

    pub fn from_preset_name(name: &str) -> Option<Self> {
        match name {
            "product_studio" => Some(Self::product_studio()),
            "indoor" => Some(Self::indoor()),
            "outdoor" => Some(Self::outdoor()),
            "mixed" => Some(Self::mixed()),
            _ => None,
        }
    }

    pub const fn new(target_luminance: f32) -> Self {
        Self {
            target_luminance,
            min_ev: DEFAULT_MIN_EV,
            max_ev: DEFAULT_MAX_EV,
            highlight_percentile: DEFAULT_HIGHLIGHT_PERCENTILE,
            highlight_target_luminance: DEFAULT_HIGHLIGHT_TARGET_LUMINANCE,
        }
    }

    /// Product-viewer exposure for controlled studio lighting.
    ///
    /// Uses a slightly brighter target, a tight EV range, and an aggressive
    /// highlight guard so light product surfaces do not wash out while the
    /// renderer lifts a dark studio background.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::{AutoExposureConfig, Renderer};
    /// # fn example() -> scena::Result<()> {
    /// let mut renderer = Renderer::headless(1280, 720)?;
    /// renderer.set_auto_exposure(AutoExposureConfig::product_studio());
    /// # Ok(())
    /// # }
    /// ```
    pub const fn product_studio() -> Self {
        Self {
            target_luminance: 0.22,
            min_ev: -1.5,
            max_ev: 0.65,
            highlight_percentile: 0.88,
            highlight_target_luminance: 0.70,
        }
    }

    /// Indoor exposure for moderately dim scenes with practical highlight headroom.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::{AutoExposureConfig, Renderer};
    /// # fn example() -> scena::Result<()> {
    /// let mut renderer = Renderer::headless(1280, 720)?;
    /// renderer.set_auto_exposure(AutoExposureConfig::indoor());
    /// # Ok(())
    /// # }
    /// ```
    pub const fn indoor() -> Self {
        Self {
            target_luminance: 0.20,
            min_ev: -2.5,
            max_ev: 2.5,
            highlight_percentile: 0.95,
            highlight_target_luminance: 0.82,
        }
    }

    /// Outdoor exposure for bright scenes where darkening is usually safer than lifting.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::{AutoExposureConfig, Renderer};
    /// # fn example() -> scena::Result<()> {
    /// let mut renderer = Renderer::headless(1280, 720)?;
    /// renderer.set_auto_exposure(AutoExposureConfig::outdoor());
    /// # Ok(())
    /// # }
    /// ```
    pub const fn outdoor() -> Self {
        Self {
            target_luminance: 0.16,
            min_ev: -5.0,
            max_ev: 0.75,
            highlight_percentile: 0.98,
            highlight_target_luminance: 0.90,
        }
    }

    /// Conservative mixed-lighting exposure. Equivalent to [`Self::default`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use scena::{AutoExposureConfig, Renderer};
    /// # fn example() -> scena::Result<()> {
    /// let mut renderer = Renderer::headless(1280, 720)?;
    /// renderer.set_auto_exposure(AutoExposureConfig::mixed());
    /// # Ok(())
    /// # }
    /// ```
    pub const fn mixed() -> Self {
        Self {
            target_luminance: DEFAULT_TARGET_LUMINANCE,
            min_ev: DEFAULT_MIN_EV,
            max_ev: DEFAULT_MAX_EV,
            highlight_percentile: DEFAULT_HIGHLIGHT_PERCENTILE,
            highlight_target_luminance: DEFAULT_HIGHLIGHT_TARGET_LUMINANCE,
        }
    }

    pub fn with_ev_range(mut self, min_ev: f32, max_ev: f32) -> Self {
        let min_ev = finite_or(min_ev, DEFAULT_MIN_EV);
        let max_ev = finite_or(max_ev, DEFAULT_MAX_EV);
        if min_ev <= max_ev {
            self.min_ev = min_ev;
            self.max_ev = max_ev;
        } else {
            self.min_ev = max_ev;
            self.max_ev = min_ev;
        }
        self
    }

    pub fn target_luminance(self) -> f32 {
        valid_luminance_or(self.target_luminance, DEFAULT_TARGET_LUMINANCE)
    }

    pub fn with_highlight_guard(mut self, percentile: f32, target_luminance: f32) -> Self {
        self.highlight_percentile = if percentile.is_finite() {
            percentile.clamp(0.0, 1.0)
        } else {
            DEFAULT_HIGHLIGHT_PERCENTILE
        };
        self.highlight_target_luminance =
            valid_luminance_or(target_luminance, DEFAULT_HIGHLIGHT_TARGET_LUMINANCE);
        self
    }

    pub fn highlight_percentile(self) -> f32 {
        if self.highlight_percentile.is_finite() {
            self.highlight_percentile.clamp(0.0, 1.0)
        } else {
            DEFAULT_HIGHLIGHT_PERCENTILE
        }
    }

    pub fn highlight_target_luminance(self) -> f32 {
        valid_luminance_or(
            self.highlight_target_luminance,
            DEFAULT_HIGHLIGHT_TARGET_LUMINANCE,
        )
    }

    pub fn min_ev(self) -> f32 {
        finite_or(self.min_ev, DEFAULT_MIN_EV)
    }

    pub fn max_ev(self) -> f32 {
        finite_or(self.max_ev, DEFAULT_MAX_EV)
    }
}

impl Default for AutoExposureConfig {
    fn default() -> Self {
        Self::new(DEFAULT_TARGET_LUMINANCE)
    }
}

impl AutoExposureResult {
    pub const fn measured_luminance(self) -> f32 {
        self.measured_luminance
    }

    pub const fn target_luminance(self) -> f32 {
        self.target_luminance
    }

    pub const fn exposure_ev(self) -> f32 {
        self.exposure_ev
    }

    pub const fn sample_count(self) -> u32 {
        self.sample_count
    }

    pub const fn clamped(self) -> bool {
        self.clamped
    }
}

pub fn estimate_auto_exposure_from_linear_colors(
    colors: &[Color],
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let mut log_luminance_sum = 0.0_f32;
    let mut luminances = Vec::with_capacity(colors.len());
    let mut sample_count = 0_u32;
    for color in colors {
        if color.a <= 0.0 {
            continue;
        }
        let luminance = linear_luminance(*color);
        if !luminance.is_finite() {
            continue;
        }
        let luminance = luminance.max(LUMINANCE_EPSILON);
        log_luminance_sum += luminance.ln();
        luminances.push(luminance);
        sample_count = sample_count.saturating_add(1);
    }
    if sample_count == 0 {
        return None;
    }

    let measured_luminance = (log_luminance_sum / sample_count as f32).exp();
    let target_luminance = config.target_luminance();
    let raw_ev = (target_luminance / measured_luminance.max(LUMINANCE_EPSILON)).log2();
    let highlight_ev = highlight_guard_ev(&mut luminances, config);
    let guarded_ev = raw_ev.min(highlight_ev);
    let min_ev = config.min_ev();
    let max_ev = config.max_ev();
    let exposure_ev = guarded_ev.clamp(min_ev, max_ev);
    Some(AutoExposureResult {
        measured_luminance,
        target_luminance,
        exposure_ev,
        sample_count,
        clamped: (exposure_ev - guarded_ev).abs() > f32::EPSILON,
    })
}

pub fn estimate_auto_exposure_from_srgb8(
    rgba8: &[u8],
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let colors: Vec<Color> = rgba8
        .chunks_exact(4)
        .map(|pixel| {
            let color = Color::from_srgb_u8(pixel[0], pixel[1], pixel[2]);
            Color::from_linear_rgba(color.r, color.g, color.b, f32::from(pixel[3]) / 255.0)
        })
        .collect();
    estimate_auto_exposure_from_linear_colors(&colors, config)
}

fn estimate_auto_exposure_from_linear_colors_with_background(
    colors: &[Color],
    background: Color,
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let background = linear_rgba_to_srgb8(background);
    let foreground = colors
        .iter()
        .copied()
        .filter(|color| {
            color.a > 0.0
                && color_differs_from_background(
                    linear_rgba_to_srgb8(*color).as_slice(),
                    background,
                    AUTO_EXPOSURE_BACKGROUND_TOLERANCE_RGBA8,
                )
        })
        .collect::<Vec<_>>();
    if foreground.len() >= MIN_FOREGROUND_AUTO_EXPOSURE_SAMPLES {
        estimate_auto_exposure_from_linear_colors(&foreground, config)
    } else {
        estimate_auto_exposure_from_linear_colors(colors, config)
    }
}

fn estimate_auto_exposure_from_srgb8_with_background(
    rgba8: &[u8],
    background: Color,
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let background = linear_rgba_to_srgb8(background);
    let mut colors = Vec::with_capacity(rgba8.len() / 4);
    let mut foreground = Vec::new();
    for pixel in rgba8.chunks_exact(4) {
        let color = Color::from_srgb_u8(pixel[0], pixel[1], pixel[2]);
        let color = Color::from_linear_rgba(color.r, color.g, color.b, f32::from(pixel[3]) / 255.0);
        colors.push(color);
        if color.a > 0.0
            && color_differs_from_background(
                pixel,
                background,
                AUTO_EXPOSURE_BACKGROUND_TOLERANCE_RGBA8,
            )
        {
            foreground.push(color);
        }
    }
    if foreground.len() >= MIN_FOREGROUND_AUTO_EXPOSURE_SAMPLES {
        estimate_auto_exposure_from_linear_colors(&foreground, config)
    } else {
        estimate_auto_exposure_from_linear_colors(&colors, config)
    }
}

impl Renderer {
    pub fn set_auto_exposure(&mut self, config: AutoExposureConfig) {
        self.auto_exposure = Some(config);
        self.last_auto_exposure = None;
        self.mark_output_changed();
    }

    pub fn clear_auto_exposure(&mut self) {
        if self.auto_exposure.take().is_some() {
            self.last_auto_exposure = None;
            self.mark_output_changed();
        }
    }

    pub const fn auto_exposure(&self) -> Option<AutoExposureConfig> {
        self.auto_exposure
    }

    pub const fn last_auto_exposure(&self) -> Option<AutoExposureResult> {
        self.last_auto_exposure
    }

    pub fn estimate_auto_exposure_from_last_cpu_frame(
        &self,
        config: AutoExposureConfig,
    ) -> Option<AutoExposureResult> {
        estimate_auto_exposure_from_linear_colors_with_background(
            self.linear_frame.as_deref()?,
            self.background_color(),
            config,
        )
    }

    pub fn apply_auto_exposure_from_last_cpu_frame(
        &mut self,
        config: AutoExposureConfig,
    ) -> Option<AutoExposureResult> {
        let result = self.estimate_auto_exposure_from_last_cpu_frame(config)?;
        self.set_exposure_ev(result.exposure_ev());
        Some(result)
    }

    pub(super) fn apply_managed_auto_exposure_after_render(&mut self) -> bool {
        let Some(config) = self.auto_exposure else {
            self.last_auto_exposure = None;
            return false;
        };
        let Some(result) = self.estimate_auto_exposure_from_current_frame(config) else {
            self.last_auto_exposure = None;
            return false;
        };
        let exposure_changed = (self.exposure_ev() - result.exposure_ev()).abs() > 0.01;
        self.last_auto_exposure = Some(result);
        if exposure_changed {
            self.set_exposure_ev(result.exposure_ev());
        }
        exposure_changed
    }

    fn estimate_auto_exposure_from_current_frame(
        &self,
        config: AutoExposureConfig,
    ) -> Option<AutoExposureResult> {
        if let Some(linear_frame) = self.linear_frame.as_deref() {
            return estimate_auto_exposure_from_linear_colors_with_background(
                linear_frame,
                self.background_color(),
                config,
            );
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(result) = self
            .gpu
            .as_ref()
            .and_then(|gpu| gpu.estimate_browser_canvas_auto_exposure(config))
        {
            return Some(result);
        }
        if matches!(self.target.backend, Backend::WebGpu | Backend::WebGl2) {
            return None;
        }
        estimate_auto_exposure_from_srgb8_with_background(
            &self.frame,
            self.background_color(),
            config,
        )
    }
}

fn linear_luminance(color: Color) -> f32 {
    0.2126 * color.r.max(0.0) + 0.7152 * color.g.max(0.0) + 0.0722 * color.b.max(0.0)
}

fn highlight_guard_ev(luminances: &mut [f32], config: AutoExposureConfig) -> f32 {
    if luminances.is_empty() {
        return config.max_ev();
    }
    luminances.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((luminances.len().saturating_sub(1)) as f32 * config.highlight_percentile())
        .round() as usize;
    let highlight_luminance = luminances[index.min(luminances.len() - 1)].max(LUMINANCE_EPSILON);
    (config.highlight_target_luminance() / highlight_luminance).log2()
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn valid_luminance_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

fn color_differs_from_background(pixel: &[u8], background: [u8; 4], tolerance: u8) -> bool {
    (0..3).any(|channel| pixel[channel].abs_diff(background[channel]) > tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_exposure_prefers_foreground_over_flat_background() {
        let background = Color::STUDIO_BACKDROP;
        let subject = Color::from_srgb_u8(60, 68, 78);
        let mut colors = vec![background; 900];
        colors.extend(std::iter::repeat_n(subject, 100));

        let full_frame = estimate_auto_exposure_from_linear_colors(
            &colors,
            AutoExposureConfig::product_studio(),
        )
        .expect("full-frame auto exposure estimates");
        let foreground = estimate_auto_exposure_from_linear_colors_with_background(
            &colors,
            background,
            AutoExposureConfig::product_studio(),
        )
        .expect("foreground auto exposure estimates");

        assert!(
            foreground.exposure_ev() > full_frame.exposure_ev() + 1.0,
            "product exposure must meter the foreground subject rather than the bright studio background"
        );
        assert_eq!(foreground.sample_count(), 100);
    }

    #[test]
    fn auto_exposure_falls_back_when_foreground_is_too_sparse() {
        let background = Color::STUDIO_BACKDROP;
        let subject = Color::from_srgb_u8(60, 68, 78);
        let mut colors = vec![background; 900];
        colors.extend(std::iter::repeat_n(
            subject,
            MIN_FOREGROUND_AUTO_EXPOSURE_SAMPLES - 1,
        ));

        let full_frame = estimate_auto_exposure_from_linear_colors(
            &colors,
            AutoExposureConfig::product_studio(),
        )
        .expect("full-frame auto exposure estimates");
        let foreground = estimate_auto_exposure_from_linear_colors_with_background(
            &colors,
            background,
            AutoExposureConfig::product_studio(),
        )
        .expect("foreground auto exposure estimates");

        assert_eq!(foreground.sample_count(), full_frame.sample_count());
        assert!((foreground.exposure_ev() - full_frame.exposure_ev()).abs() < f32::EPSILON);
    }
}
