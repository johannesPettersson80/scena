use crate::app::prelude::*;

pub(super) fn check_m8_visual_material_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_visual_proof.rs",
        &[
            "m8-unlit-textured-asset",
            "m8-metallic-roughness-asset",
            "m8-normal-mapped-asset",
            "m8-emissive-asset",
            "m8-alpha-mask",
            "m8-alpha-blend",
            "m8-texture-slots",
            "m8-environment-color-management",
            "m8-clearcoat-material-feature",
            "clearcoat-before-after-cpu-headless-256",
            "m8-sheen-material-feature",
            "sheen-before-after-cpu-headless-256",
            "m8-anisotropy-material-feature",
            "anisotropy-before-after-cpu-headless-256",
            "m8-iridescence-material-feature",
            "iridescence-before-after-cpu-headless-256",
            "m8-dispersion-material-feature",
            "dispersion-before-after-cpu-headless-256",
            "max_luminance_in_region",
            "max_rgb_in_region",
            "(256, 256)",
            "png_rgba8",
            "TextureColorSpace::Srgb",
            "TextureColorSpace::Linear",
        ],
    );
}
