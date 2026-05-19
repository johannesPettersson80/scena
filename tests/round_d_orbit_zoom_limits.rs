use scena::{OrbitControlAction, OrbitControls, PointerEvent, TouchEvent, Vec3};

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 1.0e-4,
        "expected {left} to be close to {right}"
    );
}

#[test]
fn orbit_zoom_limits_are_relative_to_current_framed_distance() {
    let controls = OrbitControls::new(Vec3::ZERO, 10.0).zoom_limits_bounds_relative(0.5, 4.0);

    assert_close(controls.min_distance(), 5.0);
    assert_close(controls.max_distance(), 40.0);
    assert_close(controls.distance(), 10.0);
}

#[test]
fn wheel_and_pinch_zoom_are_clamped_to_named_limits() {
    let mut controls = OrbitControls::new(Vec3::ZERO, 10.0).zoom_limits_bounds_relative(0.5, 1.25);

    for _ in 0..32 {
        assert_eq!(
            controls.handle_pointer(PointerEvent::wheel(0.0, 0.0, -10.0)),
            OrbitControlAction::Zoom
        );
    }
    assert_close(controls.distance(), 5.0);

    for _ in 0..32 {
        assert_eq!(
            controls.handle_touch(TouchEvent::pinch(0.0, 0.0, 10.0)),
            OrbitControlAction::Zoom
        );
    }
    assert_close(controls.distance(), 12.5);
}
