use super::{Scene, Vec3};

impl Scene {
    pub fn set_origin_shift(&mut self, origin_shift: Vec3) {
        if self.origin_shift != origin_shift {
            self.origin_shift = origin_shift;
            self.structure_revision = self.structure_revision.saturating_add(1);
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
    }

    pub fn origin_shift(&self) -> Vec3 {
        self.origin_shift
    }
}
