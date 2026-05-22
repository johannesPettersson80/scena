use crate::app::prelude::*;

pub(super) fn check_honest_material_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/material/presets.rs",
        &[
            "pub const fn matte(",
            "pub const fn plastic(",
            "pub const fn metal(",
            "pub const fn rough_metal(",
            "pub const fn chrome()",
            "pub const fn brushed_steel()",
            "pub const fn clearcoat_plastic(",
            "pub const fn satin(",
            "pub const fn leather(",
            "pub const fn clear_glass(",
            "pub const fn frosted_glass(",
            "pub const fn rubber()",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/round_b_material_presets.rs",
        &[
            "honest_material_presets_are_public_pbr_shortcuts",
            "expanded_material_presets_use_only_backed_material_lanes",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_b_material_preset_reference_docs_image",
            "round-b-material-preset-reference-docs-image",
            "reference-image+docs-image",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "docs/guides/easy-scene-setup.md",
        &[
            "MaterialDesc::matte(",
            "MaterialDesc::plastic(",
            "MaterialDesc::metal(",
            "MaterialDesc::rough_metal(",
            "MaterialDesc::chrome()",
            "MaterialDesc::brushed_steel()",
            "MaterialDesc::clearcoat_plastic(",
            "MaterialDesc::satin(",
            "MaterialDesc::leather(",
            "MaterialDesc::clear_glass(",
            "MaterialDesc::frosted_glass(",
            "MaterialDesc::rubber()",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/browser_probe/workflows/pbr/material_presets.rs",
        &[
            "material_presets_scene",
            "browser-pbr-material-preset-expanded-set",
            "webgl2_smooth_metal_sample_floor",
            "blend-plus-transmission-preview-no-refraction-claim",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "assertMaterialPresetProof",
            "pbr-material-presets",
            "webgl2_smooth_metal_sample_floor < 96",
        ],
    );
    require_contains(
        root,
        findings,
        "WEBGL2-IBL-SMOOTH-METAL",
        "src/render/prepare/environment_prefilter.rs",
        &[
            "sample_count_for_roughness(0.28, EnvironmentPrefilterQuality::InteractiveWebGl2)",
            "2 => 96",
            "_ => 192",
        ],
    );
}
