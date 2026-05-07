//! Scene graph, typed keys, transforms, bounds, anchors, clipping, and queries.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

use slotmap::{SlotMap, new_key_type};

use crate::animation::{AnimationMixer, AnimationMixerKey};
use crate::assets::{GeometryHandle, MaterialHandle, ModelHandle};
use crate::diagnostics::LookupError;
use crate::geometry::Primitive;
use crate::picking::InteractionContext;

mod builders;
mod camera;
mod dirty;
mod import;
#[cfg(feature = "inspection")]
mod inspection;
mod instances;
mod labels;
mod lights;
mod materials;
mod math;
mod mixers;
mod morphs;
mod origin;
mod picking;
mod render_nodes;
mod skinning;
mod transforms;
mod view;
mod visibility;
pub use camera::{Camera, DepthRange, OrthographicCamera, PerspectiveCamera};
pub use dirty::SceneDirtyState;
pub use import::{
    ImportAnchor, ImportAnchorDebugMetadata, ImportClip, ImportOptions, ImportPivot, SceneImport,
    SourceCoordinateSystem, SourceUnits,
};
#[cfg(feature = "inspection")]
pub use inspection::{SceneInspectionReport, SceneNodeInspection};
pub use instances::{Instance, InstanceCullingPolicy, InstanceId, InstanceSet};
pub use labels::{LabelBillboard, LabelDesc, LabelRasterization};
pub use lights::{DirectionalLight, Light, LightBuilder, PointLight, SpotLight};
pub use math::{Angle, Quat, Transform, Vec3};
pub use skinning::SceneSkinBinding;

new_key_type! {
    pub struct NodeKey;
    pub struct CameraKey;
    pub struct LightKey;
    pub struct ClippingPlaneKey;
    pub struct InstanceSetKey;
    pub struct LabelKey;
}

#[derive(Debug)]
pub struct Scene {
    identity: Arc<()>,
    nodes: SlotMap<NodeKey, Node>,
    cameras: SlotMap<CameraKey, Camera>,
    lights: SlotMap<LightKey, Light>,
    instance_sets: SlotMap<InstanceSetKey, InstanceSet>,
    animation_mixers: SlotMap<AnimationMixerKey, AnimationMixer>,
    labels: SlotMap<LabelKey, LabelDesc>,
    morph_weights: BTreeMap<NodeKey, Vec<f32>>,
    skin_bindings: BTreeMap<NodeKey, SceneSkinBinding>,
    clipping_planes: SlotMap<ClippingPlaneKey, ClippingPlane>,
    active_clipping_planes: ClippingPlaneSet,
    origin_shift: Vec3,
    root: NodeKey,
    active_camera: Option<CameraKey>,
    camera_layer_masks: BTreeMap<CameraKey, u64>,
    interaction: InteractionContext,
    structure_revision: u64,
    transform_revision: u64,
    not_sync: PhantomData<Cell<()>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    parent: Option<NodeKey>,
    children: Vec<NodeKey>,
    transform: Transform,
    kind: NodeKind,
    visible: bool,
    tags: BTreeSet<String>,
    layer_mask: u64,
    render_group: i16,
    helper_on_top: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Empty,
    Renderable(RenderableNode),
    Mesh(MeshNode),
    Model(ModelNode),
    InstanceSet(InstanceSetKey),
    Label(LabelKey),
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
            instance_sets: SlotMap::with_key(),
            animation_mixers: SlotMap::with_key(),
            labels: SlotMap::with_key(),
            morph_weights: BTreeMap::new(),
            skin_bindings: BTreeMap::new(),
            clipping_planes: SlotMap::with_key(),
            active_clipping_planes: ClippingPlaneSet::new(),
            origin_shift: Vec3::ZERO,
            root,
            active_camera: None,
            camera_layer_masks: BTreeMap::new(),
            interaction: InteractionContext::default(),
            structure_revision: 0,
            transform_revision: 0,
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

    pub(crate) fn identity(&self) -> Weak<()> {
        Arc::downgrade(&self.identity)
    }

    pub(crate) fn structure_revision(&self) -> u64 {
        self.structure_revision
            .saturating_add(self.interaction.revision())
    }

    pub(crate) fn mesh_nodes(&self) -> impl Iterator<Item = (NodeKey, MeshNode, Transform)> + '_ {
        self.nodes.iter().filter_map(|(key, node)| match node.kind {
            NodeKind::Mesh(mesh) if self.visible_for_active_camera(key) => {
                Some((key, mesh, node.transform))
            }
            NodeKind::Empty
            | NodeKind::Renderable(_)
            | NodeKind::Mesh(_)
            | NodeKind::Model(_)
            | NodeKind::InstanceSet(_)
            | NodeKind::Label(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        })
    }

    pub(crate) fn instance_set_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, &InstanceSet, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::InstanceSet(instance_set) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(node_key) {
                return None;
            }
            self.instance_sets
                .get(instance_set)
                .map(|instance_set| (node_key, instance_set, node.transform))
        })
    }

    pub(crate) fn model_nodes(&self) -> impl Iterator<Item = NodeKey> + '_ {
        self.nodes.iter().filter_map(|(key, node)| match node.kind {
            NodeKind::Model(_) if self.visible_for_active_camera(key) => Some(key),
            NodeKind::Empty
            | NodeKind::Renderable(_)
            | NodeKind::Mesh(_)
            | NodeKind::Model(_)
            | NodeKind::InstanceSet(_)
            | NodeKind::Label(_)
            | NodeKind::Camera(_)
            | NodeKind::Light(_) => None,
        })
    }

    pub(crate) fn label_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, LabelKey, &LabelDesc, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::Label(label) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(node_key) {
                return None;
            }
            self.labels
                .get(label)
                .map(|label_desc| (node_key, label, label_desc, node.transform))
        })
    }

    pub(crate) fn light_nodes(
        &self,
    ) -> impl Iterator<Item = (NodeKey, LightKey, Light, Transform)> + '_ {
        self.nodes.iter().filter_map(|(node_key, node)| {
            let NodeKind::Light(light_key) = node.kind else {
                return None;
            };
            if !self.visible_for_active_camera(node_key) {
                return None;
            }
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
        self.camera_layer_masks.insert(camera, u64::MAX);
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
            visible: true,
            tags: BTreeSet::new(),
            layer_mask: u64::MAX,
            render_group: 0,
            helper_on_top: false,
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
            visible: true,
            tags: BTreeSet::new(),
            layer_mask: u64::MAX,
            render_group: 0,
            helper_on_top: false,
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

    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub fn tags(&self) -> impl Iterator<Item = &str> {
        self.tags.iter().map(String::as_str)
    }

    pub const fn layer_mask(&self) -> u64 {
        self.layer_mask
    }

    pub const fn render_group(&self) -> i16 {
        self.render_group
    }

    pub const fn helper_on_top(&self) -> bool {
        self.helper_on_top
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
