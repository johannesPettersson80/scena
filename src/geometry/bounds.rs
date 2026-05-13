use crate::scene::{Angle, Transform, Vec3};

use super::{Aabb, GeometryVertex};

/// Phase 5.2: viewing angles (in degrees) for the auto-framing camera
/// helpers. Yaw rotates around the world Y axis (left ↔ right); pitch
/// rotates around the camera's local X axis (up ↔ down). The defaults
/// place the camera at a mild 3/4 angle that flatters most CAD-style
/// product renders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FramingAngles {
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
}

impl FramingAngles {
    /// 3/4-front view: 25° to the right of the asset's local Z axis,
    /// 10° above horizontal. Matches the Khronos sample-thumbnail pose.
    pub const THREE_QUARTER_FRONT: Self = Self {
        yaw_degrees: 25.0,
        pitch_degrees: -10.0,
    };

    pub const FRONT: Self = Self {
        yaw_degrees: 0.0,
        pitch_degrees: 0.0,
    };

    pub const fn new(yaw_degrees: f32, pitch_degrees: f32) -> Self {
        Self {
            yaw_degrees,
            pitch_degrees,
        }
    }
}

impl Default for FramingAngles {
    fn default() -> Self {
        Self::THREE_QUARTER_FRONT
    }
}

impl Aabb {
    pub const fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_vertices(vertices: &[GeometryVertex]) -> Option<Self> {
        let first = vertices.first()?;
        let mut min = first.position;
        let mut max = first.position;
        for vertex in &vertices[1..] {
            min.x = min.x.min(vertex.position.x);
            min.y = min.y.min(vertex.position.y);
            min.z = min.z.min(vertex.position.z);
            max.x = max.x.max(vertex.position.x);
            max.y = max.y.max(vertex.position.y);
            max.z = max.z.max(vertex.position.z);
        }
        Some(Self { min, max })
    }

    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.y >= self.min.y
            && point.z >= self.min.z
            && point.x <= self.max.x
            && point.y <= self.max.y
            && point.z <= self.max.z
    }

    pub fn center(self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    pub fn half_extent(self) -> Vec3 {
        Vec3::new(
            (self.max.x - self.min.x).abs() * 0.5,
            (self.max.y - self.min.y).abs() * 0.5,
            (self.max.z - self.min.z).abs() * 0.5,
        )
    }

    pub fn bounding_sphere_radius(self) -> f32 {
        let half = self.half_extent();
        (half.x * half.x + half.y * half.y + half.z * half.z).sqrt()
    }

    /// Phase 5.2: compute a camera `Transform` that frames the asset
    /// at the requested viewing angles, with the asset's bounding
    /// sphere occupying `fill_fraction` of the vertical frame (e.g.
    /// 0.7 = 70%). `fov_y` is the camera's vertical field of view.
    ///
    /// Tests/examples previously hand-rolled this math with
    /// `extent * 1.25`-style fudges and ad-hoc rotation order. This
    /// helper deduplicates it and stays consistent across the surface.
    pub fn framing_transform(
        self,
        angles: FramingAngles,
        fill_fraction: f32,
        fov_y: Angle,
    ) -> Transform {
        let centre = self.center();
        let radius = self.bounding_sphere_radius().max(f32::EPSILON);
        let fill = fill_fraction.clamp(0.05, 1.0);
        let half_fov = (fov_y.radians() * 0.5).max(1e-4);
        // distance such that the bounding sphere's radius subtends
        // `fill_fraction` of the half-height of the view frustum:
        //   tan(half_fov) * distance * fill = radius
        let distance = radius / (half_fov.tan() * fill);
        // Orbit camera around the asset's centre: rotate the local +Z
        // offset by yaw (Y) then pitch (X). The result is the camera's
        // position in world space; the camera's rotation matches the
        // orbit so it aims at the centre.
        let yaw = angles.yaw_degrees.to_radians();
        let pitch = angles.pitch_degrees.to_radians();
        // Start from offset (0, 0, distance), rotate by pitch around X
        // (raises camera up), then by yaw around Y (orbits horizontally).
        let after_pitch = Vec3::new(0.0, -distance * pitch.sin(), distance * pitch.cos());
        let after_yaw = Vec3::new(
            after_pitch.x * yaw.cos() + after_pitch.z * yaw.sin(),
            after_pitch.y,
            -after_pitch.x * yaw.sin() + after_pitch.z * yaw.cos(),
        );
        Transform::at(Vec3::new(
            centre.x + after_yaw.x,
            centre.y + after_yaw.y,
            centre.z + after_yaw.z,
        ))
        .rotate_y_deg(angles.yaw_degrees)
        .rotate_x_deg(angles.pitch_degrees)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_transform_centred_unit_box_front_view_places_camera_along_positive_z() {
        let bounds = Aabb::new(Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, 0.5, 0.5));
        let transform =
            bounds.framing_transform(FramingAngles::FRONT, 0.7, Angle::from_degrees(60.0));
        // FRONT (yaw=0, pitch=0) places the camera on the +Z axis,
        // looking down -Z back at the centre.
        assert!(transform.translation.x.abs() < 1e-5);
        assert!(transform.translation.y.abs() < 1e-5);
        assert!(transform.translation.z > 0.0);
    }

    #[test]
    fn framing_transform_distance_scales_inversely_with_fill_fraction() {
        let bounds = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let close = bounds
            .framing_transform(FramingAngles::FRONT, 0.9, Angle::from_degrees(60.0))
            .translation
            .z;
        let far = bounds
            .framing_transform(FramingAngles::FRONT, 0.3, Angle::from_degrees(60.0))
            .translation
            .z;
        assert!(
            far > close,
            "smaller fill_fraction → further camera (got close={close}, far={far})"
        );
    }

    #[test]
    fn framing_transform_offcentre_bounds_position_camera_relative_to_centre() {
        // Bounds centered at (10, 0, 0): camera should be at (10, 0, z>0)
        // for FRONT view, not at (0, 0, z>0).
        let bounds = Aabb::new(Vec3::new(9.5, -0.5, -0.5), Vec3::new(10.5, 0.5, 0.5));
        let transform =
            bounds.framing_transform(FramingAngles::FRONT, 0.7, Angle::from_degrees(60.0));
        assert!((transform.translation.x - 10.0).abs() < 1e-5);
        assert!(transform.translation.y.abs() < 1e-5);
        assert!(transform.translation.z > 0.0);
    }

    #[test]
    fn framing_transform_three_quarter_front_pose_orbits_to_positive_x() {
        // Yaw 25° to the right means camera offset has +X component.
        let bounds = Aabb::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        let transform = bounds.framing_transform(
            FramingAngles::THREE_QUARTER_FRONT,
            0.7,
            Angle::from_degrees(60.0),
        );
        assert!(
            transform.translation.x > 0.5,
            "3/4-front yaw must orbit camera to +X (got {})",
            transform.translation.x
        );
        assert!(transform.translation.z > 0.0);
    }
}
