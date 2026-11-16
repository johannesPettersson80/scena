//! Re-placing a retained triangle under a new draw transform.

use std::sync::Arc;

use super::{PreparedDrawTransform, PreparedPrimitive};
use crate::render::prepare::transforms::transform_point_matrix4;

impl PreparedPrimitive {
    /// Marks a triangle whose corners are world positions rather than a
    /// bake of its source node's geometry.
    pub(in crate::render) const fn with_world_authored_corners(mut self) -> Self {
        self.world_authored = true;
        self
    }

    pub(in crate::render) const fn world_authored(&self) -> bool {
        self.world_authored
    }

    /// Re-places the triangle under `draw_transform`, the way the retained
    /// prepare path moves a draw without baking it again.
    ///
    /// The GPU holds the triangle in model space and applies the per-draw
    /// matrix, so swapping the matrix moves the pixels. Frustum culling and
    /// blend ordering read world-space corners on the CPU instead, so those
    /// are carried to the new placement here; otherwise the renderer would
    /// keep culling and sorting the triangle where it was baked.
    ///
    /// Returns `false` when the triangle cannot be carried by swapping
    /// matrices because either placement has no inverse: the vertices the GPU
    /// holds are then not the model-space vertices the new matrix expects.
    #[must_use]
    pub(in crate::render) fn place(&mut self, draw_transform: Arc<PreparedDrawTransform>) -> bool {
        if draw_transform.world_from_model == self.draw_transform.world_from_model {
            self.draw_transform = draw_transform;
            return true;
        }
        let (Some(baked_world_to_model), Some(_)) = (
            self.draw_transform.world_to_model,
            draw_transform.world_to_model,
        ) else {
            return false;
        };
        let world_from_model = draw_transform.world_from_model;
        let positions = match self.model_vertices() {
            Some(model) => [
                model[0].vertex.position,
                model[1].vertex.position,
                model[2].vertex.position,
            ],
            None => self
                .vertices()
                .map(|vertex| transform_point_matrix4(&baked_world_to_model, vertex.position)),
        }
        .map(|model_position| transform_point_matrix4(&world_from_model, model_position));
        self.primitive.set_positions(positions);
        self.draw_transform = draw_transform;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Primitive;
    use crate::material::Color;
    use crate::render::prepare::share_model_space_vertex_buffer;
    use crate::render::prepare::transforms::{
        normal_from_model_matrix, prepared_primitive, world_from_model_matrix,
    };
    use crate::scene::{Transform, Vec3, WorldTransform};

    fn draw_transform(transform: Transform) -> Arc<PreparedDrawTransform> {
        PreparedDrawTransform::shared(
            world_from_model_matrix(transform, Vec3::ZERO),
            normal_from_model_matrix(transform),
        )
    }

    /// The unlit test triangle baked under `transform`, as a full prepare
    /// leaves it, optionally with the shared model-space vertex buffer.
    fn baked(transform: Transform, share_model_vertices: bool) -> PreparedPrimitive {
        let mut primitives = vec![PreparedPrimitive::new_with_draw_transform(
            prepared_primitive(
                &Primitive::unlit_triangle(),
                WorldTransform::from(transform),
                Vec3::ZERO,
            ),
            None,
            Color::WHITE,
            draw_transform(transform),
        )];
        if share_model_vertices {
            share_model_space_vertex_buffer(&mut primitives);
        }
        primitives.remove(0)
    }

    fn corners(primitive: &PreparedPrimitive) -> [Vec3; 3] {
        primitive.vertices().map(|vertex| vertex.position)
    }

    #[test]
    fn placing_a_triangle_carries_its_world_corners_to_the_new_transform() {
        let first = Transform::at(Vec3::new(50.0, 0.0, 0.0)).rotate_y_deg(30.0);
        let second = Transform::at(Vec3::new(-1.0, 2.0, 0.5))
            .rotate_z_deg(70.0)
            .with_scale(Vec3::new(2.0, 0.5, 1.5));
        let expected = corners(&baked(second, false));

        for share_model_vertices in [true, false] {
            let mut primitive = baked(first, share_model_vertices);
            assert!(primitive.place(draw_transform(second)));
            for (placed, expected) in corners(&primitive).into_iter().zip(expected) {
                assert!(
                    placed.abs_diff_eq(expected, 1.0e-4),
                    "a re-placed triangle must sit where a full prepare bakes it \
                     (shared model vertices: {share_model_vertices}): {placed:?} vs {expected:?}"
                );
            }
            assert_eq!(
                primitive.world_from_model(),
                draw_transform(second).world_from_model
            );
        }
    }

    #[test]
    fn placing_a_triangle_where_it_already_is_keeps_its_baked_corners_bit_for_bit() {
        let transform = Transform::at(Vec3::new(0.3, -0.2, 1.0)).rotate_x_deg(40.0);
        let mut primitive = baked(transform, true);
        let before = corners(&primitive);

        assert!(primitive.place(draw_transform(transform)));

        assert_eq!(
            corners(&primitive),
            before,
            "an unmoved triangle must keep the exact corners blend ordering was baked from"
        );
    }

    #[test]
    fn a_placement_without_an_inverse_cannot_be_reached_or_left_by_swapping_matrices() {
        let regular = Transform::at(Vec3::new(0.1, 0.0, 0.0));
        let flattened = regular.with_scale(Vec3::new(1.0, 1.0, 0.0));

        let mut into_singular = baked(regular, true);
        assert!(!into_singular.place(draw_transform(flattened)));
        assert_eq!(
            corners(&into_singular),
            corners(&baked(regular, true)),
            "a refused placement must leave the triangle untouched"
        );

        let mut out_of_singular = baked(flattened, true);
        assert!(!out_of_singular.place(draw_transform(regular)));
    }
}
