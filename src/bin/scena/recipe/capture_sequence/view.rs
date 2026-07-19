use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scena_recipe) enum CanonicalView {
    Front,
    Top,
    Right,
    Isometric,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::scena_recipe) struct SubjectBounds {
    pub(in crate::scena_recipe) min: scena::Vec3,
    pub(in crate::scena_recipe) max: scena::Vec3,
}

impl CanonicalView {
    pub(in crate::scena_recipe) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "front" => Ok(Self::Front),
            "top" => Ok(Self::Top),
            "right" => Ok(Self::Right),
            "isometric" | "iso" => Ok(Self::Isometric),
            other => Err(format!(
                "unknown canonical view '{other}'; expected front,top,right,isometric"
            )),
        }
    }

    pub(in crate::scena_recipe) const fn id(self) -> &'static str {
        match self {
            Self::Front => "front",
            Self::Top => "top",
            Self::Right => "right",
            Self::Isometric => "isometric",
        }
    }

    pub(in crate::scena_recipe) const fn purpose(self) -> &'static str {
        match self {
            Self::Front => "look from +Z toward the framed target with +Y screen-up",
            Self::Top => "look from near +Y with a one-degree pole offset and -Z screen-up",
            Self::Right => "look from +X toward the framed target with +Y screen-up",
            Self::Isometric => "look from equal +X,+Y,+Z components with +Y world-up",
        }
    }

    pub(in crate::scena_recipe) fn camera_state(
        self,
        target: scena::Vec3,
        distance: f32,
    ) -> scena::SceneHostCameraState {
        let (yaw_radians, pitch_radians) = match self {
            Self::Front => (0.0, 0.0),
            Self::Top => (0.0, FRAC_PI_2 - 0.017_453_292),
            Self::Right => (FRAC_PI_2, 0.0),
            Self::Isometric => (FRAC_PI_4, (1.0_f32 / 3.0_f32.sqrt()).asin()),
        };
        scena::SceneHostCameraState {
            target,
            distance,
            yaw_radians,
            pitch_radians,
        }
    }

    pub(in crate::scena_recipe) fn ideal_eye_direction(self) -> scena::Vec3 {
        match self {
            Self::Front => scena::Vec3::Z,
            Self::Top => scena::Vec3::Y,
            Self::Right => scena::Vec3::X,
            Self::Isometric => scena::Vec3::new(1.0, 1.0, 1.0).normalize(),
        }
    }

    pub(in crate::scena_recipe) fn screen_up(self) -> scena::Vec3 {
        match self {
            Self::Top => -scena::Vec3::Z,
            Self::Front | Self::Right | Self::Isometric => scena::Vec3::Y,
        }
    }
}

impl SubjectBounds {
    fn union(self, other: Self) -> Self {
        Self {
            min: scena::Vec3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: scena::Vec3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }

    pub(in crate::scena_recipe) fn center(self) -> scena::Vec3 {
        (self.min + self.max) * 0.5
    }

    pub(in crate::scena_recipe) fn extent(self) -> scena::Vec3 {
        (self.max - self.min).abs()
    }

    pub(in crate::scena_recipe) fn radius(self) -> f32 {
        self.extent().length() * 0.5
    }
}

pub(in crate::scena_recipe) fn subject_bounds(
    inspection: &scena::SceneInspectionReportV1,
) -> Result<SubjectBounds, String> {
    let mut bounds: Option<SubjectBounds> = None;
    for draw in &inspection.draw_list {
        let draw_bounds = transform_bounds(draw.local_bounds, draw.world_transform);
        bounds = Some(match bounds {
            Some(existing) => existing.union(draw_bounds),
            None => draw_bounds,
        });
    }
    bounds.ok_or_else(|| "recipe has no drawable geometry to capture".to_owned())
}

fn transform_bounds(bounds: scena::Aabb, transform: scena::Transform) -> SubjectBounds {
    let corners = [
        scena::Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        scena::Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        scena::Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        scena::Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        scena::Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        scena::Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        scena::Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        scena::Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ];
    let first = transform_point(corners[0], transform);
    let mut min = first;
    let mut max = first;
    for corner in corners.into_iter().skip(1) {
        let point = transform_point(corner, transform);
        min = scena::Vec3::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z));
        max = scena::Vec3::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z));
    }
    SubjectBounds { min, max }
}

fn transform_point(point: scena::Vec3, transform: scena::Transform) -> scena::Vec3 {
    transform.translation + transform.rotation * (point * transform.scale)
}
