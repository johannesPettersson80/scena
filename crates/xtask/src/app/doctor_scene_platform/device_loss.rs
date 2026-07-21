use crate::app::prelude::*;

pub(crate) fn check_c11_terminal_device_loss_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "C11-TERMINAL-DEVICE-LOSS";

    require_contains(
        root,
        findings,
        RULE,
        "src/diagnostics.rs",
        &["GpuDeviceRebuildRequired", "recoverable: bool"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "package.json",
        &["browser:c11-lifecycle", "--lifecycle-only"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/diagnostics/help.rs",
        &[
            "Self::GpuDeviceRebuildRequired",
            "recreate the Renderer",
            "a lost wgpu Device/Queue cannot be reused",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/surface.rs",
        &[
            "if let Some(recoverable) = self.device_lost",
            "PrepareError::GpuDeviceRebuildRequired",
            "pub(super) fn prepare_device_ready",
            "self.device_lost = None;",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/render/prepare_lifecycle.rs",
        &[
            "self.prepare_device_ready()?;",
            "gpu.poll_device_nonblocking();",
            "GPU_PREPARE_DESTRUCTION_PRESSURE_LIMIT",
        ],
    );
    if let Ok(source) = fs::read_to_string(root.join("src/render/prepare_lifecycle.rs")) {
        let guard = source.find("self.prepare_device_ready()?;");
        let poll = source.find("gpu.poll_device_nonblocking();");
        if !matches!((guard, poll), (Some(guard), Some(poll)) if guard < poll) {
            findings.push(Finding::new(
                RULE,
                "prepare must reject latched device loss before polling or allocating on the dead device",
            ));
        }
    }
    if let Ok(source) = fs::read_to_string(root.join("src/render/surface.rs")) {
        let terminal = source.find("if let Some(recoverable) = self.device_lost");
        let retention = source.find("if assets.retain_policy() == RetainPolicy::Never");
        if !matches!((terminal, retention), (Some(terminal), Some(retention)) if terminal < retention)
        {
            findings.push(Finding::new(
                RULE,
                "recover_context must report terminal device loss before context-retention handling",
            ));
        }
    }
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c11_device_loss_recovery.rs",
        &[
            "injected_device_loss_requires_rebuild_and_rejects_prepare_and_render",
            "repeated_device_loss_never_clears_the_terminal_state",
            "device_loss_before_first_prepare_is_rejected_at_the_prepare_boundary",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/browser_probe/probes.rs",
        &[
            "verify_device_rebuild_required",
            "device_rebuild_required",
            "device_rebuilt",
            "device_recovered\": null",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "device-rebuild-required",
            "prepare-blocked-after-device-loss",
            "result.device_recovered !== null",
            "lifecycleOnly || viewerElementOnly || forceBrowserPackage",
            "proof_class: \"synthetic-headless-browser\"",
            "--lifecycle-only",
        ],
    );
    for (path, needle) in [
        (
            "docs/lifecycle.md",
            "`SurfaceEvent::DeviceLost` is terminal",
        ),
        (
            "docs/browser.md",
            "loss is different: browser wgpu Device/Queue objects are terminal",
        ),
        ("docs/errors.md", "`PrepareError::GpuDeviceRebuildRequired`"),
        ("CHANGELOG.md", "Treat wgpu device loss as terminal"),
        (
            "docs/release-notes/v1.8.0.md",
            "retaining the terminal wgpu Device/Queue",
        ),
    ] {
        require_contains(root, findings, RULE, path, &[needle]);
    }
}
