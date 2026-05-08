use crate::diagnostics::LookupError;

use super::{ClippingPlaneKey, Scene, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClippingPlane {
    normal: Vec3,
    distance: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClippingPlaneSet {
    planes: Vec<ClippingPlaneKey>,
}

impl Scene {
    pub fn add_clipping_plane(&mut self, plane: ClippingPlane) -> ClippingPlaneKey {
        self.structure_revision = self.structure_revision.saturating_add(1);
        self.clipping_planes.insert(plane)
    }

    pub fn clipping_plane(&self, plane: ClippingPlaneKey) -> Option<ClippingPlane> {
        self.clipping_planes.get(plane).copied()
    }

    pub fn set_clipping_planes(&mut self, set: ClippingPlaneSet) -> Result<(), LookupError> {
        for plane in set.planes() {
            if !self.clipping_planes.contains_key(*plane) {
                return Err(LookupError::ClippingPlaneNotFound(*plane));
            }
        }
        self.active_clipping_planes = set;
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(())
    }

    pub fn clipping_planes(&self) -> &ClippingPlaneSet {
        &self.active_clipping_planes
    }

    pub(crate) fn active_clipping_plane_values(&self) -> impl Iterator<Item = ClippingPlane> + '_ {
        self.active_clipping_planes
            .planes()
            .iter()
            .filter_map(|plane| self.clipping_plane(*plane))
    }
}

impl ClippingPlane {
    pub const fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    pub const fn normal(self) -> Vec3 {
        self.normal
    }

    pub const fn distance(self) -> f32 {
        self.distance
    }

    pub fn contains(self, point: Vec3) -> bool {
        self.normal.x * point.x + self.normal.y * point.y + self.normal.z * point.z
            >= -self.distance
    }
}

impl ClippingPlaneSet {
    pub fn new() -> Self {
        Self { planes: Vec::new() }
    }

    pub fn with_plane(mut self, plane: ClippingPlaneKey) -> Self {
        self.planes.push(plane);
        self
    }

    pub fn planes(&self) -> &[ClippingPlaneKey] {
        &self.planes
    }
}
