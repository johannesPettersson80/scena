//! Turning a ray hit into a shaded surface point, and the visibility queries
//! the integrator makes against the traced scene.

use glam::Vec3;

use super::bsdf::dielectric_fresnel;
use super::bvh::Hit;
use super::material::{Coverage, Surface, TraceMaterial};
use super::sampling::Rng;
use super::scene::{NONE, TraceObject, TraceScene};

pub(super) struct Shading<'a> {
    pub(super) position: Vec3,
    /// Geometric normal on the side the ray arrived from.
    pub(super) geometric: Vec3,
    /// Shading normal on that same side.
    pub(super) normal: Vec3,
    /// The ray met the front face.
    pub(super) front: bool,
    pub(super) surface: Surface,
    pub(super) material: &'a TraceMaterial,
}

fn mix(values: [Vec3; 3], hit: &Hit) -> Vec3 {
    values[0] * (1.0 - hit.u - hit.v) + values[1] * hit.u + values[2] * hit.v
}

impl TraceScene {
    fn object(&self, triangle: u32) -> &TraceObject {
        &self.objects[self.attributes[triangle as usize].object as usize]
    }

    /// Normal of a triangle as it is wound, not normalised.
    pub(super) fn winding_normal(&self, triangle: u32) -> Vec3 {
        let [a, b, c] = self.positions[triangle as usize];
        (b - a).cross(c - a)
    }

    fn uv(&self, hit: &Hit) -> [f32; 2] {
        let uvs = &self.attributes[hit.triangle as usize].uvs;
        let weight = 1.0 - hit.u - hit.v;
        [
            uvs[0][0] * weight + uvs[1][0] * hit.u + uvs[2][0] * hit.v,
            uvs[0][1] * weight + uvs[1][1] * hit.u + uvs[2][1] * hit.v,
        ]
    }

    /// Interpolated vertex colour and alpha; white where there is none.
    fn vertex_color(&self, hit: &Hit) -> (Vec3, f32) {
        let index = self.attributes[hit.triangle as usize].colors;
        if index == NONE {
            return (Vec3::ONE, 1.0);
        }
        let colors = &self.colors[index as usize];
        let weights = [1.0 - hit.u - hit.v, hit.u, hit.v];
        let channel = |channel: usize| {
            (0..3)
                .map(|corner| colors[corner][channel] * weights[corner])
                .sum::<f32>()
        };
        (Vec3::new(channel(0), channel(1), channel(2)), channel(3))
    }

    /// Chance that the surface at `hit` stops a ray rather than letting it
    /// through a cut-out or a blended, see-through texel.
    fn opacity(&self, hit: &Hit) -> f32 {
        let object = self.object(hit.triangle);
        let material = &self.materials[object.material as usize];
        if material.coverage == Coverage::Opaque {
            return object.tint_alpha;
        }
        let alpha = self.vertex_color(hit).1 * object.tint_alpha;
        material.opacity(&self.textures, self.uv(hit), alpha)
    }

    /// Whether a ray along `direction` interacts with the surface at `hit`.
    fn accepts(&self, hit: &Hit, direction: Vec3, skip: u32, rng: &mut Rng) -> bool {
        if hit.triangle == skip {
            return false;
        }
        let object = self.object(hit.triangle);
        if object.culls_back_faces && self.winding_normal(hit.triangle).dot(direction) > 0.0 {
            return false;
        }
        if object.tests_coverage {
            let opacity = self.opacity(hit);
            if opacity < 1.0 && rng.next() >= opacity {
                return false;
            }
        }
        true
    }

    /// Nearest surface along a ray. `skip` names the triangle the ray leaves,
    /// which a straight ray cannot meet again.
    pub(super) fn closest(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        skip: u32,
        rng: &mut Rng,
    ) -> Option<Hit> {
        self.bvh.intersect(
            &self.positions,
            origin,
            direction,
            max_distance,
            false,
            |hit| self.accepts(hit, direction, skip, rng),
        )
    }

    /// Share of light that travels `max_distance` from `origin` along
    /// `direction`, and whether it crossed glass on the way.
    ///
    /// Glass lets the light through, tinted and dimmed by its reflection,
    /// without bending it. Light focused by refraction is therefore not
    /// traced; a glass object casts a soft tinted shadow instead.
    pub(super) fn transmittance(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        skip: u32,
        rng: &mut Rng,
    ) -> (Vec3, bool) {
        let mut transmittance = Vec3::ONE;
        let mut crossed_glass = false;
        let blocked = self
            .bvh
            .intersect(
                &self.positions,
                origin,
                direction,
                max_distance,
                true,
                |hit| {
                    if !self.accepts(hit, direction, skip, rng) {
                        return false;
                    }
                    let material = &self.materials[self.object(hit.triangle).material as usize];
                    let share = material.shadow_transmission();
                    if share <= 0.0 || rng.next() >= share {
                        return true;
                    }
                    let normal = self.winding_normal(hit.triangle).normalize_or_zero();
                    let reflected =
                        dielectric_fresnel(normal.dot(direction).abs(), 1.0 / material.ior());
                    transmittance *= material.transmission_tint(&self.textures, self.uv(hit))
                        * (1.0 - reflected);
                    crossed_glass = true;
                    transmittance.max_element() <= 1.0e-4
                },
            )
            .is_some();
        (
            if blocked { Vec3::ZERO } else { transmittance },
            crossed_glass,
        )
    }

    pub(super) fn shade(&self, direction: Vec3, hit: &Hit) -> Shading<'_> {
        let attributes = &self.attributes[hit.triangle as usize];
        let object = &self.objects[attributes.object as usize];
        let material = &self.materials[object.material as usize];
        let winding = self.winding_normal(hit.triangle).normalize_or_zero();
        let front = winding.dot(direction) < 0.0;
        let mut normal = mix(attributes.normals, hit).normalize_or_zero();
        if normal.dot(winding) <= 0.0 {
            // Vertex normals that disagree with the winding would light the
            // wrong side; the face normal is the safe reading.
            normal = winding;
        }
        let (vertex_color, _) = self.vertex_color(hit);
        let surface = material.surface(&self.textures, self.uv(hit), vertex_color * object.tint);
        if let Some(mapped) = surface.normal_map
            && attributes.tangents != NONE
        {
            let tangents = &self.tangents[attributes.tangents as usize];
            let tangent = mix(tangents.map(|t| Vec3::new(t[0], t[1], t[2])), hit);
            let tangent = (tangent - normal * normal.dot(tangent)).normalize_or_zero();
            if tangent != Vec3::ZERO {
                let bitangent = normal.cross(tangent) * tangents[0][3];
                let bent = (tangent * mapped.x + bitangent * mapped.y + normal * mapped.z)
                    .normalize_or_zero();
                if bent != Vec3::ZERO {
                    normal = bent;
                }
            }
        }
        let side = if front { 1.0 } else { -1.0 };
        let geometric = winding * side;
        let mut normal = normal * side;
        if normal.dot(direction) > -1.0e-4 {
            // A bent normal that faces away from the viewer would put the
            // view direction below the shading horizon.
            normal = geometric;
        }
        Shading {
            position: mix(self.positions[hit.triangle as usize], hit),
            geometric,
            normal,
            front,
            surface,
            material,
        }
    }
}
