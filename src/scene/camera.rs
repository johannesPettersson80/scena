use super::Angle;

#[derive(Debug, Clone, PartialEq)]
pub enum Camera {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerspectiveCamera {
    pub vertical_fov: Angle,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthRange {
    near: f32,
    far: f32,
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        Self {
            vertical_fov: Angle::from_degrees(60.0),
            aspect: 1.0,
            near: DepthRange::DEFAULT.near,
            far: DepthRange::DEFAULT.far,
        }
    }
}

impl PerspectiveCamera {
    pub const fn with_depth_range(mut self, range: DepthRange) -> Self {
        self.near = range.near;
        self.far = range.far;
        self
    }
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        Self {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: -1.0,
            far: 1.0,
        }
    }
}

impl OrthographicCamera {
    pub const fn with_depth_range(mut self, range: DepthRange) -> Self {
        self.near = range.near;
        self.far = range.far;
        self
    }
}

impl DepthRange {
    pub const DEFAULT: Self = Self {
        near: 0.01,
        far: 1000.0,
    };
    const MIN_NEAR: f32 = 0.001;

    pub const fn new(near: f32, far: f32) -> Self {
        if near.is_finite() && far.is_finite() && near > 0.0 && far > near {
            Self { near, far }
        } else {
            Self::DEFAULT
        }
    }

    pub const fn fit_sphere(center_distance: f32, radius: f32) -> Self {
        if !center_distance.is_finite()
            || !radius.is_finite()
            || center_distance <= 0.0
            || radius < 0.0
        {
            return Self::DEFAULT;
        }
        let near = positive_max(center_distance - radius, Self::MIN_NEAR);
        let far = center_distance + radius;
        Self::new(near, far)
    }

    pub const fn near(self) -> f32 {
        self.near
    }

    pub const fn far(self) -> f32 {
        self.far
    }

    pub const fn contains_interval(self, near: f32, far: f32) -> bool {
        near >= self.near && far <= self.far
    }
}

const fn positive_max(left: f32, right: f32) -> f32 {
    if left > right { left } else { right }
}
