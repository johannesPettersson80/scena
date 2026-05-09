#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
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
        let added = Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), Angle::from_degrees(degrees));
        self.rotation = compose_rotations(self.rotation, added);
        self
    }

    /// Composes a degrees-around-Y rotation onto the existing rotation. See
    /// [`Self::rotate_x_deg`] for the compose semantics.
    pub fn rotate_y_deg(mut self, degrees: f32) -> Self {
        let added = Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), Angle::from_degrees(degrees));
        self.rotation = compose_rotations(self.rotation, added);
        self
    }

    /// Composes a degrees-around-Z rotation onto the existing rotation. See
    /// [`Self::rotate_x_deg`] for the compose semantics.
    pub fn rotate_z_deg(mut self, degrees: f32) -> Self {
        let added = Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), Angle::from_degrees(degrees));
        self.rotation = compose_rotations(self.rotation, added);
        self
    }
}

fn compose_rotations(base: Quat, added: Quat) -> Quat {
    multiply_quat(base, added)
}

fn multiply_quat(left: Quat, right: Quat) -> Quat {
    let out = Quat {
        x: left.w * right.x + left.x * right.w + left.y * right.z - left.z * right.y,
        y: left.w * right.y - left.x * right.z + left.y * right.w + left.z * right.x,
        z: left.w * right.z + left.x * right.y - left.y * right.x + left.z * right.w,
        w: left.w * right.w - left.x * right.x - left.y * right.y - left.z * right.z,
    };
    let length_sq = out.x * out.x + out.y * out.y + out.z * out.z + out.w * out.w;
    if length_sq <= f32::EPSILON || !length_sq.is_finite() {
        return Quat::IDENTITY;
    }
    let inv = length_sq.sqrt().recip();
    Quat {
        x: out.x * inv,
        y: out.y * inv,
        z: out.z * inv,
        w: out.w * inv,
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl Quat {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    pub fn from_axis_angle(axis: Vec3, angle: Angle) -> Self {
        let length = (axis.x * axis.x + axis.y * axis.y + axis.z * axis.z).sqrt();
        if length <= f32::EPSILON || !length.is_finite() {
            return Self::IDENTITY;
        }
        let half = angle.radians() * 0.5;
        let sin = half.sin();
        Self {
            x: axis.x / length * sin,
            y: axis.y / length * sin,
            z: axis.z / length * sin,
            w: half.cos(),
        }
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
