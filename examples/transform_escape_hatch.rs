//! Transform escape hatch — when degree helpers and `looking_at` are not enough.
//!
//! Most scene code should reach for the named transform helpers first:
//!
//! - `Transform::at(position)` for translation
//! - `Transform::default().rotate_x_deg(...)` / `rotate_y_deg(...)` / `rotate_z_deg(...)`
//! - `Transform::looking_at(target, up)` when a node should face a point
//!
//! Raw `Quat::from_*` constructors are reserved for the rare cases where a
//! caller already has an axis-angle, a quaternion-valued asset import, or a
//! per-frame rotation derived from physics. This example shows the escape
//! hatch so beginners can find it without copying it into first-path code.

use scena::{Quat, Transform, Vec3};

fn main() {
    let degree_form = Transform::default().rotate_y_deg(45.0);

    let axis_angle = Transform {
        rotation: Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4),
        ..Transform::default()
    };

    let composed = Transform {
        rotation: Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_4)
            * Quat::from_axis_angle(Vec3::X, -std::f32::consts::FRAC_PI_8),
        ..Transform::default()
    };

    assert!((degree_form.rotation.y - axis_angle.rotation.y).abs() < 1.0e-5);
    let _ = composed;
}
