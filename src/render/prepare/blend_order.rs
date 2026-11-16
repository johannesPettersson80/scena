//! Back-to-front ordering of alpha-blended triangles.
//!
//! Blended triangles are drawn after everything opaque, farthest first, so
//! each one blends over what lies behind it. That order depends on the camera,
//! which makes it the one part of a prepared triangle list a camera move
//! invalidates. A full prepare orders the triangles once with
//! [`into_back_to_front`]; the retained prepare path re-orders the same
//! triangles for the camera of the frame with [`resort_blended_primitives`]
//! instead of baking the scene again. Both produce the same order for the same
//! camera, including between triangles at equal depth.

use std::cmp::Ordering;

use super::types::{PreparedPrimitive, TransparentPrimitive};
use crate::render::camera::CameraProjection;

/// Depth a blended triangle is ordered by: the mean distance of its corners in
/// front of the camera, or their mean z when no camera can measure them.
pub(super) fn blend_depth(
    primitive: &PreparedPrimitive,
    camera_projection: Option<&CameraProjection>,
) -> f32 {
    if let Some(camera_projection) = camera_projection {
        let vertices = primitive.vertices();
        let mut depth_sum = 0.0;
        let mut depth_count = 0;
        for vertex in vertices {
            if let Some(depth) = camera_projection.camera_depth(vertex.position) {
                depth_sum += depth;
                depth_count += 1;
            }
        }
        if depth_count > 0 {
            return depth_sum / depth_count as f32;
        }
    }

    let vertices = primitive.vertices();
    (vertices[0].position.z + vertices[1].position.z + vertices[2].position.z) / 3.0
}

/// Orders the collected blended triangles farthest first.
///
/// Each triangle is numbered in collection order first. Triangles at exactly
/// the same depth keep that order, and the number travels with the triangle so
/// a later re-sort can break the same ties the same way.
pub(super) fn into_back_to_front(
    mut transparent: Vec<TransparentPrimitive>,
) -> impl Iterator<Item = PreparedPrimitive> {
    for (sequence, entry) in transparent.iter_mut().enumerate() {
        entry.primitive.set_blend_sequence(sequence as u32);
    }
    transparent.sort_by(|left, right| right.depth.total_cmp(&left.depth));
    transparent.into_iter().map(|entry| entry.primitive)
}

/// Re-orders the blended triangles of a prepared list for `camera_projection`.
///
/// Blended triangles form the tail of a prepared list and stay its tail
/// through visibility filtering and culling, which only remove entries. The
/// result is the order [`into_back_to_front`] gives a full prepare from the
/// same camera: farthest first, collection order between equal depths.
pub(in crate::render) fn resort_blended_primitives(
    primitives: &mut [PreparedPrimitive],
    camera_projection: Option<&CameraProjection>,
) {
    let Some(first_blended) = primitives
        .iter()
        .position(|primitive| primitive.blend_sequence().is_some())
    else {
        return;
    };
    primitives[first_blended..].sort_by_cached_key(|primitive| BackToFront {
        depth: blend_depth(primitive, camera_projection),
        sequence: primitive.blend_sequence().unwrap_or(u32::MAX),
    });
}

/// Sort key that puts the farthest triangle first and breaks ties by
/// collection order.
struct BackToFront {
    depth: f32,
    sequence: u32,
}

impl Ord for BackToFront {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .depth
            .total_cmp(&self.depth)
            .then(self.sequence.cmp(&other.sequence))
    }
}

impl PartialOrd for BackToFront {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BackToFront {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for BackToFront {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Backend;
    use crate::geometry::{Primitive, Vertex};
    use crate::material::Color;
    use crate::render::RasterTarget;
    use crate::scene::{Scene, Transform, Vec3};

    const TARGET: RasterTarget = RasterTarget {
        width: 64,
        height: 64,
        backend: Backend::Headless,
    };

    /// A small triangle facing +Z, centred on `centre`.
    fn triangle_at(centre: Vec3) -> PreparedPrimitive {
        let corner = |x: f32, y: f32| Vertex {
            position: centre + Vec3::new(x, y, 0.0),
            color: Color::WHITE,
        };
        PreparedPrimitive::new(
            Primitive::triangle([corner(-0.1, -0.1), corner(0.1, -0.1), corner(0.0, 0.1)]),
            None,
            Color::WHITE,
        )
    }

    fn camera_at(eye: Vec3) -> CameraProjection {
        let mut scene = Scene::new();
        let camera = scene.add_default_camera().expect("camera inserts");
        let node = scene.camera_node(camera).expect("camera node resolves");
        scene
            .set_transform(node, Transform::at(eye).looking_at(Vec3::ZERO, Vec3::Y))
            .expect("camera moves");
        CameraProjection::from_scene(&scene, camera, TARGET).expect("camera projects")
    }

    /// What a full prepare from `camera` appends to its primitive list.
    fn full_prepare_order(
        centres: &[Vec3],
        camera: Option<&CameraProjection>,
    ) -> Vec<PreparedPrimitive> {
        into_back_to_front(
            centres
                .iter()
                .map(|centre| {
                    let primitive = triangle_at(*centre);
                    TransparentPrimitive {
                        depth: blend_depth(&primitive, camera),
                        primitive,
                    }
                })
                .collect(),
        )
        .collect()
    }

    fn sequences(primitives: &[PreparedPrimitive]) -> Vec<u32> {
        primitives
            .iter()
            .map(|primitive| primitive.blend_sequence().expect("blended triangle"))
            .collect()
    }

    #[test]
    fn a_full_prepare_orders_blended_triangles_farthest_first() {
        let centres = [
            Vec3::new(0.0, 0.0, 0.5),
            Vec3::new(0.0, 0.0, -0.5),
            Vec3::new(0.0, 0.0, 0.0),
        ];
        let front = camera_at(Vec3::new(0.0, 0.0, 3.0));
        let back = camera_at(Vec3::new(0.0, 0.0, -3.0));

        assert_eq!(
            sequences(&full_prepare_order(&centres, Some(&front))),
            [1, 2, 0]
        );
        assert_eq!(
            sequences(&full_prepare_order(&centres, Some(&back))),
            [0, 2, 1]
        );
    }

    #[test]
    fn a_re_sort_reproduces_the_full_prepare_order_of_the_new_camera() {
        // From the side cameras two pairs of triangles sit at the same
        // distance, so their order hangs on the last bit of the depth or, when
        // even that agrees, on collection order.
        let centres = [
            Vec3::new(0.4, 0.0, 0.5),
            Vec3::new(-0.4, 0.0, -0.5),
            Vec3::new(0.4, 0.0, -0.5),
            Vec3::new(-0.4, 0.0, 0.5),
            Vec3::new(0.0, 0.2, 0.0),
        ];
        let eyes = [
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -3.0),
            Vec3::new(-3.0, 0.0, 0.0),
            Vec3::new(2.0, 1.5, -1.0),
        ];
        for first_eye in eyes {
            // The retained template keeps the order of the camera it was
            // first prepared for, opaque triangles ahead of the blended tail.
            let mut retained = vec![triangle_at(Vec3::new(0.0, -0.5, 0.0))];
            retained.extend(full_prepare_order(&centres, Some(&camera_at(first_eye))));
            for eye in eyes {
                let camera = camera_at(eye);
                let mut resorted = retained.clone();
                resort_blended_primitives(&mut resorted, Some(&camera));

                assert_eq!(
                    resorted[0].blend_sequence(),
                    None,
                    "opaque triangles keep their place ahead of the blended tail"
                );
                assert_eq!(
                    sequences(&resorted[1..]),
                    sequences(&full_prepare_order(&centres, Some(&camera))),
                    "re-sorting a template first prepared from {first_eye:?} for a camera at \
                     {eye:?} must give the order of a full prepare from there"
                );
            }
        }
    }

    #[test]
    fn exact_depth_ties_fall_back_to_collection_order_whatever_order_they_arrive_in() {
        let centres = [Vec3::new(0.3, 0.0, 0.0), Vec3::new(-0.3, 0.0, 0.0)];
        let camera = camera_at(Vec3::new(0.0, 0.0, 3.0));
        let ordered = full_prepare_order(&centres, Some(&camera));
        assert_eq!(
            sequences(&ordered),
            [0, 1],
            "equal depths keep collection order"
        );

        let mut reversed = ordered.clone();
        reversed.reverse();
        resort_blended_primitives(&mut reversed, Some(&camera));

        assert_eq!(sequences(&reversed), [0, 1]);
    }

    #[test]
    fn without_a_camera_the_order_falls_back_to_mean_z() {
        let centres = [Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 2.0)];
        let mut ordered = full_prepare_order(&centres, None);
        assert_eq!(sequences(&ordered), [1, 0]);

        ordered.reverse();
        resort_blended_primitives(&mut ordered, None);
        assert_eq!(sequences(&ordered), [1, 0]);
    }
}
