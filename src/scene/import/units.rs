use crate::scene::{SourceUnits, Transform, Vec3};

pub(super) fn convert_marker_units(
    transform: Transform,
    marker_units: SourceUnits,
    import_units: SourceUnits,
) -> Transform {
    let factor = marker_units.meters_per_unit() / import_units.meters_per_unit();
    Transform {
        translation: Vec3::new(
            transform.translation.x * factor,
            transform.translation.y * factor,
            transform.translation.z * factor,
        ),
        rotation: transform.rotation,
        scale: transform.scale,
    }
}
