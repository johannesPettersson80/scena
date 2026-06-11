use std::collections::BTreeMap;

use super::camera::controls_from_scene_camera;
use super::handles::HandleTable;
use super::inputs::validate_transform;
use super::instances::{HostInstanceBinding, INSTANCE_HANDLE_GENERATION_BASE};
use super::reporting::{diagnostics_json, stats_json};
use super::transitions::HostTransitions;
use super::{SceneHostError, SceneHostErrorCode};
use crate::{
    Aabb, AssetFetcher, AssetPath, Assets, Backend, CursorPosition, DefaultAssetFetcher, HitTarget,
    ImportOptions, OrbitControls, RenderOutcome, Renderer, Scene, SceneImport, SurfaceEvent,
    SurfaceViewport, Transform, Vec3, Viewport,
};
use crate::{AnimationMixerKey, CameraKey, InstanceId, NodeKey};
use crate::{AnnotationAnchor, Color};

const ANIMATION_HANDLE_GENERATION_BASE: u32 = 6;

#[derive(Debug)]
pub struct SceneHostCore<F = DefaultAssetFetcher> {
    pub(super) assets: Assets<F>,
    pub(super) scene: Scene,
    pub(super) renderer: Renderer,
    pub(super) viewport: SurfaceViewport,
    pub(super) active_camera: CameraKey,
    pub(super) camera_controls: OrbitControls,
    pub(super) node_handles: HandleTable<NodeKey>,
    import_handles: HandleTable<SceneImport>,
    pub(super) instance_handles: HandleTable<HostInstanceBinding>,
    pub(super) animation_handles: HandleTable<AnimationMixerKey>,
    pub(super) transitions: HostTransitions,
    pub(super) node_handle_map: BTreeMap<NodeKey, u64>,
    pub(super) instance_handle_map: BTreeMap<(NodeKey, InstanceId), u64>,
    next_byte_asset: u64,
}

impl SceneHostCore<DefaultAssetFetcher> {
    pub fn headless(width: u32, height: u32) -> Result<Self, SceneHostError> {
        Self::headless_with_fetcher(DefaultAssetFetcher::default(), width, height)
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn headless_with_fetcher(
        fetcher: F,
        width: u32,
        height: u32,
    ) -> Result<Self, SceneHostError> {
        let viewport = SurfaceViewport::new(width as f32, height as f32, 1.0).ok_or_else(|| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidViewport,
                format!("invalid viewport {width}x{height} at DPR 1"),
            )
        })?;
        Self::from_renderer(
            Assets::with_fetcher(fetcher),
            Renderer::headless(width, height)?,
            viewport,
        )
    }

    pub fn from_renderer(
        assets: Assets<F>,
        renderer: Renderer,
        viewport: SurfaceViewport,
    ) -> Result<Self, SceneHostError> {
        let mut scene = Scene::new();
        let active_camera = scene.add_default_camera()?;
        let camera_controls = controls_from_scene_camera(&scene, active_camera, Vec3::ZERO)?;
        let mut host = Self {
            assets,
            scene,
            renderer,
            viewport,
            active_camera,
            camera_controls,
            node_handles: HandleTable::new(),
            import_handles: HandleTable::new(),
            instance_handles: HandleTable::with_generation_base(INSTANCE_HANDLE_GENERATION_BASE),
            animation_handles: HandleTable::with_generation_base(ANIMATION_HANDLE_GENERATION_BASE),
            transitions: HostTransitions::default(),
            node_handle_map: BTreeMap::new(),
            instance_handle_map: BTreeMap::new(),
            next_byte_asset: 1,
        };
        let root = host.scene.root();
        host.register_node(root);
        if let Some(camera_node) = host.scene.camera_node(active_camera) {
            host.register_node(camera_node);
        }
        Ok(host)
    }

    pub fn assets(&self) -> &Assets<F> {
        &self.assets
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub fn root_handle(&self) -> u64 {
        self.node_handle_map[&self.scene.root()]
    }

    pub fn backend(&self) -> Backend {
        self.renderer.capabilities().backend
    }

    pub fn resize(
        &mut self,
        logical_width: f32,
        logical_height: f32,
        device_pixel_ratio: f32,
    ) -> Result<(), SceneHostError> {
        let viewport = SurfaceViewport::new(logical_width, logical_height, device_pixel_ratio)
            .ok_or_else(|| {
                SceneHostError::new(
                    SceneHostErrorCode::InvalidViewport,
                    format!(
                        "invalid viewport {logical_width}x{logical_height} at DPR {device_pixel_ratio}"
                    ),
                )
            })?;
        self.viewport = viewport;
        self.renderer
            .handle_surface_event(SurfaceEvent::ViewportChanged(viewport))?;
        Ok(())
    }

    pub fn handle_surface_event(&mut self, event: SurfaceEvent) -> Result<(), SceneHostError> {
        if let SurfaceEvent::ViewportChanged(viewport) = event {
            self.viewport = viewport;
        }
        self.renderer.handle_surface_event(event)?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn attach_surface(
        &mut self,
        surface: crate::PlatformSurface,
    ) -> Result<(), SceneHostError> {
        self.renderer.attach_surface_async(surface).await?;
        Ok(())
    }

    pub fn add_empty(
        &mut self,
        parent: Option<u64>,
        transform: Transform,
        tag: Option<&str>,
    ) -> Result<u64, SceneHostError> {
        let transform = validate_transform(transform)?;
        let parent = self.resolve_parent(parent)?;
        let node = self.scene.add_empty(parent, transform)?;
        if let Some(tag) = tag {
            self.scene.add_tag(node, tag)?;
        }
        Ok(self.register_node(node))
    }

    pub fn set_tag(&mut self, node: u64, tag: &str) -> Result<(), SceneHostError> {
        let node = self.resolve_node(node)?;
        self.scene.add_tag(node, tag)?;
        Ok(())
    }

    pub fn clear_tag(&mut self, node: u64, tag: &str) -> Result<bool, SceneHostError> {
        let node = self.resolve_node(node)?;
        Ok(self.scene.remove_tag(node, tag)?)
    }

    pub fn find_by_tag(&mut self, tag: &str) -> Vec<u64> {
        let nodes = self.scene.tagged(tag).collect::<Vec<_>>();
        nodes
            .into_iter()
            .map(|node| self.register_node(node))
            .collect()
    }

    pub async fn instantiate_url(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> Result<u64, SceneHostError> {
        self.instantiate_url_under(self.root_handle(), path).await
    }

    pub async fn instantiate_url_instanced(
        &mut self,
        path: impl Into<AssetPath>,
        count: usize,
    ) -> Result<Vec<u64>, SceneHostError> {
        self.instantiate_url_instanced_under(self.root_handle(), path, count)
            .await
    }

    pub async fn instantiate_url_under(
        &mut self,
        parent: u64,
        path: impl Into<AssetPath>,
    ) -> Result<u64, SceneHostError> {
        let parent = self.resolve_node(parent)?;
        let scene_asset = self.assets.load_scene(path).await?;
        self.instantiate_scene_asset_under(parent, &scene_asset)
    }

    pub async fn instantiate_url_instanced_under(
        &mut self,
        parent: u64,
        path: impl Into<AssetPath>,
        count: usize,
    ) -> Result<Vec<u64>, SceneHostError> {
        let parent = self.resolve_node(parent)?;
        let scene_asset = self.assets.load_scene(path).await?;
        self.instantiate_scene_asset_instanced_under(parent, &scene_asset, count)
    }

    pub async fn instantiate_glb(&mut self, bytes: &[u8]) -> Result<u64, SceneHostError> {
        self.instantiate_glb_under(self.root_handle(), bytes).await
    }

    pub async fn instantiate_glb_under(
        &mut self,
        parent: u64,
        bytes: &[u8],
    ) -> Result<u64, SceneHostError> {
        let parent = self.resolve_node(parent)?;
        let path = AssetPath::from(format!(
            "memory://scena-scene-host/{}.glb",
            self.next_byte_asset
        ));
        self.next_byte_asset = self.next_byte_asset.saturating_add(1);
        let scene_asset = self.assets.load_scene_from_bytes(path, bytes).await?;
        self.instantiate_scene_asset_under(parent, &scene_asset)
    }

    pub fn instantiate_scene_asset_under(
        &mut self,
        parent: NodeKey,
        scene_asset: &crate::SceneAsset,
    ) -> Result<u64, SceneHostError> {
        let import =
            self.scene
                .instantiate_under(parent, scene_asset, ImportOptions::gltf_default())?;
        let roots = import.roots().to_vec();
        for root in roots {
            self.register_subtree(root);
        }
        Ok(self.import_handles.insert(import))
    }

    pub fn import_roots(&mut self, import: u64) -> Result<Vec<u64>, SceneHostError> {
        let roots = {
            let import = self.resolve_import(import)?;
            import.roots().to_vec()
        };
        Ok(roots
            .into_iter()
            .map(|node| self.register_node(node))
            .collect())
    }

    pub fn node_handle(&mut self, import: u64, path: &str) -> Result<u64, SceneHostError> {
        let node = {
            let import = self.resolve_import(import)?;
            import.path(path)?
        };
        Ok(self.register_node(node))
    }

    pub fn node_handle_by_name(&mut self, import: u64, name: &str) -> Result<u64, SceneHostError> {
        let node = {
            let import = self.resolve_import(import)?;
            import.node(name)?
        };
        Ok(self.register_node(node))
    }

    pub fn node_handle_from_inspection(&self, handle: u64) -> Result<u64, SceneHostError> {
        self.resolve_node(handle)?;
        Ok(handle)
    }

    pub fn set_node_tint(&mut self, node: u64, tint: Option<Color>) -> Result<(), SceneHostError> {
        let handle = node;
        if self.is_instance_root_handle(handle) {
            if tint.is_some_and(|tint| tint.a < 1.0) {
                return Err(SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    "instanced scene roots only accept opaque per-instance tint in this release",
                ));
            }
            self.cancel_tint_transition(handle);
            return self.set_instance_root_tint(handle, tint);
        }
        let node = self.resolve_node(handle)?;
        self.cancel_tint_transition(handle);
        self.scene.set_node_tint(node, tint)?;
        Ok(())
    }

    pub fn set_node_annotation(
        &mut self,
        id: &str,
        node: u64,
        local_offset: [f32; 3],
    ) -> Result<(), SceneHostError> {
        let node = self.resolve_node(node)?;
        self.scene.set_annotation_anchor(AnnotationAnchor::node(
            id,
            node,
            Vec3::new(local_offset[0], local_offset[1], local_offset[2]),
        ))?;
        Ok(())
    }

    pub fn set_world_annotation(
        &mut self,
        id: &str,
        position: [f32; 3],
    ) -> Result<(), SceneHostError> {
        self.scene.set_annotation_anchor(AnnotationAnchor::world(
            id,
            Vec3::new(position[0], position[1], position[2]),
        ))?;
        Ok(())
    }

    pub fn clear_annotation(&mut self, id: &str) -> bool {
        self.scene.clear_annotation_anchor(id)
    }

    pub fn remove_node(&mut self, node: u64) -> Result<(), SceneHostError> {
        if self.is_instance_root_handle(node) {
            return self.remove_instance_root(node);
        }
        let node_key = self.resolve_node(node)?;
        let removed = self.scene.subtree_nodes(node_key)?;
        self.scene.remove_node(node_key)?;
        self.invalidate_instance_bindings_for_nodes(&removed);
        self.invalidate_node_handles(&removed);
        Ok(())
    }

    pub fn remove_import(&mut self, import: u64) -> Result<(), SceneHostError> {
        let import_snapshot = self.resolve_import(import)?.clone();
        let mut removed = Vec::new();
        for root in import_snapshot.roots() {
            removed.extend(self.scene.subtree_nodes(*root)?);
        }
        self.scene.remove_import(&import_snapshot)?;
        self.invalidate_node_handles(&removed);
        self.import_handles.remove(
            import,
            SceneHostErrorCode::ImportHandleNotFound,
            SceneHostErrorCode::StaleImportHandle,
        )?;
        Ok(())
    }

    pub fn world_distance(&self, a: u64, b: u64) -> Result<f32, SceneHostError> {
        let a = self.resolve_node(a)?;
        let b = self.resolve_node(b)?;
        Ok(self.scene.world_distance(a, b)?)
    }

    pub fn node_world_bounds(&self, node: u64) -> Result<Option<Aabb>, SceneHostError> {
        let node = self.resolve_node(node)?;
        Ok(self.scene.node_world_bounds(node, &self.assets)?)
    }

    pub fn node_world_bounds_json(&self, node: u64) -> Result<String, SceneHostError> {
        serde_json::to_string(&self.node_world_bounds(node)?).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("node bounds serialization failed: {error}"),
            )
        })
    }

    pub fn prepare(&mut self) -> Result<(), SceneHostError> {
        self.renderer
            .prepare_with_assets(&mut self.scene, &self.assets)?;
        Ok(())
    }

    pub fn render(&mut self) -> Result<RenderOutcome, SceneHostError> {
        Ok(self.renderer.render_active(&self.scene)?)
    }

    pub fn pick(&mut self, x: f32, y: f32) -> Result<Option<u64>, SceneHostError> {
        let size = self.viewport.physical_size();
        let viewport = Viewport::new(size.width, size.height, self.viewport.device_pixel_ratio())
            .ok_or_else(|| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidViewport,
                format!(
                    "invalid viewport {}x{} at DPR {}",
                    size.width,
                    size.height,
                    self.viewport.device_pixel_ratio()
                ),
            )
        })?;
        let hit = self.scene.pick_with_assets(
            self.active_camera,
            CursorPosition::logical(x, y),
            viewport,
            &self.assets,
        )?;
        match hit.map(|hit| hit.target()) {
            Some(HitTarget::Node(node)) => Ok(Some(self.register_node(node))),
            Some(HitTarget::Instance { node, instance }) => {
                Ok(self.instance_handle_map.get(&(node, instance)).copied())
            }
            None => Ok(None),
        }
    }

    pub fn inspect_json(&self) -> Result<String, SceneHostError> {
        let mut report = self
            .scene
            .inspect_with_assets(&self.assets)
            .to_schema_report_with_node_handles(&self.node_handle_map);
        let instance_sets = self.instance_bindings_report();
        if !instance_sets.is_empty() {
            report.instance_sets = Some(instance_sets);
        }
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("scene inspection serialization failed: {error}"),
            )
        })
    }

    pub fn annotation_projections_json(&self) -> Result<String, SceneHostError> {
        let width = self.viewport.logical_width().round().max(1.0) as u32;
        let height = self.viewport.logical_height().round().max(1.0) as u32;
        let mut report = self.scene.annotation_projection_report_with_node_handles(
            self.active_camera,
            width,
            height,
            &self.node_handle_map,
        )?;
        report.coordinate_space = "css_pixels".to_owned();
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("annotation projection serialization failed: {error}"),
            )
        })
    }

    pub fn capabilities_json(&self) -> Result<String, SceneHostError> {
        serde_json::to_string(&self.renderer.capability_report().to_schema_report()).map_err(
            |error| {
                SceneHostError::new(
                    SceneHostErrorCode::Inspect,
                    format!("capability serialization failed: {error}"),
                )
            },
        )
    }

    pub fn diagnostics_json(&self) -> String {
        diagnostics_json(self.renderer.diagnostics()).to_string()
    }

    pub fn stats_json(&self) -> String {
        stats_json(self.renderer.stats()).to_string()
    }

    fn resolve_parent(&self, parent: Option<u64>) -> Result<NodeKey, SceneHostError> {
        parent.map_or(Ok(self.scene.root()), |parent| self.resolve_node(parent))
    }

    pub(super) fn resolve_node(&self, handle: u64) -> Result<NodeKey, SceneHostError> {
        self.node_handles
            .get(
                handle,
                SceneHostErrorCode::NodeHandleNotFound,
                SceneHostErrorCode::StaleNodeHandle,
            )
            .copied()
    }

    pub(super) fn resolve_import(&self, handle: u64) -> Result<&SceneImport, SceneHostError> {
        self.import_handles.get(
            handle,
            SceneHostErrorCode::ImportHandleNotFound,
            SceneHostErrorCode::StaleImportHandle,
        )
    }

    pub(super) fn register_node(&mut self, node: NodeKey) -> u64 {
        if let Some(handle) = self.node_handle_map.get(&node).copied() {
            return handle;
        }
        let handle = self.node_handles.insert(node);
        self.node_handle_map.insert(node, handle);
        handle
    }

    fn register_subtree(&mut self, node: NodeKey) {
        self.register_node(node);
        let children = self
            .scene
            .node(node)
            .map(|node| node.children().to_vec())
            .unwrap_or_default();
        for child in children {
            self.register_subtree(child);
        }
    }

    fn invalidate_node_handles(&mut self, nodes: &[NodeKey]) {
        for node in nodes {
            let Some(handle) = self.node_handle_map.remove(node) else {
                continue;
            };
            let _ = self.node_handles.remove(
                handle,
                SceneHostErrorCode::NodeHandleNotFound,
                SceneHostErrorCode::StaleNodeHandle,
            );
        }
    }

    pub(super) fn is_instance_root_handle(&self, handle: u64) -> bool {
        handle / (1_u64 << 32) >= u64::from(INSTANCE_HANDLE_GENERATION_BASE)
    }
}
