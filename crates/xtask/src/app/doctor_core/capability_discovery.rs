use crate::app::prelude::*;

pub(crate) fn check_a03_live_capability_discovery(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "A03-LIVE-CAPABILITY-DISCOVERY";

    for (path, needles) in [
        (
            "src/bin/scena.rs",
            &[
                "scena_capabilities::run_capabilities_command(rest)",
                "\"browser_probe\": cfg!(feature = \"browser-probe\")",
                "\"hot_reload\": cfg!(feature = \"hot-reload\")",
                "\"production_assets\": cfg!(feature = \"production-assets\")",
                "\"scene_host\": cfg!(feature = \"scene-host\")",
            ][..],
        ),
        (
            "src/bin/scena/capabilities.rs",
            &[
                "CapabilityProbeStatusV1::StaticNoDevice",
                "CapabilityProbeStatusV1::Unavailable",
                "source: \"compiled_backend_table\".to_owned()",
                "source: \"live_wgpu_adapter_request\".to_owned()",
                "Renderer::headless_gpu(1, 1)",
                "failed to serialize unavailable capability report",
            ][..],
        ),
        (
            "src/render/gpu/lifecycle.rs",
            &[
                "source: \"live_wgpu_adapter\".to_owned()",
                "features: format!(\"{:?}\", self.device.features())",
                "self.adapter.get_texture_format_features(format)",
                "self.color_target_format()",
                "wgpu::TextureFormat::Depth32Float",
                "texture_format_supports_sample_count",
                "headless target is configured for COPY_SRC readback",
                "headless probe has no presentation surface",
            ][..],
        ),
        (
            "src/diagnostics/capabilities.rs",
            &["pub probe: Option<CapabilityProbeV1>"][..],
        ),
        (
            "src/diagnostics/capabilities/capability_probe.rs",
            &[
                "pub enum CapabilityProbeModeV1",
                "pub enum CapabilityProbeStatusV1",
                "pub struct GpuDeviceReport",
                "pub struct CapabilityTargetProbeV1",
            ][..],
        ),
        (
            "src/bin/scena/help.rs",
            &[
                "capabilities [--live] [--json]",
                "scena.capability_report.v1",
            ][..],
        ),
        (
            "tests/assets/stable-contracts/capability_report.v1.json",
            &[
                "\"mode\": \"static\"",
                "\"status\": \"static_no_device\"",
                "\"measured\": false",
            ][..],
        ),
        (
            "docs/capabilities.md",
            &[
                "scena capabilities --live --json",
                "static_no_device",
                "live_wgpu_adapter",
                "structured `unavailable`",
            ][..],
        ),
        (
            "docs/guides/llm-app-builder.md",
            &["scena capabilities --live --json", "probe.status"][..],
        ),
        (
            "CHANGELOG.md",
            &["`scena capabilities [--live] [--json]`"][..],
        ),
    ] {
        require_contains(root, findings, RULE, path, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/a03_capabilities_cli.rs",
        &[
            "static_capabilities_are_explicitly_no_device_and_json_alias_matches",
            "live_capabilities_are_measured_or_fail_closed_with_a_structured_reason",
            "cli_version_reports_every_compiled_feature_that_changes_availability",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/stable_contracts.rs",
        &["capability_report_v1_accepts_old_shape_without_post_processing"],
    );
}
