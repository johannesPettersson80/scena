use crate::app::prelude::*;

pub(crate) fn check_reference_image_regression(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "REFERENCE-IMAGE-REGRESSION",
        "src/lib.rs",
        &[
            "pub mod reference_image;",
            "ReferenceImage",
            "ReferenceImageError",
            "ReferenceImageTolerance",
            "regress",
            "regress_with_tolerance",
        ],
    );
    require_contains(
        root,
        findings,
        "REFERENCE-IMAGE-REGRESSION",
        "src/reference_image.rs",
        &[
            "pub struct ReferenceImage",
            "pub struct ReferenceImageTolerance",
            "pub struct ReferenceImageReport",
            "pub enum ReferenceImageError",
            "pub fn regress(",
            "pub fn regress_with_tolerance(",
            "DiffExceeded(ReferenceImageReport)",
        ],
    );
    require_contains(
        root,
        findings,
        "REFERENCE-IMAGE-REGRESSION",
        "tests/reference_image_regression_api.rs",
        &[
            "reference_image_regression_accepts_exact_rgba8_match",
            "reference_image_regression_reports_tolerance_failure",
            "reference_image_regression_rejects_invalid_rgba_length",
            "reference_image_regression_rejects_dimension_mismatch",
        ],
    );
    require_contains(
        root,
        findings,
        "REFERENCE-IMAGE-REGRESSION",
        "docs/api.md",
        &[
            "ReferenceImage::from_rgba8",
            "regress",
            "regress_with_tolerance",
            "ReferenceImageError",
        ],
    );
    require_contains(
        root,
        findings,
        "REFERENCE-IMAGE-REGRESSION",
        "docs/guides/easy-scene-setup.md",
        &[
            "Reference-image regression",
            "ReferenceImage::from_rgba8",
            "regress_with_tolerance",
            "ReferenceImageTolerance::new().with_max_abs_diff",
        ],
    );
    require_contains(
        root,
        findings,
        "REFERENCE-IMAGE-REGRESSION",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "Reference-image regression as a public API",
            "Status:\n  **[shipped]**",
            "ReferenceImage::from_rgba8",
            "REFERENCE-IMAGE-REGRESSION",
        ],
    );
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::check_reference_image_regression;
    use crate::app::doctor_core::repo_root;

    #[test]
    fn reference_image_regression_rule_rejects_missing_public_api() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let fixture_root = root.join("target/xtask-doctor-regressions/reference-image-regression");
        let _ = fs::remove_dir_all(&fixture_root);
        for dir in ["src", "tests", "docs/guides", "docs/checklists"] {
            fs::create_dir_all(fixture_root.join(dir)).expect("fixture dir");
        }
        fs::write(fixture_root.join("src/lib.rs"), "").expect("lib fixture");
        fs::write(fixture_root.join("src/reference_image.rs"), "").expect("reference fixture");
        fs::write(
            fixture_root.join("tests/reference_image_regression_api.rs"),
            "",
        )
        .expect("test fixture");
        fs::write(fixture_root.join("docs/api.md"), "").expect("api fixture");
        fs::write(fixture_root.join("docs/guides/easy-scene-setup.md"), "").expect("guide fixture");
        fs::write(
            fixture_root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md"),
            "",
        )
        .expect("checklist fixture");
        let mut findings = Vec::new();

        check_reference_image_regression(&fixture_root, &mut findings);

        assert!(
            findings
                .iter()
                .any(|finding| finding.rule == "REFERENCE-IMAGE-REGRESSION"),
            "doctor must reject missing public reference-image regression API: {findings:?}",
        );
    }
}
