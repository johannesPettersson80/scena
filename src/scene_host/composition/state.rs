use serde_json::json;

use super::checks::{checked_check, error_check, observed_pairs};
use crate::{
    SceneCompositionCheckV1, SceneInspectionReportV1, SceneRecipeBuildV1, SceneRecipeExpectV1,
};

pub(super) fn composition_state_checks(
    manifest: &SceneRecipeBuildV1,
    inspection: &SceneInspectionReportV1,
    expect: Option<&SceneRecipeExpectV1>,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    let Some(expect) = expect else {
        return checks;
    };
    for expected in &expect.expect_state {
        let id = format!("expect_state.{}", expected.id);
        let Some(import) = manifest
            .imports
            .iter()
            .find(|import| import.id == expected.import)
        else {
            checks.push(error_check(
                id,
                "state_variant",
                "state_import_missing",
                Some(expected.id.clone()),
                Vec::new(),
                observed_pairs([("import", json!(expected.import))]),
                (
                    "state expectation references an import missing from the build manifest",
                    "declare the import or update expect_state.import",
                ),
            ));
            continue;
        };
        let Some(inspection_import) = inspection.imports.as_ref().and_then(|imports| {
            imports
                .iter()
                .find(|candidate| candidate.handle == import.import_handle)
        }) else {
            checks.push(error_check(
                id,
                "state_variant",
                "state_import_not_inspected",
                Some(expected.id.clone()),
                vec![import.import_handle],
                observed_pairs([
                    ("import", json!(expected.import)),
                    ("import_handle", json!(import.import_handle)),
                ]),
                (
                    "state expectation import is absent from the inspection report",
                    "rerun inspection after building the recipe or remove the stale expectation",
                ),
            ));
            continue;
        };
        let expected_variant = expected.active_material_variant.as_deref();
        let actual_variant = inspection_import.active_variant.as_deref();
        let observed = observed_pairs([
            ("import", json!(expected.import)),
            ("expected_active_material_variant", json!(expected_variant)),
            ("actual_active_material_variant", json!(actual_variant)),
            (
                "available_material_variants",
                json!(inspection_import.material_variants),
            ),
        ]);
        if expected_variant == actual_variant {
            checks.push(checked_check(
                id,
                "state_variant",
                "material_variant_state_satisfied",
                Some(expected.id.clone()),
                vec![import.import_handle],
                observed,
                (
                    "active material variant state satisfies the declared expectation",
                    "no action needed",
                ),
            ));
        } else {
            checks.push(error_check(
                id,
                "state_variant",
                "material_variant_state_mismatch",
                Some(expected.id.clone()),
                vec![import.import_handle],
                observed,
                (
                    "active material variant state does not match the declared expectation",
                    "apply the intended material variant before rendering or update expect_state",
                ),
            ));
        }
    }
    checks
}
