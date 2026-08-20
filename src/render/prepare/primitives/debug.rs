#[derive(Debug)]
pub(super) struct VisibilityDebugStats {
    samples: u64,
    area_sum: f64,
    area_min: f32,
    area_occluded: u64,
    ambient_sum: f64,
    ambient_min: f32,
    ambient_occluded: u64,
}

impl Default for VisibilityDebugStats {
    fn default() -> Self {
        Self {
            samples: 0,
            area_sum: 0.0,
            area_min: 1.0,
            area_occluded: 0,
            ambient_sum: 0.0,
            ambient_min: 1.0,
            ambient_occluded: 0,
        }
    }
}

impl VisibilityDebugStats {
    pub(super) fn record(&mut self, area: f32, ambient: f32) {
        self.samples = self.samples.saturating_add(1);
        self.area_sum += f64::from(area);
        self.area_min = self.area_min.min(area);
        self.area_occluded = self.area_occluded.saturating_add(u64::from(area < 0.999));
        self.ambient_sum += f64::from(ambient);
        self.ambient_min = self.ambient_min.min(ambient);
        self.ambient_occluded = self
            .ambient_occluded
            .saturating_add(u64::from(ambient < 0.999));
    }

    pub(super) fn log(self, node: crate::NodeKey) {
        if self.samples == 0 {
            return;
        }
        let samples = self.samples as f64;
        eprintln!(
            "[visibility] node={node:?} samples={} area_min={:.4} area_mean={:.4} area_occluded_fraction={:.4} ambient_min={:.4} ambient_mean={:.4} ambient_occluded_fraction={:.4}",
            self.samples,
            self.area_min,
            self.area_sum / samples,
            self.area_occluded as f64 / samples,
            self.ambient_min,
            self.ambient_sum / samples,
            self.ambient_occluded as f64 / samples
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn visibility_debug_enabled() -> bool {
    std::env::var_os("SCENA_DEBUG_LOG_VISIBILITY").is_some()
}

#[cfg(target_arch = "wasm32")]
pub(super) const fn visibility_debug_enabled() -> bool {
    false
}
