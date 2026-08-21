use std::collections::BTreeMap;

mod surface_events;

use super::camera::controls_from_scene_camera;
use super::events::{HostEventHitV1, HostEventQueue, HostEventV1};
use super::handles::{HandleKind, HandleTable};
use super::inputs::validate_transform;
use super::instances::HostInstanceBinding;
use super::product_options::ProductOptionsV1;
use super::reflection_probe_capture::PhotographicReflectionProbeBakeCache;
use super::reporting::{diagnostics_json, stats_json};
use super::transitions::HostTransitions;
use super::visual_states::SceneHostVisualStateV1;
use super::{SceneHostError, SceneHostErrorCode};
use crate::Color;
use crate::{
    Aabb, AssetFetcher, AssetPath, Assets, Backend, DefaultAssetFetcher,
    HeadlessBackendSelectionReport, ImportOptions, OrbitControls, RenderOutcome, Renderer, Scene,
    SceneImport, SurfaceViewport, Transform, Vec3,
};
use crate::{AnimationMixerKey, CameraKey, InstanceId, NodeKey, SceneImportInspectionV1};

#[derive(Debug)]
pub(super) enum RendererSlot {
    Active(Box<Renderer>),
    ManifestOnly,
}

impl std::ops::Deref for RendererSlot {
    type Target = Renderer;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Active(renderer) => renderer,
            Self::ManifestOnly => unreachable!("manifest-only recipe state never renders"),
        }
    }
}

impl std::ops::DerefMut for RendererSlot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Active(renderer) => renderer,
            Self::ManifestOnly => unreachable!("manifest-only recipe state never renders"),
        }
    }
}

#[derive(Debug)]
pub struct SceneHostCore<F = DefaultAssetFetcher> {
    pub(super) assets: Assets<F>,
    pub(super) scene: Scene,
    pub(super) renderer: RendererSlot,
    pub(super) backend_selection_report: Option<HeadlessBackendSelectionReport>,
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
    pub(super) last_hover_event_hit: Option<HostEventHitV1>,
    pub(super) node_handle_map: BTreeMap<NodeKey, u64>,
    pub(super) instance_handle_map: BTreeMap<(NodeKey, InstanceId), u64>,
    pub(super) section_box_helper: Option<u64>,
    pub(super) host_clipping_planes: Vec<crate::ClippingPlaneKey>,
    pub(super) visual_states: BTreeMap<String, SceneHostVisualStateV1>,
    pub(super) product_options: ProductOptionsV1,
    /// The environment the photographic lighting solver installed, if any.
    ///
    /// `renderer.environment().is_some()` cannot answer "did the user author an
    /// environment", because the solver installs one itself. Recording the
    /// handle keeps that distinction independent of the order the photographic
    /// passes run in.
    pub(super) generated_environment: Option<crate::assets::EnvironmentHandle>,
    pub(super) photographic_reflection_probe_cache: Option<PhotographicReflectionProbeBakeCache>,
    next_byte_asset: u64,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    /// True when an environment is installed that the photographic lighting
    /// solver did not derive, i.e. one the caller authored.
    pub(super) fn has_authored_environment(&self) -> bool {
        let current = self.renderer.environment();
        current.is_some() && current != self.generated_environment
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
            renderer: RendererSlot::Active(Box::new(renderer)),
            backend_selection_report: None,
            viewport,
            active_camera,
            camera_controls,
            node_handles: HandleTable::new(HandleKind::Node),
            import_handles: HandleTable::new(HandleKind::Import),
            instance_handles: HandleTable::new(HandleKind::InstanceRoot),
            animation_handles: HandleTable::new(HandleKind::Animation),
            transitions: HostTransitions::default(),
            events: HostEventQueue::default(),
            last_diagnostic_events: Vec::new(),
            last_hover_event_hit: None,
            node_handle_map: BTreeMap::new(),
            instance_handle_map: BTreeMap::new(),
            section_box_helper: None,
            host_clipping_planes: Vec::new(),
            visual_states: BTreeMap::new(),
            product_options: ProductOptionsV1::empty(),
            generated_environment: None,
            photographic_reflection_probe_cache: None,
            next_byte_asset: 1,
        };
        let root = host.scene.root();
        host.register_node(root);
        if let Some(camera_node) = host.scene.camera_node(active_camera) {
            host.register_node(camera_node);
        }
        Ok(host)
    }

    pub(super) fn for_manifest_build(
        assets: Assets<F>,
        viewport: SurfaceViewport,
    ) -> Result<Self, SceneHostError> {
        let mut scene = Scene::new();
        let active_camera = scene.add_default_camera()?;
        let camera_controls = controls_from_scene_camera(&scene, active_camera, Vec3::ZERO)?;
        let mut host = Self {
            assets,
            scene,
            renderer: RendererSlot::ManifestOnly,
            backend_selection_report: None,
            viewport,
            active_camera,
            camera_controls,
            node_handles: HandleTable::new(HandleKind::Node),
            import_handles: HandleTable::new(HandleKind::Import),
            instance_handles: HandleTable::new(HandleKind::InstanceRoot),
            animation_handles: HandleTable::new(HandleKind::Animation),
            transitions: HostTransitions::default(),
            events: HostEventQueue::default(),
            last_diagnostic_events: Vec::new(),
            last_hover_event_hit: None,
            node_handle_map: BTreeMap::new(),
            instance_handle_map: BTreeMap::new(),
            section_box_helper: None,
            host_clipping_planes: Vec::new(),
            visual_states: BTreeMap::new(),
            product_options: ProductOptionsV1::empty(),
            generated_environment: None,
            photographic_reflection_probe_cache: None,
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

    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub(super) fn recipe_max_clipping_planes(&self) -> usize {
        match &self.renderer {
            RendererSlot::Active(renderer) => {
                renderer
                    .capability_report()
                    .capabilities()
                    .max_clipping_planes as usize
            }
            RendererSlot::ManifestOnly => {
                crate::Capabilities::for_backend(crate::Backend::Headless).max_clipping_planes
                    as usize
            }
        }
    }

    pub fn root_handle(&self) -> u64 {
        self.node_handle_map[&self.scene.root()]
    }

    pub fn backend(&self) -> Backend {
        self.renderer.capabilities().backend
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
        let removed = self.scene.node_removal_closure(node_key)?;
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
        self.invalidate_stale_animation_handles();
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

    pub fn nodes_world_bounds(&self, nodes: &[u64]) -> Result<Option<Aabb>, SceneHostError> {
        let mut bounds = None;
        for node in nodes {
            let node = self.resolve_node(*node)?;
            if let Some(next) = self.scene.node_world_bounds(node, &self.assets)? {
                bounds = Some(bounds.map_or(next, |current: Aabb| current.union(next)));
            }
        }
        Ok(bounds)
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
