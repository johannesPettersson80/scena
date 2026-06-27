use super::PreparedLights;

impl PreparedLights {
    pub(in crate::render::prepare) fn gpu_uniform_counts(&self) -> [usize; 4] {
        [
            self.directional.len(),
            self.point.len(),
            self.spot.len(),
            self.area.len(),
        ]
    }
}
