use std::collections::BTreeMap;

use super::common::vec3;
use crate::assets::DefaultAssetFetcher;
use crate::geometry::Aabb;
use crate::scene::recipe::{
    SceneRecipeDiagnosticV1, SceneRecipeLookAtTargetV1, SceneRecipeTransformV1,
};
use crate::scene::view_math::transform_aabb;
use crate::scene::{ScenePlacementDiagnosticV1, WorldTransform};
use crate::scene_host::SceneHostCore;
use crate::{
    NodeKey, PlacementFrame, SceneImport, Transform, Vec3, placement_center_transform,
    placement_ground_transform,
};

use super::super::{error_diagnostic, scene_host_error_diagnostic};

pub(in crate::scene_host::recipe) fn apply_import_transform(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    roots: &[u64],
    transform: Option<&SceneRecipeTransformV1>,
    import_path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(transform) = transform else {
        return;
    };
    let transform = match Transform::try_from(transform) {
        Ok(transform) => transform,
        Err(error) => {
            diagnostics.push(error_diagnostic(
                format!("{import_path}.transform"),
                "invalid_import_transform",
                error.to_string(),
                "use a canonical kind:raw or kind:trs local transform",
            ));
            return;
        }
    };
    for root in roots {
        if let Err(error) = host.set_transform(*root, transform) {
            diagnostics.push(scene_host_error_diagnostic(
                import_path,
                "import_transform_failed",
                error,
            ));
        }
    }
}

pub(super) struct TransformResolutionInput<'a> {
    pub(super) node_keys: &'a BTreeMap<String, NodeKey>,
    pub(super) imports: &'a BTreeMap<String, u64>,
    pub(super) parent: Option<NodeKey>,
    pub(super) current_bounds: Option<Aabb>,
}

pub(super) fn transform_from_recipe(
    transform: Option<&SceneRecipeTransformV1>,
    context: TransformResolutionInput<'_>,
    host: &SceneHostCore<DefaultAssetFetcher>,
) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
    let Some(transform) = transform else {
        return Ok(Transform::IDENTITY);
    };
    match transform {
        local @ (SceneRecipeTransformV1::Raw { .. } | SceneRecipeTransformV1::Trs { .. }) => {
            Transform::try_from(local).map_err(|error| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_transform",
                    error.to_string(),
                    "use finite f32-range values and [0,0,0,1] for identity rotation",
                ))
            })
        }
        SceneRecipeTransformV1::LookAt { eye, target, up } => {
            let target = match target {
                SceneRecipeLookAtTargetV1::Position(position) => vec3(*position),
                SceneRecipeLookAtTargetV1::Node(id) => {
                    let node = context.node_keys.get(id).ok_or_else(|| {
                        Box::new(error_diagnostic(
                            "$",
                            "unknown_node_ref",
                            format!("look_at target references unknown node '{id}'"),
                            "target an authored node id or provide a [x,y,z] position",
                        ))
                    })?;
                    node_target_position(host, *node).ok_or_else(|| {
                        Box::new(error_diagnostic(
                            "$",
                            "node_bounds_missing",
                            format!("look_at target node '{id}' has no position or bounds"),
                            "target a renderable node or provide an explicit [x,y,z] position",
                        ))
                    })?
                }
            };
            let space = AuthoringSpace::of(host, context.parent)?;
            solved(space.frame.look_from(vec3(*eye), target, vec3(*up)))
        }
        SceneRecipeTransformV1::Center {} => {
            let bounds = require_bounds(context.current_bounds, "center")?;
            AuthoringSpace::of(host, context.parent)?.moved(
                |current| placement_center_transform(bounds, current, Vec3::ZERO),
                |world| Vec3::ZERO - transform_aabb(bounds, world).center(),
            )
        }
        SceneRecipeTransformV1::Ground { plane_y } => {
            let bounds = require_bounds(context.current_bounds, "ground")?;
            let plane_y = *plane_y as f32;
            AuthoringSpace::of(host, context.parent)?.moved(
                |current| placement_ground_transform(bounds, current, plane_y),
                |world| Vec3::new(0.0, plane_y - transform_aabb(bounds, world).min.y, 0.0),
            )
        }
        SceneRecipeTransformV1::FitToSize { size } => {
            let bounds = require_bounds(context.current_bounds, "fit_to_size")?;
            let space = AuthoringSpace::of(host, context.parent)?;
            let current_bounds = transform_aabb(bounds, space.world);
            let extent = current_bounds.max - current_bounds.min;
            let target = vec3(*size);
            let factors = [
                target.x / extent.x.max(f32::EPSILON),
                target.y / extent.y.max(f32::EPSILON),
                target.z / extent.z.max(f32::EPSILON),
            ];
            if factors
                .into_iter()
                .any(|factor| !factor.is_finite() || factor <= 0.0)
            {
                return Err(Box::new(error_diagnostic(
                    "$",
                    "invalid_bounds",
                    "fit_to_size requires finite non-zero source bounds",
                    "use renderable geometry with non-degenerate bounds",
                )));
            }
            let factor = factors.into_iter().min_by(f32::total_cmp).unwrap_or(1.0);
            match space.exact {
                Some(current) => space.local_for(Transform {
                    scale: current.scale * factor,
                    ..current
                }),
                // A uniform local scale scales the world bounds by the same
                // factor whatever the parent does to them.
                None => Ok(Transform::IDENTITY.with_uniform_scale(factor)),
            }
        }
        SceneRecipeTransformV1::PlaceOn { target, offset } => {
            let bounds = require_bounds(context.current_bounds, "place_on")?;
            let target = context.node_keys.get(target).copied().ok_or_else(|| {
                Box::new(error_diagnostic(
                    "$",
                    "unknown_node_ref",
                    "place_on target references an unknown node",
                    "declare target nodes before nodes that place_on them",
                ))
            })?;
            let target_bounds = host
                .scene
                .node_world_bounds(target, &host.assets)
                .map_err(|error| {
                    Box::new(error_diagnostic(
                        "$",
                        "node_bounds_failed",
                        format!("place_on target bounds could not be resolved: {error}"),
                        "target a renderable node with finite bounds",
                    ))
                })?
                .ok_or_else(|| {
                    Box::new(error_diagnostic(
                        "$",
                        "node_bounds_missing",
                        "place_on target has no renderable bounds",
                        "target a renderable node with finite bounds",
                    ))
                })?;
            let space = AuthoringSpace::of(host, context.parent)?;
            let source_bounds = transform_aabb(bounds, space.world);
            let source_anchor = Vec3::new(
                source_bounds.center().x,
                source_bounds.min.y,
                source_bounds.center().z,
            );
            let target_anchor = Vec3::new(
                target_bounds.center().x,
                target_bounds.max.y,
                target_bounds.center().z,
            ) + vec3(*offset);
            space.moved(
                |current| {
                    current.with_translation(current.translation + target_anchor - source_anchor)
                },
                |_| target_anchor - source_anchor,
            )
        }
        SceneRecipeTransformV1::AlignToAnchor { anchor } => {
            let anchor_world = anchor_world_placement(host, context.imports, anchor)?;
            let space = AuthoringSpace::of(host, context.parent)?;
            solved(space.frame.fitted_to(anchor_world))
        }
    }
}

pub(in crate::scene_host::recipe) fn local_transform_from_recipe(
    transform: Option<&SceneRecipeTransformV1>,
) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
    let Some(transform) = transform else {
        return Ok(Transform::IDENTITY);
    };
    match transform {
        local @ (SceneRecipeTransformV1::Raw { .. } | SceneRecipeTransformV1::Trs { .. }) => {
            Transform::try_from(local).map_err(|error| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_spatial_transform",
                    error.to_string(),
                    "use finite f32-range values and [0,0,0,1] for identity rotation",
                ))
            })
        }
        _ => Err(Box::new(error_diagnostic(
            "$",
            "invalid_spatial_transform",
            "spatial frames and named states accept only raw or trs local transforms",
            "use kind:raw or kind:trs",
        ))),
    }
}

fn node_target_position(host: &SceneHostCore<DefaultAssetFetcher>, node: NodeKey) -> Option<Vec3> {
    host.scene
        .node_world_bounds(node, &host.assets)
        .ok()
        .flatten()
        .map(|bounds| bounds.center())
        .or_else(|| {
            host.scene
                .world_transform(node)
                .map(|transform| transform.translation)
        })
}

fn require_bounds(bounds: Option<Aabb>, verb: &str) -> Result<Aabb, Box<SceneRecipeDiagnosticV1>> {
    bounds.ok_or_else(|| {
        Box::new(error_diagnostic(
            "$",
            "node_bounds_missing",
            format!("{verb} transform requires renderable geometry bounds"),
            "use the transform on a renderable node with geometry",
        ))
    })
}

/// The space a node is authored in: the exact world placement of its parent.
///
/// A node is authored at its parent's origin, unrotated and unscaled, and a
/// placement verb then gives it a local transform. Under a parent that is
/// exactly translation, rotation, and scale, the verbs keep the arithmetic
/// recipes have always used. A sheared parent has no such description, so its
/// verbs are solved against the matrix.
struct AuthoringSpace {
    frame: PlacementFrame,
    world: WorldTransform,
    /// The parent placement as one transform, when that is exact.
    exact: Option<Transform>,
    has_parent: bool,
}

impl AuthoringSpace {
    fn of(
        host: &SceneHostCore<DefaultAssetFetcher>,
        parent: Option<NodeKey>,
    ) -> Result<Self, Box<SceneRecipeDiagnosticV1>> {
        let Some(parent) = parent else {
            return Ok(Self {
                frame: PlacementFrame::WORLD,
                world: WorldTransform::IDENTITY,
                exact: Some(Transform::IDENTITY),
                has_parent: false,
            });
        };
        let world = host.scene.resolved_world(parent).ok_or_else(|| {
            Box::new(error_diagnostic(
                "$",
                "unknown_node_ref",
                "transform parent could not be resolved",
                "declare parent nodes before their children",
            ))
        })?;
        Ok(Self {
            frame: PlacementFrame::from_world(world),
            world,
            exact: world.exact_trs(),
            has_parent: true,
        })
    }

    /// The local transform that gives the node the world transform `world`
    /// under a parent that is exactly translation, rotation, and scale.
    fn local_for(&self, world: Transform) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
        if !self.has_parent {
            return Ok(world);
        }
        local_transform_from_parent(self.world.trs(), world).ok_or_else(non_invertible_parent)
    }

    /// Moves the node off its parent's origin. `world_transform` states the
    /// move as a new world transform, for a parent that has one; `world_delta`
    /// states it as an offset in the world, which a sheared parent carries
    /// back into its own space exactly.
    fn moved(
        &self,
        world_transform: impl FnOnce(Transform) -> Transform,
        world_delta: impl FnOnce(WorldTransform) -> Vec3,
    ) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
        match self.exact {
            Some(current) => self.local_for(world_transform(current)),
            None => solved(self.frame.moved_from_origin(world_delta(self.world))),
        }
    }
}

fn solved(
    placement: Result<Transform, Box<ScenePlacementDiagnosticV1>>,
) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
    placement.map_err(|_| non_invertible_parent())
}

fn non_invertible_parent() -> Box<SceneRecipeDiagnosticV1> {
    Box::new(error_diagnostic(
        "$",
        "non_invertible_parent_transform",
        "parent transform has a non-finite or zero scale",
        "use a parent transform with finite non-zero scale",
    ))
}

fn local_transform_from_parent(parent: Transform, world: Transform) -> Option<Transform> {
    if ![parent.scale.x, parent.scale.y, parent.scale.z]
        .into_iter()
        .all(|component| component.is_finite() && component.abs() > f32::EPSILON)
    {
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

/// The exact world placement of an imported anchor frame: its host node's
/// world matrix times the anchor's local transform.
fn anchor_world_placement(
    host: &SceneHostCore<DefaultAssetFetcher>,
    imports: &BTreeMap<String, u64>,
    anchor: &str,
) -> Result<WorldTransform, Box<SceneRecipeDiagnosticV1>> {
    let (import_id, anchor_name) = anchor.split_once('.').ok_or_else(|| {
        Box::new(error_diagnostic(
            "$",
            "invalid_anchor_ref",
            "anchor ref must be in the form <import_id>.<anchor_name>",
            "target an imported anchor such as machine.mount",
        ))
    })?;
    let import_handle = imports.get(import_id).copied().ok_or_else(|| {
        Box::new(error_diagnostic(
            "$",
            "unknown_import_ref",
            format!("anchor ref references unknown import '{import_id}'"),
            "target an import id declared in imports",
        ))
    })?;
    let import = host.resolve_import(import_handle).map_err(|error| {
        Box::new(error_diagnostic(
            "$",
            "import_resolve_failed",
            format!("import '{import_id}' could not be resolved: {error}"),
            "target a live imported asset",
        ))
    })?;
    anchor_from_import(import, anchor_name).and_then(|anchor| {
        let node_world = host.scene.resolved_world(anchor.node()).ok_or_else(|| {
            Box::new(error_diagnostic(
                "$",
                "anchor_node_missing",
                format!("anchor '{anchor_name}' host node could not be resolved"),
                "target an anchor attached to a live import node",
            ))
        })?;
        Ok(node_world.child(anchor.transform()))
    })
}

fn anchor_from_import<'a>(
    import: &'a SceneImport,
    anchor_name: &str,
) -> Result<&'a crate::scene::ImportAnchor, Box<SceneRecipeDiagnosticV1>> {
    import.anchor(anchor_name).map_err(|error| {
        Box::new(error_diagnostic(
            "$",
            "anchor_not_found",
            format!("anchor '{anchor_name}' could not be resolved: {error}"),
            "target an anchor declared by the imported asset",
        ))
    })
}
