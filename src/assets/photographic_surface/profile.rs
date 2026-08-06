use super::PhotographicSurfaceKind;

#[derive(Clone, Copy)]
pub(super) struct SurfaceProfile {
    pub(super) metallic: f32,
    pub(super) roughness: f32,
    pub(super) roughness_spread: f32,
    pub(super) height_strength: f32,
    pub(super) color_variation: f32,
    pub(super) directionality: f32,
    pub(super) normal_scale: f32,
    pub(super) occlusion_strength: f32,
    pub(super) clearcoat_factor: f32,
    pub(super) clearcoat_roughness: f32,
    pub(super) default_feature_scale_m: f32,
}

impl SurfaceProfile {
    pub(super) const fn for_kind(kind: PhotographicSurfaceKind) -> Self {
        match kind {
            PhotographicSurfaceKind::PolishedMetal => Self::new(
                1.0, 0.09, 0.07, 0.08, 0.018, 0.15, 0.18, 0.35, 0.0, 0.0, 0.000_08,
            ),
            PhotographicSurfaceKind::SatinMetal => Self::new(
                1.0, 0.28, 0.09, 0.09, 0.018, 0.0, 0.16, 0.3, 0.0, 0.0, 0.000_4,
            ),
            PhotographicSurfaceKind::BrushedMetal => Self::new(
                1.0, 0.24, 0.15, 0.24, 0.028, 0.88, 0.32, 0.45, 0.0, 0.0, 0.000_2,
            ),
            PhotographicSurfaceKind::MachinedMetal => Self::new(
                1.0, 0.3, 0.17, 0.2, 0.025, 0.72, 0.35, 0.5, 0.0, 0.0, 0.000_35,
            ),
            PhotographicSurfaceKind::CastMetal => Self::new(
                1.0, 0.56, 0.22, 0.32, 0.045, 0.08, 0.38, 0.65, 0.0, 0.0, 0.001_2,
            ),
            PhotographicSurfaceKind::PaintedMetal => Self::new(
                0.0, 0.32, 0.14, 0.18, 0.035, 0.08, 0.28, 0.5, 0.45, 0.18, 0.000_65,
            ),
            PhotographicSurfaceKind::PowderCoatedMetal => Self::new(
                0.0, 0.46, 0.19, 0.3, 0.045, 0.05, 0.38, 0.58, 0.0, 0.0, 0.000_45,
            ),
            PhotographicSurfaceKind::MoldedPlastic => Self::new(
                0.0, 0.36, 0.15, 0.19, 0.035, 0.04, 0.3, 0.45, 0.0, 0.0, 0.000_8,
            ),
            PhotographicSurfaceKind::ClearcoatPlastic => Self::new(
                0.0, 0.3, 0.1, 0.14, 0.025, 0.03, 0.22, 0.38, 0.85, 0.11, 0.000_7,
            ),
            PhotographicSurfaceKind::Rubber => Self::new(
                0.0, 0.73, 0.16, 0.25, 0.035, 0.04, 0.38, 0.58, 0.0, 0.0, 0.001_1,
            ),
            PhotographicSurfaceKind::Fabric => Self::new(
                0.0, 0.82, 0.13, 0.34, 0.055, 0.5, 0.55, 0.68, 0.0, 0.0, 0.001_4,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    const fn new(
        metallic: f32,
        roughness: f32,
        roughness_spread: f32,
        height_strength: f32,
        color_variation: f32,
        directionality: f32,
        normal_scale: f32,
        occlusion_strength: f32,
        clearcoat_factor: f32,
        clearcoat_roughness: f32,
        default_feature_scale_m: f32,
    ) -> Self {
        Self {
            metallic,
            roughness,
            roughness_spread,
            height_strength,
            color_variation,
            directionality,
            normal_scale,
            occlusion_strength,
            clearcoat_factor,
            clearcoat_roughness,
            default_feature_scale_m,
        }
    }
}
