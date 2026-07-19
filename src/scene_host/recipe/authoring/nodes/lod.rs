use std::collections::BTreeMap;

use crate::GeometryHandle;
use crate::scene::MeshLodLevel;
use crate::scene::recipe::{SceneRecipeDiagnosticV1, SceneRecipeNodeV1};

use super::error_diagnostic;

pub(super) fn resolve_lod_levels(
    recipe: &SceneRecipeNodeV1,
    geometries: &BTreeMap<String, GeometryHandle>,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Vec<MeshLodLevel>> {
    let mut levels = Vec::new();
    for (index, lod) in recipe.lods.iter().enumerate() {
        let lod_path = format!("{path}.lods[{index}]");
        let Some(geometry) = geometries.get(&lod.geometry).copied() else {
            diagnostics.push(error_diagnostic(
                &lod_path,
                "unknown_geometry_ref",
                format!(
                    "LOD level for node '{}' references missing geometry '{}'",
                    recipe.id, lod.geometry
                ),
                "declare the LOD geometry before the node",
            ));
            return None;
        };
        if !lod.max_screen_fraction.is_finite()
            || lod.max_screen_fraction <= 0.0
            || lod.max_screen_fraction > 1.0
        {
            diagnostics.push(error_diagnostic(
                format!("{lod_path}.max_screen_fraction"),
                "invalid_lod_threshold",
                "LOD max_screen_fraction must be finite and in (0, 1]",
                "use a fraction such as 0.15 for distant or small-on-screen geometry",
            ));
            return None;
        }
        levels.push(MeshLodLevel::new(lod.max_screen_fraction as f32, geometry));
    }
    Some(levels)
}
