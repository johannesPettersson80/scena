use crate::app::prelude::*;

pub(super) fn check_named_light_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "NAMED-LIGHT-PRESETS",
        "src/scene/lights.rs",
        &[
            "pub fn sun()",
            "pub fn key_light()",
            "pub fn fill_light()",
            "pub fn rim_light()",
            "pub fn softbox()",
            "pub fn bulb_warm()",
            "pub fn bulb_cool()",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-LIGHT-PRESETS",
        "tests/round_b_light_presets.rs",
        &[
            "named_directional_light_presets_are_public_and_ordered",
            "named_point_light_presets_are_kelvin_tinted_and_range_limited",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-LIGHT-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_b_light_preset_reference_docs_image",
            "round-b-light-preset-reference-docs-image",
            "reference-image+docs-image",
        ],
    );
}

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
            "pub const fn rubber()",
        ],
    );
    if fs::read_to_string(root.join("src/material/presets.rs")).is_ok_and(|text| {
        [
            "pub fn chrome(",
            "pub fn brushed_steel(",
            "pub fn clear_glass(",
            "pub fn frosted_glass(",
            "pub fn leather(",
        ]
        .into_iter()
        .any(|needle| text.contains(needle))
    }) {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            "material presets must not expose chrome/glass/leather names before the renderer supports their visual contract",
        ));
    }
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/round_b_material_presets.rs",
        &["honest_material_presets_are_public_pbr_shortcuts"],
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
}
