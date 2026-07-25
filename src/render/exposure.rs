#[cfg(not(target_arch = "wasm32"))]
use crate::diagnostics::RenderError;
use crate::{diagnostics::Backend, material::Color};

use super::Renderer;
use super::color_contract::linear_rgba_to_srgb8;

mod meter;
#[cfg(test)]
use meter::LuminanceHistogram;
use meter::LuminanceMeter;

const DEFAULT_TARGET_LUMINANCE: f32 = 0.18;
const DEFAULT_MIN_EV: f32 = -4.0;
const DEFAULT_MAX_EV: f32 = 4.0;
const DEFAULT_HIGHLIGHT_PERCENTILE: f32 = 0.95;
const DEFAULT_HIGHLIGHT_TARGET_LUMINANCE: f32 = 0.85;
const LUMINANCE_EPSILON: f32 = 1.0e-4;
const AUTO_EXPOSURE_BACKGROUND_TOLERANCE_RGBA8: u8 = 2;
const MIN_FOREGROUND_AUTO_EXPOSURE_SAMPLES: usize = 64;
/// Fraction of a metered correction applied per frame on the **continuous**
/// attached-surface loop.
///
/// Values in `(0, 1]` all converge; below 1 they also absorb tonemapper
/// nonlinearity and per-frame sample noise instead of ringing on the way to the
/// target. A surface re-meters every frame, so the residual error is removed by
/// the frames that follow.
const SURFACE_AUTO_EXPOSURE_SMOOTHING: f32 = 0.5;

/// Fraction applied on the **one-shot** headless path.
///
/// A headless render meters once and re-renders once (`frame.rs` breaks after
/// `auto_exposure_attempted`), so there is no later frame to finish the job.
/// Damping there would ship a deliberately half-corrected image. Stability is
/// not at stake because the step never repeats.
const ONE_SHOT_AUTO_EXPOSURE_SMOOTHING: f32 = 1.0;
const LUMINANCE_HISTOGRAM_BINS: usize = 1_024;
const LUMINANCE_HISTOGRAM_MIN_EV: f32 = -16.0;
const LUMINANCE_HISTOGRAM_MAX_EV: f32 = 16.0;
const LUMINANCE_HISTOGRAM_BIN_EV: f32 =
    (LUMINANCE_HISTOGRAM_MAX_EV - LUMINANCE_HISTOGRAM_MIN_EV) / LUMINANCE_HISTOGRAM_BINS as f32;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AutoExposureStatus {
    #[default]
    Disabled,
    Pending,
    Converged,
    /// The attached surface cannot be copied into the bounded meter.
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoExposureFramePolicy {
    ImmediateDeterministic,
    PriorAsyncMeterSample,
}

pub(crate) const fn auto_exposure_frame_policy(
    gpu_active: bool,
    surface_attached: bool,
) -> AutoExposureFramePolicy {
    if gpu_active && surface_attached {
        AutoExposureFramePolicy::PriorAsyncMeterSample
    } else {
        AutoExposureFramePolicy::ImmediateDeterministic
    }
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

/// Chooses the next exposure EV for a metering path whose sample was captured
/// from an **already-exposed** frame (the attached surface, or the encoded
/// output frame when no linear scene buffer exists).
///
/// Such a sample closes a feedback loop: the meter reports
/// `log2(target / measured)`, and `measured` already contains the exposure
/// that produced it. Treating that correction as an absolute EV makes the
/// next EV a reflection of the current one, which oscillates instead of
/// converging.
/// The sample is therefore applied as a damped **delta** from the current
/// exposure rather than as an absolute EV. With an identity tonemapper the
/// loop's derivative becomes `1 - smoothing`; under a compressive tonemapper
/// it is `1 - smoothing * k` for `k` in `(0, 1)`. Both are contractions, so
/// the loop converges for any scene instead of reflecting.
fn next_feedback_exposure_ev(
    current_ev: f32,
    sample: AutoExposureResult,
    config: AutoExposureConfig,
    smoothing: f32,
) -> f32 {
    let correction_ev = sample.exposure_ev();
    if !correction_ev.is_finite() || !current_ev.is_finite() {
        return current_ev;
    }
    let damped = current_ev + correction_ev * smoothing;
    // The meter clamps the *correction* to the configured range; the absolute
    // exposure it accumulates into must be clamped as well.
    damped.clamp(config.min_ev(), config.max_ev())
}

pub fn estimate_auto_exposure_from_linear_colors(
    colors: &[Color],
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let mut meter = LuminanceMeter::default();
    for color in colors {
        meter.record(*color);
    }
    meter.finish(config)
}

pub fn estimate_auto_exposure_from_srgb8(
    rgba8: &[u8],
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let mut meter = LuminanceMeter::default();
    for pixel in rgba8.chunks_exact(4) {
        let color = Color::from_srgb_u8(pixel[0], pixel[1], pixel[2]);
        meter.record(Color::from_linear_rgba(
            color.r,
            color.g,
            color.b,
            f32::from(pixel[3]) / 255.0,
        ));
    }
    meter.finish(config)
}

fn estimate_auto_exposure_from_linear_colors_with_background(
    colors: &[Color],
    background: Color,
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let background = linear_rgba_to_srgb8(background);
    let mut all = LuminanceMeter::default();
    let mut foreground = LuminanceMeter::default();
    for color in colors.iter().copied() {
        all.record(color);
        if color.a > 0.0
            && color_differs_from_background(
                linear_rgba_to_srgb8(color).as_slice(),
                background,
                AUTO_EXPOSURE_BACKGROUND_TOLERANCE_RGBA8,
            )
        {
            foreground.record(color);
        }
    }
    if foreground.sample_count() as usize >= MIN_FOREGROUND_AUTO_EXPOSURE_SAMPLES {
        foreground.finish(config)
    } else {
        all.finish(config)
    }
}

fn estimate_auto_exposure_from_srgb8_with_background(
    rgba8: &[u8],
    background: Color,
    config: AutoExposureConfig,
) -> Option<AutoExposureResult> {
    let background = linear_rgba_to_srgb8(background);
    let mut all = LuminanceMeter::default();
    let mut foreground = LuminanceMeter::default();
    for pixel in rgba8.chunks_exact(4) {
        let color = Color::from_srgb_u8(pixel[0], pixel[1], pixel[2]);
        let color = Color::from_linear_rgba(color.r, color.g, color.b, f32::from(pixel[3]) / 255.0);
        all.record(color);
        if color.a > 0.0
            && color_differs_from_background(
                pixel,
                background,
                AUTO_EXPOSURE_BACKGROUND_TOLERANCE_RGBA8,
            )
        {
            foreground.record(color);
        }
    }
    if foreground.sample_count() as usize >= MIN_FOREGROUND_AUTO_EXPOSURE_SAMPLES {
        foreground.finish(config)
    } else {
        all.finish(config)
    }
}

impl Renderer {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn apply_pending_surface_auto_exposure(&mut self) -> Result<(), RenderError> {
        let Some(config) = self.auto_exposure else {
            self.auto_exposure_status = AutoExposureStatus::Disabled;
            return Ok(());
        };
        if !self
            .gpu
            .as_ref()
            .expect("surface auto exposure requires a GPU device")
            .auto_exposure_meter_supported()
        {
            self.auto_exposure_status = AutoExposureStatus::Unavailable;
            return Ok(());
        }
        let Some(sample_rgba8) = self
            .gpu
            .as_mut()
            .expect("surface auto exposure requires a GPU device")
            .poll_auto_exposure_meter(self.target.backend)?
        else {
            if self.last_auto_exposure.is_none() {
                self.auto_exposure_status = AutoExposureStatus::Pending;
            }
            return Ok(());
        };
        let Some(result) = estimate_auto_exposure_from_srgb8_with_background(
            &sample_rgba8,
            self.background_color(),
            config,
        ) else {
            self.auto_exposure_status = AutoExposureStatus::Pending;
            return Ok(());
        };
        let next_ev = next_feedback_exposure_ev(
            self.exposure_ev(),
            result,
            config,
            SURFACE_AUTO_EXPOSURE_SMOOTHING,
        );
        let exposure_changed = (self.exposure_ev() - next_ev).abs() > 0.01;
        self.last_auto_exposure = Some(result);
        self.auto_exposure_status = AutoExposureStatus::Converged;
        if exposure_changed {
            self.set_exposure_ev(next_ev);
        }
        Ok(())
    }

    pub fn set_auto_exposure(&mut self, config: AutoExposureConfig) {
        self.auto_exposure = Some(config);
        self.last_auto_exposure = None;
        self.auto_exposure_status = AutoExposureStatus::Pending;
        self.mark_output_changed();
    }

    pub fn clear_auto_exposure(&mut self) {
        if self.auto_exposure.take().is_some() {
            self.last_auto_exposure = None;
            self.auto_exposure_status = AutoExposureStatus::Disabled;
            self.mark_output_changed();
        }
    }

    pub const fn auto_exposure(&self) -> Option<AutoExposureConfig> {
        self.auto_exposure
    }

    pub const fn last_auto_exposure(&self) -> Option<AutoExposureResult> {
        self.last_auto_exposure
    }

    pub const fn auto_exposure_status(&self) -> AutoExposureStatus {
        self.auto_exposure_status
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
            self.auto_exposure_status = AutoExposureStatus::Disabled;
            return false;
        };
        let Some((result, sample_is_feedback)) =
            self.estimate_auto_exposure_from_current_frame(config)
        else {
            self.last_auto_exposure = None;
            self.auto_exposure_status = AutoExposureStatus::Pending;
            return false;
        };
        let next_ev = if sample_is_feedback {
            // One-shot: the metered value is a correction *relative to the
            // current exposure*, so it must be added, not assigned. Assigning it
            // discarded `current_ev`, which is what made the surface loop
            // reflect. Applying it in full is correct here because this path
            // runs exactly once per render and has no later frame to converge.
            next_feedback_exposure_ev(
                self.exposure_ev(),
                result,
                config,
                ONE_SHOT_AUTO_EXPOSURE_SMOOTHING,
            )
        } else {
            result.exposure_ev()
        };
        let exposure_changed = (self.exposure_ev() - next_ev).abs() > 0.01;
        self.last_auto_exposure = Some(result);
        self.auto_exposure_status = AutoExposureStatus::Converged;
        if exposure_changed {
            self.set_exposure_ev(next_ev);
        }
        exposure_changed
    }

    /// Returns the metered sample and whether it was captured from an
    /// already-exposed frame.
    ///
    /// The linear scene buffer is written before tone mapping and exposure are
    /// applied, so it yields an absolute EV directly. Every other source is the
    /// encoded output, which closes a feedback loop and must be damped.
    fn estimate_auto_exposure_from_current_frame(
        &self,
        config: AutoExposureConfig,
    ) -> Option<(AutoExposureResult, bool)> {
        if let Some(linear_frame) = self.linear_frame.as_deref() {
            return estimate_auto_exposure_from_linear_colors_with_background(
                linear_frame,
                self.background_color(),
                config,
            )
            .map(|result| (result, false));
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(result) = self
            .gpu
            .as_ref()
            .and_then(|gpu| gpu.estimate_browser_canvas_auto_exposure(config))
        {
            return Some((result, true));
        }
        if matches!(self.target.backend, Backend::WebGpu | Backend::WebGl2) {
            return None;
        }
        estimate_auto_exposure_from_srgb8_with_background(
            &self.frame,
            self.background_color(),
            config,
        )
        .map(|result| (result, true))
    }
}

fn linear_luminance(color: Color) -> f32 {
    if !color.r.is_finite() || !color.g.is_finite() || !color.b.is_finite() {
        return f32::NAN;
    }
    0.2126 * color.r.max(0.0) + 0.7152 * color.g.max(0.0) + 0.0722 * color.b.max(0.0)
}

#[cfg(test)]
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

    /// Drives the real production feedback decision over several frames.
    ///
    /// Models an attached surface with an identity tonemapper: a frame
    /// rendered at `ev` measures `scene_luminance * 2^ev`. That is the sample
    /// the surface meter copies back, so this reproduces the exact loop the
    /// native path runs, without needing a GPU.
    fn simulate_feedback_exposure(
        scene_luminance: f32,
        config: AutoExposureConfig,
        frames: usize,
    ) -> Vec<f32> {
        let mut ev = 0.0_f32;
        let mut history = Vec::with_capacity(frames);
        for _ in 0..frames {
            let measured = scene_luminance * 2.0_f32.powf(ev);
            let colors = vec![Color::from_linear_rgba(measured, measured, measured, 1.0); 256];
            let sample = estimate_auto_exposure_from_linear_colors(&colors, config)
                .expect("uniform frame meters");
            ev = next_feedback_exposure_ev(ev, sample, config, SURFACE_AUTO_EXPOSURE_SMOOTHING);
            history.push(ev);
        }
        history
    }

    #[test]
    fn surface_auto_exposure_converges_instead_of_oscillating() {
        let config = AutoExposureConfig::new(0.18);
        let scene_luminance = 0.02_f32;
        let desired_ev = (0.18_f32 / scene_luminance).log2();

        let history = simulate_feedback_exposure(scene_luminance, config, 10);

        let settled = history.last().copied().expect("frames were simulated");
        assert!(
            (settled - desired_ev).abs() < 0.1,
            "exposure must settle at the scene's desired EV {desired_ev:.3}, got {settled:.3}; \
             history: {history:?}",
        );

        let last_step = (history[history.len() - 1] - history[history.len() - 2]).abs();
        let first_step = (history[1] - history[0]).abs();
        assert!(
            last_step < first_step * 0.25,
            "per-frame exposure steps must decay toward zero, not alternate: \
             first step {first_step:.3}, last step {last_step:.3}; history: {history:?}",
        );
    }

    #[test]
    fn feedback_exposure_never_reflects_around_the_current_value() {
        // A sample metered from an already-exposed frame reports a *relative*
        // correction. Applying it as an absolute EV yields
        // `next = desired - current`, whose derivative is -1: a two-cycle.
        let config = AutoExposureConfig::new(0.18);
        let scene_luminance = 0.02_f32;
        let history = simulate_feedback_exposure(scene_luminance, config, 6);

        for window in history.windows(3) {
            let returned_to_start = (window[2] - window[0]).abs() < 1.0e-3;
            let moved_between = (window[1] - window[0]).abs() > 0.5;
            assert!(
                !(returned_to_start && moved_between),
                "exposure returned to a previous value after one round trip \
                 (two-cycle oscillation): {history:?}",
            );
        }
    }

    /// R01: the one-shot headless path and the continuous surface loop need
    /// different step sizes, and confusing them is a real defect in both
    /// directions.
    ///
    /// A headless render meters once and re-renders once, so a damped step
    /// ships a knowingly half-corrected image. A surface re-meters every frame,
    /// so a full step there amplifies tonemapper nonlinearity and sample noise.
    #[test]
    fn one_shot_applies_the_whole_correction_and_the_surface_loop_damps_it() {
        let config = AutoExposureConfig::new(0.18);
        let current_ev = 0.0_f32;
        // A sample metered from an already-exposed frame: a relative correction.
        let measured = 0.09_f32;
        let colors = vec![Color::from_linear_rgba(measured, measured, measured, 1.0); 256];
        let sample = estimate_auto_exposure_from_linear_colors(&colors, config)
            .expect("uniform frame meters");
        let correction = sample.exposure_ev();
        assert!(
            correction.abs() > 0.1,
            "the fixture must request a correction worth measuring, got {correction}",
        );
        // Keep the fixture inside the configured EV range so this test measures
        // the step size, not the clamp.
        assert!(
            current_ev + correction < config.max_ev() && current_ev + correction > config.min_ev(),
            "fixture must stay inside [{}, {}], got {}",
            config.min_ev(),
            config.max_ev(),
            current_ev + correction,
        );

        let one_shot =
            next_feedback_exposure_ev(current_ev, sample, config, ONE_SHOT_AUTO_EXPOSURE_SMOOTHING);
        assert!(
            (one_shot - (current_ev + correction)).abs() < 1.0e-4,
            "the one-shot path must apply the whole correction: expected {}, got {one_shot}",
            current_ev + correction,
        );

        let surface =
            next_feedback_exposure_ev(current_ev, sample, config, SURFACE_AUTO_EXPOSURE_SMOOTHING);
        assert!(
            (surface - (current_ev + correction * SURFACE_AUTO_EXPOSURE_SMOOTHING)).abs() < 1.0e-4,
            "the surface loop must damp the correction, got {surface}",
        );
        assert!(
            (surface - current_ev).abs() < (one_shot - current_ev).abs(),
            "damping must move less far than a full step: surface={surface}, one_shot={one_shot}",
        );
        // Both must still move toward the same target, not past it.
        assert_eq!(
            (one_shot - current_ev).signum(),
            (surface - current_ev).signum(),
            "damping must not reverse the direction of the correction",
        );
    }

    #[test]
    fn bounded_histogram_matches_sorted_highlight_reference_within_one_bin() {
        let luminances = (0..4_096)
            .map(|index| 2.0_f32.powf(-12.0 + index as f32 * 24.0 / 4_095.0))
            .collect::<Vec<_>>();
        let config = AutoExposureConfig::mixed().with_highlight_guard(0.95, 0.85);
        let mut sorted = luminances.clone();
        let reference = highlight_guard_ev(&mut sorted, config);
        let mut histogram = LuminanceHistogram::default();
        for luminance in luminances {
            histogram.record(luminance);
        }
        let bounded = histogram.highlight_guard_ev(config);

        assert!(
            (bounded - reference).abs() <= LUMINANCE_HISTOGRAM_BIN_EV,
            "bounded percentile must stay within one EV bin: bounded={bounded} reference={reference}",
        );
    }

    #[test]
    fn bounded_meter_covers_exact_flat_outlier_and_invalid_distributions() {
        let config = AutoExposureConfig::mixed().with_ev_range(-16.0, 16.0);
        let flat = vec![Color::from_linear_rgb(0.25, 0.25, 0.25); 4_096];
        let flat_result = estimate_auto_exposure_from_linear_colors(&flat, config)
            .expect("flat luminance distribution meters");
        assert!((flat_result.measured_luminance() - 0.25).abs() <= 1.0e-5);
        assert!(
            (flat_result.exposure_ev() - (config.target_luminance() / 0.25).log2()).abs() <= 1.0e-5,
            "flat distribution must preserve the exact geometric-mean solution",
        );

        let guarded = AutoExposureConfig::mixed()
            .with_ev_range(-16.0, 16.0)
            .with_highlight_guard(0.95, 0.85);
        let mut histogram = LuminanceHistogram::default();
        for _ in 0..999 {
            histogram.record(0.25);
        }
        histogram.record(65_536.0);
        let expected_guard = (guarded.highlight_target_luminance() / 0.25).log2();
        assert!(
            (histogram.highlight_guard_ev(guarded) - expected_guard).abs()
                <= LUMINANCE_HISTOGRAM_BIN_EV,
            "a single extreme outlier above the configured percentile must not move the guard by more than one histogram bin",
        );

        let invalid = [
            Color::from_linear_rgba(f32::NAN, 0.0, 0.0, 1.0),
            Color::from_linear_rgba(1.0, 1.0, 1.0, 0.0),
            Color::from_linear_rgb(0.5, 0.5, 0.5),
        ];
        let invalid_result = estimate_auto_exposure_from_linear_colors(&invalid, config)
            .expect("one finite opaque sample meters");
        assert_eq!(invalid_result.sample_count(), 1);
        assert!((invalid_result.measured_luminance() - 0.5).abs() <= 1.0e-5);
    }

    #[test]
    fn luminance_meter_storage_is_resolution_independent() {
        assert!(
            std::mem::size_of::<LuminanceMeter>() <= 8 * 1_024,
            "auto-exposure metering must use fixed bounded storage",
        );
    }

    #[test]
    fn attached_gpu_auto_exposure_uses_prior_async_meter_sample() {
        assert_eq!(
            auto_exposure_frame_policy(true, true),
            AutoExposureFramePolicy::PriorAsyncMeterSample,
        );
        assert_eq!(
            auto_exposure_frame_policy(true, false),
            AutoExposureFramePolicy::ImmediateDeterministic,
        );
        assert_eq!(
            auto_exposure_frame_policy(false, false),
            AutoExposureFramePolicy::ImmediateDeterministic,
        );

        let mut renderer = Renderer::headless(8, 8).expect("renderer builds");
        assert_eq!(
            renderer.auto_exposure_status(),
            AutoExposureStatus::Disabled
        );
        renderer.set_auto_exposure(AutoExposureConfig::mixed());
        assert_eq!(renderer.auto_exposure_status(), AutoExposureStatus::Pending);
        renderer.clear_auto_exposure();
        assert_eq!(
            renderer.auto_exposure_status(),
            AutoExposureStatus::Disabled
        );
    }

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
