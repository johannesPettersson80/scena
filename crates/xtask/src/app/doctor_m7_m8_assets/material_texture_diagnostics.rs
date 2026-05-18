use crate::app::prelude::*;

pub(crate) fn check_material_texture_diagnostic_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/scene_loading.rs",
        &[
            "external_image_paths",
            "AssetLoadWarning::ExternalImageMissing",
            "options.strict_textures()",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/load.rs",
        &[
            "AssetLoadOptions",
            "strict_textures",
            "AssetLoadWarning",
            "ExternalImageMissing",
            "warnings",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/render/prepare/resources.rs",
        &[
            "collect_material_texture_diagnostics",
            "MaterialTextureMissingDecodedPixels",
            "material_textures_missing_decoded_pixels",
            "has_decoded_pixels",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/diagnostics/diagnostic.rs",
        &["MaterialTextureMissingDecodedPixels"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/diagnostics.rs",
        &["material_textures_missing_decoded_pixels"],
    );
}
