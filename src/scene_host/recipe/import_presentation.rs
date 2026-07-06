use std::collections::BTreeMap;

use super::{SceneHostCore, error_diagnostic, has_errors, scene_host_error_diagnostic};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildTargetV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeImportV1,
};
use crate::scene_host::recipe::authoring::{DiagnosticPathExt, authored_color};
use crate::{Color, MaterialDesc, MaterialHandle};

pub(super) fn apply_import_presentation(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    import: &SceneRecipeImportV1,
    root_handles: &[u64],
    import_path: &str,
    generated_nodes: &mut Vec<SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let material_handle = import_material_handle(host, colors, import, import_path, diagnostics);
    let edge_material_handle =
        import_edge_material_handle(host, colors, import, import_path, diagnostics);
    if has_errors(diagnostics) {
        return;
    }
    for (root_index, root_handle) in root_handles.iter().enumerate() {
        let root = match host.resolve_node(*root_handle) {
            Ok(root) => root,
            Err(error) => {
                diagnostics.push(scene_host_error_diagnostic(
                    import_path,
                    "import_presentation_failed",
                    error,
                ));
                continue;
            }
        };
        if let Some(material) = material_handle
            && let Err(error) = host.scene.set_subtree_mesh_material(root, material)
        {
            diagnostics.push(error_diagnostic(
                import_path,
                "import_material_failed",
                format!("failed to apply import material override: {error}"),
                "check that the import root still resolves to imported mesh nodes",
            ));
        }
        let edge_nodes = match edge_material_handle {
            Some(edge_material) => {
                match host.scene.add_subtree_edge_overlays(root, edge_material) {
                    Ok(nodes) => nodes,
                    Err(error) => {
                        diagnostics.push(error_diagnostic(
                            import_path,
                            "import_edge_emphasis_failed",
                            format!("failed to add import edge overlays: {error}"),
                            "check that the import root still resolves to imported mesh nodes",
                        ));
                        Vec::new()
                    }
                }
            }
            None => Vec::new(),
        };
        for (overlay_index, overlay_node) in edge_nodes.into_iter().enumerate() {
            let handle = host.register_node(overlay_node);
            generated_nodes.push(SceneRecipeBuildTargetV1 {
                id: format!("{}.edge_emphasis.{root_index}.{overlay_index}", import.id),
                handle,
                kind: "generated_overlay".to_owned(),
                parent: Some(*root_handle),
                name: None,
                active: None,
            });
        }
    }
}

fn import_material_handle(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    import: &SceneRecipeImportV1,
    import_path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<MaterialHandle> {
    let material = import.material.as_ref()?;
    let base_color = import_presentation_color(
        colors,
        &material.base_color,
        format!("{import_path}.material.base_color"),
        diagnostics,
    )?;
    let material = MaterialDesc::pbr_metallic_roughness(
        base_color,
        material.metallic.unwrap_or(0.0) as f32,
        material.roughness.unwrap_or(1.0) as f32,
    );
    Some(host.assets.create_material(material))
}

fn import_edge_material_handle(
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
        Some(color) => import_presentation_color(
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

fn import_presentation_color(
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
