//! How much a node must grow or shrink to fit a size range.

use crate::geometry::Aabb;

use super::ScenePlacementDiagnosticV1;

/// The uniform scale factor that brings the largest extent of `world_bounds`
/// into `min_size..=max_size`, or one when it is already inside.
pub(super) fn fit_to_size_factor(
    world_bounds: Aabb,
    min_size: Option<f32>,
    max_size: Option<f32>,
) -> Result<f32, Box<ScenePlacementDiagnosticV1>> {
    let extent = world_bounds.max - world_bounds.min;
    let current_size = extent.x.max(extent.y).max(extent.z);
    if !current_size.is_finite() || current_size <= f32::EPSILON {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_bounds",
            "$.imports[].uri",
            "import bounds must have finite non-zero extent for fit_to_size",
            "use an asset with renderable bounds or choose center/ground instead",
        )));
    }

    if min_size.is_none() && max_size.is_none() {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_size_range",
            "$.verb.fit_to_size",
            "fit_to_size requires --min-size, --max-size, or both",
            "pass a target size range for fit_to_size",
        )));
    }
    if min_size.is_some_and(|value| !value.is_finite() || value < 0.0)
        || max_size.is_some_and(|value| !value.is_finite() || value <= 0.0)
        || min_size
            .zip(max_size)
            .is_some_and(|(min_size, max_size)| max_size < min_size)
    {
        return Err(Box::new(ScenePlacementDiagnosticV1::new(
            "invalid_size_range",
            "$.verb.fit_to_size",
            "fit_to_size requires finite non-negative min and positive max with max >= min",
            "pass --min-size and/or --max-size with a valid positive range",
        )));
    }

    let min_size = min_size.unwrap_or(0.0);
    let max_size = max_size.unwrap_or(f32::INFINITY);
    let scale_factor = if current_size > max_size {
        max_size / current_size
    } else if current_size < min_size {
        min_size / current_size
    } else {
        1.0
    };
    Ok(scale_factor)
}
