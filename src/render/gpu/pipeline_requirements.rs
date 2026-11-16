use crate::render::prepare::{PreparedInstanceSet, PreparedPrimitive};

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

    /// The culling variants the retained template can ask for.
    ///
    /// This looks at every retained triangle, not at the draw list of the
    /// frame being prepared: the retained prepare path keeps these pipelines
    /// while the camera moves, so a double-sided mesh that starts outside the
    /// view must not find only the back-face-culling pipeline when it arrives.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(super) fn from_retained(
        primitives: &[PreparedPrimitive],
        instances: &[PreparedInstanceSet],
    ) -> Self {
        let mut requirements = Self {
            single_sided: false,
            double_sided: false,
        };
        for primitive in primitives
            .iter()
            .chain(instances.iter().flat_map(|set| set.primitives().iter()))
        {
            if primitive.double_sided() {
                requirements.double_sided = true;
            } else {
                requirements.single_sided = true;
            }
        }
        // A prepared surface still owns a valid pipeline set when the scene
        // contains only overlays.
        if !requirements.single_sided && !requirements.double_sided {
            requirements.single_sided = true;
        }
        requirements
    }

    pub(super) const fn compiled_pipeline_count(self) -> u64 {
        self.single_sided as u64 + self.double_sided as u64
    }
}
