use std::collections::BTreeMap;

use super::SceneHostCore;
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeImportV1};
use crate::scene_host::recipe::authoring::{DiagnosticPathExt, authored_color};
use crate::{Color, MaterialDesc, MaterialHandle};

pub(super) fn import_edge_material_handle(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    import: &SceneRecipeImportV1,
    import_path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<MaterialHandle> {
    let edge = import.edge_emphasis.as_ref()?;
    if !edge.enabled {
        return None;
    }
    let base_color = match &edge.base_color {
        Some(color) => presentation_color(
            colors,
            color,
            format!("{import_path}.edge_emphasis.base_color"),
            diagnostics,
        )?,
        None => Color::from_srgb_u8(255, 176, 0),
    };
    let mut material = MaterialDesc::edge(base_color, edge.stroke_width_px.unwrap_or(1.75) as f32);
    if let Some(threshold) = edge.edge_angle_threshold_degrees {
        material = material.with_edge_angle_threshold_degrees(threshold as f32);
    }
    Some(host.assets.create_material(material))
}

pub(super) fn presentation_color(
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    value: &str,
    path: String,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Color> {
    match authored_color(colors, value) {
        Ok(color) => Some(color),
        Err(diagnostic) => {
            diagnostics.push((*diagnostic).with_path(path));
            None
        }
    }
}
