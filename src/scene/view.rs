use crate::assets::Assets;
use crate::diagnostics::LookupError;
use crate::geometry::Aabb;

use super::transforms::{compose_transform, local_transform_from_world};
use super::view_math::{
    inverse_unit_quat, look_rotation, merge_optional_bounds, multiply_quat, normalize_or,
    positive_min, positive_or, subtract_vec3, transform_aabb, union_aabb,
};
use super::{
    Camera, CameraKey, ImportAnchor, NodeKey, NodeKind, PerspectiveCamera, Quat, Scene,
    SceneImport, Transform, Vec3,
};

impl Scene {
    /// Returns the scene node that owns a camera descriptor.
    pub fn camera_node(&self, camera: CameraKey) -> Option<NodeKey> {
        self.nodes.iter().find_map(|(node_key, node)| {
            if node.kind == NodeKind::Camera(camera) {
                Some(node_key)
            } else {
                None
            }
        })
    }

    /// Frames bounds with the selected camera and tightens the camera depth range.
    pub fn frame(&mut self, camera: CameraKey, bounds: Aabb) -> Result<(), LookupError> {
        let camera_node = self
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let center = bounds.center();
        let radius = bounds.bounding_sphere_radius().max(MIN_FRAME_RADIUS);
        let camera_descriptor = self
            .cameras
            .get_mut(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;

        let transform = match camera_descriptor {
            Camera::Perspective(camera) => {
                let half_vertical_fov = camera.vertical_fov.radians() * 0.5;
                let half_horizontal_fov =
                    (half_vertical_fov.tan() * positive_or(camera.aspect, 1.0)).atan();
                let limiting_half_fov = half_vertical_fov.min(half_horizontal_fov).max(0.001);
                let distance = radius / limiting_half_fov.tan() * FRAME_PADDING;
                let depth_radius = radius * FRAME_PADDING;
                let depth = super::DepthRange::fit_sphere(distance, depth_radius);
                camera.near = depth.near();
                camera.far = depth.far();
                Transform {
                    translation: Vec3::new(center.x, center.y, center.z + distance),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                }
            }
            Camera::Orthographic(camera) => {
                let half = bounds.half_extent();
                let half_width = half.x.max(radius) * FRAME_PADDING;
                let half_height = half.y.max(radius) * FRAME_PADDING;
                let distance = (radius * FRAME_PADDING).max(1.0);
                let depth = super::DepthRange::fit_sphere(distance, radius * FRAME_PADDING);
                camera.left = -half_width;
                camera.right = half_width;
                camera.bottom = -half_height;
                camera.top = half_height;
                camera.near = depth.near();
                camera.far = depth.far();
                Transform {
                    translation: Vec3::new(center.x, center.y, center.z + distance),
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                }
            }
        };

        let transform = self.local_transform_for_world(camera_node, transform)?;
        self.set_node_transform_and_mark_changed(camera_node, transform)
    }

    /// Adds a perspective camera under the root and makes it active.
    pub fn add_default_camera(&mut self) -> Result<CameraKey, LookupError> {
        let camera = self.add_perspective_camera(
            self.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )?;
        self.set_active_camera(camera)?;
        Ok(camera)
    }

    /// Convenience constructor returning a fresh `Scene` plus a default
    /// active camera in one call. The renderer-as-library analog of
    /// Three.js's `new THREE.Scene()` + camera one-liner: callers who
    /// only need a default perspective camera framed at z=2 can drop the
    /// two-step `Scene::new()` + `add_default_camera()` boilerplate.
    /// Closes scena-api-ergonomics-reviewer Phase 6 finding F1.
    pub fn with_default_camera() -> Result<(Self, CameraKey), LookupError> {
        let mut scene = Self::new();
        let camera = scene.add_default_camera()?;
        Ok((scene, camera))
    }

    /// Frames the world-space bounds of an imported scene.
    pub fn frame_import(
        &mut self,
        camera: CameraKey,
        import: &SceneImport,
    ) -> Result<(), LookupError> {
        let bounds = import
            .bounds_world(self)
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame(camera, bounds)
    }

    /// Frames all currently visible mesh bounds known to the scene.
    pub fn frame_all(&mut self, camera: CameraKey) -> Result<(), LookupError> {
        let bounds = self
            .scene_bounds_world()
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame(camera, bounds)
    }

    /// Frames all visible mesh and instance bounds, resolving direct geometry handles through
    /// `Assets`.
    pub fn frame_all_with_assets<F>(
        &mut self,
        camera: CameraKey,
        assets: &Assets<F>,
    ) -> Result<(), LookupError> {
        let bounds = self
            .scene_bounds_world()
            .into_iter()
            .chain(self.asset_backed_scene_bounds_world(assets))
            .reduce(union_aabb)
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame(camera, bounds)
    }

    /// Frames the world-space bounds of a node and any bounded descendants.
    pub fn frame_node(&mut self, camera: CameraKey, node: NodeKey) -> Result<(), LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        let bounds = self
            .node_subtree_bounds_world(node)
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame(camera, bounds)
    }

    /// Frames a node or bounded descendants, resolving direct geometry handles through
    /// `Assets`.
    pub fn frame_node_with_assets<F>(
        &mut self,
        camera: CameraKey,
        node: NodeKey,
        assets: &Assets<F>,
    ) -> Result<(), LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        let bounds = self
            .node_subtree_bounds_world(node)
            .into_iter()
            .chain(self.asset_backed_node_subtree_bounds_world(node, assets))
            .reduce(union_aabb)
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame(camera, bounds)
    }

    fn scene_bounds_world(&self) -> Option<Aabb> {
        self.mesh_bounds_nodes()
            .filter_map(|(node, bounds)| {
                let transform = self.world_transform(node)?;
                Some(transform_aabb(bounds, transform))
            })
            .reduce(union_aabb)
    }

    fn node_subtree_bounds_world(&self, node: NodeKey) -> Option<Aabb> {
        let node_ref = self.nodes.get(node)?;
        let local_bounds = self.node_bounds.get(&node).and_then(|bounds| {
            let transform = self.world_transform(node)?;
            Some(transform_aabb(*bounds, transform))
        });
        node_ref
            .children
            .iter()
            .filter_map(|child| self.node_subtree_bounds_world(*child))
            .fold(local_bounds, |bounds, child_bounds| {
                Some(match bounds {
                    Some(bounds) => union_aabb(bounds, child_bounds),
                    None => child_bounds,
                })
            })
    }

    fn asset_backed_scene_bounds_world<F>(&self, assets: &Assets<F>) -> Option<Aabb> {
        let mut bounds = None;
        for (node, node_ref) in self.nodes.iter() {
            if !self.visible_for_active_camera(node) {
                continue;
            }
            if let Some(node_bounds) = self.asset_backed_node_bounds_world(node, node_ref, assets) {
                bounds = Some(merge_optional_bounds(bounds, node_bounds));
            }
        }
        bounds
    }

    fn asset_backed_node_subtree_bounds_world<F>(
        &self,
        node: NodeKey,
        assets: &Assets<F>,
    ) -> Option<Aabb> {
        let node_ref = self.nodes.get(node)?;
        node_ref
            .children
            .iter()
            .filter_map(|child| self.asset_backed_node_subtree_bounds_world(*child, assets))
            .fold(
                self.asset_backed_node_bounds_world(node, node_ref, assets),
                |bounds, child_bounds| Some(merge_optional_bounds(bounds, child_bounds)),
            )
    }

    fn asset_backed_node_bounds_world<F>(
        &self,
        node: NodeKey,
        node_ref: &super::Node,
        assets: &Assets<F>,
    ) -> Option<Aabb> {
        match &node_ref.kind {
            NodeKind::Mesh(mesh) => {
                let geometry = assets.geometry(mesh.geometry())?;
                let transform = self.world_transform(node)?;
                Some(transform_aabb(geometry.bounds(), transform))
            }
            NodeKind::InstanceSet(instance_set) => {
                let instance_set = self.instance_sets.get(*instance_set)?;
                let geometry = assets.geometry(instance_set.geometry())?;
                let node_transform = self.world_transform(node)?;
                instance_set
                    .instances()
                    .map(|instance| {
                        transform_aabb(
                            geometry.bounds(),
                            compose_transform(node_transform, instance.transform()),
                        )
                    })
                    .reduce(union_aabb)
            }
            NodeKind::Empty
            | NodeKind::Renderable(_)
            | NodeKind::Model(_)
            | NodeKind::Label(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        }
    }

    /// Rotates the selected camera node so its local -Z axis points at `target`.
    pub fn look_at(&mut self, camera: CameraKey, target: NodeKey) -> Result<(), LookupError> {
        if !self.cameras.contains_key(camera) {
            return Err(LookupError::CameraNotFound(camera));
        }
        let target_position = self
            .world_transform(target)
            .ok_or(LookupError::NodeNotFound(target))?
            .translation;
        self.look_at_point(camera, target_position)
    }

    /// Rotates the selected camera node so its local -Z axis points at a world-space point.
    pub fn look_at_point(
        &mut self,
        camera: CameraKey,
        target_position: Vec3,
    ) -> Result<(), LookupError> {
        let camera_node = self
            .camera_node(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        if !self.cameras.contains_key(camera) {
            return Err(LookupError::CameraNotFound(camera));
        }
        let camera_node_desc = self
            .nodes
            .get(camera_node)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let mut camera_transform = camera_node_desc.transform;
        let camera_parent = camera_node_desc.parent;
        let camera_world = self
            .world_transform(camera_node)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let forward = normalize_or(
            subtract_vec3(target_position, camera_world.translation),
            Vec3::new(0.0, 0.0, -1.0),
        );
        let desired_world_rotation = look_rotation(forward, Vec3::new(0.0, 1.0, 0.0));

        camera_transform.rotation = if let Some(parent) = camera_parent {
            let parent_world = self
                .world_transform(parent)
                .ok_or(LookupError::NodeNotFound(parent))?;
            multiply_quat(
                inverse_unit_quat(parent_world.rotation),
                desired_world_rotation,
            )
        } else {
            desired_world_rotation
        };
        self.set_node_transform_and_mark_changed(camera_node, camera_transform)
    }

    pub fn center_on(&mut self, node: NodeKey, center: Vec3) -> Result<(), LookupError> {
        let mut world_transform = self
            .world_transform(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        world_transform.translation = center;
        let transform = self.local_transform_for_world(node, world_transform)?;
        self.set_node_transform_and_mark_changed(node, transform)
    }

    pub fn align_to(&mut self, node: NodeKey, transform: Transform) -> Result<(), LookupError> {
        let transform = self.local_transform_for_world(node, transform)?;
        self.set_node_transform_and_mark_changed(node, transform)
    }

    pub fn snap_anchor(&mut self, node: NodeKey, anchor: &ImportAnchor) -> Result<(), LookupError> {
        self.align_to(node, anchor.transform())
    }

    pub fn fit_inside(
        &mut self,
        node: NodeKey,
        source: Aabb,
        target: Aabb,
    ) -> Result<(), LookupError> {
        let source_half = source.half_extent();
        let target_half = target.half_extent();
        let scale = positive_min([
            target_half.x / source_half.x.max(f32::EPSILON),
            target_half.y / source_half.y.max(f32::EPSILON),
            target_half.z / source_half.z.max(f32::EPSILON),
        ]);
        let mut world_transform = self
            .world_transform(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        world_transform.translation = target.center();
        world_transform.scale = Vec3::new(scale, scale, scale);
        let transform = self.local_transform_for_world(node, world_transform)?;
        self.set_node_transform_and_mark_changed(node, transform)
    }

    fn local_transform_for_world(
        &self,
        node: NodeKey,
        world_transform: Transform,
    ) -> Result<Transform, LookupError> {
        let parent = self
            .nodes
            .get(node)
            .ok_or(LookupError::NodeNotFound(node))?
            .parent;
        let Some(parent) = parent else {
            return Ok(world_transform);
        };
        let parent_world = self
            .world_transform(parent)
            .ok_or(LookupError::NodeNotFound(parent))?;
        local_transform_from_world(parent_world, world_transform)
            .ok_or(LookupError::NonInvertibleParentTransform { node, parent })
    }

    fn set_node_transform_and_mark_changed(
        &mut self,
        node: NodeKey,
        transform: Transform,
    ) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.transform != transform {
            node.transform = transform;
            self.structure_revision = self.structure_revision.saturating_add(1);
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }
}

const FRAME_PADDING: f32 = 1.15;
const MIN_FRAME_RADIUS: f32 = 0.05;
