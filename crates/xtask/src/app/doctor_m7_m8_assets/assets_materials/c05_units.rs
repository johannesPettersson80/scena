use crate::app::prelude::*;

pub(crate) fn check_c05_unit_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ASSETS-C05";
    let required: &[(&str, &[&str])] = &[
        (
            "src/scene/import/options.rs",
            &[
                "fn unit_root_transform",
                "Transform::IDENTITY.scale_by(self.source_units.meters_per_unit())",
                "AnimationTarget::Translation",
                "AnimationTarget::Scale",
                ".convert_scale(value)",
            ],
        ),
        (
            "src/scene/import.rs",
            &[
                "options.unit_root_transform()",
                "let source_parent = unit_root.unwrap_or(parent)",
                "if unit_root.is_none()",
                "import.roots.push(unit_root)",
                "convert_marker_units",
            ],
        ),
        (
            "src/scene/import/units.rs",
            &[
                "marker_units.meters_per_unit() / import_units.meters_per_unit()",
                "rotation: transform.rotation",
                "scale: transform.scale",
            ],
        ),
        (
            "docs/guides/units-axes-handedness.md",
            &[
                "one synthetic import placement root",
                "remain source-local beneath it",
                "converted to meters exactly once",
                "animation scale keys remain",
                "marker locals to meters",
            ],
        ),
        (
            "docs/assets.md",
            &[
                "Import unit boundary",
                "one synthetic placement root",
                "animation scale keys stay",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c05_import_unit_contracts.rs",
        &[
            "nested_non_meter_imports_apply_units_once_to_translations_bounds_and_scale",
            "non_meter_import_scale_animation_remains_dimensionless",
            "nested_inherited_and_explicit_marker_units_align_in_world_space",
            "marker_locals_stay_in_import_units_until_the_single_unit_root",
        ],
    );

    let options_path = "src/scene/import/options.rs";
    if let Ok(source) = fs::read_to_string(root.join(options_path)) {
        let boundary_uses = source
            .matches("self.source_units.meters_per_unit()")
            .count();
        if boundary_uses != 1 {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{options_path} must apply meters_per_unit exactly once at the synthetic unit root; found {boundary_uses} applications"
                ),
            ));
        }

        for forbidden in [
            "scale_vec3(transform.translation",
            "scale_vec3(transform.scale",
            "scale_vec3(value",
        ] {
            if source.contains(forbidden) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{options_path} contains forbidden per-value unit conversion `{forbidden}`"
                    ),
                ));
            }
        }
    }
}
