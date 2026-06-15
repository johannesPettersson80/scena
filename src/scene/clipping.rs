use crate::diagnostics::LookupError;
use crate::geometry::Aabb;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectionBox {
    bounds: Aabb,
    margin: f32,
    inverted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SceneSectionBoxState {
    section: SectionBox,
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

    pub fn set_section_box(&mut self, section: SectionBox) -> Result<bool, LookupError> {
        section.validate()?;
        if self
            .section_box
            .as_ref()
            .is_some_and(|state| state.section == section)
        {
            return Ok(false);
        }

        let planes = section.planes();
        if let Some(state) = self.section_box.as_mut() {
            for (handle, plane) in state.planes.iter().copied().zip(planes) {
                if let Some(stored) = self.clipping_planes.get_mut(handle) {
                    *stored = plane;
                }
            }
            state.section = section;
        } else {
            let handles = planes
                .into_iter()
                .map(|plane| self.clipping_planes.insert(plane))
                .collect::<Vec<_>>();
            self.section_box = Some(SceneSectionBoxState {
                section,
                planes: handles,
            });
        }
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(true)
    }

    pub fn clear_section_box(&mut self) -> bool {
        let Some(state) = self.section_box.take() else {
            return false;
        };
        for plane in state.planes {
            self.clipping_planes.remove(plane);
        }
        self.structure_revision = self.structure_revision.saturating_add(1);
        true
    }

    pub fn invert_section_box(&mut self, inverted: bool) -> Result<bool, LookupError> {
        let Some(state) = self.section_box.as_mut() else {
            return Err(LookupError::InvalidBounds {
                reason: "section box must be active before it can be inverted",
            });
        };
        if state.section.inverted == inverted {
            return Ok(false);
        }
        state.section.inverted = inverted;
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(true)
    }

    pub fn section_box(&self) -> Option<SectionBox> {
        self.section_box.as_ref().map(|state| state.section)
    }

    pub fn section_box_planes(&self) -> Option<Vec<ClippingPlane>> {
        let state = self.section_box.as_ref()?;
        Some(
            state
                .planes
                .iter()
                .filter_map(|plane| self.clipping_plane(*plane))
                .collect(),
        )
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

impl SectionBox {
    pub const fn from_bounds(bounds: Aabb) -> Self {
        Self {
            bounds,
            margin: 0.0,
            inverted: false,
        }
    }

    pub const fn with_margin(mut self, margin: f32) -> Self {
        self.margin = margin;
        self
    }

    pub const fn with_inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    pub const fn bounds(self) -> Aabb {
        self.bounds
    }

    pub const fn margin(self) -> f32 {
        self.margin
    }

    pub const fn inverted(self) -> bool {
        self.inverted
    }

    pub fn clips(self, point: Vec3) -> bool {
        let inside = self.expanded_bounds().contains(point);
        if self.inverted { inside } else { !inside }
    }

    pub fn planes(self) -> [ClippingPlane; 6] {
        let bounds = self.expanded_bounds();
        [
            ClippingPlane::new(Vec3::X, -bounds.min.x),
            ClippingPlane::new(-Vec3::X, bounds.max.x),
            ClippingPlane::new(Vec3::Y, -bounds.min.y),
            ClippingPlane::new(-Vec3::Y, bounds.max.y),
            ClippingPlane::new(Vec3::Z, -bounds.min.z),
            ClippingPlane::new(-Vec3::Z, bounds.max.z),
        ]
    }

    pub fn expanded_bounds(self) -> Aabb {
        let margin = self.margin.max(0.0);
        Aabb::new(
            self.bounds.min - Vec3::splat(margin),
            self.bounds.max + Vec3::splat(margin),
        )
    }

    fn validate(self) -> Result<(), LookupError> {
        if !self.margin.is_finite() || self.margin < 0.0 {
            return Err(LookupError::InvalidBounds {
                reason: "section box margin must be finite and non-negative",
            });
        }
        let min = self.bounds.min;
        let max = self.bounds.max;
        if !min.is_finite() || !max.is_finite() {
            return Err(LookupError::InvalidBounds {
                reason: "section box bounds must be finite",
            });
        }
        if min.x >= max.x || min.y >= max.y || min.z >= max.z {
            return Err(LookupError::InvalidBounds {
                reason: "section box min must be less than max on every axis",
            });
        }
        Ok(())
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
