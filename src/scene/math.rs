//! Stage D2: Vec3, Quat, and Transform now delegate to the `glam` crate.
//! `glam` is the industry-standard Rust 3D math library — SIMD-optimized,
//! battle-tested across bevy/rend3/wgpu-rs ecosystems, and offers complete
//! operator overloads + a wide cross product / dot / normalize surface.
//! Replacing scena's hand-rolled Vec3/Quat means every math op is shared
//! with the broader Rust 3D world instead of being a private
//! reimplementation that might subtly disagree at edges.
//!
//! Public type names are preserved as `pub use` re-exports so downstream
//! code that constructs `Vec3 { x, y, z }` literals, calls `Vec3::new(...)`,
//! or accesses `.x`/`.y`/`.z`/`.w` continues to work — glam exposes the
//! same field layout and the same constructors.

pub use glam::{Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle {
    radians: f32,
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub const fn at(translation: Vec3) -> Self {
        Self {
            translation,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }

    pub const fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    pub const fn scale_by(mut self, scale: f32) -> Self {
        self.scale = Vec3::new(scale, scale, scale);
        self
    }

    /// Composes a degrees-around-X rotation onto the existing rotation, so
    /// that `Transform::default().rotate_y_deg(90.0).rotate_x_deg(45.0)`
    /// yields the chained rotation Y(90°) ∘ X(45°) instead of silently
    /// discarding the prior rotation. Closes scena-api-ergonomics-reviewer
    /// finding F2.
    pub fn rotate_x_deg(mut self, degrees: f32) -> Self {
        let added = Quat::from_axis_angle(
            Vec3::new(1.0, 0.0, 0.0),
            Angle::from_degrees(degrees).radians(),
        );
        self.rotation = compose_rotations(self.rotation, added);
        self
    }

    /// Composes a degrees-around-Y rotation onto the existing rotation. See
    /// [`Self::rotate_x_deg`] for the compose semantics.
    pub fn rotate_y_deg(mut self, degrees: f32) -> Self {
        let added = Quat::from_axis_angle(
            Vec3::new(0.0, 1.0, 0.0),
            Angle::from_degrees(degrees).radians(),
        );
        self.rotation = compose_rotations(self.rotation, added);
        self
    }

    /// Composes a degrees-around-Z rotation onto the existing rotation. See
    /// [`Self::rotate_x_deg`] for the compose semantics.
    pub fn rotate_z_deg(mut self, degrees: f32) -> Self {
        let added = Quat::from_axis_angle(
            Vec3::new(0.0, 0.0, 1.0),
            Angle::from_degrees(degrees).radians(),
        );
        self.rotation = compose_rotations(self.rotation, added);
        self
    }
}

/// Multiply two quaternions and re-normalize the result. glam's `*`
/// operator multiplies without normalizing; scena keeps quaternion
/// magnitudes bounded so floating-point drift across many composed
/// rotations doesn't accumulate.
fn compose_rotations(base: Quat, added: Quat) -> Quat {
    let product = base * added;
    let length_sq = product.length_squared();
    if length_sq <= f32::EPSILON || !length_sq.is_finite() {
        return Quat::IDENTITY;
    }
    product.normalize()
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Angle {
    pub fn from_degrees(degrees: f32) -> Self {
        Self::from_radians(degrees.to_radians())
    }

    pub const fn from_radians(radians: f32) -> Self {
        Self { radians }
    }

    pub const fn radians(self) -> f32 {
        self.radians
    }
}
