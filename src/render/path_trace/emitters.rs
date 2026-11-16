//! Emissive surfaces as light sources: a table of the scene's glowing
//! triangles that shading points sample directly instead of waiting for a
//! bounce ray to find them by chance.

use glam::Vec3;

use super::bvh::Hit;
use super::scene::TraceScene;

pub(super) struct Emitters {
    /// Emissive triangle indices in ascending order.
    triangles: Vec<u32>,
    /// Probability of choosing each triangle, per unit of its area.
    density: Vec<f32>,
    cdf: Vec<f32>,
}

/// A point on an emissive triangle as seen from a shading point.
pub(super) struct EmitterSample {
    pub(super) direction: Vec3,
    pub(super) distance: f32,
    pub(super) radiance: Vec3,
    /// Density of `direction` per unit solid angle.
    pub(super) pdf: f32,
}

impl Emitters {
    pub(super) fn build(scene: &TraceScene) -> Self {
        let mut triangles = Vec::new();
        let mut power = Vec::new();
        let mut areas = Vec::new();
        for (index, attributes) in scene.attributes.iter().enumerate() {
            let material =
                &scene.materials[scene.objects[attributes.object as usize].material as usize];
            let emission = material.emissive_luminance();
            let area = 0.5 * scene.winding_normal(index as u32).length();
            if emission > 0.0 && area > 0.0 {
                triangles.push(index as u32);
                power.push(f64::from(emission) * f64::from(area));
                areas.push(area);
            }
        }
        let total: f64 = power.iter().sum();
        let mut running = 0.0_f64;
        let cdf = power
            .iter()
            .map(|power| {
                running += power;
                (running / total) as f32
            })
            .collect();
        let density = power
            .iter()
            .zip(&areas)
            .map(|(power, area)| (power / total) as f32 / area)
            .collect();
        Self {
            triangles,
            density,
            cdf,
        }
    }

    pub(super) fn len(&self) -> usize {
        self.triangles.len()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Density per unit solid angle with which [`Self::sample`] reaches
    /// `triangle` from a point `distance` away that sees it at `emitter_cos`.
    /// Zero for a triangle that does not emit.
    pub(super) fn pdf(&self, triangle: u32, distance: f32, emitter_cos: f32) -> f32 {
        match self.triangles.binary_search(&triangle) {
            Ok(slot) => self.density[slot] * distance * distance / emitter_cos.abs().max(1.0e-6),
            Err(_) => 0.0,
        }
    }

    /// Picks a point on an emissive triangle and reports what `point` sees.
    pub(super) fn sample(
        &self,
        scene: &TraceScene,
        point: Vec3,
        pick: f32,
        u: (f32, f32),
    ) -> Option<EmitterSample> {
        let slot = self
            .cdf
            .partition_point(|value| *value <= pick)
            .min(self.triangles.len().checked_sub(1)?);
        let triangle = self.triangles[slot];
        // Uniform point on the triangle.
        let root = u.0.sqrt();
        let hit = Hit {
            distance: 0.0,
            u: u.1 * root,
            v: root * (1.0 - u.1),
            triangle,
        };
        let [a, b, c] = scene.positions[triangle as usize];
        let to_emitter = a * (1.0 - hit.u - hit.v) + b * hit.u + c * hit.v - point;
        let distance = to_emitter.length();
        if distance <= 0.0 || distance.is_nan() {
            return None;
        }
        let direction = to_emitter / distance;
        let emitter_cos = scene
            .winding_normal(triangle)
            .normalize_or_zero()
            .dot(direction);
        let shading = scene.shade(direction, &hit);
        // A single-sided surface glows only from its front.
        if emitter_cos.abs() < 1.0e-6 || !(shading.front || shading.material.double_sided) {
            return None;
        }
        Some(EmitterSample {
            direction,
            distance,
            radiance: shading.surface.emission,
            pdf: self.density[slot] * distance * distance / emitter_cos.abs(),
        })
    }
}
