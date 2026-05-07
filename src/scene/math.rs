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

    pub fn rotate_x_deg(mut self, degrees: f32) -> Self {
        self.rotation =
            Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), Angle::from_degrees(degrees));
        self
    }

    pub fn rotate_y_deg(mut self, degrees: f32) -> Self {
        self.rotation =
            Quat::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), Angle::from_degrees(degrees));
        self
    }

    pub fn rotate_z_deg(mut self, degrees: f32) -> Self {
        self.rotation =
            Quat::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), Angle::from_degrees(degrees));
        self
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
