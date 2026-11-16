//! Adds an object's rounded edges and surface finish to a shaded hit.

use glam::Vec3;

use super::bvh::Hit;
use super::sampling::Rng;
use super::scene::TraceScene;
use super::shading::Shading;

/// How a render treats surface detail, from its options.
#[derive(Clone, Copy)]
pub(super) struct DetailSettings {
    /// Finishes and rounded edges are traced at all.
    pub(super) enabled: bool,
    /// Edge radius of surfaces whose material has no finish.
    pub(super) edge_radius: f32,
}

/// Smallest cosine between a shading normal and the direction to the viewer.
const MIN_VIEW_COSINE: f32 = 1.0e-3;

impl TraceScene {
    /// Whether the surface `triangle` belongs to has rounded edges or a
    /// finish.
    pub(super) fn detailed(&self, triangle: u32) -> bool {
        let object = &self.objects[self.attributes[triangle as usize].object as usize];
        object.edge_radius > 0.0 || object.finish.is_some()
    }

    /// Rounds the edges and applies the finish of the surface at `hit`.
    /// `footprint` is the width of an image pixel on the surface there.
    pub(super) fn refine(
        &self,
        shading: &mut Shading<'_>,
        hit: &Hit,
        direction: Vec3,
        footprint: f32,
        rng: &mut Rng,
    ) {
        if shading.surface.unlit {
            return;
        }
        let object = &self.objects[self.attributes[hit.triangle as usize].object as usize];
        let side = if shading.front { 1.0 } else { -1.0 };
        let outward = shading.geometric * side;
        let mut normal = shading.normal * side;
        let mut convexity = 0.0;
        if object.edge_radius > 0.0 {
            let (_, smooth) = self.outward_normals(hit);
            let rounded = self.rounded(
                hit.triangle,
                shading.position,
                outward,
                smooth,
                object.edge_radius,
                rng,
            );
            // The rounding turns the surface, and whatever a normal map
            // added to the smooth normal turns with it.
            normal = (normal + rounded.normal - smooth).normalize_or(rounded.normal);
            convexity = rounded.convexity;
        }
        if let Some(finish) = &object.finish {
            let relief = finish.relief(shading.position, normal, footprint, rng);
            let along = relief.slope - normal * relief.slope.dot(normal);
            normal = (normal - along).normalize_or(normal);
            let enclosed = finish.grime_reach > 0.0
                && self.enclosed(
                    hit.triangle,
                    shading.position,
                    outward,
                    finish.grime_reach,
                    rng,
                );
            finish.apply(&mut shading.surface, &relief, convexity, enclosed);
        }
        // A normal leaning away from the viewer would put the view direction
        // below the shading horizon; it is turned back just far enough.
        let mut facing = normal * side;
        let shortfall = MIN_VIEW_COSINE - facing.dot(-direction);
        if shortfall > 0.0 {
            facing = (facing - direction * shortfall).normalize_or(shading.normal);
        }
        shading.normal = facing;
    }
}
