use crate::app::prelude::*;

pub(super) fn check_khronos_samples(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "KHRONOS-SAMPLES",
        "Cargo.toml",
        &["khronos-samples = []"],
    );
    require_contains(
        root,
        findings,
        "KHRONOS-SAMPLES",
        "src/assets.rs",
        &[
            "mod khronos;",
            "KhronosSample",
            "KhronosSampleMetadata",
            "KhronosSamples",
        ],
    );
    require_contains(
        root,
        findings,
        "KHRONOS-SAMPLES",
        "src/assets/khronos.rs",
        &[
            "pub enum KhronosSample",
            "pub const ALL",
            "PACKAGE_SIZE_BUDGET_BYTES",
            "pub fn khronos(&self)",
            "pub async fn water_bottle",
            "pub async fn transmission_test",
            "pub async fn rigged_simple",
            "primary_sha256",
            "license_reference",
        ],
    );
    if fs::read_to_string(root.join("src/assets/khronos.rs"))
        .is_ok_and(|text| text.contains("include_bytes!"))
    {
        findings.push(Finding::new(
            "KHRONOS-SAMPLES",
            "src/assets/khronos.rs must not embed Khronos sample bytes into the library binary",
        ));
    }
    require_contains(
        root,
        findings,
        "KHRONOS-SAMPLES",
        "src/lib.rs",
        &["KhronosSample", "KhronosSampleMetadata", "KhronosSamples"],
    );
    require_contains(
        root,
        findings,
        "KHRONOS-SAMPLES",
        "tests/round_c_khronos_samples.rs",
        &[
            "khronos_sample_catalog_exposes_manifest_metadata_and_package_budget",
            "khronos_sample_loader_loads_every_catalog_entry_without_user_paths",
            "khronos_sample_loader_has_named_shortcuts_for_headline_assets",
            "khronos_sample_loader_renders_rigged_sample_reference_artifact",
            "rigged-simple-sample-loader-reference.ppm",
        ],
    );
    require_contains(
        root,
        findings,
        "KHRONOS-SAMPLES",
        "docs/guides/easy-scene-setup.md",
        &[
            "khronos-samples",
            "assets.khronos().water_bottle().await?",
            "KhronosSample::ALL",
        ],
    );
    require_contains(
        root,
        findings,
        "KHRONOS-SAMPLES",
        "docs/feature-flags.md",
        &["khronos-samples", "Khronos glTF sample-asset catalog"],
    );
}
