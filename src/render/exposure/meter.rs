use super::{
    AutoExposureConfig, AutoExposureResult, Color, LUMINANCE_EPSILON, LUMINANCE_HISTOGRAM_BIN_EV,
    LUMINANCE_HISTOGRAM_BINS, LUMINANCE_HISTOGRAM_MAX_EV, LUMINANCE_HISTOGRAM_MIN_EV,
    linear_luminance,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LuminanceHistogram {
    bins: [u32; LUMINANCE_HISTOGRAM_BINS],
    sample_count: u32,
}

impl Default for LuminanceHistogram {
    fn default() -> Self {
        Self {
            bins: [0; LUMINANCE_HISTOGRAM_BINS],
            sample_count: 0,
        }
    }
}

impl LuminanceHistogram {
    pub(super) fn record(&mut self, luminance: f32) {
        let ev = luminance.max(LUMINANCE_EPSILON).log2();
        let normalized = ((ev - LUMINANCE_HISTOGRAM_MIN_EV)
            / (LUMINANCE_HISTOGRAM_MAX_EV - LUMINANCE_HISTOGRAM_MIN_EV))
            .clamp(0.0, 1.0 - f32::EPSILON);
        let index = (normalized * LUMINANCE_HISTOGRAM_BINS as f32) as usize;
        self.bins[index] = self.bins[index].saturating_add(1);
        self.sample_count = self.sample_count.saturating_add(1);
    }

    pub(super) fn highlight_guard_ev(&self, config: AutoExposureConfig) -> f32 {
        if self.sample_count == 0 {
            return config.max_ev();
        }
        let rank = ((self.sample_count.saturating_sub(1)) as f32 * config.highlight_percentile())
            .round() as u32;
        let mut cumulative = 0_u32;
        let index = self
            .bins
            .iter()
            .position(|count| {
                cumulative = cumulative.saturating_add(*count);
                cumulative > rank
            })
            .unwrap_or(LUMINANCE_HISTOGRAM_BINS - 1);
        let ev = LUMINANCE_HISTOGRAM_MIN_EV + (index as f32 + 0.5) * LUMINANCE_HISTOGRAM_BIN_EV;
        let highlight_luminance = 2.0_f32.powf(ev).max(LUMINANCE_EPSILON);
        (config.highlight_target_luminance() / highlight_luminance).log2()
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct LuminanceMeter {
    histogram: LuminanceHistogram,
    log_luminance_sum: f64,
}

impl LuminanceMeter {
    pub(super) fn record(&mut self, color: Color) {
        if color.a <= 0.0 {
            return;
        }
        let luminance = linear_luminance(color);
        if !luminance.is_finite() {
            return;
        }
        let luminance = luminance.max(LUMINANCE_EPSILON);
        self.log_luminance_sum += f64::from(luminance).ln();
        self.histogram.record(luminance);
    }

    pub(super) fn sample_count(&self) -> u32 {
        self.histogram.sample_count
    }

    pub(super) fn finish(&self, config: AutoExposureConfig) -> Option<AutoExposureResult> {
        let sample_count = self.sample_count();
        if sample_count == 0 {
            return None;
        }
        let measured_luminance = (self.log_luminance_sum / f64::from(sample_count)).exp() as f32;
        let target_luminance = config.target_luminance();
        let raw_ev = (target_luminance / measured_luminance.max(LUMINANCE_EPSILON)).log2();
        let guarded_ev = raw_ev.min(self.highlight_guard_ev(config));
        let exposure_ev = guarded_ev.clamp(config.min_ev(), config.max_ev());
        Some(AutoExposureResult {
            measured_luminance,
            target_luminance,
            exposure_ev,
            sample_count,
            clamped: (exposure_ev - guarded_ev).abs() > f32::EPSILON,
        })
    }

    fn highlight_guard_ev(&self, config: AutoExposureConfig) -> f32 {
        self.histogram.highlight_guard_ev(config)
    }
}
