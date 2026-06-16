use std::f32::consts::TAU;

use scena::{OrbitControlAction, OrbitControls, Vec3};

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 1.0e-4,
        "expected {left} to be close to {right}"
    );
}

#[test]
fn turntable_presets_expose_explicit_frame_advance_semantics() {
    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0).turntable(6.0);
    assert_close(controls.auto_rotate_rpm(), 6.0);
    assert_close(controls.auto_rotate_radians_per_second(), TAU * 0.1);

    assert_eq!(controls.advance(0.25), OrbitControlAction::Orbit);
    assert_close(controls.yaw_radians(), TAU * 0.025);

    assert_eq!(controls.advance(0.0), OrbitControlAction::None);
    assert_eq!(
        OrbitControls::new(Vec3::ZERO, 2.0).advance(0.25),
        OrbitControlAction::None
    );
}

#[test]
fn presentation_sets_slow_turntable_motion() {
    let mut controls = OrbitControls::new(Vec3::ZERO, 2.0).presentation();

    assert_close(controls.auto_rotate_rpm(), 1.0);
    assert_eq!(controls.advance(1.0), OrbitControlAction::Orbit);
    assert_close(controls.yaw_radians(), TAU / 60.0);
}
