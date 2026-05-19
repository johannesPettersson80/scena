use scena::{
    Aabb, CameraOrbitUrlState, FramingOptions, OrbitControls, PerspectiveCamera, Scene, Transform,
    Vec3,
};

#[test]
fn camera_orbit_url_state_round_trips_orbit_controls() {
    let controls = OrbitControls::new(Vec3::new(0.25, -0.5, 1.0), 2.5)
        .with_angles((-28.0_f32).to_radians(), 18.0_f32.to_radians())
        .with_distance_limits(0.5, 5.0);

    let state = controls.url_state();
    let query = state.to_query_string();

    assert_eq!(
        query,
        "?camera-orbit=-28deg%2018deg%202.5m&camera-target=0.25m%20-0.5m%201m"
    );

    let decoded = CameraOrbitUrlState::from_url_query(&query).expect("url state parses");
    assert_state_close(decoded, state);

    let restored = OrbitControls::new(Vec3::ZERO, 1.0)
        .with_url_state(decoded)
        .expect("state applies to orbit controls");
    assert_vec_close(restored.target(), Vec3::new(0.25, -0.5, 1.0));
    assert_close(restored.distance(), 2.5);
    assert_close(restored.yaw_radians().to_degrees(), -28.0);
    assert_close(restored.pitch_radians().to_degrees(), 18.0);
}

#[test]
fn camera_orbit_url_state_accepts_compact_checklist_query_shape() {
    let decoded = CameraOrbitUrlState::from_url_query("?camera-orbit=-28,18,2.5")
        .expect("compact camera-orbit parses");

    assert_close(decoded.yaw_degrees(), -28.0);
    assert_close(decoded.pitch_degrees(), 18.0);
    assert_close(decoded.distance(), 2.5);
    assert_eq!(
        decoded.to_query_string(),
        "?camera-orbit=-28deg%2018deg%202.5m"
    );
}

#[test]
fn camera_orbit_url_state_omits_asset_urls_and_secrets() {
    let source = "https://app.example/viewer?src=https%3A%2F%2Fuser%3Asecret%40assets.example%2Fmotor.glb\
         &token=secret-token&camera-orbit=-28deg%2018deg%202.5m\
         &camera-target=0m%200m%200m";

    let state = CameraOrbitUrlState::from_url_query(source).expect("url state parses");
    let serialized = state.to_query_string();

    assert_eq!(serialized, "?camera-orbit=-28deg%2018deg%202.5m");
    assert!(!serialized.contains("src"));
    assert!(!serialized.contains("secret"));
    assert!(!serialized.contains("token"));
    assert!(!serialized.contains("assets.example"));
}

#[test]
fn framing_outcome_exports_camera_orbit_url_state() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::default(),
        )
        .expect("camera inserts");
    let bounds = Aabb::new(Vec3::new(-1.0, -0.5, -0.5), Vec3::new(1.0, 0.5, 0.5));

    let framing = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .azimuth_elevation(-28.0, 18.0)
                .viewport(1280, 720),
        )
        .expect("bounds frame");
    let state = framing.url_state();

    assert_vec_close(state.target(), framing.target);
    assert_close(state.distance(), framing.distance);
    assert_close(state.yaw_degrees(), framing.yaw_radians.to_degrees());
    assert_close(state.pitch_degrees(), framing.pitch_radians.to_degrees());

    let json = serde_json::to_string(&state).expect("state serializes via serde");
    let from_json: CameraOrbitUrlState =
        serde_json::from_str(&json).expect("state deserializes via serde");
    assert_state_close(from_json, state);
}

fn assert_state_close(actual: CameraOrbitUrlState, expected: CameraOrbitUrlState) {
    assert_close(actual.yaw_degrees(), expected.yaw_degrees());
    assert_close(actual.pitch_degrees(), expected.pitch_degrees());
    assert_close(actual.distance(), expected.distance());
    assert_vec_close(actual.target(), expected.target());
}

fn assert_vec_close(actual: Vec3, expected: Vec3) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.z, expected.z);
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-4,
        "expected {actual} to be close to {expected}"
    );
}
