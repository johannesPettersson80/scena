use crate::assets::Assets;
use crate::diagnostics::LookupError;
use crate::geometry::Aabb;

use super::transforms::{local_transform_from_world, validate_transform};
use super::view_math::{
    inverse_unit_quat, look_rotation, multiply_quat, normalize_or, positive_min, positive_or,
    subtract_vec3, union_aabb,
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
        let options = self.legacy_framing_options(camera)?;
        self.frame_import_with_options(camera, import, options)
            .map(|_| ())
    }

    /// Frames visible imported bounds with an explicit target viewport and view.
    pub fn frame_import_with_options(
        &mut self,
        camera: CameraKey,
        import: &SceneImport,
        options: super::FramingOptions,
    ) -> Result<super::FramingOutcome, LookupError> {
        import
            .bounds_world(self)
            .ok_or(LookupError::ImportHasNoBounds)?;
        let bounds = import
            .roots()
            .iter()
            .filter_map(|root| {
                self.visible_node_subtree_bounds_world(*root, options.includes_helpers())
            })
            .reduce(union_aabb)
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame_bounds(camera, bounds, options)
    }

    /// Frames all currently visible mesh bounds known to the scene.
    pub fn frame_all(&mut self, camera: CameraKey) -> Result<(), LookupError> {
        let options = self.legacy_framing_options(camera)?;
        self.frame_all_with_options(camera, options).map(|_| ())
    }

    /// Frames all visible bounds known directly to the scene with explicit options.
    pub fn frame_all_with_options(
        &mut self,
        camera: CameraKey,
        options: super::FramingOptions,
    ) -> Result<super::FramingOutcome, LookupError> {
        let bounds = self
            .scene_bounds_world(options.includes_helpers())
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame_bounds(camera, bounds, options)
    }

    /// Frames all visible mesh and instance bounds, resolving direct geometry handles through
    /// `Assets`.
    pub fn frame_all_with_assets<F>(
        &mut self,
        camera: CameraKey,
        assets: &Assets<F>,
    ) -> Result<(), LookupError> {
        let options = self.legacy_framing_options(camera)?;
        self.frame_all_with_assets_and_options(camera, assets, options)
            .map(|_| ())
    }

    /// Frames all visible scene content using the target viewport and view in `options`.
    pub fn frame_all_with_assets_and_options<F>(
        &mut self,
        camera: CameraKey,
        assets: &Assets<F>,
        options: super::FramingOptions,
    ) -> Result<super::FramingOutcome, LookupError> {
        let include_helpers = options.includes_helpers();
        let bounds = self
            .scene_bounds_world(include_helpers)
            .into_iter()
            .chain(self.asset_backed_scene_bounds_world(assets, include_helpers))
            .reduce(union_aabb)
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.frame_bounds(camera, bounds, options)
    }

    /// Frames visible scene content plus overlay anchors with enough viewport margin for
    /// screen-aligned label glyphs.
    ///
    /// This is intended for documentation, callout, and measurement captures where the
    /// generated leader lines are part of the scene bounds but the label glyphs are
    /// screen-space billboards. The helper keeps the camera solve geometric and only
    /// reserves pixel margin derived from visible label metrics.
    pub fn frame_all_with_overlays<F>(
        &mut self,
        camera: CameraKey,
        assets: &Assets<F>,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<super::FramingOutcome, LookupError> {
        let bounds = self
            .scene_bounds_world(true)
            .into_iter()
            .chain(self.asset_backed_scene_bounds_world(assets, true))
            .chain(self.visible_label_anchor_bounds_world())
            .reduce(union_aabb)
            .ok_or(LookupError::ImportHasNoBounds)?;
        let margin_px = self.visible_label_margin_px(viewport_width, viewport_height);
        self.frame_bounds(
            camera,
            bounds,
            super::FramingOptions::new()
                .viewport(viewport_width, viewport_height)
                .margin_px(margin_px)
                .tighten_depth_range(true),
        )
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

    /// Returns the world-space bounds for a node subtree, including geometry
    /// resolved through `Assets`.
    pub fn node_world_bounds<F>(
        &self,
        node: NodeKey,
        assets: &Assets<F>,
    ) -> Result<Option<Aabb>, LookupError> {
        if !self.nodes.contains_key(node) {
            return Err(LookupError::NodeNotFound(node));
        }
        Ok(self
            .node_subtree_bounds_world(node)
            .into_iter()
            .chain(self.asset_backed_node_subtree_bounds_world(node, assets))
            .reduce(union_aabb))
    }

    /// Returns the distance between two node origins in world space.
    pub fn world_distance(&self, a: NodeKey, b: NodeKey) -> Result<f32, LookupError> {
        let a = self
            .world_transform(a)
            .ok_or(LookupError::NodeNotFound(a))?
            .translation;
        let b = self
            .world_transform(b)
            .ok_or(LookupError::NodeNotFound(b))?
            .translation;
        Ok((a - b).length())
    }

    fn legacy_framing_options(
        &self,
        camera: CameraKey,
    ) -> Result<super::FramingOptions, LookupError> {
        let descriptor = self
            .camera(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let Camera::Perspective(perspective) = descriptor else {
            return Err(LookupError::UnsupportedCameraType {
                camera,
                operation: "legacy framing",
                supported: "perspective",
            });
        };
        let aspect = positive_or(perspective.aspect, 1.0);
        let height = 1_000_u32;
        let width = (aspect * height as f32).round().max(1.0) as u32;
        Ok(super::FramingOptions::new().viewport(width, height))
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

    /// Moves a node origin to an exact world-space point.
    pub fn move_origin_to(&mut self, node: NodeKey, center: Vec3) -> Result<(), LookupError> {
        let mut world_transform = self
            .world_transform(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        world_transform.translation = center;
        let transform = self.local_transform_for_world(node, world_transform)?;
        self.set_node_transform_and_mark_changed(node, transform)
    }

    /// Legacy origin-alignment name. Use `move_origin_to` for origins or
    /// `center_visible_bounds_on` for visible content.
    #[deprecated(note = "use move_origin_to or center_visible_bounds_on")]
    pub fn center_on(&mut self, node: NodeKey, center: Vec3) -> Result<(), LookupError> {
        self.move_origin_to(node, center)
    }

    /// Translates a subtree so its visible, non-helper bounds center reaches `center`.
    pub fn center_visible_bounds_on<F>(
        &mut self,
        node: NodeKey,
        assets: &Assets<F>,
        center: Vec3,
    ) -> Result<(), LookupError> {
        let bounds = self
            .visible_asset_backed_node_subtree_bounds_world(node, assets, false)
            .ok_or(LookupError::ImportHasNoBounds)?;
        let mut world = self
            .world_transform(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        world.translation += center - bounds.center();
        let local = self.local_transform_for_world(node, world)?;
        self.set_node_transform_and_mark_changed(node, local)
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
        let world_transform = validate_transform(world_transform)?;
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
        let transform = validate_transform(transform)?;
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        if node.transform != transform {
            node.transform = transform;
            self.transform_revision = self.transform_revision.saturating_add(1);
        }
        Ok(())
    }
}

const FRAME_PADDING: f32 = 1.15;
const MIN_FRAME_RADIUS: f32 = 0.05;
