use std::collections::BTreeMap;

use super::common::vec3;
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeDiagnosticV1, SceneRecipeLookAtTargetV1, SceneRecipeTransformV1,
};
use crate::scene_host::SceneHostCore;
use crate::{NodeKey, Quat, Transform, Vec3};

use super::super::error_diagnostic;

pub(super) fn transform_from_recipe(
    transform: Option<&SceneRecipeTransformV1>,
    node_keys: &BTreeMap<String, NodeKey>,
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
                    let node = node_keys.get(id).ok_or_else(|| {
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
            Ok(Transform::at(vec3(*eye)).looking_at(target, vec3(*up)))
        }
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
