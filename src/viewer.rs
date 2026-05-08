//! High-level viewer helpers built from `Scene`, `Assets`, and `Renderer`.

use crate::assets::{AssetPath, Assets};
use crate::diagnostics::{Diagnostic, RenderOutcome};
use crate::render::{Profile, Quality, RenderMode, Renderer, RendererOptions};
use crate::scene::{DirectionalLight, Scene, SceneImport};

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
        if self.default_environment {
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
