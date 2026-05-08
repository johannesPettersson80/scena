use crate::assets::{
    Assets, GeometryHandle, MaterialHandle, TextureDesc, TextureHandle, TextureSamplerDesc,
    TextureSourceFormat,
};
use crate::geometry::{Aabb, GeometryTopology};
use crate::material::{AlphaMode, Color, MaterialDesc, MaterialKind, TextureColorSpace};

use super::{CameraKey, InstanceId, LightKey, NodeKey, NodeKind, Scene, Transform, Vec3};

mod builders;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneInspectionReport {
    nodes: Vec<SceneNodeInspection>,
    draw_list: Vec<SceneDrawInspection>,
    camera_frustums: Vec<SceneCameraFrustumInspection>,
    normal_overlays: Vec<SceneNormalInspection>,
    active_camera: Option<CameraKey>,
    visible_drawable_count: usize,
    camera_count: usize,
    light_count: usize,
    anchor_count: usize,
    connector_count: usize,
    bounded_node_count: usize,
    clipping_plane_count: usize,
    structure_revision: u64,
    transform_revision: u64,
    interaction_revision: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneNodeInspection {
    node: NodeKey,
    parent: Option<NodeKey>,
    kind: &'static str,
    transform: Transform,
    world_transform: Transform,
    visible: bool,
    tags: Vec<String>,
    bounds: Option<Aabb>,
    mesh_geometry: Option<GeometryHandle>,
    mesh_material: Option<MaterialHandle>,
    material_preview: Option<SceneMaterialInspection>,
    camera: Option<CameraKey>,
    light: Option<LightKey>,
    layer_mask: u64,
    render_group: i16,
    helper_on_top: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneDrawInspection {
    node: NodeKey,
    instance: Option<InstanceId>,
    geometry: GeometryHandle,
    material: MaterialHandle,
    material_preview: Option<SceneMaterialInspection>,
    topology: GeometryTopology,
    primitive_count: usize,
    vertex_count: usize,
    index_count: usize,
    local_bounds: Aabb,
    world_transform: Transform,
    visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneCameraFrustumInspection {
    camera: CameraKey,
    node: NodeKey,
    near: f32,
    far: f32,
    corners: [Vec3; 8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneNormalInspection {
    node: NodeKey,
    instance: Option<InstanceId>,
    geometry: GeometryHandle,
    length: f32,
    segments: Vec<[Vec3; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneMaterialInspection {
    material: MaterialHandle,
    kind: MaterialKind,
    base_color: Color,
    alpha_mode: AlphaMode,
    base_color_texture: Option<SceneTextureInspection>,
    normal_texture: Option<SceneTextureInspection>,
    metallic_roughness_texture: Option<SceneTextureInspection>,
    occlusion_texture: Option<SceneTextureInspection>,
    emissive_texture: Option<SceneTextureInspection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneTextureInspection {
    texture: TextureHandle,
    color_space: TextureColorSpace,
    sampler: TextureSamplerDesc,
    source_format: TextureSourceFormat,
    decoded_dimensions: Option<(u32, u32)>,
    has_decoded_pixels: bool,
}

impl Scene {
    pub fn inspect(&self) -> SceneInspectionReport {
        self.inspect_inner(None::<&Assets>)
    }

    pub fn inspect_with_assets<F>(&self, assets: &Assets<F>) -> SceneInspectionReport {
        self.inspect_inner(Some(assets))
    }

    fn inspect_inner<F>(&self, assets: Option<&Assets<F>>) -> SceneInspectionReport {
        let dirty = self.dirty_state();
        SceneInspectionReport {
            nodes: self
                .nodes
                .iter()
                .map(|(node_key, node)| SceneNodeInspection {
                    node: node_key,
                    parent: node.parent,
                    kind: kind_name(&node.kind),
                    transform: node.transform,
                    world_transform: self.world_transform(node_key).unwrap_or(node.transform),
                    visible: node.visible,
                    tags: node.tags.iter().cloned().collect(),
                    bounds: self.node_bounds.get(&node_key).copied(),
                    mesh_geometry: mesh_geometry(&node.kind),
                    mesh_material: mesh_material(&node.kind),
                    material_preview: material_preview(&node.kind, assets),
                    camera: camera_key(&node.kind),
                    light: light_key(&node.kind),
                    layer_mask: node.layer_mask,
                    render_group: node.render_group,
                    helper_on_top: node.helper_on_top,
                })
                .collect(),
            draw_list: self.inspect_draw_list(assets),
            camera_frustums: self.inspect_camera_frustums(),
            normal_overlays: self.inspect_normal_overlays(assets),
            active_camera: self.active_camera,
            visible_drawable_count: self.visible_drawable_count(),
            camera_count: self.cameras.len(),
            light_count: self.lights.len(),
            anchor_count: self.anchors.len(),
            connector_count: self.connectors.len(),
            bounded_node_count: self.node_bounds.len(),
            clipping_plane_count: self.clipping_planes.len(),
            structure_revision: dirty.structure_revision,
            transform_revision: dirty.transform_revision,
            interaction_revision: dirty.interaction_revision,
        }
    }
}

impl SceneInspectionReport {
    pub fn nodes(&self) -> &[SceneNodeInspection] {
        &self.nodes
    }

    pub fn draw_list(&self) -> &[SceneDrawInspection] {
        &self.draw_list
    }

    pub fn camera_frustums(&self) -> &[SceneCameraFrustumInspection] {
        &self.camera_frustums
    }

    pub fn normal_overlays(&self) -> &[SceneNormalInspection] {
        &self.normal_overlays
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub const fn active_camera(&self) -> Option<CameraKey> {
        self.active_camera
    }

    pub const fn visible_drawable_count(&self) -> usize {
        self.visible_drawable_count
    }

    pub const fn camera_count(&self) -> usize {
        self.camera_count
    }

    pub const fn light_count(&self) -> usize {
        self.light_count
    }

    pub const fn anchor_count(&self) -> usize {
        self.anchor_count
    }

    pub const fn connector_count(&self) -> usize {
        self.connector_count
    }

    pub const fn bounded_node_count(&self) -> usize {
        self.bounded_node_count
    }

    pub const fn clipping_plane_count(&self) -> usize {
        self.clipping_plane_count
    }

    pub const fn structure_revision(&self) -> u64 {
        self.structure_revision
    }

    pub const fn transform_revision(&self) -> u64 {
        self.transform_revision
    }

    pub const fn interaction_revision(&self) -> u64 {
        self.interaction_revision
    }
}

impl SceneNodeInspection {
    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn parent(&self) -> Option<NodeKey> {
        self.parent
    }

    pub const fn kind(&self) -> &'static str {
        self.kind
    }

    pub const fn transform(&self) -> Transform {
        self.transform
    }

    pub const fn world_transform(&self) -> Transform {
        self.world_transform
    }

    pub const fn visible(&self) -> bool {
        self.visible
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub const fn bounds(&self) -> Option<Aabb> {
        self.bounds
    }

    pub const fn mesh_geometry(&self) -> Option<GeometryHandle> {
        self.mesh_geometry
    }

    pub const fn mesh_material(&self) -> Option<MaterialHandle> {
        self.mesh_material
    }

    pub const fn material_preview(&self) -> Option<SceneMaterialInspection> {
        self.material_preview
    }

    pub const fn camera(&self) -> Option<CameraKey> {
        self.camera
    }

    pub const fn light(&self) -> Option<LightKey> {
        self.light
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

impl SceneDrawInspection {
    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn instance(&self) -> Option<InstanceId> {
        self.instance
    }

    pub const fn geometry(&self) -> GeometryHandle {
        self.geometry
    }

    pub const fn material(&self) -> MaterialHandle {
        self.material
    }

    pub const fn material_preview(&self) -> Option<SceneMaterialInspection> {
        self.material_preview
    }

    pub const fn topology(&self) -> GeometryTopology {
        self.topology
    }

    pub const fn primitive_count(&self) -> usize {
        self.primitive_count
    }

    pub const fn vertex_count(&self) -> usize {
        self.vertex_count
    }

    pub const fn index_count(&self) -> usize {
        self.index_count
    }

    pub const fn local_bounds(&self) -> Aabb {
        self.local_bounds
    }

    pub const fn world_transform(&self) -> Transform {
        self.world_transform
    }

    pub const fn visible(&self) -> bool {
        self.visible
    }
}

impl SceneCameraFrustumInspection {
    pub const fn camera(&self) -> CameraKey {
        self.camera
    }

    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn near(&self) -> f32 {
        self.near
    }

    pub const fn far(&self) -> f32 {
        self.far
    }

    pub const fn corners(&self) -> &[Vec3; 8] {
        &self.corners
    }
}

impl SceneNormalInspection {
    pub const fn node(&self) -> NodeKey {
        self.node
    }

    pub const fn instance(&self) -> Option<InstanceId> {
        self.instance
    }

    pub const fn geometry(&self) -> GeometryHandle {
        self.geometry
    }

    pub const fn length(&self) -> f32 {
        self.length
    }

    pub fn segments(&self) -> &[[Vec3; 2]] {
        &self.segments
    }
}

impl SceneMaterialInspection {
    pub const fn material(&self) -> MaterialHandle {
        self.material
    }

    pub const fn kind(&self) -> MaterialKind {
        self.kind
    }

    pub const fn base_color(&self) -> Color {
        self.base_color
    }

    pub const fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    pub const fn has_base_color_texture(&self) -> bool {
        self.base_color_texture.is_some()
    }

    pub const fn base_color_texture(&self) -> Option<SceneTextureInspection> {
        self.base_color_texture
    }

    pub const fn has_normal_texture(&self) -> bool {
        self.normal_texture.is_some()
    }

    pub const fn normal_texture(&self) -> Option<SceneTextureInspection> {
        self.normal_texture
    }

    pub const fn has_metallic_roughness_texture(&self) -> bool {
        self.metallic_roughness_texture.is_some()
    }

    pub const fn metallic_roughness_texture(&self) -> Option<SceneTextureInspection> {
        self.metallic_roughness_texture
    }

    pub const fn has_occlusion_texture(&self) -> bool {
        self.occlusion_texture.is_some()
    }

    pub const fn occlusion_texture(&self) -> Option<SceneTextureInspection> {
        self.occlusion_texture
    }

    pub const fn has_emissive_texture(&self) -> bool {
        self.emissive_texture.is_some()
    }

    pub const fn emissive_texture(&self) -> Option<SceneTextureInspection> {
        self.emissive_texture
    }
}

impl SceneTextureInspection {
    pub const fn texture(&self) -> TextureHandle {
        self.texture
    }

    pub const fn color_space(&self) -> TextureColorSpace {
        self.color_space
    }

    pub const fn sampler(&self) -> TextureSamplerDesc {
        self.sampler
    }

    pub const fn source_format(&self) -> TextureSourceFormat {
        self.source_format
    }

    pub const fn decoded_dimensions(&self) -> Option<(u32, u32)> {
        self.decoded_dimensions
    }

    pub const fn has_decoded_pixels(&self) -> bool {
        self.has_decoded_pixels
    }
}

const fn mesh_geometry(kind: &NodeKind) -> Option<GeometryHandle> {
    match kind {
        NodeKind::Mesh(mesh) => Some(mesh.geometry()),
        _ => None,
    }
}

const fn mesh_material(kind: &NodeKind) -> Option<MaterialHandle> {
    match kind {
        NodeKind::Mesh(mesh) => Some(mesh.material()),
        _ => None,
    }
}

fn material_preview<F>(
    kind: &NodeKind,
    assets: Option<&Assets<F>>,
) -> Option<SceneMaterialInspection> {
    let material_handle = mesh_material(kind)?;
    let assets = assets?;
    let material = assets.material(material_handle)?;
    Some(SceneMaterialInspection::new(
        material_handle,
        material,
        assets,
    ))
}

impl SceneMaterialInspection {
    fn new<F>(material: MaterialHandle, desc: MaterialDesc, assets: &Assets<F>) -> Self {
        Self {
            material,
            kind: desc.kind(),
            base_color: desc.base_color(),
            alpha_mode: desc.alpha_mode(),
            base_color_texture: texture_preview(desc.base_color_texture(), assets),
            normal_texture: texture_preview(desc.normal_texture(), assets),
            metallic_roughness_texture: texture_preview(desc.metallic_roughness_texture(), assets),
            occlusion_texture: texture_preview(desc.occlusion_texture(), assets),
            emissive_texture: texture_preview(desc.emissive_texture(), assets),
        }
    }
}

fn texture_preview<F>(
    texture: Option<TextureHandle>,
    assets: &Assets<F>,
) -> Option<SceneTextureInspection> {
    let texture = texture?;
    assets
        .texture(texture)
        .map(|desc| SceneTextureInspection::new(texture, desc))
}

impl SceneTextureInspection {
    fn new(texture: TextureHandle, desc: TextureDesc) -> Self {
        Self {
            texture,
            color_space: desc.color_space(),
            sampler: desc.sampler(),
            source_format: desc.source_format(),
            decoded_dimensions: desc.decoded_dimensions(),
            has_decoded_pixels: desc.has_decoded_pixels(),
        }
    }
}

const fn camera_key(kind: &NodeKind) -> Option<CameraKey> {
    match kind {
        NodeKind::Camera(camera) => Some(*camera),
        _ => None,
    }
}

const fn light_key(kind: &NodeKind) -> Option<LightKey> {
    match kind {
        NodeKind::Light(light) => Some(*light),
        _ => None,
    }
}

const fn kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Empty => "Empty",
        NodeKind::Renderable(_) => "Renderable",
        NodeKind::Mesh(_) => "Mesh",
        NodeKind::Model(_) => "Model",
        NodeKind::InstanceSet(_) => "InstanceSet",
        NodeKind::Label(_) => "Label",
        NodeKind::Camera(_) => "Camera",
        NodeKind::Light(_) => "Light",
    }
}
