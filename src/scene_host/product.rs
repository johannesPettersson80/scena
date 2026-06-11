use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, AutoExposureConfig, Background, GridFloorOptions, LookupError};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_product_studio_visuals(&mut self, background: &str) -> Result<(), SceneHostError> {
        let background = scene_host_background(background)?;
        let environment = self.assets.default_environment();
        self.renderer.set_environment(environment);
        self.renderer.set_background(background);
        self.renderer
            .set_auto_exposure(AutoExposureConfig::product_studio());
        let lights = self.scene.add_studio_lighting()?;
        self.register_node(lights.key);
        self.register_node(lights.fill);
        self.register_node(lights.rim);
        Ok(())
    }

    pub fn add_product_grid_floor_under_node(
        &mut self,
        node: u64,
    ) -> Result<Vec<u64>, SceneHostError> {
        let node = self.resolve_node(node)?;
        let bounds = self
            .scene
            .node_world_bounds(node, &self.assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        let floor = self.scene.add_grid_floor(
            &self.assets,
            GridFloorOptions::new()
                .under_bounds(bounds)
                .padding(0.24)
                .line_spacing(0.08),
        )?;
        Ok(vec![
            self.register_node(floor.slab),
            self.register_node(floor.grid),
        ])
    }
}

fn scene_host_background(background: &str) -> Result<Background, SceneHostError> {
    match background {
        "studio_neutral" | "dark_studio" => Ok(Background::DarkStudio),
        "studio" => Ok(Background::Studio),
        "neutral_gray" => Ok(Background::NeutralGray),
        "black" => Ok(Background::Black),
        "white" => Ok(Background::White),
        other => Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("unsupported SceneHost product studio background {other}"),
        )),
    }
}
