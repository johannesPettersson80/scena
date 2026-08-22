use crate::app::prelude::*;

pub(crate) fn assert_scene_host_capture_readback_is_enforced(
    fixture_root: &Path,
    findings: &mut Vec<Finding>,
) {
    let scene_host_capture = fixture_root.join("src/scene_host/capture.rs");
    let source = fs::read_to_string(&scene_host_capture).expect("read SceneHost capture fixture");
    let mutated = source.replacen(
        "RenderReadbackMode::Synchronous",
        "RenderReadbackMode::PresentOnly",
        1,
    );
    assert_ne!(
        source, mutated,
        "SceneHost capture mutation must disable explicit synchronous readback"
    );
    fs::write(&scene_host_capture, mutated).expect("disable SceneHost capture readback");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(fixture_root, findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09"
                && finding.message.contains("RenderReadbackMode::Synchronous")
        }),
        "doctor must reject SceneHost capture without an explicit readback render: {findings:?}",
    );
    fs::write(&scene_host_capture, source).expect("restore SceneHost capture fixture");
}

pub(crate) fn assert_browser_backend_selectors_are_enforced(
    fixture_root: &Path,
    findings: &mut Vec<Finding>,
) {
    let browser_selector = fixture_root.join("tests/browser/hardware_browser.js");
    let source = fs::read_to_string(&browser_selector).expect("read browser selector fixture");
    let mutated = source.replacen("gfx.webgpu.force-enabled", "removed-webgpu-force-enable", 1);
    assert_ne!(
        source, mutated,
        "browser selector mutation must remove the Firefox WebGPU preference"
    );
    fs::write(&browser_selector, mutated).expect("remove Firefox WebGPU selector preference");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(fixture_root, findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09" && finding.message.contains("gfx.webgpu.force-enabled")
        }),
        "doctor must reject loss of the per-backend Firefox WebGPU route: {findings:?}",
    );
    fs::write(&browser_selector, &source).expect("restore Firefox WebGPU selector preference");

    let mutated = source.replace("platform === \"linux\"", "platform === \"win32\"");
    assert_ne!(
        source, mutated,
        "Windows Chromium backend mutation must alter the platform selector"
    );
    fs::write(&browser_selector, mutated).expect("force Vulkan flags onto Windows Chromium");
    findings.clear();
    check_c09_gpu_resource_lifecycle_contracts(fixture_root, findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RENDER-C09" && finding.message.contains("platform === \"linux\"")
        }),
        "doctor must reject routing Windows Chromium hardware proof through Vulkan flags: {findings:?}",
    );
}
