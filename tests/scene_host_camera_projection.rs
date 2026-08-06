#![cfg(all(feature = "scene-host", not(target_arch = "wasm32")))]

use scena::{
    Camera, OrbitControlAction, SceneHostCameraProjection, SceneHostCameraState, SceneHostCore,
    Vec3,
};

#[test]
fn scene_host_switches_projection_without_losing_viewpoint_and_keeps_orthographic_zoom_live() {
    let mut host = SceneHostCore::headless(240, 120).expect("host builds");
    let viewpoint = SceneHostCameraState {
        target: Vec3::new(1.0, 2.0, 3.0),
        distance: 8.0,
        yaw_radians: 0.4,
        pitch_radians: -0.25,
    };
    host.set_camera(viewpoint).expect("viewpoint applies");

    assert_eq!(
        host.camera_projection().expect("projection is available"),
        SceneHostCameraProjection::Perspective
    );
    host.set_camera_projection(SceneHostCameraProjection::Orthographic)
        .expect("orthographic projection applies");
    assert_eq!(host.get_camera(), viewpoint);

    let active_camera = host
        .scene()
        .active_camera()
        .expect("host has an active camera");
    let before_zoom_height = match host.scene().camera(active_camera) {
        Some(Camera::Orthographic(camera)) => camera.top - camera.bottom,
        camera => panic!("expected orthographic camera, got {camera:?}"),
    };
    assert!(before_zoom_height.is_finite() && before_zoom_height > 0.0);

    assert_eq!(
        host.camera_wheel(120.0, 60.0, -10.0)
            .expect("orthographic wheel zoom applies"),
        OrbitControlAction::Zoom
    );
    let after_zoom_height = match host.scene().camera(active_camera) {
        Some(Camera::Orthographic(camera)) => camera.top - camera.bottom,
        camera => panic!("expected orthographic camera, got {camera:?}"),
    };
    assert!(
        after_zoom_height < before_zoom_height,
        "orthographic wheel zoom must change the visible scale"
    );

    host.resize(120.0, 120.0, 1.0)
        .expect("orthographic viewport resize applies");
    match host.scene().camera(active_camera) {
        Some(Camera::Orthographic(camera)) => {
            let width = camera.right - camera.left;
            let height = camera.top - camera.bottom;
            assert!((width / height - 1.0).abs() < 1.0e-5);
        }
        camera => panic!("expected orthographic camera, got {camera:?}"),
    }

    let zoomed_viewpoint = host.get_camera();
    host.set_camera_projection(SceneHostCameraProjection::Perspective)
        .expect("perspective projection restores");
    assert_eq!(host.get_camera(), zoomed_viewpoint);
    assert_eq!(
        host.camera_projection().expect("projection is available"),
        SceneHostCameraProjection::Perspective
    );
    assert!(matches!(
        host.scene().camera(active_camera),
        Some(Camera::Perspective(_))
    ));
}
