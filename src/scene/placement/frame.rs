//! Placement verbs solved against the exact world matrix of a node's parent.

use glam::Affine3A;

use crate::geometry::Aabb;

use super::super::view_math::transform_aabb;
use super::super::{Mat4, NodeKey, Scene, Transform, Vec3, WorldTransform, aim_rotation};
use super::{
    ScenePlacementDiagnosticV1, placement_align_to_feature_transform, placement_center_transform,
    placement_ground_transform, placement_look_at_transform, placement_place_on_feature_transform,
    sizing::fit_to_size_factor,
};

#[cfg(feature = "scene-host")]
mod authoring;

type Placement = Result<Transform, Box<ScenePlacementDiagnosticV1>>;

/// The exact world frame a node is placed in: the world placement of its
/// parent.
///
/// A [`Transform`] cannot describe a node under a rotated, non-uniformly
/// scaled ancestor, because that placement is sheared. The free
/// `placement_*_transform` functions take such a transform and are therefore
/// exact only for top-level nodes and shear-free hierarchies. A
/// `PlacementFrame` keeps the parent's world matrix instead, and every verb
/// returns the node's new *local* transform, so the node lands where the verb
/// says under any invertible parent.
///
/// What a sheared parent makes impossible is stated per verb. Moving and
/// uniformly scaling a node are always exact. Verbs that turn a node keep its
/// position and the aimed axis exact and bring the second axis as close as a
/// rotation can.
///
/// Hierarchies of translation, rotation, and uniform scale use the same
/// arithmetic as the free functions and return identical results.
///
/// # Examples
///
/// ```
/// use scena::{Aabb, PlacementFrame, Transform, Vec3};
///
/// // A rotated child of this frame is sheared in the world.
/// let frame = PlacementFrame::WORLD
///     .child(Transform::IDENTITY.with_scale(Vec3::new(3.0, 1.0, 1.0)));
/// let local = Transform::at(Vec3::new(0.5, 2.0, 0.0)).rotate_z_deg(40.0);
/// let bounds = Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5));
///
/// let grounded = frame.ground(bounds, local, 0.0).expect("frame is invertible");
///
/// let resting = frame.world_bounds(bounds, grounded);
/// assert!(resting.min.y.abs() < 1.0e-5);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementFrame {
    /// `None` is the world itself, which has no transform to undo.
    world: Option<WorldTransform>,
}

/// A node being placed: its frame, its local transform, and where the two
/// put it in the world.
#[derive(Clone, Copy)]
struct Placed {
    parent: Option<WorldTransform>,
    local: Transform,
    world: WorldTransform,
}

impl Scene {
    /// The exact world frame of `node`, for placing its children.
    ///
    /// Returns `None` when `node` is not in this scene.
    pub fn placement_frame(&self, node: NodeKey) -> Option<PlacementFrame> {
        self.resolved_world(node).map(PlacementFrame::from_world)
    }
}

impl PlacementFrame {
    /// The world itself: the frame of a node with no parent transform.
    pub const WORLD: Self = Self { world: None };

    /// The frame an arbitrary affine world matrix describes, such as
    /// [`Scene::world_matrix`] of the parent node.
    pub fn from_world_matrix(matrix: Mat4) -> Self {
        Self::from_world(WorldTransform::from_matrix(Affine3A::from_mat4(matrix)))
    }

    pub(crate) const fn from_world(world: WorldTransform) -> Self {
        Self { world: Some(world) }
    }

    /// The frame of a node placed at `local` in this frame, for placing that
    /// node's own children.
    pub fn child(self, local: Transform) -> Self {
        Self::from_world(self.node_world(local))
    }

    /// The frame of `inner`, a frame given in this frame's space: the two
    /// composed exactly. A feature frame such as
    /// [`ConnectorFrame::local_frame`](crate::ConnectorFrame::local_frame)
    /// becomes a world frame this way.
    pub fn child_frame(self, inner: Self) -> Self {
        match (self.world, inner.world) {
            (Some(outer), Some(inner)) => Self::from_world(outer.then(inner)),
            (outer, inner) => Self {
                world: outer.or(inner),
            },
        }
    }

    /// The exact world matrix of this frame.
    pub fn world_matrix(self) -> Mat4 {
        self.world.map_or(Mat4::IDENTITY, WorldTransform::to_mat4)
    }

    /// World bounds of `bounds` on a node placed at `local` in this frame,
    /// through the exact world matrix.
    pub fn world_bounds(self, bounds: Aabb, local: Transform) -> Aabb {
        transform_aabb(bounds, self.node_world(local))
    }

    /// Moves the node so the centre of its world bounds is `target`. Exact in
    /// every invertible frame; rotation and scale are untouched.
    pub fn center(self, bounds: Aabb, local: Transform, target: Vec3) -> Placement {
        let placed = self.placed(local);
        match placed.exact() {
            Some((node, parent)) => {
                local_in_parent(parent, placement_center_transform(bounds, node, target))
            }
            None => placed.moved_by(target - transform_aabb(bounds, placed.world).center()),
        }
    }

    /// Moves the node along world Y so its world bounds rest on `ground_y`.
    /// Exact in every invertible frame; rotation and scale are untouched.
    pub fn ground(self, bounds: Aabb, local: Transform, ground_y: f32) -> Placement {
        let placed = self.placed(local);
        match placed.exact() {
            Some((node, parent)) => {
                local_in_parent(parent, placement_ground_transform(bounds, node, ground_y))
            }
            None => {
                let lowest = transform_aabb(bounds, placed.world).min.y;
                placed.moved_by(Vec3::new(0.0, ground_y - lowest, 0.0))
            }
        }
    }

    /// Scales the node uniformly about its origin so the largest extent of
    /// its world bounds falls inside `min_size..=max_size`. Exact in every
    /// frame: a uniform local scale scales the world bounds by the same
    /// factor whatever the parent does.
    pub fn fit_to_size(
        self,
        bounds: Aabb,
        local: Transform,
        min_size: Option<f32>,
        max_size: Option<f32>,
    ) -> Placement {
        let placed = self.placed(local);
        let factor = fit_to_size_factor(transform_aabb(bounds, placed.world), min_size, max_size)?;
        placed.scaled_by(factor)
    }

    /// Turns the node so its local -Z axis points at the world point
    /// `target`, keeping its position and scale.
    ///
    /// The position and the viewing direction are exact in every invertible
    /// frame. The node's Y axis lies in the plane of the viewing direction
    /// and `up`; under a sheared frame it cannot also be perpendicular to the
    /// viewing direction.
    pub fn look_at(self, local: Transform, target: Vec3, up: Vec3) -> Placement {
        let placed = self.placed(local);
        if let Some((node, parent)) = placed.exact() {
            let desired = placement_look_at_transform(node, target, up)?;
            if let Some(local) = turned_local_in_parent(parent, desired) {
                return local;
            }
        }
        let eye = placed.world.translation();
        // Shares the validation and its diagnostics with the free function.
        placement_look_at_transform(Transform::at(eye), target, up)?;
        let aimed = placed.aimed_from(eye, target, up, local.scale)?;
        Ok(Transform {
            translation: local.translation,
            ..aimed
        })
    }

    /// Turns and moves the node so `feature`, a frame given in the node's
    /// local space, mates with `target`, keeping the node's scale.
    ///
    /// `feature` is `PlacementFrame::WORLD.child(transform)` for a frame
    /// authored as a transform, or
    /// [`ConnectorFrame::local_frame`](crate::ConnectorFrame::local_frame).
    /// `target` is the exact world frame of the feature to mate with, built
    /// with [`Scene::placement_frame`] and [`Self::child_frame`]. When either
    /// feature is sheared in the world, the frames that mate are the rigid
    /// frames of the two features: the origins coincide and the X axes are
    /// parallel exactly, and the Y axes agree once each is made perpendicular
    /// to its X axis. Shear-free features of uniformly scaled nodes mate as
    /// whole frames.
    pub fn align_feature(self, local: Transform, feature: Self, target: Self) -> Placement {
        let placed = self.placed(local);
        let feature = feature.world.unwrap_or(WorldTransform::IDENTITY);
        let target_world = target.world.unwrap_or(WorldTransform::IDENTITY);
        if let (Some((node, parent)), Some(feature), Some(target_feature)) = (
            placed.exact(),
            feature.exact_trs(),
            target_world.exact_trs(),
        ) && scale_keeps_feature_rigid(node.scale, feature)
        {
            let desired = placement_align_to_feature_transform(node, feature, target_feature)?;
            if let Some(local) = turned_local_in_parent(parent, desired) {
                return local;
            }
        }
        // Shares the validation and its diagnostics with the free function.
        placement_align_to_feature_transform(
            placed.world.trs(),
            feature.trs(),
            target_world.trs(),
        )?;
        let target_frame = target_world.rigid_frame().ok_or_else(|| {
            diagnostic(
                "non_invertible_feature",
                "$.target",
                "target feature frame collapses an axis in the world",
                "fix the scale of the target feature or of the nodes above it",
            )
        })?;
        let parent = placed.parent.unwrap_or(WorldTransform::IDENTITY);
        let rotation = aim_rotation(
            parent,
            local.scale * feature.transform_vector(Vec3::X),
            local.scale * feature.transform_vector(Vec3::Y),
            target_frame.x_axis(),
            target_frame.y_axis(),
        )
        .ok_or_else(non_invertible_frame)?;
        let origin = placed.local_point(target_frame.origin)?;
        finite(Transform {
            translation: origin - rotation * (local.scale * feature.translation()),
            rotation,
            scale: local.scale,
        })
    }

    /// Moves the node so the origin of `feature`, a frame given in the node's
    /// local space, lands on the origin of `target`. Exact in every
    /// invertible frame; rotation and scale are untouched.
    pub fn place_feature_on(self, local: Transform, feature: Self, target: Self) -> Placement {
        let placed = self.placed(local);
        let feature = feature.world.unwrap_or(WorldTransform::IDENTITY);
        let target_world = target.world.unwrap_or(WorldTransform::IDENTITY);
        if let (Some((node, parent)), Some(feature), Some(target_feature)) = (
            placed.exact(),
            feature.exact_trs(),
            target_world.exact_trs(),
        ) {
            return local_in_parent(
                parent,
                placement_place_on_feature_transform(node, feature, target_feature)?,
            );
        }
        // Shares the validation and its diagnostics with the free function.
        placement_place_on_feature_transform(
            placed.world.trs(),
            feature.trs(),
            target_world.trs(),
        )?;
        placed.moved_by(
            target_world.translation() - placed.world.transform_point(feature.translation()),
        )
    }

    fn node_world(self, local: Transform) -> WorldTransform {
        match self.world {
            Some(parent) => parent.child(local),
            None => WorldTransform::from(local),
        }
    }

    fn placed(self, local: Transform) -> Placed {
        Placed {
            parent: self.world,
            local,
            world: self.node_world(local),
        }
    }
}

impl Placed {
    /// The node's world transform and its parent's, when both are exactly
    /// translation, rotation, and scale. The arithmetic the placement verbs
    /// have always used is exact for such a node and is kept bit for bit.
    fn exact(self) -> Option<(Transform, Option<Transform>)> {
        let node = self.world.exact_trs()?;
        match self.parent {
            None => Some((node, None)),
            Some(parent) => Some((node, Some(parent.exact_trs()?))),
        }
    }

    /// The node moved by a world-space offset. A translation in the world is
    /// a translation in the parent, carried back by the parent's linear part.
    fn moved_by(self, world_delta: Vec3) -> Placement {
        let local_delta = match self.parent {
            None => world_delta,
            Some(parent) => parent
                .inverse()
                .ok_or_else(non_invertible_frame)?
                .transform_vector(world_delta),
        };
        finite(Transform {
            translation: self.local.translation + local_delta,
            ..self.local
        })
    }

    /// The node scaled uniformly about its own origin.
    fn scaled_by(self, factor: f32) -> Placement {
        match self.exact() {
            Some((node, parent)) => local_in_parent(
                parent,
                Transform {
                    scale: node.scale * factor,
                    ..node
                },
            ),
            None => finite(Transform {
                scale: self.local.scale * factor,
                ..self.local
            }),
        }
    }

    /// A world point in the space the node's local transform is written in.
    fn local_point(self, world_point: Vec3) -> Result<Vec3, Box<ScenePlacementDiagnosticV1>> {
        match self.parent {
            None => Ok(world_point),
            Some(parent) => Ok(parent
                .inverse()
                .ok_or_else(non_invertible_frame)?
                .transform_point(world_point)),
        }
    }

    /// The node put at the world point `eye` with its -Z axis on `target`.
    /// Aiming in the parent's space aims exactly in the world, because the
    /// parent carries lines onto lines.
    fn aimed_from(self, eye: Vec3, target: Vec3, up: Vec3, scale: Vec3) -> Placement {
        let (local_eye, local_target, local_up) = match self.parent {
            None => (eye, target, up),
            Some(parent) => {
                let local_from_world = parent.inverse().ok_or_else(non_invertible_frame)?;
                (
                    local_from_world.transform_point(eye),
                    local_from_world.transform_point(target),
                    local_from_world.transform_vector(up),
                )
            }
        };
        finite(Transform {
            scale,
            ..Transform::at(local_eye).looking_at(local_target, local_up)
        })
    }
}

/// The local transform that gives a node the world transform `world` under
/// an exact-TRS `parent`. Exact whenever `world` has the rotation its node
/// already had under that parent, which is every verb that only moves or
/// uniformly scales.
fn local_in_parent(parent: Option<Transform>, world: Transform) -> Placement {
    match parent {
        None => Ok(world),
        Some(parent) => trs_local(parent, world).ok_or_else(non_invertible_frame),
    }
}

/// [`local_in_parent`] for a world transform whose rotation is new. Returns
/// `None` when the parent's non-uniform scale would shear that rotation, so
/// the answer has to come from the matrix instead.
fn turned_local_in_parent(parent: Option<Transform>, world: Transform) -> Option<Placement> {
    let Some(parent) = parent else {
        return Some(Ok(world));
    };
    let Some(local) = trs_local(parent, world) else {
        return Some(Err(non_invertible_frame()));
    };
    let unrotated = local.rotation.x == 0.0 && local.rotation.y == 0.0 && local.rotation.z == 0.0;
    (is_uniform(parent.scale) || unrotated).then_some(Ok(local))
}

/// A node's scale leaves a feature frame rigid when it is uniform or the
/// feature is not rotated against it.
fn scale_keeps_feature_rigid(scale: Vec3, feature: Transform) -> bool {
    is_uniform(scale)
        || (feature.rotation.x == 0.0 && feature.rotation.y == 0.0 && feature.rotation.z == 0.0)
}

fn is_uniform(scale: Vec3) -> bool {
    scale.x == scale.y && scale.y == scale.z
}

/// Inverts one translation, rotation, and scale against another, with the
/// arithmetic recipe and command-line placement have always used.
fn trs_local(parent: Transform, world: Transform) -> Option<Transform> {
    if !parent.scale.is_finite() || parent.scale.abs().min_element() <= f32::EPSILON {
        return None;
    }
    let inverse_parent_rotation = parent.rotation.normalize().inverse();
    let local_translation = inverse_parent_rotation * (world.translation - parent.translation);
    Some(Transform {
        translation: local_translation / parent.scale,
        rotation: (inverse_parent_rotation * world.rotation).normalize(),
        scale: world.scale / parent.scale,
    })
}

fn finite(transform: Transform) -> Placement {
    if transform.translation.is_finite()
        && transform.rotation.is_finite()
        && transform.scale.is_finite()
    {
        Ok(transform)
    } else {
        Err(non_invertible_frame())
    }
}

fn non_invertible_frame() -> Box<ScenePlacementDiagnosticV1> {
    diagnostic(
        "non_invertible_parent_transform",
        "$",
        "parent transform cannot be inverted: its scale is zero or not finite",
        "use a parent transform with finite non-zero scale",
    )
}

fn diagnostic(
    code: &str,
    path: &str,
    message: &str,
    help: &str,
) -> Box<ScenePlacementDiagnosticV1> {
    Box::new(ScenePlacementDiagnosticV1::new(code, path, message, help))
}
