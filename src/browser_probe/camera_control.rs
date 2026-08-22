use serde_json::json;
use wasm_bindgen::JsValue;

use crate::{
    CameraKey, FlyControls, FollowControls, OrbitControlAction, OrbitControls, PerspectiveCamera,
    PointerEvent, Scene, Transform, Vec3,
};

pub(super) fn render_camera_control_kit_probe() -> Result<String, JsValue> {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 5.0)),
        )
        .map_err(|error| JsValue::from_str(&format!("camera insert failed: {error:?}")))?;
    scene
        .set_active_camera(camera)
        .map_err(|error| JsValue::from_str(&format!("active camera failed: {error:?}")))?;

    let mut orbit = OrbitControls::new(Vec3::ZERO, 4.0)
        .presentation()
        .zoom_limits_bounds_relative(0.5, 2.0);
    let initial_distance = orbit.distance();
    let orbit_actions = vec![
        orbit.handle_pointer(PointerEvent::primary_pressed(160.0, 120.0)),
        orbit.handle_pointer(PointerEvent::moved(184.0, 108.0, 24.0, -12.0)),
        orbit.handle_pointer(PointerEvent::wheel(184.0, 108.0, -1.0)),
        orbit.handle_pointer(PointerEvent::released(184.0, 108.0)),
        orbit.advance(1.0 / 30.0),
    ];
    orbit
        .apply_to_scene(&mut scene, camera)
        .map_err(|error| JsValue::from_str(&format!("orbit apply failed: {error:?}")))?;
    let orbit_camera = camera_translation(&scene, camera)?;

    let target = scene
        .add_empty(scene.root(), Transform::at(Vec3::new(2.0, 0.5, -1.0)))
        .map_err(|error| JsValue::from_str(&format!("target insert failed: {error:?}")))?;
    FollowControls::behind_and_above(3.0, 1.25)
        .apply_to_scene(&mut scene, camera, target)
        .map_err(|error| JsValue::from_str(&format!("follow apply failed: {error:?}")))?;
    let follow_camera = camera_translation(&scene, camera)?;

    let mut fly = FlyControls::new(Vec3::ZERO)
        .with_yaw_pitch_degrees(90.0, 0.0)
        .with_move_speed(2.0);
    let fly_move = fly.move_local(1.0, 0.5, 0.25, 2.0);
    let fly_look = fly.look_delta(8.0, -4.0);
    fly.apply_to_scene(&mut scene, camera)
        .map_err(|error| JsValue::from_str(&format!("fly apply failed: {error:?}")))?;
    let fly_camera = camera_translation(&scene, camera)?;

    let passed = orbit_actions.contains(&OrbitControlAction::BeginOrbit)
        && orbit_actions.contains(&OrbitControlAction::Orbit)
        && orbit_actions.contains(&OrbitControlAction::Zoom)
        && orbit_actions.contains(&OrbitControlAction::End)
        && orbit.distance() < initial_distance
        && follow_camera.y > 0.5
        && fly_camera.x > 0.0
        && fly_camera.z < 0.0
        && matches!(fly_move, OrbitControlAction::Pan)
        && matches!(fly_look, OrbitControlAction::Orbit);

    Ok(json!({
        "schema": "scena.m6.camera_control_kit_browser_proof.v1",
        "status": if passed { "passed" } else { "failed" },
        "proof_class": "browser-demo",
        "visual_proof": "browser-demo",
        "orbit": {
            "initial_distance": initial_distance,
            "distance_after_zoom": orbit.distance(),
            "yaw_radians": orbit.yaw_radians(),
            "pitch_radians": orbit.pitch_radians(),
            "actions": orbit_actions.iter().map(|action| action_name(*action)).collect::<Vec<_>>(),
            "camera_translation": vec3_json(orbit_camera),
        },
        "follow": {
            "target_translation": [2.0, 0.5, -1.0],
            "camera_translation": vec3_json(follow_camera),
        },
        "fly": {
            "move_action": action_name(fly_move),
            "look_action": action_name(fly_look),
            "camera_translation": vec3_json(fly_camera),
            "yaw_radians": fly.yaw_radians(),
            "pitch_radians": fly.pitch_radians(),
        },
    })
    .to_string())
}

fn camera_translation(scene: &Scene, camera: CameraKey) -> Result<Vec3, JsValue> {
    let camera_node = scene
        .camera_node(camera)
        .ok_or_else(|| JsValue::from_str("camera node missing"))?;
    let transform = scene
        .world_transform(camera_node)
        .ok_or_else(|| JsValue::from_str("camera world transform missing"))?;
    Ok(transform.translation)
}

fn action_name(action: OrbitControlAction) -> &'static str {
    match action {
        OrbitControlAction::None => "None",
        OrbitControlAction::BeginOrbit => "BeginOrbit",
        OrbitControlAction::Orbit => "Orbit",
        OrbitControlAction::Pan => "Pan",
        OrbitControlAction::Zoom => "Zoom",
        OrbitControlAction::End => "End",
    }
}

fn vec3_json(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}
