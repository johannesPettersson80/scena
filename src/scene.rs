//! Scene graph, typed keys, transforms, bounds, anchors, clipping, and queries.

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

use slotmap::{SlotMap, new_key_type};

use crate::assets::{GeometryHandle, MaterialHandle, ModelHandle};
use crate::diagnostics::LookupError;
use crate::geometry::Primitive;

mod camera;
mod import;
mod lights;
pub use camera::{Camera, DepthRange, OrthographicCamera, PerspectiveCamera};
pub use import::SceneImport;
pub use lights::{DirectionalLight, Light, LightBuilder, PointLight, SpotLight};

new_key_type! {
    pub struct NodeKey;
    pub struct CameraKey;
    pub struct LightKey;
    pub struct ClippingPlaneKey;
}

#[derive(Debug)]
pub struct Scene {
    identity: Arc<()>,
    nodes: SlotMap<NodeKey, Node>,
    cameras: SlotMap<CameraKey, Camera>,
    lights: SlotMap<LightKey, Light>,
    clipping_planes: SlotMap<ClippingPlaneKey, ClippingPlane>,
    active_clipping_planes: ClippingPlaneSet,
    origin_shift: Vec3,
    root: NodeKey,
    active_camera: Option<CameraKey>,
    structure_revision: u64,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    parent: Option<NodeKey>,
    children: Vec<NodeKey>,
    transform: Transform,
    kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Empty,
    Renderable(RenderableNode),
    Mesh(MeshNode),
    Model(ModelNode),
    Camera(CameraKey),
    Light(LightKey),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderableNode {
    primitives: Vec<Primitive>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshNode {
    geometry: GeometryHandle,
    material: MaterialHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelNode {
    model: ModelHandle,
}

/// Builder returned by [`Scene::mesh`].
#[must_use = "mesh builders do nothing until add() is called"]
pub struct MeshBuilder<'scene> {
    scene: &'scene mut Scene,
    parent: NodeKey,
    transform: Transform,
    geometry: GeometryHandle,
    material: MaterialHandle,
}

/// Builder returned by [`Scene::model`].
#[must_use = "model builders do nothing until add() is called"]
pub struct ModelBuilder<'scene> {
    scene: &'scene mut Scene,
    parent: NodeKey,
    transform: Transform,
    model: ModelHandle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle {
    radians: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClippingPlane {
    normal: Vec3,
    distance: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClippingPlaneSet {
    planes: Vec<ClippingPlaneKey>,
}

impl Scene {
    pub fn new() -> Self {
        let mut nodes = SlotMap::with_key();
        let root = nodes.insert(Node::empty_root());
        Self {
            identity: Arc::new(()),
            nodes,
            cameras: SlotMap::with_key(),
            lights: SlotMap::with_key(),
            clipping_planes: SlotMap::with_key(),
            active_clipping_planes: ClippingPlaneSet::new(),
            origin_shift: Vec3::ZERO,
            root,
            active_camera: None,
            structure_revision: 0,
            not_sync: PhantomData,
        }
    }

    pub fn root(&self) -> NodeKey {
        self.root
    }

    pub fn active_camera(&self) -> Option<CameraKey> {
        self.active_camera
    }

    pub fn set_active_camera(&mut self, camera: CameraKey) -> Result<(), LookupError> {
        if self.cameras.contains_key(camera) {
            self.active_camera = Some(camera);
            Ok(())
        } else {
            Err(LookupError::CameraNotFound(camera))
        }
    }

    pub fn node(&self, node: NodeKey) -> Option<&Node> {
        self.nodes.get(node)
    }

    pub fn camera(&self, camera: CameraKey) -> Option<&Camera> {
        self.cameras.get(camera)
    }

    pub fn add_empty(
        &mut self,
        parent: NodeKey,
        transform: Transform,
    ) -> Result<NodeKey, LookupError> {
        self.insert_node(parent, NodeKind::Empty, transform)
    }

    pub fn add_renderable(
        &mut self,
        parent: NodeKey,
        primitives: Vec<Primitive>,
        transform: Transform,
    ) -> Result<NodeKey, LookupError> {
        self.insert_node(
            parent,
            NodeKind::Renderable(RenderableNode { primitives }),
            transform,
        )
    }

    /// Starts a mesh-node builder under the scene root.
    ///
    /// Use [`MeshBuilder::parent`] and [`MeshBuilder::transform`] to override the default
    /// root parent and identity transform, then call [`MeshBuilder::add`] to insert the node.
    pub fn mesh(&mut self, geometry: GeometryHandle, material: MaterialHandle) -> MeshBuilder<'_> {
        let parent = self.root;
        MeshBuilder {
            scene: self,
            parent,
            transform: Transform::default(),
            geometry,
            material,
        }
    }

    /// Starts a model-node builder under the scene root.
    ///
    /// Use [`ModelBuilder::parent`] and [`ModelBuilder::transform`] to override the default
    /// root parent and identity transform, then call [`ModelBuilder::add`] to insert the node.
    pub fn model(&mut self, model: ModelHandle) -> ModelBuilder<'_> {
        let parent = self.root;
        ModelBuilder {
            scene: self,
            parent,
            transform: Transform::default(),
            model,
        }
    }

    pub fn add_perspective_camera(
        &mut self,
        parent: NodeKey,
        camera: PerspectiveCamera,
        transform: Transform,
    ) -> Result<CameraKey, LookupError> {
        self.insert_camera(parent, Camera::Perspective(camera), transform)
    }

    pub fn add_orthographic_camera(
        &mut self,
        parent: NodeKey,
        camera: OrthographicCamera,
        transform: Transform,
    ) -> Result<CameraKey, LookupError> {
        self.insert_camera(parent, Camera::Orthographic(camera), transform)
    }

    pub fn set_transform(
        &mut self,
        node: NodeKey,
        transform: Transform,
    ) -> Result<(), LookupError> {
        let node = self
            .nodes
            .get_mut(node)
            .ok_or(LookupError::NodeNotFound(node))?;
        node.transform = transform;
        Ok(())
    }

    pub fn add_clipping_plane(&mut self, plane: ClippingPlane) -> ClippingPlaneKey {
        self.structure_revision = self.structure_revision.saturating_add(1);
        self.clipping_planes.insert(plane)
    }

    pub fn clipping_plane(&self, plane: ClippingPlaneKey) -> Option<ClippingPlane> {
        self.clipping_planes.get(plane).copied()
    }

    pub fn set_clipping_planes(&mut self, set: ClippingPlaneSet) -> Result<(), LookupError> {
        for plane in set.planes() {
            if !self.clipping_planes.contains_key(*plane) {
                return Err(LookupError::ClippingPlaneNotFound(*plane));
            }
        }
        self.active_clipping_planes = set;
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(())
    }

    pub fn clipping_planes(&self) -> &ClippingPlaneSet {
        &self.active_clipping_planes
    }

    pub fn set_origin_shift(&mut self, origin_shift: Vec3) {
        if self.origin_shift != origin_shift {
            self.origin_shift = origin_shift;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
    }

    pub fn origin_shift(&self) -> Vec3 {
        self.origin_shift
    }

    pub(crate) fn identity(&self) -> Weak<()> {
        Arc::downgrade(&self.identity)
    }

    pub(crate) fn structure_revision(&self) -> u64 {
        self.structure_revision
    }

    pub(crate) fn renderables(&self) -> impl Iterator<Item = (&RenderableNode, Transform)> {
        self.nodes.values().filter_map(|node| match &node.kind {
            NodeKind::Renderable(renderable) => Some((renderable, node.transform)),
            NodeKind::Empty
            | NodeKind::Mesh(_)
            | NodeKind::Model(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        })
    }

    pub(crate) fn mesh_nodes(&self) -> impl Iterator<Item = (NodeKey, MeshNode, Transform)> + '_ {
        self.nodes.iter().filter_map(|(key, node)| match node.kind {
            NodeKind::Mesh(mesh) => Some((key, mesh, node.transform)),
            NodeKind::Empty
            | NodeKind::Renderable(_)
            | NodeKind::Model(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        })
    }

    pub(crate) fn model_nodes(&self) -> impl Iterator<Item = NodeKey> + '_ {
        self.nodes.iter().filter_map(|(key, node)| match node.kind {
            NodeKind::Model(_) => Some(key),
            NodeKind::Empty
            | NodeKind::Renderable(_)
            | NodeKind::Mesh(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        })
    }

    pub(crate) fn light_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, LightKey, Light, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::Light(light_key) = node.kind else {
                return None;
            };
            self.lights
                .get(light_key)
                .copied()
                .map(|light| (node_key, light_key, light, node.transform))
        })
    }

    pub(crate) fn node_transforms(&self) -> impl Iterator<Item = (NodeKey, Transform)> + '_ {
        self.nodes.iter().map(|(key, node)| (key, node.transform))
    }

    pub(crate) fn camera_nodes(&self) -> impl Iterator<Item = (NodeKey, CameraKey, &Camera)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::Camera(camera_key) = node.kind else {
                return None;
            };
            self.cameras
                .get(camera_key)
                .map(|camera| (node_key, camera_key, camera))
        })
    }

    pub(crate) fn active_clipping_plane_values(&self) -> impl Iterator<Item = ClippingPlane> + '_ {
        self.active_clipping_planes
            .planes()
            .iter()
            .filter_map(|plane| self.clipping_plane(*plane))
    }

    fn insert_camera(
        &mut self,
        parent: NodeKey,
        camera: Camera,
        transform: Transform,
    ) -> Result<CameraKey, LookupError> {
        let camera = self.cameras.insert(camera);
        if let Err(error) = self.insert_node(parent, NodeKind::Camera(camera), transform) {
            self.cameras.remove(camera);
            return Err(error);
        }
        Ok(camera)
    }

    fn insert_node(
        &mut self,
        parent: NodeKey,
        kind: NodeKind,
        transform: Transform,
    ) -> Result<NodeKey, LookupError> {
        if !self.nodes.contains_key(parent) {
            return Err(LookupError::NodeNotFound(parent));
        }

        let node = self.nodes.insert(Node {
            parent: Some(parent),
            children: Vec::new(),
            transform,
            kind,
        });
        self.nodes[parent].children.push(node);
        self.structure_revision = self.structure_revision.saturating_add(1);
        Ok(node)
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Node {
    fn empty_root() -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            transform: Transform::IDENTITY,
            kind: NodeKind::Empty,
        }
    }

    pub fn parent(&self) -> Option<NodeKey> {
        self.parent
    }

    pub fn children(&self) -> &[NodeKey] {
        &self.children
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }
}

impl RenderableNode {
    pub fn primitives(&self) -> &[Primitive] {
        &self.primitives
    }
}

impl MeshNode {
    /// Returns the typed geometry handle referenced by this mesh node.
    pub const fn geometry(&self) -> GeometryHandle {
        self.geometry
    }

    /// Returns the typed material handle referenced by this mesh node.
    pub const fn material(&self) -> MaterialHandle {
        self.material
    }
}

impl ModelNode {
    /// Returns the typed model handle referenced by this model node.
    pub const fn model(&self) -> ModelHandle {
        self.model
    }
}

impl MeshBuilder<'_> {
    /// Overrides the parent node. The parent is validated when [`Self::add`] is called.
    pub fn parent(mut self, parent: NodeKey) -> Self {
        self.parent = parent;
        self
    }

    /// Overrides the local transform. The default is [`Transform::IDENTITY`].
    ///
    /// Mesh geometry is transformed during render preparation, including the active scene
    /// origin shift used for large-scene precision.
    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Inserts the mesh node and returns its typed node key.
    pub fn add(self) -> Result<NodeKey, LookupError> {
        self.scene.insert_node(
            self.parent,
            NodeKind::Mesh(MeshNode {
                geometry: self.geometry,
                material: self.material,
            }),
            self.transform,
        )
    }
}

impl ModelBuilder<'_> {
    /// Overrides the parent node. The parent is validated when [`Self::add`] is called.
    pub fn parent(mut self, parent: NodeKey) -> Self {
        self.parent = parent;
        self
    }

    /// Overrides the local transform. The default is [`Transform::IDENTITY`].
    pub fn transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    /// Inserts the model node and returns its typed node key.
    pub fn add(self) -> Result<NodeKey, LookupError> {
        self.scene.insert_node(
            self.parent,
            NodeKind::Model(ModelNode { model: self.model }),
            self.transform,
        )
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

impl ClippingPlane {
    pub const fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    pub const fn normal(self) -> Vec3 {
        self.normal
    }

    pub const fn distance(self) -> f32 {
        self.distance
    }

    pub fn contains(self, point: Vec3) -> bool {
        self.normal.x * point.x + self.normal.y * point.y + self.normal.z * point.z
            >= -self.distance
    }
}

impl ClippingPlaneSet {
    pub fn new() -> Self {
        Self { planes: Vec::new() }
    }

    pub fn with_plane(mut self, plane: ClippingPlaneKey) -> Self {
        self.planes.push(plane);
        self
    }

    pub fn planes(&self) -> &[ClippingPlaneKey] {
        &self.planes
    }
}

impl Quat {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };
}

impl Angle {
    pub fn from_degrees(degrees: f32) -> Self {
        Self::from_radians(degrees.to_radians())
    }

    pub const fn from_radians(radians: f32) -> Self {
        Self { radians }
    }

    pub const fn radians(self) -> f32 {
        self.radians
    }
}
