use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    AntiAliasing, AssetFetcher, AutoExposureConfig, Background, GridFloorOptions, LookupError,
    PostBloomConfig, ScreenSpaceAmbientOcclusionConfig, Vec3,
};

pub const SCENE_HOST_GROUNDING_SCHEMA_V1: &str = "scena.scene_host_grounding.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostGroundingReportV1 {
    pub schema: String,
    pub target: u64,
    pub floor_handles: Vec<u64>,
    pub floor_receiver: bool,
    pub ssao_enabled: bool,
    pub active_paths: Vec<SceneHostGroundingPathV1>,
    pub fallbacks: Vec<SceneHostGroundingFallbackV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostGroundingPathV1 {
    FloorReceiver,
    ScreenSpaceAmbientOcclusion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostGroundingFallbackV1 {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub help: String,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_product_studio_visuals(&mut self, background: &str) -> Result<(), SceneHostError> {
        let background = scene_host_background(background)?;
        let environment = self.assets.default_environment();
        self.renderer.set_environment(environment);
        self.renderer.set_background(background);
        self.renderer
            .set_auto_exposure(AutoExposureConfig::product_studio());
        self.renderer.set_anti_aliasing(AntiAliasing::Fxaa);
        self.renderer.set_bloom(Some(PostBloomConfig::subtle()));
        self.renderer
            .set_screen_space_ambient_occlusion(Some(ScreenSpaceAmbientOcclusionConfig::subtle()));
        let lights = self.scene.add_studio_lighting()?;
        self.register_node(lights.key);
        self.register_node(lights.fill);
        self.register_node(lights.rim);
        Ok(())
    }

    pub fn apply_product_grounding_preset(
        &mut self,
        target: u64,
        background: &str,
    ) -> Result<SceneHostGroundingReportV1, SceneHostError> {
        self.ground_node_to_y_zero(target)?;
        self.apply_product_studio_visuals(background)?;
        let floor_handles = self.add_product_grid_floor_under_node(target)?;
        let floor_receiver = !floor_handles.is_empty();
        let ssao_enabled = self.renderer.screen_space_ambient_occlusion().is_some();
        let mut active_paths = Vec::new();
        if floor_receiver {
            active_paths.push(SceneHostGroundingPathV1::FloorReceiver);
        }
        if ssao_enabled {
            active_paths.push(SceneHostGroundingPathV1::ScreenSpaceAmbientOcclusion);
        }
        Ok(SceneHostGroundingReportV1 {
            schema: SCENE_HOST_GROUNDING_SCHEMA_V1.to_owned(),
            target,
            floor_handles,
            floor_receiver,
            ssao_enabled,
            active_paths,
            fallbacks: grounding_fallbacks(ssao_enabled),
        })
    }

    pub fn apply_product_grounding_preset_json(
        &mut self,
        target: u64,
        background: &str,
    ) -> Result<String, SceneHostError> {
        let report = self.apply_product_grounding_preset(target, background)?;
        serde_json::to_string(&report).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("grounding report serialization failed: {error}"),
            )
        })
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

    fn ground_node_to_y_zero(&mut self, node: u64) -> Result<(), SceneHostError> {
        let node_key = self.resolve_node(node)?;
        let bounds = self
            .scene
            .node_world_bounds(node_key, &self.assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        let drop_y = -bounds.min.y;
        if drop_y.abs() <= 1.0e-6 {
            return Ok(());
        }
        let mut world = self
            .scene
            .world_transform(node_key)
            .ok_or(LookupError::NodeNotFound(node_key))?;
        world.translation += Vec3::new(0.0, drop_y, 0.0);
        self.scene.align_to(node_key, world)?;
        Ok(())
    }
}

fn grounding_fallbacks(ssao_enabled: bool) -> Vec<SceneHostGroundingFallbackV1> {
    let mut fallbacks = Vec::new();
    if ssao_enabled {
        fallbacks.push(SceneHostGroundingFallbackV1 {
            code: "ssao_is_ambient_occlusion".to_owned(),
            severity: "info".to_owned(),
            message: "SSAO darkens depth contact edges but is not a drop-shadow substitute"
                .to_owned(),
            help: "use the report active_paths to distinguish floor receiver, SSAO, and shadow receiver claims"
                .to_owned(),
        });
    } else {
        fallbacks.push(SceneHostGroundingFallbackV1 {
            code: "ssao_unavailable".to_owned(),
            severity: "warning".to_owned(),
            message: "screen-space ambient occlusion is not active for this grounding preset"
                .to_owned(),
            help:
                "check backend capabilities or enable SSAO before relying on contact-edge darkening"
                    .to_owned(),
        });
    }
    fallbacks
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
