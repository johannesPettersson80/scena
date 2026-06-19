use std::collections::BTreeMap;

use super::camera::controls_from_scene_camera;
use super::events::{HostEventQueue, HostEventV1};
use super::handles::HandleTable;
use super::inputs::validate_transform;
use super::instances::{HostInstanceBinding, INSTANCE_HANDLE_GENERATION_BASE};
use super::product_options::ProductOptionsV1;
use super::reporting::{diagnostics_json, stats_json};
use super::transitions::HostTransitions;
use super::visual_states::SceneHostVisualStateV1;
use super::{SceneHostError, SceneHostErrorCode};
use crate::Color;
use crate::{
    Aabb, AssetFetcher, AssetPath, Assets, Backend, DefaultAssetFetcher, ImportOptions,
    OrbitControls, RenderOutcome, Renderer, Scene, SceneImport, SurfaceEvent, SurfaceViewport,
    Transform, Vec3,
};
use crate::{AnimationMixerKey, CameraKey, InstanceId, NodeKey, SceneImportInspectionV1};

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
    pub(super) import_handles: HandleTable<SceneImport>,
    pub(super) instance_handles: HandleTable<HostInstanceBinding>,
    pub(super) animation_handles: HandleTable<AnimationMixerKey>,
    pub(super) transitions: HostTransitions,
    pub(super) events: HostEventQueue,
    pub(super) last_diagnostic_events: Vec<HostEventV1>,
    pub(super) node_handle_map: BTreeMap<NodeKey, u64>,
    pub(super) instance_handle_map: BTreeMap<(NodeKey, InstanceId), u64>,
    pub(super) section_box_helper: Option<u64>,
    pub(super) visual_states: BTreeMap<String, SceneHostVisualStateV1>,
    pub(super) product_options: ProductOptionsV1,
    next_byte_asset: u64,
}

impl SceneHostCore<DefaultAssetFetcher> {
    pub fn headless(width: u32, height: u32) -> Result<Self, SceneHostError> {
        Self::headless_with_fetcher(DefaultAssetFetcher::default(), width, height)
    }

    pub fn headless_gpu(width: u32, height: u32) -> Result<Self, SceneHostError> {
        Self::headless_gpu_with_fetcher(DefaultAssetFetcher::default(), width, height)
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

    pub fn headless_gpu_with_fetcher(
        fetcher: F,
        width: u32,
        height: u32,
    ) -> Result<Self, SceneHostError> {
        let viewport = headless_viewport(width, height)?;
        let renderer = Renderer::headless_gpu(width, height)
            .or_else(|_gpu_error| Renderer::headless(width, height))?;
        Self::from_renderer(Assets::with_fetcher(fetcher), renderer, viewport)
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
            events: HostEventQueue::default(),
            last_diagnostic_events: Vec::new(),
            node_handle_map: BTreeMap::new(),
            instance_handle_map: BTreeMap::new(),
            section_box_helper: None,
            visual_states: BTreeMap::new(),
            product_options: ProductOptionsV1::empty(),
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
        self.emit_surface_resized_event(viewport);
        Ok(())
    }

    pub fn handle_surface_event(&mut self, event: SurfaceEvent) -> Result<(), SceneHostError> {
        if let SurfaceEvent::ViewportChanged(viewport) = event {
            self.viewport = viewport;
        }
        self.renderer.handle_surface_event(event)?;
        match event {
            SurfaceEvent::Resize { width, height } => {
                let dpr = self.viewport.device_pixel_ratio();
                self.emit_event(HostEventV1::SurfaceResized {
                    width_css_px: width as f32 / dpr,
                    height_css_px: height as f32 / dpr,
                    width_physical_px: width,
                    height_physical_px: height,
                    device_pixel_ratio: dpr,
                });
            }
            SurfaceEvent::ViewportChanged(viewport) => self.emit_surface_resized_event(viewport),
            SurfaceEvent::ContextLost { recoverable } => {
                self.emit_event(HostEventV1::ContextLost { recoverable });
            }
            SurfaceEvent::ContextRestored => {
                self.emit_event(HostEventV1::ContextRestored);
                self.emit_event(HostEventV1::capability_changed(self.backend()));
            }
            SurfaceEvent::DeviceLost { recoverable } => {
                self.emit_event(HostEventV1::DeviceLost { recoverable });
            }
            SurfaceEvent::ScaleFactorChanged { .. }
            | SurfaceEvent::Occluded { .. }
            | SurfaceEvent::Hidden
            | SurfaceEvent::Shown
            | SurfaceEvent::Lost => {}
        }
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
        let report = self.assets.load_scene_with_report(path).await?;
        let import = self.instantiate_scene_asset_under(parent, report.asset())?;
        let asset_report = report.to_schema_report();
        self.emit_asset_load_events(import, &asset_report);
        Ok(import)
    }

    pub async fn instantiate_url_instanced_under(
        &mut self,
        parent: u64,
        path: impl Into<AssetPath>,
        count: usize,
    ) -> Result<Vec<u64>, SceneHostError> {
        let parent = self.resolve_node(parent)?;
        let report = self.assets.load_scene_with_report(path).await?;
        let roots = self.instantiate_scene_asset_instanced_under(parent, report.asset(), count)?;
        let asset_report = report.to_schema_report();
        self.emit_asset_progress_events(&asset_report);
        Ok(roots)
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
        self.ensure_active_camera()?;
        self.renderer
            .prepare_with_assets(&mut self.scene, &self.assets)?;
        self.emit_changed_diagnostics();
        Ok(())
    }

    pub fn render(&mut self) -> Result<RenderOutcome, SceneHostError> {
        self.ensure_active_camera()?;
        Ok(self.renderer.render_active(&self.scene)?)
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
        let imports = self.import_inspection_report();
        if !imports.is_empty() {
            report.imports = Some(imports);
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

    fn import_inspection_report(&self) -> Vec<SceneImportInspectionV1> {
        self.import_handles
            .entries()
            .map(|(handle, import)| SceneImportInspectionV1 {
                handle,
                root_handles: import
                    .roots()
                    .iter()
                    .filter_map(|root| self.node_handle_map.get(root).copied())
                    .collect(),
                material_variants: import.material_variants().to_vec(),
                active_variant: import.active_variant(),
            })
            .collect()
    }
}

fn headless_viewport(width: u32, height: u32) -> Result<SurfaceViewport, SceneHostError> {
    SurfaceViewport::new(width as f32, height as f32, 1.0).ok_or_else(|| {
        SceneHostError::new(
            SceneHostErrorCode::InvalidViewport,
            format!("invalid viewport {width}x{height} at DPR 1"),
        )
    })
}
