use crate::app::prelude::*;

pub(crate) fn check_full_review_surface_acquisition_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    const RULE: &str = "C12-SURFACE-ACQUISITION";

    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/surface_frame.rs",
        &[
            "SurfaceAcquireStatus",
            "SurfaceAcquisitionPolicy",
            "retry_consumed",
            "SurfaceAcquireAction::ReconfigureAndRetry",
            "SurfaceAcquireAction::FailAfterRetry(status)",
            "SurfaceAcquireAction::FailLost",
            "SurfaceAcquireAction::SkipTimeout",
            "SurfaceAcquireAction::SkipOccluded",
            "SurfaceAcquireAction::FailValidation",
            "SurfaceAcquireAction::FailOutOfMemory",
            "refresh_surface_configuration",
            "get_default_config",
            "present_mode_changed",
            "install_gpu_error_callback",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/render/gpu/surface_frame.rs",
        &[
            "outdated_surface_reconfigures_and_retries_exactly_once",
            "lost_surface_requires_host_recreation_without_fake_retry",
            "timeout_and_occlusion_are_diagnostic_skips",
            "validation_and_out_of_memory_are_hard_failures",
            "runtime_fault_channel_preserves_validation_and_oom",
            "suboptimal_frame_is_presented_then_reconfigured",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/draw.rs",
        &[
            "surface_frame::acquire_surface_frame",
            "if surface_skip.is_some()",
            "surface_reconfigurations",
            "surface_acquire_retries",
            "reconfigure_existing_surface",
        ],
    );
    forbid_contains(
        root,
        findings,
        RULE,
        "src/render/gpu/draw.rs",
        &["wgpu::CurrentSurfaceTexture::Validation => None"],
    );
    for path in [
        "src/render/gpu/draw_surface.rs",
        "src/render/gpu/draw_surface_support.rs",
    ] {
        require_contains(
            root,
            findings,
            RULE,
            path,
            &[
                "surface_frame::acquire_surface_frame",
                "surface_skip",
                "surface_reconfigurations",
                "surface_acquire_retries",
            ],
        );
        forbid_contains(
            root,
            findings,
            RULE,
            path,
            &["wgpu::CurrentSurfaceTexture::Validation => return Ok"],
        );
    }
    require_contains(
        root,
        findings,
        RULE,
        "src/render/frame.rs",
        &[
            "surface::record_surface_result",
            "self.surface_lost = Some(recoverable)",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/frame/surface.rs",
        &[
            "result.surface_skip",
            "surface_timeout_skips",
            "surface_occluded_skips",
            "skipped: true",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/diagnostics.rs",
        &[
            "SurfaceOutdated",
            "SurfaceConfigurationChanged",
            "GpuValidation",
            "GpuOutOfMemory",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/diagnostics/stats.rs",
        &[
            "surface_timeout_skips",
            "surface_occluded_skips",
            "surface_reconfigurations",
            "surface_acquire_retries",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene_host/reporting.rs",
        &["surface_timeout_skips", "surface_acquire_retries"],
    );
    for (path, needle) in [
        ("README.md", "latches `Lost` for surface recreation"),
        ("docs/lifecycle.md", "## Attached surface acquisition"),
        (
            "docs/platforms.md",
            "refreshes surface configuration and retries acquisition",
        ),
        ("docs/errors.md", "`RenderError::SurfaceOutdated`"),
        ("docs/api.md", "`RendererStats::surface_timeout_skips`"),
        (
            "CHANGELOG.md",
            "Reconfigure and retry attached native/browser `Outdated`",
        ),
        (
            "docs/release-notes/v1.8.0.md",
            "published v1.8.0 native surface path silently ignored",
        ),
    ] {
        require_contains(root, findings, RULE, path, &[needle]);
    }
}
