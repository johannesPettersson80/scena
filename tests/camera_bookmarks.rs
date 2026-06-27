use std::f32::consts::PI;

use scena::{
    Aabb, CameraBookmark, CameraState, FramingOptions, OrbitControls, PerspectiveCamera, Scene,
    Transform, TransitionEasing, Vec3,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-4,
        "expected {actual} to be close to {expected}"
    );
}

fn assert_vec_close(actual: Vec3, expected: Vec3) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.z, expected.z);
}

fn assert_state_close(actual: CameraState, expected: CameraState) {
    assert_vec_close(actual.target, expected.target);
    assert_close(actual.distance, expected.distance);
    assert_close(actual.yaw_radians, expected.yaw_radians);
    assert_close(actual.pitch_radians, expected.pitch_radians);
}

#[test]
fn camera_bookmark_from_framing_preserves_state_bounds_and_description() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::IDENTITY,
        )
        .expect("camera inserts");
    let bounds = Aabb::new(Vec3::new(-1.0, -0.5, -0.25), Vec3::new(1.0, 0.5, 0.25));
    let framing = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .azimuth_elevation(-32.0, 14.0)
                .viewport(1280, 720),
        )
        .expect("bounds frame");

    let bookmark = CameraBookmark::from_framing("pump_detail", framing)
        .with_target_bounds(bounds)
        .with_description("service-side pump detail");

    assert_eq!(bookmark.name(), "pump_detail");
    assert_eq!(bookmark.description(), Some("service-side pump detail"));
    assert_eq!(bookmark.target_bounds(), Some(bounds));
    assert_state_close(bookmark.state(), CameraState::from_framing(framing));
}

#[test]
fn orbit_controls_fly_to_samples_shortest_path_and_zero_duration() {
    let start = OrbitControls::new(Vec3::ZERO, 4.0).with_angles(PI - 0.1, 0.0);
    let target = CameraState {
        target: Vec3::new(1.0, 2.0, 3.0),
        distance: 6.0,
        yaw_radians: -PI + 0.1,
        pitch_radians: 0.2,
    };

    let mut fly_to = start
        .fly_to(target, TransitionEasing::Linear, 1.0)
        .expect("fly-to builds");
    let halfway = fly_to.advance(0.5);

    assert!(!fly_to.is_complete());
    assert_vec_close(halfway.target(), Vec3::new(0.5, 1.0, 1.5));
    assert_close(halfway.distance(), 5.0);
    assert!(
        (halfway.yaw_radians() - PI).abs() <= 1.0e-4,
        "fly-to should cross the +/-pi wrap on the short arc, got {}",
        halfway.yaw_radians()
    );
    assert_close(halfway.pitch_radians(), 0.1);

    let final_controls = fly_to.advance(0.5);
    assert!(fly_to.is_complete());
    assert_state_close(final_controls.camera_state(), target);

    let zero = start
        .fly_to(target, TransitionEasing::EaseInOut, 0.0)
        .expect("zero-duration fly-to builds");
    assert!(zero.is_complete());
    assert_state_close(zero.sample().camera_state(), target);

    let invalid = start
        .fly_to(
            CameraState {
                distance: 0.0,
                ..target
            },
            TransitionEasing::Linear,
            1.0,
        )
        .expect_err("invalid target state is rejected");
    assert!(invalid.to_string().contains("distance"));
}

#[cfg(feature = "scene-host")]
#[test]
fn scene_host_camera_bookmark_delegates_through_camera_eased_patch() {
    use scena::{
        SceneHostCameraState, SceneHostCore, SceneHostEasing, SceneHostErrorCode,
        VisualPatchResultV1,
    };

    let mut host = SceneHostCore::headless(96, 64).expect("host builds");
    let start = SceneHostCameraState {
        target: Vec3::ZERO,
        distance: 4.0,
        yaw_radians: 0.0,
        pitch_radians: 0.0,
    };
    let target = SceneHostCameraState {
        target: Vec3::new(2.0, 0.0, 0.0),
        distance: 8.0,
        yaw_radians: 1.0,
        pitch_radians: 0.25,
    };
    host.set_camera(start).expect("camera start sets");

    let bookmark = CameraBookmark::new("detail", target);
    let scheduled = host
        .set_camera_bookmark(&bookmark, 2.0, SceneHostEasing::Linear)
        .expect("bookmark schedules through visual patch");
    assert_eq!(scheduled.applied.camera_eased, 1);
    assert!(scheduled.failed.is_empty());
    assert_state_close(host.get_camera(), start);

    host.advance(1.0).expect("transition advances halfway");
    assert_state_close(
        host.get_camera(),
        SceneHostCameraState {
            target: Vec3::new(1.0, 0.0, 0.0),
            distance: 6.0,
            yaw_radians: 0.5,
            pitch_radians: 0.125,
        },
    );

    let json = serde_json::to_string(&bookmark).expect("bookmark serializes");
    let result_json = host
        .set_camera_bookmark_json(&json, 0.0, SceneHostEasing::Linear)
        .expect("bookmark JSON applies");
    let result: VisualPatchResultV1 =
        serde_json::from_str(&result_json).expect("bookmark result JSON parses");
    assert_eq!(result.applied.camera_eased, 1);
    assert_state_close(host.get_camera(), target);

    let bad = CameraBookmark::new(
        "bad",
        SceneHostCameraState {
            distance: 0.0,
            ..target
        },
    );
    let invalid_result = host
        .set_camera_bookmark(&bad, 0.25, SceneHostEasing::Linear)
        .expect("invalid bookmark camera state is reported per patch entry");
    assert_eq!(invalid_result.failed.len(), 1);
    assert_eq!(invalid_result.failed[0].channel, "camera_eased");
    assert_eq!(
        invalid_result.failed[0].code,
        SceneHostErrorCode::InvalidInput
    );
}

#[test]
fn viewer_helpers_store_optional_camera_bookmarks() {
    let overview = CameraBookmark::new(
        "overview",
        CameraState {
            target: Vec3::ZERO,
            distance: 4.0,
            yaw_radians: 0.0,
            pitch_radians: 0.0,
        },
    );
    let detail = CameraBookmark::new(
        "detail",
        CameraState {
            target: Vec3::new(1.0, 0.0, 0.0),
            distance: 2.0,
            yaw_radians: 0.5,
            pitch_radians: 0.2,
        },
    );

    let viewer = pollster::block_on(
        scena::headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .with_camera_bookmark(overview.clone())
            .with_camera_bookmarks([detail.clone()])
            .build(),
    )
    .expect("viewer builds");

    assert_eq!(viewer.camera_bookmarks(), &[overview, detail]);
}
