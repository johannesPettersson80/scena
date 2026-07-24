use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

use scena::{
    Assets, Color, GeometryDesc, MaterialDesc, OrbitControlAction, OrbitControls, PointerEvent,
    Scene, TouchEvent, Transform, Vec3,
};

#[test]
fn orbit_pointer_touch_and_pinch_reject_non_finite_events_and_recover() {
    let mut orbit = OrbitControls::new(Vec3::ZERO, 10.0);

    assert_eq!(
        orbit.handle_pointer(PointerEvent::primary_pressed(0.0, 0.0)),
        OrbitControlAction::BeginOrbit
    );
    let before = orbit_snapshot(&orbit);
    assert_eq!(
        orbit.handle_pointer(PointerEvent::moved(1.0, 1.0, f32::NAN, 2.0)),
        OrbitControlAction::None
    );
    assert_eq!(orbit_snapshot(&orbit), before);
    assert_eq!(
        orbit.handle_pointer(PointerEvent::moved(2.0, 2.0, 3.0, -1.0)),
        OrbitControlAction::Orbit,
        "the active orbit gesture must recover on the next finite event"
    );
    orbit.handle_pointer(PointerEvent::released(2.0, 2.0));

    assert_eq!(
        orbit.handle_pointer(PointerEvent::secondary_pressed(0.0, 0.0)),
        OrbitControlAction::Pan
    );
    let before = orbit_snapshot(&orbit);
    assert_eq!(
        orbit.handle_pointer(PointerEvent::moved(1.0, 1.0, f32::INFINITY, 1.0)),
        OrbitControlAction::None
    );
    assert_eq!(orbit_snapshot(&orbit), before);
    assert_eq!(
        orbit.handle_pointer(PointerEvent::moved(2.0, 2.0, 1.0, 1.0)),
        OrbitControlAction::Pan,
        "the active pan gesture must recover on the next finite event"
    );
    orbit.handle_pointer(PointerEvent::released(2.0, 2.0));

    let before = orbit_snapshot(&orbit);
    assert_eq!(
        orbit.handle_pointer(PointerEvent::wheel(0.0, 0.0, f32::NEG_INFINITY)),
        OrbitControlAction::None
    );
    assert_eq!(orbit_snapshot(&orbit), before);
    assert_eq!(
        orbit.handle_pointer(PointerEvent::wheel(0.0, 0.0, -0.5)),
        OrbitControlAction::Zoom
    );

    assert_eq!(
        orbit.handle_touch(TouchEvent::start(0.0, 0.0)),
        OrbitControlAction::BeginOrbit
    );
    let before = orbit_snapshot(&orbit);
    assert_eq!(
        orbit.handle_touch(TouchEvent::move_by(1.0, 1.0, 1.0, f32::NAN)),
        OrbitControlAction::None
    );
    assert_eq!(orbit_snapshot(&orbit), before);
    assert_eq!(
        orbit.handle_touch(TouchEvent::move_by(2.0, 2.0, 1.0, 1.0)),
        OrbitControlAction::Orbit
    );
    orbit.handle_touch(TouchEvent::end(2.0, 2.0));

    let before = orbit_snapshot(&orbit);
    assert_eq!(
        orbit.handle_touch(TouchEvent::pinch(0.0, 0.0, f32::INFINITY)),
        OrbitControlAction::None
    );
    assert_eq!(orbit_snapshot(&orbit), before);
    assert_eq!(
        orbit.handle_touch(TouchEvent::pinch(0.0, 0.0, -0.25)),
        OrbitControlAction::Zoom
    );
}

#[test]
fn orbit_pan_uses_camera_right_and_up_at_cardinal_yaws_and_pitch() {
    for (yaw, expected) in [
        (0.0, Vec3::new(-0.1, 0.0, 0.0)),
        (FRAC_PI_2, Vec3::new(0.0, 0.0, 0.1)),
        (-FRAC_PI_2, Vec3::new(0.0, 0.0, -0.1)),
        (PI, Vec3::new(0.1, 0.0, 0.0)),
    ] {
        let mut orbit = OrbitControls::new(Vec3::ZERO, 10.0).with_angles(yaw, 0.0);
        orbit.handle_pointer(PointerEvent::secondary_pressed(0.0, 0.0));
        assert_eq!(
            orbit.handle_pointer(PointerEvent::moved(10.0, 0.0, 10.0, 0.0)),
            OrbitControlAction::Pan
        );
        assert_vec3_near(orbit.target(), expected);
    }

    let mut pitched = OrbitControls::new(Vec3::ZERO, 10.0).with_angles(FRAC_PI_2, FRAC_PI_4);
    pitched.handle_pointer(PointerEvent::secondary_pressed(0.0, 0.0));
    pitched.handle_pointer(PointerEvent::moved(0.0, 10.0, 0.0, 10.0));
    let diagonal = 0.1 / 2.0_f32.sqrt();
    assert_vec3_near(pitched.target(), Vec3::new(-diagonal, diagonal, 0.0));
}

#[test]
fn releasing_an_unrelated_pointer_button_does_not_end_the_active_gesture() {
    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0);
    assert_eq!(
        controls.handle_pointer(PointerEvent::primary_pressed(0.0, 0.0)),
        OrbitControlAction::BeginOrbit,
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent {
            kind: scena::PointerEventKind::Released,
            position: (0.0, 0.0),
            button: Some(scena::PointerButton::Secondary),
            delta: (0.0, 0.0),
            scroll_delta: 0.0,
        }),
        OrbitControlAction::None,
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent::moved(2.0, 0.0, 2.0, 0.0)),
        OrbitControlAction::Orbit,
        "the primary-owned orbit must survive a secondary-button release",
    );
}

#[test]
fn simultaneous_orbit_and_pan_end_only_when_their_owning_button_releases() {
    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0);
    controls.handle_pointer(PointerEvent::primary_pressed(0.0, 0.0));
    controls.handle_pointer(PointerEvent::secondary_pressed(0.0, 0.0));
    assert_eq!(
        controls.handle_pointer(PointerEvent::moved(1.0, 0.0, 1.0, 0.0)),
        OrbitControlAction::Orbit,
    );

    assert_eq!(
        controls.handle_pointer(PointerEvent::button_released(
            1.0,
            0.0,
            scena::PointerButton::Primary,
        )),
        OrbitControlAction::End,
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent::moved(2.0, 0.0, 1.0, 0.0)),
        OrbitControlAction::Pan,
        "secondary-owned pan must survive release of the primary-owned orbit",
    );

    assert_eq!(
        controls.handle_pointer(PointerEvent::button_released(
            2.0,
            0.0,
            scena::PointerButton::Secondary,
        )),
        OrbitControlAction::End,
    );
    assert_eq!(
        controls.handle_pointer(PointerEvent::moved(3.0, 0.0, 1.0, 0.0)),
        OrbitControlAction::None,
    );
}

#[test]
fn scene_rejects_non_finite_direct_batch_alignment_and_instance_transforms_atomically() {
    for invalid in invalid_transforms() {
        let mut scene = Scene::new();
        let node = scene
            .add_empty(scene.root(), Transform::IDENTITY)
            .expect("node inserts");
        let before = scene.dirty_state();
        let before_transform = scene.node(node).unwrap().transform();
        scene
            .set_transform(node, invalid)
            .expect_err("direct non-finite transform must fail");
        assert_eq!(scene.node(node).unwrap().transform(), before_transform);
        assert_eq!(scene.dirty_state(), before);
    }

    let mut scene = Scene::new();
    let first = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("first node inserts");
    let second = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("second node inserts");
    for batch in [
        vec![
            (first, Transform::at(Vec3::X)),
            (second, invalid_transforms()[0]),
        ],
        vec![
            (second, invalid_transforms()[1]),
            (first, Transform::at(Vec3::X)),
        ],
    ] {
        let before = scene.dirty_state();
        let first_before = scene.node(first).unwrap().transform();
        let second_before = scene.node(second).unwrap().transform();
        scene
            .set_transforms(&batch)
            .expect_err("batch with non-finite transform must fail atomically");
        assert_eq!(scene.node(first).unwrap().transform(), first_before);
        assert_eq!(scene.node(second).unwrap().transform(), second_before);
        assert_eq!(scene.dirty_state(), before);
    }

    let before = scene.dirty_state();
    let first_before = scene.node(first).unwrap().transform();
    scene
        .align_to(first, invalid_transforms()[2])
        .expect_err("world-space alignment must reject non-finite transform");
    assert_eq!(scene.node(first).unwrap().transform(), first_before);
    assert_eq!(scene.dirty_state(), before);

    let mut insertion_scene = Scene::new();
    let before = insertion_scene.dirty_state();
    insertion_scene
        .add_empty(insertion_scene.root(), invalid_transforms()[0])
        .expect_err("node insertion must reject non-finite transform");
    assert_eq!(insertion_scene.dirty_state(), before);

    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut instance_scene = Scene::new();
    let set = instance_scene
        .add_instance_set(
            instance_scene.root(),
            geometry,
            material,
            Transform::IDENTITY,
        )
        .expect("instance set inserts");
    let before = instance_scene.dirty_state();
    instance_scene
        .push_instance(set, invalid_transforms()[0])
        .expect_err("instance insertion must reject non-finite transform");
    assert_eq!(instance_scene.dirty_state(), before);
    let instance = instance_scene
        .push_instance(set, Transform::IDENTITY)
        .expect("finite instance inserts");
    let before = instance_scene.dirty_state();
    instance_scene
        .set_instance_transform(set, instance, invalid_transforms()[1])
        .expect_err("instance mutation must reject non-finite transform");
    assert_eq!(
        instance_scene
            .instance_set(set)
            .unwrap()
            .instances()
            .next()
            .unwrap()
            .transform(),
        Transform::IDENTITY
    );
    assert_eq!(instance_scene.dirty_state(), before);
}

fn invalid_transforms() -> [Transform; 3] {
    [
        Transform::IDENTITY.with_translation(Vec3::new(f32::NAN, 0.0, 0.0)),
        Transform {
            rotation: scena::Quat::from_xyzw(0.0, f32::INFINITY, 0.0, 1.0),
            ..Transform::IDENTITY
        },
        Transform {
            scale: Vec3::new(1.0, 1.0, f32::NEG_INFINITY),
            ..Transform::IDENTITY
        },
    ]
}

fn orbit_snapshot(orbit: &OrbitControls) -> (Vec3, f32, f32, f32) {
    (
        orbit.target(),
        orbit.distance(),
        orbit.yaw_radians(),
        orbit.pitch_radians(),
    )
}

fn assert_vec3_near(actual: Vec3, expected: Vec3) {
    assert!(
        actual.abs_diff_eq(expected, 1.0e-5),
        "expected {actual:?} to be near {expected:?}"
    );
}

#[cfg(feature = "scene-host")]
mod host_contracts {
    use super::*;
    use scena::{
        AssetPath, PointerButton, SceneHostCameraState, SceneHostCore, SceneHostEasing,
        SceneHostErrorCode, SceneInspectionReportV1,
    };

    #[test]
    fn scene_host_pan_matches_camera_space_directions() {
        for (yaw, pitch, delta, expected) in [
            (0.0, 0.0, (10.0, 0.0), Vec3::new(-0.04, 0.0, 0.0)),
            (FRAC_PI_2, 0.0, (10.0, 0.0), Vec3::new(0.0, 0.0, 0.04)),
            (-FRAC_PI_2, 0.0, (10.0, 0.0), Vec3::new(0.0, 0.0, -0.04)),
            (PI, 0.0, (10.0, 0.0), Vec3::new(0.04, 0.0, 0.0)),
            (
                FRAC_PI_2,
                FRAC_PI_4,
                (0.0, 10.0),
                Vec3::new(-0.04 / 2.0_f32.sqrt(), 0.04 / 2.0_f32.sqrt(), 0.0),
            ),
        ] {
            let mut host = SceneHostCore::headless(64, 64).expect("host builds");
            host.set_camera(SceneHostCameraState {
                target: Vec3::ZERO,
                distance: 4.0,
                yaw_radians: yaw,
                pitch_radians: pitch,
            })
            .expect("camera state applies");
            host.camera_pointer_down(0.0, 0.0, PointerButton::Secondary)
                .expect("pan starts");
            host.camera_pointer_move(delta.0, delta.1, delta.0, delta.1)
                .expect("pan applies");
            assert_vec3_near(host.camera_state().target, expected);
        }
    }

    #[test]
    fn scene_host_non_finite_camera_event_preserves_transition_and_recovers() {
        let mut host = SceneHostCore::headless(64, 64).expect("host builds");
        let target = SceneHostCameraState {
            target: Vec3::new(4.0, 0.0, 0.0),
            distance: 4.0,
            yaw_radians: 0.0,
            pitch_radians: 0.0,
        };
        host.set_camera_eased(target, 2.0, SceneHostEasing::Linear)
            .expect("camera transition starts");
        host.camera_pointer_down(0.0, 0.0, PointerButton::Primary)
            .expect("orbit starts");
        assert_eq!(
            host.camera_pointer_move(1.0, 1.0, f32::NAN, 1.0)
                .expect("non-finite input is a no-op"),
            OrbitControlAction::None
        );
        host.advance(1.0)
            .expect("rejected event must not cancel camera transition");
        assert_vec3_near(host.camera_state().target, Vec3::new(2.0, 0.0, 0.0));
        host.camera_pointer_down(2.0, 2.0, PointerButton::Primary)
            .expect("a fresh orbit starts after the transition sample rebuilds controls");
        assert_eq!(
            host.camera_pointer_move(2.0, 2.0, 1.0, 0.0)
                .expect("next finite orbit event recovers"),
            OrbitControlAction::Orbit
        );
    }

    #[test]
    fn scene_host_batches_preflight_stale_and_missing_instance_roots_in_both_orders() {
        for (remove_existing, expected_code) in [
            (true, SceneHostErrorCode::StaleNodeHandle),
            (false, SceneHostErrorCode::NodeHandleNotFound),
        ] {
            for invalid_first in [false, true] {
                let mut host = SceneHostCore::headless(64, 64).expect("host builds");
                let valid = host
                    .add_empty(
                        Some(host.root_handle()),
                        Transform::IDENTITY,
                        Some("valid-batch-node"),
                    )
                    .expect("valid node inserts");
                let instance_roots = pollster::block_on(host.instantiate_url_instanced(
                    AssetPath::from("tests/assets/gltf/mesh_material_vertex_color_scene.gltf"),
                    1,
                ))
                .expect("instance root creates");
                let existing = instance_roots[0];
                let invalid = if remove_existing {
                    host.remove_node(existing).expect("instance root removes");
                    existing
                } else {
                    existing + 65_534
                };
                host.set_transform_eased(
                    valid,
                    Transform::at(Vec3::new(10.0, 0.0, 0.0)),
                    10.0,
                    SceneHostEasing::Linear,
                )
                .expect("transition starts");

                let valid_update = (valid, Transform::at(Vec3::new(3.0, 0.0, 0.0)));
                let invalid_update = (invalid, Transform::at(Vec3::new(4.0, 0.0, 0.0)));
                let batch = if invalid_first {
                    [invalid_update, valid_update]
                } else {
                    [valid_update, invalid_update]
                };
                let before = host.scene().dirty_state();
                let error = host
                    .set_transforms(&batch)
                    .expect_err("invalid instance root rejects the whole batch");
                assert_eq!(error.code(), expected_code);
                assert_vec3_near(host_node_translation(&host, valid), Vec3::ZERO);
                assert_eq!(host.scene().dirty_state(), before);

                host.advance(1.0)
                    .expect("failed batch must preserve the valid node transition");
                assert_vec3_near(host_node_translation(&host, valid), Vec3::X);
            }
        }
    }

    fn host_node_translation(host: &SceneHostCore, handle: u64) -> Vec3 {
        let report: SceneInspectionReportV1 =
            serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
                .expect("inspection decodes");
        report
            .node_by_handle(handle)
            .expect("node appears in host inspection")
            .local_transform
            .translation
    }
}
