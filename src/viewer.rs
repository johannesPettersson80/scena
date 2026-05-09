//! High-level viewer helpers built from `Scene`, `Assets`, and `Renderer`.

use crate::assets::{AssetPath, Assets};
use crate::diagnostics::{Diagnostic, RenderOutcome};
use crate::platform::{PlatformSurface, SurfaceEvent};
use crate::render::{Profile, Quality, RenderMode, Renderer, RendererOptions};
use crate::scene::{CameraKey, DirectionalLight, Scene, SceneImport};

/// Owned state returned by [`first_render_gltf_headless`].
#[derive(Debug)]
pub struct FirstRender {
    pub assets: Assets,
    pub scene: Scene,
    pub renderer: Renderer,
    pub import: SceneImport,
    pub outcome: RenderOutcome,
    pub diagnostics: Vec<Diagnostic>,
}

/// Prepared owned state for a headless glTF viewer loop.
#[derive(Debug)]
pub struct HeadlessGltfViewer {
    pub assets: Assets,
    pub scene: Scene,
    pub renderer: Renderer,
    pub import: SceneImport,
}

/// Builder for the first headless glTF render.
#[derive(Debug, Clone)]
pub struct HeadlessGltfViewerBuilder {
    path: AssetPath,
    width: u32,
    height: u32,
    frame_import: bool,
    default_light: bool,
    default_environment: bool,
    environment_path: Option<AssetPath>,
    renderer_options: RendererOptions,
}

/// Starts a fluent headless glTF viewer setup.
pub fn headless_gltf_viewer(path: impl Into<AssetPath>) -> HeadlessGltfViewerBuilder {
    HeadlessGltfViewerBuilder {
        path: path.into(),
        width: 800,
        height: 600,
        frame_import: true,
        default_light: false,
        default_environment: false,
        environment_path: None,
        renderer_options: RendererOptions::default(),
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
        self.default_light = true;
        self
    }

    /// Uses the bundled default environment before the first prepare/render.
    pub const fn with_default_environment(mut self) -> Self {
        self.default_environment = true;
        self
    }

    /// Loads `path` as the environment before the first prepare/render. The
    /// asset loader resolves equirectangular HDR sources and the bundled
    /// neutral-studio fixture; any other format returns
    /// `AssetError::UnsupportedEnvironmentFormat`. Setting an explicit
    /// environment overrides any prior `with_default_environment()` call.
    pub fn with_environment(mut self, path: impl Into<AssetPath>) -> Self {
        self.environment_path = Some(path.into());
        self.default_environment = false;
        self
    }

    /// Uses a renderer profile when the headless renderer is created.
    pub const fn with_profile(mut self, profile: Profile) -> Self {
        self.renderer_options = self.renderer_options.with_profile(profile);
        self
    }

    /// Uses a renderer quality level when the headless renderer is created.
    pub const fn with_quality(mut self, quality: Quality) -> Self {
        self.renderer_options = self.renderer_options.with_quality(quality);
        self
    }

    /// Uses an explicit render mode when the headless renderer is created.
    pub const fn with_render_mode(mut self, render_mode: RenderMode) -> Self {
        self.renderer_options = self.renderer_options.with_render_mode(render_mode);
        self
    }

    /// Configures the viewer for render-on-change loops.
    pub const fn on_change(self) -> Self {
        self.with_render_mode(RenderMode::OnChange)
    }

    /// Leaves the imported asset's camera framing unchanged.
    pub const fn without_framing(mut self) -> Self {
        self.frame_import = false;
        self
    }

    /// Loads, instantiates, optionally frames/lights, and prepares a reusable viewer loop.
    pub async fn build(self) -> crate::Result<HeadlessGltfViewer> {
        let assets = Assets::new();
        let scene_asset = assets.load_scene(self.path).await?;
        let mut scene = Scene::new();
        let import = scene.instantiate(&scene_asset)?;
        let camera = scene.add_default_camera()?;
        if self.frame_import {
            scene.frame_import(camera, &import)?;
        }
        if self.default_light {
            scene.directional_light(DirectionalLight::default()).add()?;
        }

        let mut renderer =
            Renderer::headless_with_options(self.width, self.height, self.renderer_options)?;
        if let Some(environment_path) = self.environment_path {
            let environment = assets.load_environment(environment_path).await?;
            renderer.set_environment(environment);
        } else if self.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;

        Ok(HeadlessGltfViewer {
            assets,
            scene,
            renderer,
            import,
        })
    }

    /// Loads, instantiates, optionally frames/lights, prepares, and renders one frame.
    pub async fn render(self) -> crate::Result<FirstRender> {
        let mut viewer = self.build().await?;
        let outcome = viewer.render_next_frame()?;
        let diagnostics = viewer.renderer.diagnostics().to_vec();
        let HeadlessGltfViewer {
            assets,
            scene,
            renderer,
            import,
        } = viewer;

        Ok(FirstRender {
            assets,
            scene,
            renderer,
            import,
            outcome,
            diagnostics,
        })
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
#[derive(Debug)]
pub struct InteractiveGltfViewer {
    pub assets: Assets,
    pub scene: Scene,
    pub renderer: Renderer,
    pub import: SceneImport,
    pub camera: CameraKey,
}

/// Builder for [`interactive_gltf_viewer`].
#[derive(Debug)]
pub struct InteractiveGltfViewerBuilder {
    path: AssetPath,
    surface: PlatformSurface,
    frame_import: bool,
    default_light: bool,
    default_environment: bool,
    environment_path: Option<AssetPath>,
    renderer_options: RendererOptions,
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
        frame_import: true,
        default_light: false,
        default_environment: false,
        environment_path: None,
        renderer_options: RendererOptions::default(),
    }
}

impl InteractiveGltfViewerBuilder {
    /// Adds a neutral directional light before the first prepare/render.
    pub const fn with_default_light(mut self) -> Self {
        self.default_light = true;
        self
    }

    /// Uses the bundled default environment before the first prepare/render.
    pub const fn with_default_environment(mut self) -> Self {
        self.default_environment = true;
        self
    }

    /// Loads `path` as the environment before the first prepare/render.
    /// Mirrors `HeadlessGltfViewerBuilder::with_environment`; setting an
    /// explicit path overrides any prior `with_default_environment()` call.
    pub fn with_environment(mut self, path: impl Into<AssetPath>) -> Self {
        self.environment_path = Some(path.into());
        self.default_environment = false;
        self
    }

    /// Uses a renderer profile when the renderer is created.
    pub const fn with_profile(mut self, profile: Profile) -> Self {
        self.renderer_options = self.renderer_options.with_profile(profile);
        self
    }

    /// Uses a renderer quality level when the renderer is created.
    pub const fn with_quality(mut self, quality: Quality) -> Self {
        self.renderer_options = self.renderer_options.with_quality(quality);
        self
    }

    /// Uses an explicit render mode when the renderer is created.
    pub const fn with_render_mode(mut self, render_mode: RenderMode) -> Self {
        self.renderer_options = self.renderer_options.with_render_mode(render_mode);
        self
    }

    /// Configures the viewer for render-on-change loops.
    pub const fn on_change(self) -> Self {
        self.with_render_mode(RenderMode::OnChange)
    }

    /// Leaves the imported asset's camera framing unchanged.
    pub const fn without_framing(mut self) -> Self {
        self.frame_import = false;
        self
    }

    /// Synchronously builds the interactive viewer. Use this for native window
    /// surfaces and surface descriptors. Browser surfaces require async wgpu
    /// adapter discovery; call [`Self::build_async`] for those. Gated on
    /// non-wasm32 targets because the sync build path uses `pollster::block_on`,
    /// which is incompatible with the browser event loop.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build(self) -> crate::Result<InteractiveGltfViewer> {
        let assets = Assets::new();
        let scene_asset = pollster::block_on(assets.load_scene(self.path.clone()))?;
        let mut scene = Scene::new();
        let import = scene.instantiate(&scene_asset)?;
        let camera = scene.add_default_camera()?;
        if self.frame_import {
            scene.frame_import(camera, &import)?;
        }
        if self.default_light {
            scene.directional_light(DirectionalLight::default()).add()?;
        }
        let mut renderer =
            Renderer::from_surface_with_options(self.surface, self.renderer_options)?;
        if let Some(environment_path) = self.environment_path {
            let environment = pollster::block_on(assets.load_environment(environment_path))?;
            renderer.set_environment(environment);
        } else if self.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;
        Ok(InteractiveGltfViewer {
            assets,
            scene,
            renderer,
            import,
            camera,
        })
    }

    /// Async build path that supports browser-canvas surfaces.
    pub async fn build_async(self) -> crate::Result<InteractiveGltfViewer> {
        let assets = Assets::new();
        let scene_asset = assets.load_scene(self.path.clone()).await?;
        let mut scene = Scene::new();
        let import = scene.instantiate(&scene_asset)?;
        let camera = scene.add_default_camera()?;
        if self.frame_import {
            scene.frame_import(camera, &import)?;
        }
        if self.default_light {
            scene.directional_light(DirectionalLight::default()).add()?;
        }
        let mut renderer =
            Renderer::from_surface_async_with_options(self.surface, self.renderer_options).await?;
        if let Some(environment_path) = self.environment_path {
            let environment = assets.load_environment(environment_path).await?;
            renderer.set_environment(environment);
        } else if self.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;
        Ok(InteractiveGltfViewer {
            assets,
            scene,
            renderer,
            import,
            camera,
        })
    }
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
