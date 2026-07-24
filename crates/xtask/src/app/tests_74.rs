use crate::app::prelude::*;

#[test]
fn doctor_rejects_renderer_stats_missing_required_counters_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/renderer-stats-stub");
    let diagnostics_path = fixture_root.join("src/diagnostics.rs");
    fs::create_dir_all(diagnostics_path.parent().expect("diagnostics parent"))
        .expect("fixture dir");
    fs::write(
        &diagnostics_path,
        "pub struct RendererStats { pub frames_rendered: u64 }\n",
    )
    .expect("diagnostics fixture");
    let mut findings = Vec::new();
    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-RENDER-STATS",
        "src/diagnostics.rs",
        &[
            "pub struct RendererStats",
            "pub buffers: u64",
            "pub textures: u64",
            "pub materials: u64",
        ],
    );
    assert!(findings.iter().any(|finding| {
        finding.rule == "ARCH-RENDER-STATS" && finding.message.contains("pub buffers: u64")
    }));
}

#[test]
fn doctor_rejects_camera_depth_missing_perspective_camera_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/camera-depth-stub");
    let camera_path = fixture_root.join("src/scene/camera.rs");
    fs::create_dir_all(camera_path.parent().expect("camera parent")).expect("fixture dir");
    fs::write(&camera_path, "pub struct CameraStub {}\n").expect("camera fixture");
    let mut findings = Vec::new();
    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene/camera.rs",
        &[
            "pub enum Camera",
            "pub struct PerspectiveCamera",
            "pub struct OrthographicCamera",
            "pub struct DepthRange",
        ],
    );
    assert!(findings.iter().any(|finding| {
        finding.rule == "ARCH-CAMERA-DEPTH" && finding.message.contains("PerspectiveCamera")
    }));
}

#[test]
fn doctor_rejects_clipping_missing_clipping_plane_key_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/clipping-stub");
    let scene_path = fixture_root.join("src/scene.rs");
    fs::create_dir_all(scene_path.parent().expect("scene parent")).expect("fixture dir");
    fs::write(&scene_path, "pub struct Scene {}\n").expect("scene fixture");
    let mut findings = Vec::new();
    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-CLIPPING",
        "src/scene.rs",
        &["pub struct ClippingPlaneKey"],
    );
    assert!(findings.iter().any(|finding| {
        finding.rule == "ARCH-CLIPPING" && finding.message.contains("ClippingPlaneKey")
    }));
}
