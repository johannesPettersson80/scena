use super::instancing::InstanceDrawBatch;
use super::vertices::PrimitiveDrawBatch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MeshPipelineRequirements {
    pub(super) single_sided: bool,
    pub(super) double_sided: bool,
}

impl MeshPipelineRequirements {
    pub(super) const ALL: Self = Self {
        single_sided: true,
        double_sided: true,
    };

    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(super) fn from_batches(
        draw_batches: &[PrimitiveDrawBatch],
        instance_batches: &[InstanceDrawBatch],
    ) -> Self {
        let mut requirements = Self {
            single_sided: false,
            double_sided: false,
        };
        for double_sided in draw_batches
            .iter()
            .map(|batch| batch.double_sided)
            .chain(instance_batches.iter().map(|batch| batch.double_sided))
        {
            if double_sided {
                requirements.double_sided = true;
            } else {
                requirements.single_sided = true;
            }
        }
        // A prepared surface still owns a valid pipeline set when every
        // triangle was culled or when the frame contains only overlays.
        if !requirements.single_sided && !requirements.double_sided {
            requirements.single_sided = true;
        }
        requirements
    }

    pub(super) const fn compiled_pipeline_count(self) -> u64 {
        self.single_sided as u64 + self.double_sided as u64
    }
}
