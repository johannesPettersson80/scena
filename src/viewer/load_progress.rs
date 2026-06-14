use crate::assets::{AssetLoadProgress, Assets};
use crate::render::Renderer;
use crate::scene::{DirectionalLight, Scene, SceneImport, Transform};

use super::{
    FirstRender, HeadlessGltfViewer, HeadlessGltfViewerBuilder, InteractiveGltfViewer,
    InteractiveGltfViewerBuilder, build_orbit_controls,
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
        if self.common.default_light {
            scene.directional_light(DirectionalLight::default()).add()?;
        }

        let mut renderer =
            Renderer::headless_with_options(self.width, self.height, self.common.renderer_options)?;
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
        } = viewer;

        Ok(FirstRender {
            assets,
            scene,
            renderer,
            import,
            outcome,
            diagnostics,
            load_progress_events,
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
        if self.common.default_light {
            scene.directional_light(DirectionalLight::default()).add()?;
        }
        let mut renderer =
            Renderer::from_surface_with_options(self.surface, self.common.renderer_options)?;
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
        if self.common.default_light {
            scene.directional_light(DirectionalLight::default()).add()?;
        }
        let mut renderer =
            Renderer::from_surface_async_with_options(self.surface, self.common.renderer_options)
                .await?;
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

impl InteractiveGltfViewer {
    /// Asset loading progress observed while building this viewer.
    pub fn load_progress_events(&self) -> &[AssetLoadProgress] {
        &self.load_progress_events
    }
}
