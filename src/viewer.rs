//! High-level viewer helpers built from `Scene`, `Assets`, and `Renderer`.

mod animation;
mod asset_catalog_preview;
mod capture;
mod interaction;
mod load_progress;
mod material_variants;
mod profile;

pub use asset_catalog_preview::{
    AssetCatalogPreviewError, AssetCatalogPreviewPng, render_asset_catalog_preview_png,
};
pub use capture::{ViewerCaptureError, ViewerPngError};
pub use profile::{VIEWER_PROFILE_NAMES, ViewerProfile, ViewerProfileLighting};

use crate::assets::{AssetLoadProgress, AssetPath, Assets};
use crate::controls::{
    CameraBookmark, OrbitControlAction, OrbitControls, PointerEvent, TouchEvent,
};
use crate::diagnostics::{Diagnostic, LookupError, RenderOutcome};
use crate::material::Color;
use crate::picking::Hit;
use crate::platform::{PlatformSurface, SurfaceEvent};
use crate::render::{Background, Profile, Quality, RenderMode, Renderer, RendererOptions};
use crate::scene::{CameraKey, Scene, SceneImport, Transform, Vec3};

type ViewerPickCallback = Box<dyn FnMut(std::result::Result<Option<Hit>, LookupError>) + 'static>;

/// Owned state returned by [`first_render_gltf_headless`].
#[derive(Debug)]
pub struct FirstRender {
    assets: Assets,
    scene: Scene,
    renderer: Renderer,
    import: SceneImport,
    outcome: RenderOutcome,
    diagnostics: Vec<Diagnostic>,
    load_progress_events: Vec<AssetLoadProgress>,
    camera_bookmarks: Vec<CameraBookmark>,
}

/// Prepared owned state for a headless glTF viewer loop.
#[derive(Debug)]
pub struct HeadlessGltfViewer {
    assets: Assets,
    scene: Scene,
    renderer: Renderer,
    import: SceneImport,
    load_progress_events: Vec<AssetLoadProgress>,
    camera_bookmarks: Vec<CameraBookmark>,
}

/// Builder for the first headless glTF render.
#[derive(Debug, Clone)]
pub struct HeadlessGltfViewerBuilder {
    path: AssetPath,
    width: u32,
    height: u32,
    prefer_gpu: bool,
    common: ViewerCommonOptions,
}

#[derive(Debug, Clone)]
struct ViewerCommonOptions {
    frame_import: bool,
    lighting: ViewerProfileLighting,
    default_environment: bool,
    environment_path: Option<AssetPath>,
    renderer_options: RendererOptions,
    import_transform: Option<Transform>,
    background: Option<Background>,
    camera_bookmarks: Vec<CameraBookmark>,
    grid_floor: bool,
}

impl ViewerCommonOptions {
    fn new() -> Self {
        Self {
            frame_import: true,
            lighting: ViewerProfileLighting::None,
            default_environment: false,
            environment_path: None,
            renderer_options: RendererOptions::default(),
            import_transform: None,
            background: None,
            camera_bookmarks: Vec::new(),
            grid_floor: false,
        }
    }

    fn with_environment(mut self, path: impl Into<AssetPath>) -> Self {
        self.environment_path = Some(path.into());
        self.default_environment = false;
        self
    }

    fn apply_viewer_profile(&mut self, profile: ViewerProfile) {
        self.renderer_options = self
            .renderer_options
            .with_profile(profile.renderer_profile())
            .with_render_mode(profile.render_mode());
        self.default_environment = profile.default_environment();
        self.environment_path = None;
        self.lighting = profile.lighting();
        self.grid_floor = profile.grid();
        self.background = profile.background();
    }
}

/// Starts a fluent headless glTF viewer setup.
pub fn headless_gltf_viewer(path: impl Into<AssetPath>) -> HeadlessGltfViewerBuilder {
    HeadlessGltfViewerBuilder {
        path: path.into(),
        width: 800,
        height: 600,
        prefer_gpu: false,
        common: ViewerCommonOptions::new(),
    }
}

impl FirstRender {
    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub fn import(&self) -> &SceneImport {
        &self.import
    }

    pub fn outcome(&self) -> &RenderOutcome {
        &self.outcome
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn camera_bookmarks(&self) -> &[CameraBookmark] {
        &self.camera_bookmarks
    }
}

impl HeadlessGltfViewerBuilder {
    /// Sets the headless render target size.
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Adds a neutral directional light before the first prepare/render.
    pub const fn with_default_light(mut self) -> Self {
        self.common.lighting = ViewerProfileLighting::Directional;
        self
    }

    /// Uses the bundled default environment before the first prepare/render.
    pub const fn with_default_environment(mut self) -> Self {
        self.common.default_environment = true;
        self
    }

    /// Loads `path` as the environment before the first prepare/render. The
    /// asset loader resolves equirectangular HDR sources and the bundled
    /// neutral-studio fixture; any other format returns
    /// `AssetError::UnsupportedEnvironmentFormat`. Setting an explicit
    /// environment overrides any prior `with_default_environment()` call.
    pub fn with_environment(mut self, path: impl Into<AssetPath>) -> Self {
        self.common = self.common.with_environment(path);
        self
    }

    /// Uses a renderer profile when the headless renderer is created.
    pub const fn with_profile(mut self, profile: Profile) -> Self {
        self.common.renderer_options = self.common.renderer_options.with_profile(profile);
        self
    }

    /// Applies a named viewer profile as composable defaults for lighting,
    /// background, renderer profile, render mode, grid, and interaction styles.
    ///
    /// This remains a builder preset: it does not load assets, prepare, or
    /// render until the caller invokes [`Self::build`] or [`Self::render`].
    pub fn with_viewer_profile(mut self, profile: ViewerProfile) -> Self {
        self.common.apply_viewer_profile(profile);
        self
    }

    /// Uses a renderer quality level when the headless renderer is created.
    pub const fn with_quality(mut self, quality: Quality) -> Self {
        self.common.renderer_options = self.common.renderer_options.with_quality(quality);
        self
    }

    /// Uses an explicit render mode when the headless renderer is created.
    pub const fn with_render_mode(mut self, render_mode: RenderMode) -> Self {
        self.common.renderer_options = self.common.renderer_options.with_render_mode(render_mode);
        self
    }

    /// Requests the native headless GPU renderer for the first render. When no
    /// compatible GPU adapter is available, the builder falls back to the CPU
    /// headless renderer; inspect [`FirstRender::renderer`] capabilities to see
    /// which backend was actually used.
    pub const fn with_headless_gpu(mut self) -> Self {
        self.prefer_gpu = true;
        self
    }

    /// Applies a transform to the imported glTF roots immediately after
    /// instantiation and before optional framing, lighting, prepare, or render.
    pub const fn with_import_transform(mut self, transform: Transform) -> Self {
        self.common.import_transform = Some(transform);
        self
    }

    /// Sets the renderer clear color before the first prepare/render.
    pub const fn with_background_color(mut self, color: Color) -> Self {
        self.common.background = Some(Background::Custom(color));
        self
    }

    /// Configures the viewer for render-on-change loops.
    pub const fn on_change(self) -> Self {
        self.with_render_mode(RenderMode::OnChange)
    }

    /// Leaves the imported asset's camera framing unchanged.
    pub const fn without_framing(mut self) -> Self {
        self.common.frame_import = false;
        self
    }

    pub fn with_camera_bookmark(mut self, bookmark: CameraBookmark) -> Self {
        self.common.camera_bookmarks.push(bookmark);
        self
    }

    pub fn with_camera_bookmarks(
        mut self,
        bookmarks: impl IntoIterator<Item = CameraBookmark>,
    ) -> Self {
        self.common.camera_bookmarks.extend(bookmarks);
        self
    }

    /// Loads, instantiates, optionally frames/lights, and prepares a reusable viewer loop.
    pub async fn build(self) -> crate::Result<HeadlessGltfViewer> {
        self.build_with_progress(|_| {}).await
    }

    /// Loads, instantiates, optionally frames/lights, prepares, and renders one frame.
    pub async fn render(self) -> crate::Result<FirstRender> {
        self.render_with_progress(|_| {}).await
    }
}

impl HeadlessGltfViewer {
    /// Re-runs the explicit prepare step after scene, asset, renderer, or environment changes.
    pub fn prepare(&mut self) -> crate::Result<()> {
        self.renderer
            .prepare_with_assets(&mut self.scene, &self.assets)?;
        Ok(())
    }

    /// Renders the next frame using the active camera.
    pub fn render_next_frame(&mut self) -> crate::Result<RenderOutcome> {
        Ok(self.renderer.render_active(&self.scene)?)
    }

    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub fn import(&self) -> &SceneImport {
        &self.import
    }

    /// Returns the most recently rendered frame's interleaved RGBA8 bytes.
    /// Convenience for screenshots and visual-proof artifacts; equivalent
    /// to `viewer.renderer().frame_rgba8()`.
    pub fn snapshot_rgba8(&self) -> &[u8] {
        self.renderer.frame_rgba8()
    }

    /// Returns the renderer's capability snapshot. Forwards to the same
    /// `Capabilities` struct that callers can also reach via
    /// `viewer.renderer().capabilities()`.
    pub fn capabilities(&self) -> &crate::Capabilities {
        self.renderer.capabilities()
    }

    pub fn camera_bookmarks(&self) -> &[CameraBookmark] {
        &self.camera_bookmarks
    }
}

/// Owned interactive viewer state returned by [`InteractiveGltfViewerBuilder::build`].
///
/// Holds the loaded asset, scene, attached-surface renderer, the imported scene's typed
/// handle, and the active camera. The host owns the event loop and drives the viewer
/// through `handle_surface_event`, `prepare`, and `render_next_frame`. This is the
/// renderer-as-library shape: scena ships the placement glue (load → instantiate →
/// frame → light → environment → prepare) but never owns the application's event loop,
/// matching the public-API non-goal that scena does not replace winit / wasm-bindgen
/// host loops.
pub struct InteractiveGltfViewer {
    assets: Assets,
    scene: Scene,
    renderer: Renderer,
    import: SceneImport,
    camera: CameraKey,
    load_progress_events: Vec<AssetLoadProgress>,
    camera_bookmarks: Vec<CameraBookmark>,
    /// Phase 5B step 2: optional orbit-camera controller. Populated when
    /// the builder was configured with `with_orbit_controls()`. Pointer +
    /// touch events route through `handle_pointer_event` /
    /// `handle_touch_event`; the controller applies the resulting
    /// transform to the active camera.
    orbit_controls: Option<OrbitControls>,
    click_callback: Option<ViewerPickCallback>,
    hover_callback: Option<ViewerPickCallback>,
}

impl std::fmt::Debug for InteractiveGltfViewer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InteractiveGltfViewer")
            .field("assets", &self.assets)
            .field("scene", &self.scene)
            .field("renderer", &self.renderer)
            .field("import", &self.import)
            .field("camera", &self.camera)
            .field("load_progress_events", &self.load_progress_events)
            .field("camera_bookmarks", &self.camera_bookmarks)
            .field("orbit_controls", &self.orbit_controls)
            .field("click_callback_registered", &self.click_callback.is_some())
            .field("hover_callback_registered", &self.hover_callback.is_some())
            .finish()
    }
}

/// Builder for [`interactive_gltf_viewer`].
#[derive(Debug)]
pub struct InteractiveGltfViewerBuilder {
    path: AssetPath,
    surface: PlatformSurface,
    orbit_controls: bool,
    common: ViewerCommonOptions,
}

/// Starts a fluent interactive glTF viewer setup against an attached surface.
///
/// The surface argument can be a native window descriptor, a browser canvas, or a
/// surface descriptor - whatever [`PlatformSurface`] constructor matches the host.
/// Use [`InteractiveGltfViewerBuilder::build`] for native/descriptor surfaces and
/// [`InteractiveGltfViewerBuilder::build_async`] for browser surfaces (which require
/// async wgpu adapter discovery).
pub fn interactive_gltf_viewer(
    path: impl Into<AssetPath>,
    surface: PlatformSurface,
) -> InteractiveGltfViewerBuilder {
    InteractiveGltfViewerBuilder {
        path: path.into(),
        surface,
        orbit_controls: false,
        common: ViewerCommonOptions::new(),
    }
}

impl InteractiveGltfViewerBuilder {
    /// Adds a neutral directional light before the first prepare/render.
    pub const fn with_default_light(mut self) -> Self {
        self.common.lighting = ViewerProfileLighting::Directional;
        self
    }

    /// Uses the bundled default environment before the first prepare/render.
    pub const fn with_default_environment(mut self) -> Self {
        self.common.default_environment = true;
        self
    }

    /// Loads `path` as the environment before the first prepare/render.
    /// Mirrors `HeadlessGltfViewerBuilder::with_environment`; setting an
    /// explicit path overrides any prior `with_default_environment()` call.
    pub fn with_environment(mut self, path: impl Into<AssetPath>) -> Self {
        self.common = self.common.with_environment(path);
        self
    }

    /// Phase 5B step 2: attaches an `OrbitControls` instance derived from
    /// the imported scene's bounds and the framed camera position. Call
    /// sites route input through `InteractiveGltfViewer::handle_pointer_event`
    /// / `handle_touch_event` to apply orbit/pan/zoom to the active camera
    /// without piercing the renderer or scene.
    pub const fn with_orbit_controls(mut self) -> Self {
        self.orbit_controls = true;
        self
    }

    /// Uses a renderer profile when the renderer is created.
    pub const fn with_profile(mut self, profile: Profile) -> Self {
        self.common.renderer_options = self.common.renderer_options.with_profile(profile);
        self
    }

    /// Applies a named viewer profile as composable defaults for lighting,
    /// background, renderer profile, render mode, grid, interaction styles, and
    /// optional orbit controls.
    ///
    /// The profile does not own the host event loop. Pointer and touch input
    /// still flow through [`InteractiveGltfViewer::handle_pointer_event`] and
    /// [`InteractiveGltfViewer::handle_touch_event`].
    pub fn with_viewer_profile(mut self, profile: ViewerProfile) -> Self {
        self.orbit_controls = self.orbit_controls || profile.orbit_controls();
        self.common.apply_viewer_profile(profile);
        self
    }

    /// Uses a renderer quality level when the renderer is created.
    pub const fn with_quality(mut self, quality: Quality) -> Self {
        self.common.renderer_options = self.common.renderer_options.with_quality(quality);
        self
    }

    /// Uses an explicit render mode when the renderer is created.
    pub const fn with_render_mode(mut self, render_mode: RenderMode) -> Self {
        self.common.renderer_options = self.common.renderer_options.with_render_mode(render_mode);
        self
    }

    /// Applies a transform to the imported glTF roots immediately after
    /// instantiation and before optional framing, lighting, prepare, or render.
    pub const fn with_import_transform(mut self, transform: Transform) -> Self {
        self.common.import_transform = Some(transform);
        self
    }

    /// Sets the renderer clear color before the first prepare/render.
    pub const fn with_background_color(mut self, color: Color) -> Self {
        self.common.background = Some(Background::Custom(color));
        self
    }

    /// Configures the viewer for render-on-change loops.
    pub const fn on_change(self) -> Self {
        self.with_render_mode(RenderMode::OnChange)
    }

    /// Leaves the imported asset's camera framing unchanged.
    pub const fn without_framing(mut self) -> Self {
        self.common.frame_import = false;
        self
    }

    pub fn with_camera_bookmark(mut self, bookmark: CameraBookmark) -> Self {
        self.common.camera_bookmarks.push(bookmark);
        self
    }

    pub fn with_camera_bookmarks(
        mut self,
        bookmarks: impl IntoIterator<Item = CameraBookmark>,
    ) -> Self {
        self.common.camera_bookmarks.extend(bookmarks);
        self
    }

    /// Synchronously builds the interactive viewer. Use this for native window
    /// surfaces and surface descriptors. Browser surfaces require async wgpu
    /// adapter discovery; call [`Self::build_async`] for those. Gated on
    /// non-wasm32 targets because the sync build path uses `pollster::block_on`,
    /// which is incompatible with the browser event loop.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build(self) -> crate::Result<InteractiveGltfViewer> {
        self.build_with_progress(|_| {})
    }

    /// Async build path that supports browser-canvas surfaces.
    pub async fn build_async(self) -> crate::Result<InteractiveGltfViewer> {
        self.build_async_with_progress(|_| {}).await
    }
}

/// Phase 5B step 2: derives the initial OrbitControls transform from the
/// imported scene's bounds and the framed camera position. Called by both
/// the sync and async build paths so the controller starts at exactly the
/// distance/target combination that `frame_import` placed the camera at;
/// the first orbit/zoom delta therefore composes correctly with the
/// initial framing.
fn build_orbit_controls(
    enabled: bool,
    scene: &Scene,
    import: &SceneImport,
    camera: CameraKey,
) -> Option<OrbitControls> {
    if !enabled {
        return None;
    }
    let bounds = import.bounds_world(scene);
    let target = bounds.map(|aabb| aabb.center()).unwrap_or(Vec3::ZERO);
    let distance = scene
        .camera_node(camera)
        .and_then(|node| scene.world_transform(node))
        .map(|transform| {
            let dx = transform.translation.x - target.x;
            let dy = transform.translation.y - target.y;
            let dz = transform.translation.z - target.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .filter(|distance| distance.is_finite() && *distance > 0.0)
        .unwrap_or(2.0);
    Some(OrbitControls::new(target, distance))
}

impl InteractiveGltfViewer {
    /// Forwards a host platform-surface event (resize, lost, recovered) to the renderer.
    pub fn handle_surface_event(&mut self, event: SurfaceEvent) -> crate::Result<()> {
        self.renderer.handle_surface_event(event)?;
        Ok(())
    }

    /// Re-runs prepare with the current scene + assets. Call after scene or asset edits.
    pub fn prepare(&mut self) -> crate::Result<()> {
        self.renderer
            .prepare_with_assets(&mut self.scene, &self.assets)?;
        Ok(())
    }

    /// Renders the next frame using the active camera.
    pub fn render_next_frame(&mut self) -> crate::Result<RenderOutcome> {
        Ok(self.renderer.render_active(&self.scene)?)
    }

    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    pub fn import(&self) -> &SceneImport {
        &self.import
    }

    pub fn camera(&self) -> CameraKey {
        self.camera
    }

    pub fn orbit_controls(&self) -> Option<&OrbitControls> {
        self.orbit_controls.as_ref()
    }

    /// Renderer diagnostics emitted during prepare or render.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.renderer.diagnostics().to_vec()
    }

    /// Returns the most recently rendered frame's interleaved RGBA8 bytes.
    /// Convenience for screenshots and visual-proof artifacts; equivalent
    /// to `viewer.renderer().frame_rgba8()`.
    pub fn snapshot_rgba8(&self) -> &[u8] {
        self.renderer.frame_rgba8()
    }

    /// Returns the renderer's capability snapshot. Forwards to the same
    /// `Capabilities` struct that callers can also reach via
    /// `viewer.renderer().capabilities()`.
    pub fn capabilities(&self) -> &crate::Capabilities {
        self.renderer.capabilities()
    }

    pub fn camera_bookmarks(&self) -> &[CameraBookmark] {
        &self.camera_bookmarks
    }

    /// Phase 5B step 2: routes a pointer event through the attached
    /// `OrbitControls` (if any). When the controller reports a non-`None`
    /// action, the resulting camera transform is applied to the active
    /// scene camera. Returns the action so the host loop can react (e.g.
    /// flip the renderer to render-on-change for idle frames after `End`).
    /// When no controller is attached, returns `OrbitControlAction::None`.
    pub fn handle_pointer_event(
        &mut self,
        event: PointerEvent,
    ) -> Result<OrbitControlAction, LookupError> {
        let Some(orbit_controls) = self.orbit_controls.as_mut() else {
            return Ok(OrbitControlAction::None);
        };
        let action = orbit_controls.handle_pointer(event);
        if !matches!(action, OrbitControlAction::None) {
            orbit_controls.apply_to_scene(&mut self.scene, self.camera)?;
        }
        Ok(action)
    }

    /// Phase 5B step 2: touch-event mirror of `handle_pointer_event`.
    pub fn handle_touch_event(
        &mut self,
        event: TouchEvent,
    ) -> Result<OrbitControlAction, LookupError> {
        let Some(orbit_controls) = self.orbit_controls.as_mut() else {
            return Ok(OrbitControlAction::None);
        };
        let action = orbit_controls.handle_touch(event);
        if !matches!(action, OrbitControlAction::None) {
            orbit_controls.apply_to_scene(&mut self.scene, self.camera)?;
        }
        Ok(action)
    }
}

/// Load a glTF/GLB scene, instantiate it, frame it, prepare it, and render one headless frame.
///
/// This is a convenience orchestration API for examples, tests, and first viewer setup. It
/// keeps ownership explicit: assets stay in [`Assets`], scene graph state stays in [`Scene`],
/// and the renderer only prepares and renders already-loaded scene state.
pub async fn first_render_gltf_headless(
    path: impl Into<AssetPath>,
    width: u32,
    height: u32,
) -> crate::Result<FirstRender> {
    headless_gltf_viewer(path)
        .size(width, height)
        .render()
        .await
}
