use super::{
    AutoExposureConfig, AutoExposureMeteringDomain, AutoExposureResult, Color, LUMINANCE_EPSILON,
    LUMINANCE_HISTOGRAM_BIN_EV, LUMINANCE_HISTOGRAM_BINS, LUMINANCE_HISTOGRAM_MAX_EV,
    LUMINANCE_HISTOGRAM_MIN_EV, MAX_HIGHLIGHT_GUARD_REDUCTION_EV, linear_luminance,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LuminanceHistogram {
    bins: [f32; LUMINANCE_HISTOGRAM_BINS],
    sample_count: u32,
    weight_sum: f32,
}

impl Default for LuminanceHistogram {
    fn default() -> Self {
        Self {
            bins: [0.0; LUMINANCE_HISTOGRAM_BINS],
            sample_count: 0,
            weight_sum: 0.0,
        }
    }
}

impl LuminanceHistogram {
    #[cfg(test)]
    pub(super) fn record(&mut self, luminance: f32) {
        self.record_weighted(luminance, 1.0);
    }

    pub(super) fn record_weighted(&mut self, luminance: f32, weight: f32) {
        let weight = weight.clamp(0.0, 1.0);
        if weight <= 0.0 {
            return;
        }
        let ev = luminance.max(LUMINANCE_EPSILON).log2();
        let normalized = ((ev - LUMINANCE_HISTOGRAM_MIN_EV)
            / (LUMINANCE_HISTOGRAM_MAX_EV - LUMINANCE_HISTOGRAM_MIN_EV))
            .clamp(0.0, 1.0 - f32::EPSILON);
        let index = (normalized * LUMINANCE_HISTOGRAM_BINS as f32) as usize;
        self.bins[index] += weight;
        self.sample_count = self.sample_count.saturating_add(1);
        self.weight_sum += weight;
    }

    pub(super) fn highlight_guard_ev(&self, config: AutoExposureConfig) -> f32 {
        if self.sample_count == 0 || self.weight_sum <= 0.0 {
            return config.max_ev();
        }
        let rank = ((self.weight_sum - 1.0).max(0.0) * config.highlight_percentile()).round();
        let mut cumulative = 0.0_f32;
        let index = self
            .bins
            .iter()
            .position(|count| {
                cumulative += *count;
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
    weight_sum: f64,
}

impl LuminanceMeter {
    pub(super) fn record(&mut self, color: Color) {
        self.record_weighted(color, 1.0);
    }

    pub(super) fn record_weighted(&mut self, color: Color, weight: f32) {
        if color.a <= 0.0 {
            return;
        }
        let weight = weight.clamp(0.0, 1.0);
        if weight <= 0.0 {
            return;
        }
        let luminance = linear_luminance(color);
        if !luminance.is_finite() {
            return;
        }
        let luminance = luminance.max(LUMINANCE_EPSILON);
        self.log_luminance_sum += f64::from(luminance).ln() * f64::from(weight);
        self.weight_sum += f64::from(weight);
        self.histogram.record_weighted(luminance, weight);
    }

    pub(super) fn sample_count(&self) -> u32 {
        self.histogram.sample_count
    }

    pub(super) fn finish(
        &self,
        config: AutoExposureConfig,
        metering_domain: AutoExposureMeteringDomain,
    ) -> Option<AutoExposureResult> {
        self.finish_with_counts_and_highlight(config, 0, 0, &self.histogram, metering_domain)
    }

    pub(super) fn finish_with_counts_and_highlight(
        &self,
        config: AutoExposureConfig,
        subject_sample_count: u32,
        rejected_sample_count: u32,
        highlight_histogram: &LuminanceHistogram,
        metering_domain: AutoExposureMeteringDomain,
    ) -> Option<AutoExposureResult> {
        let sample_count = self.sample_count();
        if sample_count == 0 || self.weight_sum <= 0.0 {
            return None;
        }
        let measured_luminance = (self.log_luminance_sum / self.weight_sum).exp() as f32;
        let target_luminance = config.target_luminance();
        let raw_ev = (target_luminance / measured_luminance.max(LUMINANCE_EPSILON)).log2();
        let highlight_ev = highlight_histogram
            .highlight_guard_ev(config)
            .max(raw_ev - MAX_HIGHLIGHT_GUARD_REDUCTION_EV);
        let guarded_ev = raw_ev.min(highlight_ev);
        let base_exposure_ev = guarded_ev.clamp(config.min_ev(), config.max_ev());
        let compensated_ev = guarded_ev + config.compensation_ev();
        let exposure_ev = compensated_ev.clamp(config.min_ev(), config.max_ev());
        Some(AutoExposureResult {
            measured_luminance,
            target_luminance,
            base_exposure_ev,
            compensation_ev: config.compensation_ev(),
            exposure_ev,
            metering_domain,
            sample_count,
            subject_sample_count: subject_sample_count.min(sample_count),
            rejected_sample_count,
            clamped: (exposure_ev - compensated_ev).abs() > f32::EPSILON,
        })
    }
}
