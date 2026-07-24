use crate::app::prelude::*;

pub(crate) fn check_q09_structured_adapter_expectations(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "Q09-STRUCTURED-ADAPTER-EXPECTATIONS";
    require_contains(
        root,
        findings,
        RULE,
        "tests/m8_real_asset_proof.rs",
        &[
            "WaterBottleAdapterExpectationKey",
            "GITHUB_MACOS_14_PARAVIRTUAL_METAL_KEY.matches(report)",
            "github-macos-14-apple-paravirtual-metal-v1",
            "portable-physical-gpu-v1",
            "2239bbb25313877e32dd5431fdae14660608257a4c11c60c383804fecbf6285f",
            "waterbottle_adapter_expectations_ignore_display_name_and_reject_label_spoofing",
            "waterbottle_adapter_profiles_pin_reviewed_samples_at_tolerance_25",
            "portable_waterbottle_regions_leave_full_frame_tolerance_headroom",
            "tolerance: 25",
        ],
    );
    if let Ok(text) = fs::read_to_string(root.join("tests/m8_real_asset_proof.rs"))
        && (text.contains("gpu_adapter_label.contains(\"Apple Paravirtual device\")")
            || text.contains("report.name.contains(\"Apple Paravirtual device\")")
            || text.contains("body_olive_tolerance"))
    {
        findings.push(Finding::new(
            RULE,
            "WaterBottle acceptance must not select tolerance from a free-form adapter display name",
        ));
    }
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/release/waterbottle_results.rs",
        &[
            "validate_waterbottle_adapter_expectation",
            "scena.m8.waterbottle_adapter_expectation.v1",
            "does not match structured profile",
            "reviewed Chebyshev-25 sample",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/build.rs",
        &[
            "wgpu::Backends::all().with_env()",
            "native_instance_descriptor(backends)",
            "native_instance_honors_wgpu_backend_filter",
            "headless_gpu_uses_the_filtered_native_instance_path",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/release/windows_complete_hardware_proof_validation.js",
        &[
            "scena.m8.waterbottle_adapter_expectation.v1",
            "portable-physical-gpu-v1",
            "reviewed Chebyshev-25 sample",
            "normalizedBackend(adapter.backend) === \"dx12\"",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
        &[
            "github-macos-14-apple-paravirtual-metal-v1",
            "structured adapter key",
            "expires_at = \"2026-10-31\"",
        ],
    );
}
