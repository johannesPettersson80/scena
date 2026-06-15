use crate::assets::{AssetLoadProgress, Assets};
use crate::render::{Background, Renderer};
use crate::scene::{DirectionalLight, GridFloorOptions, Scene, SceneImport, Transform};

use super::{
    FirstRender, HeadlessGltfViewer, HeadlessGltfViewerBuilder, InteractiveGltfViewer,
    InteractiveGltfViewerBuilder, ViewerProfileLighting, build_orbit_controls,
};

impl FirstRender {
    /// Asset loading progress observed while building this first render.
    pub fn load_progress_events(&self) -> &[AssetLoadProgress] {
        &self.load_progress_events
    }
}

impl HeadlessGltfViewerBuilder {
    /// Loads and prepares a reusable viewer loop while reporting asset progress.
    pub async fn build_with_progress<P>(self, progress: P) -> crate::Result<HeadlessGltfViewer>
    where
        P: FnMut(AssetLoadProgress),
    {
        let assets = Assets::new();
        let scene_report = assets.load_scene_with_progress(self.path, progress).await?;
        let load_progress_events = scene_report.progress_events().to_vec();
        let scene_asset = scene_report.into_asset();
        let mut scene = Scene::new();
        let import = scene.instantiate(&scene_asset)?;
        apply_import_transform(&mut scene, &import, self.common.import_transform)?;
        let camera = scene.add_default_camera()?;
        if self.common.frame_import {
            scene.frame_import(camera, &import)?;
        }
        apply_viewer_grid(&mut scene, &assets, &import, self.common.grid_floor)?;
        apply_viewer_lighting(&mut scene, self.common.lighting)?;

        let mut renderer =
            Renderer::headless_with_options(self.width, self.height, self.common.renderer_options)?;
        apply_viewer_renderer_settings(
            &mut renderer,
            self.common.background,
            self.common.hover_style,
            self.common.selection_style,
        );
        if let Some(environment_path) = self.common.environment_path {
            let environment = assets.load_environment(environment_path).await?;
            renderer.set_environment(environment);
        } else if self.common.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;

        Ok(HeadlessGltfViewer {
            assets,
            scene,
            renderer,
            import,
            load_progress_events,
            camera_bookmarks: self.common.camera_bookmarks,
        })
    }

    /// Loads, prepares, and renders one frame while reporting asset progress.
    pub async fn render_with_progress<P>(self, progress: P) -> crate::Result<FirstRender>
    where
        P: FnMut(AssetLoadProgress),
    {
        let mut viewer = self.build_with_progress(progress).await?;
        let outcome = viewer.render_next_frame()?;
        let diagnostics = viewer.renderer.diagnostics().to_vec();
        let HeadlessGltfViewer {
            assets,
            scene,
            renderer,
            import,
            load_progress_events,
            camera_bookmarks,
        } = viewer;

        Ok(FirstRender {
            assets,
            scene,
            renderer,
            import,
            outcome,
            diagnostics,
            load_progress_events,
            camera_bookmarks,
        })
    }
}

impl HeadlessGltfViewer {
    /// Asset loading progress observed while building this viewer.
    pub fn load_progress_events(&self) -> &[AssetLoadProgress] {
        &self.load_progress_events
    }
}

impl InteractiveGltfViewerBuilder {
    /// Synchronously builds the interactive viewer while reporting asset progress.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_with_progress<P>(self, progress: P) -> crate::Result<InteractiveGltfViewer>
    where
        P: FnMut(AssetLoadProgress),
    {
        let assets = Assets::new();
        let scene_report =
            pollster::block_on(assets.load_scene_with_progress(self.path.clone(), progress))?;
        let load_progress_events = scene_report.progress_events().to_vec();
        let scene_asset = scene_report.into_asset();
        let mut scene = Scene::new();
        let import = scene.instantiate(&scene_asset)?;
        apply_import_transform(&mut scene, &import, self.common.import_transform)?;
        let camera = scene.add_default_camera()?;
        if self.common.frame_import {
            scene.frame_import(camera, &import)?;
        }
        apply_viewer_grid(&mut scene, &assets, &import, self.common.grid_floor)?;
        apply_viewer_lighting(&mut scene, self.common.lighting)?;
        let mut renderer =
            Renderer::from_surface_with_options(self.surface, self.common.renderer_options)?;
        apply_viewer_renderer_settings(
            &mut renderer,
            self.common.background,
            self.common.hover_style,
            self.common.selection_style,
        );
        if let Some(environment_path) = self.common.environment_path {
            let environment = pollster::block_on(assets.load_environment(environment_path))?;
            renderer.set_environment(environment);
        } else if self.common.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;
        let orbit_controls = build_orbit_controls(self.orbit_controls, &scene, &import, camera);
        Ok(InteractiveGltfViewer {
            assets,
            scene,
            renderer,
            import,
            camera,
            load_progress_events,
            camera_bookmarks: self.common.camera_bookmarks,
            orbit_controls,
            click_callback: None,
            hover_callback: None,
        })
    }

    /// Async build path that supports browser-canvas surfaces and reports asset progress.
    pub async fn build_async_with_progress<P>(
        self,
        progress: P,
    ) -> crate::Result<InteractiveGltfViewer>
    where
        P: FnMut(AssetLoadProgress),
    {
        let assets = Assets::new();
        let scene_report = assets
            .load_scene_with_progress(self.path.clone(), progress)
            .await?;
        let load_progress_events = scene_report.progress_events().to_vec();
        let scene_asset = scene_report.into_asset();
        let mut scene = Scene::new();
        let import = scene.instantiate(&scene_asset)?;
        apply_import_transform(&mut scene, &import, self.common.import_transform)?;
        let camera = scene.add_default_camera()?;
        if self.common.frame_import {
            scene.frame_import(camera, &import)?;
        }
        apply_viewer_grid(&mut scene, &assets, &import, self.common.grid_floor)?;
        apply_viewer_lighting(&mut scene, self.common.lighting)?;
        let mut renderer =
            Renderer::from_surface_async_with_options(self.surface, self.common.renderer_options)
                .await?;
        apply_viewer_renderer_settings(
            &mut renderer,
            self.common.background,
            self.common.hover_style,
            self.common.selection_style,
        );
        if let Some(environment_path) = self.common.environment_path {
            let environment = assets.load_environment(environment_path).await?;
            renderer.set_environment(environment);
        } else if self.common.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;
        let orbit_controls = build_orbit_controls(self.orbit_controls, &scene, &import, camera);
        Ok(InteractiveGltfViewer {
            assets,
            scene,
            renderer,
            import,
            camera,
            load_progress_events,
            camera_bookmarks: self.common.camera_bookmarks,
            orbit_controls,
            click_callback: None,
            hover_callback: None,
        })
    }
}

fn apply_import_transform(
    scene: &mut Scene,
    import: &SceneImport,
    transform: Option<Transform>,
) -> crate::Result<()> {
    let Some(transform) = transform else {
        return Ok(());
    };
    for root in import.roots() {
        scene.set_transform(*root, transform)?;
    }
    Ok(())
}

fn apply_viewer_lighting(scene: &mut Scene, lighting: ViewerProfileLighting) -> crate::Result<()> {
    match lighting {
        ViewerProfileLighting::None => {}
        ViewerProfileLighting::Directional => {
            scene.directional_light(DirectionalLight::default()).add()?;
        }
        ViewerProfileLighting::Studio => {
            scene.add_studio_lighting()?;
        }
    }
    Ok(())
}

fn apply_viewer_grid<F>(
    scene: &mut Scene,
    assets: &crate::assets::Assets<F>,
    import: &SceneImport,
    enabled: bool,
) -> crate::Result<()> {
    if !enabled {
        return Ok(());
    }
    let bounds = import
        .bounds_world(scene)
        .ok_or(crate::LookupError::ImportHasNoBounds)?;
    scene.add_grid_floor(assets, GridFloorOptions::new().under_bounds(bounds))?;
    Ok(())
}

fn apply_viewer_renderer_settings(
    renderer: &mut Renderer,
    background: Option<Background>,
    hover_style: Option<crate::InteractionStyle>,
    selection_style: Option<crate::InteractionStyle>,
) {
    if let Some(background) = background {
        renderer.set_background(background);
    }
    if let Some(style) = hover_style {
        renderer.set_hover_style(style);
    }
    if let Some(style) = selection_style {
        renderer.set_selection_style(style);
    }
}

impl InteractiveGltfViewer {
    /// Asset loading progress observed while building this viewer.
    pub fn load_progress_events(&self) -> &[AssetLoadProgress] {
        &self.load_progress_events
    }
}
