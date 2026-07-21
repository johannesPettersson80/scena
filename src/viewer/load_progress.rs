use crate::assets::{AssetLoadProgress, Assets};
use crate::render::{Background, Renderer};
use crate::scene::{
    DirectionalLight, FramingOptions, GridFloorOptions, Scene, SceneImport, Transform,
};

use super::interaction::build_orbit_controls;
use super::{
    FirstRender, HeadlessGltfViewer, HeadlessGltfViewerBuilder, InteractiveGltfViewer,
    InteractiveGltfViewerBuilder, ViewerProfileLighting,
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
            scene.frame_import_with_options(
                camera,
                &import,
                FramingOptions::new()
                    .three_quarter_front_right()
                    .viewport(self.width, self.height),
            )?;
        }
        apply_viewer_grid(&mut scene, &assets, &import, self.common.grid_floor)?;
        let setup_diagnostics = apply_viewer_lighting(
            &mut scene,
            self.common.lighting,
            self.common.fallback_lighting,
            self.common.environment_path.is_some() || self.common.default_environment,
        )?;

        let (mut renderer, backend_selection_report) = match self.backend_policy {
            super::HeadlessBackendPolicy::Cpu => (
                Renderer::headless_with_options(
                    self.width,
                    self.height,
                    self.common.renderer_options,
                )?,
                None,
            ),
            super::HeadlessBackendPolicy::StrictGpu => (
                Renderer::headless_gpu_with_options(
                    self.width,
                    self.height,
                    self.common.renderer_options,
                )?,
                Some(crate::HeadlessBackendSelectionReport::gpu()),
            ),
            super::HeadlessBackendPolicy::PreferGpu => {
                match Renderer::headless_gpu_with_options(
                    self.width,
                    self.height,
                    self.common.renderer_options,
                ) {
                    Ok(renderer) => (renderer, Some(crate::HeadlessBackendSelectionReport::gpu())),
                    Err(gpu_error) => (
                        Renderer::headless_with_options(
                            self.width,
                            self.height,
                            self.common.renderer_options,
                        )?,
                        Some(crate::HeadlessBackendSelectionReport::cpu_fallback(
                            gpu_error,
                        )),
                    ),
                }
            }
        };
        apply_viewer_renderer_settings(&mut renderer, self.common.background);
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
            backend_selection_report,
            import,
            load_progress_events,
            camera_bookmarks: self.common.camera_bookmarks,
            setup_diagnostics,
        })
    }

    /// Loads, prepares, and renders one frame while reporting asset progress.
    pub async fn render_with_progress<P>(self, progress: P) -> crate::Result<FirstRender>
    where
        P: FnMut(AssetLoadProgress),
    {
        let mut viewer = self.build_with_progress(progress).await?;
        let outcome = viewer.render_next_frame()?;
        let diagnostics = viewer.diagnostics();
        let HeadlessGltfViewer {
            assets,
            scene,
            renderer,
            backend_selection_report,
            import,
            load_progress_events,
            camera_bookmarks,
            setup_diagnostics: _,
        } = viewer;

        Ok(FirstRender {
            assets,
            scene,
            renderer,
            backend_selection_report,
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

    /// Returns GPU selection evidence when a GPU policy was requested.
    pub const fn backend_selection_report(&self) -> Option<&crate::HeadlessBackendSelectionReport> {
        self.backend_selection_report.as_ref()
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
        let surface_size = self.surface.size();
        let camera = scene.add_default_camera()?;
        let framing = if self.common.frame_import {
            Some(
                scene.frame_import_with_options(
                    camera,
                    &import,
                    FramingOptions::new()
                        .three_quarter_front_right()
                        .viewport(surface_size.width, surface_size.height),
                )?,
            )
        } else {
            None
        };
        apply_viewer_grid(&mut scene, &assets, &import, self.common.grid_floor)?;
        let setup_diagnostics = apply_viewer_lighting(
            &mut scene,
            self.common.lighting,
            self.common.fallback_lighting,
            self.common.environment_path.is_some() || self.common.default_environment,
        )?;
        let mut renderer =
            Renderer::from_surface_with_options(self.surface, self.common.renderer_options)?;
        apply_viewer_renderer_settings(&mut renderer, self.common.background);
        if let Some(environment_path) = self.common.environment_path {
            let environment = pollster::block_on(assets.load_environment(environment_path))?;
            renderer.set_environment(environment);
        } else if self.common.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;
        let orbit_controls =
            build_orbit_controls(self.orbit_controls, &scene, &import, camera, framing);
        Ok(InteractiveGltfViewer {
            assets,
            scene,
            renderer,
            import,
            camera,
            load_progress_events,
            camera_bookmarks: self.common.camera_bookmarks,
            setup_diagnostics,
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
        let surface_size = self.surface.size();
        let camera = scene.add_default_camera()?;
        let framing = if self.common.frame_import {
            Some(
                scene.frame_import_with_options(
                    camera,
                    &import,
                    FramingOptions::new()
                        .three_quarter_front_right()
                        .viewport(surface_size.width, surface_size.height),
                )?,
            )
        } else {
            None
        };
        apply_viewer_grid(&mut scene, &assets, &import, self.common.grid_floor)?;
        let setup_diagnostics = apply_viewer_lighting(
            &mut scene,
            self.common.lighting,
            self.common.fallback_lighting,
            self.common.environment_path.is_some() || self.common.default_environment,
        )?;
        let mut renderer =
            Renderer::from_surface_async_with_options(self.surface, self.common.renderer_options)
                .await?;
        apply_viewer_renderer_settings(&mut renderer, self.common.background);
        if let Some(environment_path) = self.common.environment_path {
            let environment = assets.load_environment(environment_path).await?;
            renderer.set_environment(environment);
        } else if self.common.default_environment {
            renderer.set_environment(assets.default_environment());
        }
        renderer.prepare_with_assets(&mut scene, &assets)?;
        let orbit_controls =
            build_orbit_controls(self.orbit_controls, &scene, &import, camera, framing);
        Ok(InteractiveGltfViewer {
            assets,
            scene,
            renderer,
            import,
            camera,
            load_progress_events,
            camera_bookmarks: self.common.camera_bookmarks,
            setup_diagnostics,
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

fn apply_viewer_lighting(
    scene: &mut Scene,
    lighting: ViewerProfileLighting,
    fallback_lighting: bool,
    environment_configured: bool,
) -> crate::Result<Vec<crate::Diagnostic>> {
    let mut diagnostics = Vec::new();
    match lighting {
        ViewerProfileLighting::None => {
            if fallback_lighting && !environment_configured && scene.light_nodes().next().is_none()
            {
                scene.directional_light(DirectionalLight::default()).add()?;
                diagnostics.push(
                    crate::Diagnostic::warning(
                        crate::DiagnosticCode::MissingLightingOrEnvironment,
                        "scene had no authored lighting or environment; the viewer applied a neutral directional fallback",
                        "author a scene light/environment or configure the viewer explicitly to replace the fallback",
                    )
                    .with_applied_fallback("viewer.lighting"),
                );
            }
        }
        ViewerProfileLighting::Directional => {
            scene.directional_light(DirectionalLight::default()).add()?;
        }
        ViewerProfileLighting::Studio => {
            scene.add_studio_lighting()?;
        }
    }
    Ok(diagnostics)
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

fn apply_viewer_renderer_settings(renderer: &mut Renderer, background: Option<Background>) {
    if let Some(background) = background {
        renderer.set_background(background);
    }
}

impl InteractiveGltfViewer {
    /// Asset loading progress observed while building this viewer.
    pub fn load_progress_events(&self) -> &[AssetLoadProgress] {
        &self.load_progress_events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_viewer_lighting_preserves_an_authored_light() {
        let mut scene = Scene::new();
        scene
            .directional_light(DirectionalLight::key_light())
            .add()
            .expect("authored light adds");
        let authored_count = scene.light_nodes().count();

        let diagnostics =
            apply_viewer_lighting(&mut scene, ViewerProfileLighting::None, true, false)
                .expect("viewer lighting setup succeeds");

        assert_eq!(scene.light_nodes().count(), authored_count);
        assert!(
            diagnostics.is_empty(),
            "authored lighting must not be replaced or mislabeled as fallback"
        );
    }
}
