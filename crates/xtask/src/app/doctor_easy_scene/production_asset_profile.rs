use crate::app::prelude::*;

pub(super) fn check_production_asset_profile(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "Cargo.toml",
        &[
            "default = []",
            "ktx2 = [\"dep:ktx2\", \"dep:basisu_c_sys\"]",
            "meshopt = [\"dep:meshopt\"]",
            "production-assets = [\"ktx2\", \"meshopt\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "docs/feature-flags.md",
        &[
            "`production-assets`",
            "enables `ktx2` + `meshopt`",
            "features = [\"production-assets\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "tests/production_asset_profile.rs",
        &["production_asset_profile_enables_compressed_asset_decoders_without_default_bloat"],
    );
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "tests/m8_compressed_asset_release_proof.rs",
        &[
            "m8_ktx2_material_role_visual_rows_write_release_artifacts",
            "m8_meshopt_visual_rows_write_release_artifacts",
            "m8_ext_mesh_gpu_instancing_visual_row_writes_release_artifacts",
            "m8_compressed_native_gpu_lane_records_fail_closed_unavailable_artifact",
            "scena.compressed_asset_visual_proof.v1",
        ],
    );
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "Production-grade asset pipeline complete and production-profile ready",
            "Status: **[shipped]** for the production profile",
            "tests/m8_compressed_asset_release_proof.rs",
            "target/gate-artifacts/m8-compressed-assets",
        ],
    );
}
