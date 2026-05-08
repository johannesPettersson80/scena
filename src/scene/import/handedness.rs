use crate::assets::SceneAsset;
use crate::diagnostics::InstantiateError;
use crate::scene::ImportOptions;

pub(super) fn reject_unproven_left_handed_mesh_import(
    scene_asset: &SceneAsset,
    options: ImportOptions,
) -> Result<(), InstantiateError> {
    if options
        .source_coordinate_system()
        .has_negative_determinant()
        && scene_asset
            .nodes()
            .iter()
            .any(|node| !node.meshes().is_empty())
    {
        return Err(InstantiateError::UnsupportedCoordinateSystem {
            coordinate_system: options.source_coordinate_system(),
            reason: "left-handed mesh imports require explicit winding and normal correction proof"
                .to_string(),
        });
    }
    Ok(())
}
