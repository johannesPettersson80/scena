//! The surface finish the photographic solver gives a material whose model
//! says nothing about how the part was made.

use crate::{MaterialDesc, PhotographicSurfaceKind, SurfaceFinish};

/// Subject radius, in metres, at which an inferred finish has its real size.
const REFERENCE_RADIUS_M: f32 = 0.15;

/// Gives `material` its finish for a subject of `subject_radius` metres: the
/// one its model states, or else the `inferred` kind.
///
/// A photograph frames its subject to fill the picture whatever its size,
/// and models are often not built to scale, so an inferred finish is sized
/// to the subject: a part twice as large gets grain twice as coarse, and the
/// picture reads the same. A finish the model states is left as stated.
pub(super) fn give(
    material: MaterialDesc,
    inferred: PhotographicSurfaceKind,
    subject_radius: f32,
) -> MaterialDesc {
    if material.surface_finish().is_some() {
        return material;
    }
    let scale = (subject_radius / REFERENCE_RADIUS_M).clamp(0.2, 8.0);
    material.with_surface_finish(SurfaceFinish::new(inferred).with_scale(scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_inferred_finish_is_sized_to_the_subject_and_a_stated_one_is_kept() {
        let cast = PhotographicSurfaceKind::CastMetal;
        let finish = |material: MaterialDesc, radius: f32| {
            give(material, cast, radius)
                .surface_finish()
                .expect("a finish is given")
        };
        let plain = MaterialDesc::default;
        assert_eq!(finish(plain(), 0.15), SurfaceFinish::new(cast));
        assert_eq!(finish(plain(), 0.6).scale(), 4.0);
        assert_eq!(finish(plain(), 0.001).scale(), 0.2);
        assert_eq!(finish(plain(), 90.0).scale(), 8.0);
        let stated = SurfaceFinish::new(PhotographicSurfaceKind::BrushedMetal).with_wear(0.5);
        assert_eq!(finish(plain().with_surface_finish(stated), 0.6), stated);
    }
}
