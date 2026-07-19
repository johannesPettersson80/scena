use std::collections::BTreeMap;

use super::common::vec3;
use crate::assets::DefaultAssetFetcher;
use crate::geometry::Aabb;
use crate::scene::recipe::{
    SceneRecipeDiagnosticV1, SceneRecipeLookAtTargetV1, SceneRecipeTransformV1,
};
use crate::scene::view_math::transform_aabb;
use crate::scene_host::SceneHostCore;
use crate::{
    NodeKey, Quat, SceneImport, Transform, Vec3, placement_center_transform,
    placement_ground_transform,
};

use super::super::error_diagnostic;

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
        SceneRecipeTransformV1::Raw {
            translation,
            rotation,
            scale,
        } => {
            let rotation = Quat::from_xyzw(
                rotation[0] as f32,
                rotation[1] as f32,
                rotation[2] as f32,
                rotation[3] as f32,
            );
            let length_sq = rotation.length_squared();
            if !length_sq.is_finite() || length_sq <= f32::EPSILON {
                return Err(Box::new(error_diagnostic(
                    "$",
                    "invalid_rotation",
                    "raw transform rotation must be a finite non-zero quaternion",
                    "use [0,0,0,1] for identity",
                )));
            }
            Ok(Transform {
                translation: vec3(*translation),
                rotation: rotation.normalize(),
                scale: vec3(*scale),
            })
        }
        SceneRecipeTransformV1::Trs {
            translation,
            rotation_degrees,
            scale,
        } => Ok(Transform::IDENTITY
            .with_translation(vec3(*translation))
            .rotate_x_deg(rotation_degrees[0] as f32)
            .rotate_y_deg(rotation_degrees[1] as f32)
            .rotate_z_deg(rotation_degrees[2] as f32)
            .with_scale(vec3(*scale))),
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
            let desired_world = Transform::at(vec3(*eye)).looking_at(target, vec3(*up));
            local_from_world(host, context.parent, desired_world)
        }
        SceneRecipeTransformV1::Center {} => {
            let bounds = require_bounds(context.current_bounds, "center")?;
            let current_world = current_world_transform(host, context.parent)?;
            let desired_world = placement_center_transform(bounds, current_world, Vec3::ZERO);
            local_from_world(host, context.parent, desired_world)
        }
        SceneRecipeTransformV1::Ground { plane_y } => {
            let bounds = require_bounds(context.current_bounds, "ground")?;
            let current_world = current_world_transform(host, context.parent)?;
            let desired_world = placement_ground_transform(bounds, current_world, *plane_y as f32);
            local_from_world(host, context.parent, desired_world)
        }
        SceneRecipeTransformV1::FitToSize { size } => {
            let bounds = require_bounds(context.current_bounds, "fit_to_size")?;
            let current_world = current_world_transform(host, context.parent)?;
            let current_bounds = transform_aabb(bounds, current_world);
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
            let desired_world = Transform {
                scale: current_world.scale * factor,
                ..current_world
            };
            local_from_world(host, context.parent, desired_world)
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
            let current_world = current_world_transform(host, context.parent)?;
            let source_bounds = transform_aabb(bounds, current_world);
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
            let desired_world = current_world
                .with_translation(current_world.translation + target_anchor - source_anchor);
            local_from_world(host, context.parent, desired_world)
        }
        SceneRecipeTransformV1::AlignToAnchor { anchor } => {
            let target_world = anchor_world_transform(host, context.imports, anchor)?;
            local_from_world(host, context.parent, target_world)
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
        SceneRecipeTransformV1::Raw {
            translation,
            rotation,
            scale,
        } => {
            let rotation = Quat::from_xyzw(
                rotation[0] as f32,
                rotation[1] as f32,
                rotation[2] as f32,
                rotation[3] as f32,
            );
            let length_sq = rotation.length_squared();
            if !length_sq.is_finite() || length_sq <= f32::EPSILON {
                return Err(Box::new(error_diagnostic(
                    "$",
                    "invalid_spatial_transform",
                    "raw local transform rotation must be a finite non-zero quaternion",
                    "use [0,0,0,1] for identity",
                )));
            }
            Ok(Transform {
                translation: vec3(*translation),
                rotation: rotation.normalize(),
                scale: vec3(*scale),
            })
        }
        SceneRecipeTransformV1::Trs {
            translation,
            rotation_degrees,
            scale,
        } => Ok(Transform::IDENTITY
            .with_translation(vec3(*translation))
            .rotate_x_deg(rotation_degrees[0] as f32)
            .rotate_y_deg(rotation_degrees[1] as f32)
            .rotate_z_deg(rotation_degrees[2] as f32)
            .with_scale(vec3(*scale))),
        _ => Err(Box::new(error_diagnostic(
            "$",
            "invalid_spatial_transform",
            "spatial frames and named states accept only raw or trs local transforms",
            "use kind:raw or kind:trs",
        ))),
    }
}

trait TransformScaleExt {
    fn with_scale(self, scale: Vec3) -> Self;
}

impl TransformScaleExt for Transform {
    fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
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

fn current_world_transform(
    host: &SceneHostCore<DefaultAssetFetcher>,
    parent: Option<NodeKey>,
) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
    match parent {
        Some(parent) => host.scene.world_transform(parent).ok_or_else(|| {
            Box::new(error_diagnostic(
                "$",
                "unknown_node_ref",
                "transform parent could not be resolved",
                "declare parent nodes before their children",
            ))
        }),
        None => Ok(Transform::IDENTITY),
    }
}

fn local_from_world(
    host: &SceneHostCore<DefaultAssetFetcher>,
    parent: Option<NodeKey>,
    world: Transform,
) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
    let Some(parent) = parent else {
        return Ok(world);
    };
    let parent_world = host.scene.world_transform(parent).ok_or_else(|| {
        Box::new(error_diagnostic(
            "$",
            "unknown_node_ref",
            "transform parent could not be resolved",
            "declare parent nodes before their children",
        ))
    })?;
    local_transform_from_parent(parent_world, world).ok_or_else(|| {
        Box::new(error_diagnostic(
            "$",
            "non_invertible_parent_transform",
            "parent transform has a non-finite or zero scale",
            "use a parent transform with finite non-zero scale",
        ))
    })
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

fn anchor_world_transform(
    host: &SceneHostCore<DefaultAssetFetcher>,
    imports: &BTreeMap<String, u64>,
    anchor: &str,
) -> Result<Transform, Box<SceneRecipeDiagnosticV1>> {
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
        let node_world = host.scene.world_transform(anchor.node()).ok_or_else(|| {
            Box::new(error_diagnostic(
                "$",
                "anchor_node_missing",
                format!("anchor '{anchor_name}' host node could not be resolved"),
                "target an anchor attached to a live import node",
            ))
        })?;
        Ok(Transform::compose(node_world, anchor.transform()))
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
