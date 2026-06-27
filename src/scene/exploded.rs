use crate::assets::{AssetFetcher, Assets};
use crate::diagnostics::LookupError;
use crate::geometry::Aabb;

use super::transforms::local_transform_from_world;
use super::{NodeKey, Scene, SceneImport, Transform, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExplodedViewMode {
    DirectChildren,
    HierarchyDepth,
}

/// Presentation-only exploded-view transform planner.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplodedView {
    roots: Vec<NodeKey>,
    mode: ExplodedViewMode,
    axis: Option<Vec3>,
    factor: f32,
    distance: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExplodedViewPlan {
    updates: Vec<ExplodedTransformUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplodedTransformUpdate {
    pub node: NodeKey,
    pub original: Transform,
    pub transform: Transform,
}

#[derive(Debug, Clone, Copy)]
struct ExplodedTarget {
    node: NodeKey,
    depth: usize,
}

impl ExplodedView {
    pub fn from_node(root: NodeKey) -> Self {
        Self::from_roots([root])
    }

    pub fn from_import(import: &SceneImport) -> Self {
        Self::from_roots(import.roots().iter().copied())
    }

    pub fn from_roots(roots: impl IntoIterator<Item = NodeKey>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
            mode: ExplodedViewMode::DirectChildren,
            axis: None,
            factor: 1.0,
            distance: 1.0,
        }
    }

    pub const fn factor(mut self, factor: f32) -> Self {
        self.factor = factor;
        self
    }

    pub const fn distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    pub const fn by_hierarchy_depth(mut self) -> Self {
        self.mode = ExplodedViewMode::HierarchyDepth;
        self
    }

    pub fn along_axis(mut self, axis: Vec3) -> Self {
        self.axis = Some(axis);
        self
    }

    pub fn transforms<F>(
        &self,
        scene: &Scene,
        assets: &Assets<F>,
    ) -> Result<ExplodedViewPlan, LookupError>
    where
        F: AssetFetcher,
    {
        validate_factor(self.factor)?;
        validate_distance(self.distance)?;
        let axis = self.axis.map(validate_axis).transpose()?;
        let assembly_bounds = self.assembly_bounds(scene, assets)?;
        let assembly_center = assembly_bounds.center();
        let targets = self.targets(scene)?;
        let mut updates = Vec::new();

        for (index, target) in targets.into_iter().enumerate() {
            let Some(bounds) = scene.node_world_bounds(target.node, assets)? else {
                continue;
            };
            let original = scene
                .node(target.node)
                .ok_or(LookupError::NodeNotFound(target.node))?
                .transform();
            let world = scene
                .world_transform(target.node)
                .ok_or(LookupError::NodeNotFound(target.node))?;
            let direction = match axis {
                Some(axis) => axis_direction(axis, bounds.center(), assembly_center, index),
                None => radial_direction(bounds.center(), assembly_center, index),
            };
            let depth_multiplier = match self.mode {
                ExplodedViewMode::DirectChildren => 1.0,
                ExplodedViewMode::HierarchyDepth => target.depth.max(1) as f32,
            };
            let offset = direction * self.distance * self.factor * depth_multiplier;
            let target_world = world.with_translation(world.translation + offset);
            let transform = local_transform_for_node(scene, target.node, target_world)?;
            updates.push(ExplodedTransformUpdate {
                node: target.node,
                original,
                transform,
            });
        }

        if updates.is_empty() {
            return Err(LookupError::ImportHasNoBounds);
        }
        Ok(ExplodedViewPlan { updates })
    }

    fn assembly_bounds<F>(&self, scene: &Scene, assets: &Assets<F>) -> Result<Aabb, LookupError>
    where
        F: AssetFetcher,
    {
        let mut bounds: Option<Aabb> = None;
        for root in &self.roots {
            let Some(root_bounds) = scene.node_world_bounds(*root, assets)? else {
                continue;
            };
            bounds = Some(match bounds {
                Some(bounds) => bounds.union(root_bounds),
                None => root_bounds,
            });
        }
        bounds.ok_or(LookupError::ImportHasNoBounds)
    }

    fn targets(&self, scene: &Scene) -> Result<Vec<ExplodedTarget>, LookupError> {
        let mut targets = Vec::new();
        for root in &self.roots {
            let root_node = scene.node(*root).ok_or(LookupError::NodeNotFound(*root))?;
            match self.mode {
                ExplodedViewMode::DirectChildren => {
                    if root_node.children().is_empty() {
                        targets.push(ExplodedTarget {
                            node: *root,
                            depth: 1,
                        });
                    } else {
                        targets.extend(root_node.children().iter().map(|node| ExplodedTarget {
                            node: *node,
                            depth: 1,
                        }));
                    }
                }
                ExplodedViewMode::HierarchyDepth => {
                    if root_node.children().is_empty() {
                        targets.push(ExplodedTarget {
                            node: *root,
                            depth: 1,
                        });
                    } else {
                        for child in root_node.children() {
                            collect_descendants(scene, *child, 1, &mut targets)?;
                        }
                    }
                }
            }
        }
        Ok(targets)
    }
}

impl ExplodedViewPlan {
    pub fn len(&self) -> usize {
        self.updates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }

    pub fn updates(&self) -> &[ExplodedTransformUpdate] {
        &self.updates
    }

    pub fn as_transform_updates(&self) -> Vec<(NodeKey, Transform)> {
        self.updates
            .iter()
            .map(|update| (update.node, update.transform))
            .collect()
    }
}

fn collect_descendants(
    scene: &Scene,
    node: NodeKey,
    depth: usize,
    targets: &mut Vec<ExplodedTarget>,
) -> Result<(), LookupError> {
    let node_ref = scene.node(node).ok_or(LookupError::NodeNotFound(node))?;
    targets.push(ExplodedTarget { node, depth });
    for child in node_ref.children() {
        collect_descendants(scene, *child, depth + 1, targets)?;
    }
    Ok(())
}

fn local_transform_for_node(
    scene: &Scene,
    node: NodeKey,
    world: Transform,
) -> Result<Transform, LookupError> {
    let parent = scene
        .node(node)
        .ok_or(LookupError::NodeNotFound(node))?
        .parent();
    let Some(parent) = parent else {
        return Ok(world);
    };
    let parent_world = scene
        .world_transform(parent)
        .ok_or(LookupError::NodeNotFound(parent))?;
    local_transform_from_world(parent_world, world)
        .ok_or(LookupError::NonInvertibleParentTransform { node, parent })
}

fn validate_factor(factor: f32) -> Result<(), LookupError> {
    if factor.is_finite() && (0.0..=1.0).contains(&factor) {
        Ok(())
    } else {
        Err(LookupError::InvalidFramingOption {
            field: "factor",
            reason: "exploded view factor must be finite and between 0 and 1",
        })
    }
}

fn validate_distance(distance: f32) -> Result<(), LookupError> {
    if distance.is_finite() && distance >= 0.0 {
        Ok(())
    } else {
        Err(LookupError::InvalidFramingOption {
            field: "distance",
            reason: "exploded view distance must be finite and non-negative",
        })
    }
}

fn validate_axis(axis: Vec3) -> Result<Vec3, LookupError> {
    if axis.is_finite() && axis.length_squared() > f32::EPSILON {
        Ok(axis.normalize())
    } else {
        Err(LookupError::InvalidFramingOption {
            field: "axis",
            reason: "exploded view axis must be finite and non-zero",
        })
    }
}

fn axis_direction(axis: Vec3, center: Vec3, assembly_center: Vec3, index: usize) -> Vec3 {
    let projected = (center - assembly_center).dot(axis);
    if projected.abs() > f32::EPSILON {
        axis * projected.signum()
    } else {
        axis * fallback_sign(index)
    }
}

fn radial_direction(center: Vec3, assembly_center: Vec3, index: usize) -> Vec3 {
    let direction = center - assembly_center;
    if direction.is_finite() && direction.length_squared() > f32::EPSILON {
        direction.normalize()
    } else {
        fallback_direction(index)
    }
}

fn fallback_direction(index: usize) -> Vec3 {
    match index % 6 {
        0 => Vec3::X,
        1 => -Vec3::X,
        2 => Vec3::Y,
        3 => -Vec3::Y,
        4 => Vec3::Z,
        _ => -Vec3::Z,
    }
}

fn fallback_sign(index: usize) -> f32 {
    if index.is_multiple_of(2) { 1.0 } else { -1.0 }
}
