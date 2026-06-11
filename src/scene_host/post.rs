use serde_json::Value;

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AntiAliasing, AssetFetcher, PostBloomConfig, ScreenSpaceAmbientOcclusionConfig};

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_anti_aliasing(&mut self, mode: &str) -> Result<(), SceneHostError> {
        let anti_aliasing = match mode {
            "fxaa" | "on" | "true" => AntiAliasing::Fxaa,
            "none" | "off" | "false" => AntiAliasing::None,
            other => {
                return Err(invalid_input(format!(
                    "unsupported SceneHost anti-aliasing mode {other}"
                )));
            }
        };
        self.renderer.set_anti_aliasing(anti_aliasing);
        Ok(())
    }

    pub fn set_bloom_json(&mut self, config_json: Option<&str>) -> Result<(), SceneHostError> {
        let config = config_json.map(parse_bloom_config).transpose()?;
        self.renderer.set_bloom(config);
        Ok(())
    }

    pub fn set_ambient_occlusion_json(
        &mut self,
        config_json: Option<&str>,
    ) -> Result<(), SceneHostError> {
        let config = config_json
            .map(parse_ambient_occlusion_config)
            .transpose()?;
        self.renderer.set_screen_space_ambient_occlusion(config);
        Ok(())
    }
}

fn parse_bloom_config(json: &str) -> Result<PostBloomConfig, SceneHostError> {
    let value: Value = serde_json::from_str(json)
        .map_err(|error| invalid_input(format!("invalid bloom config JSON: {error}")))?;
    let default = PostBloomConfig::subtle();
    Ok(PostBloomConfig::new(
        u8_field(&value, "threshold_srgb", default.threshold_srgb())?,
        f32_field(&value, "intensity", default.intensity())?,
        u8_field(&value, "radius_px", default.radius_px())?,
    ))
}

fn parse_ambient_occlusion_config(
    json: &str,
) -> Result<ScreenSpaceAmbientOcclusionConfig, SceneHostError> {
    let value: Value = serde_json::from_str(json).map_err(|error| {
        invalid_input(format!("invalid ambient occlusion config JSON: {error}"))
    })?;
    let default = ScreenSpaceAmbientOcclusionConfig::subtle();
    Ok(ScreenSpaceAmbientOcclusionConfig::new(
        u8_field(&value, "radius_px", default.radius_px())?,
        f32_field(&value, "intensity", default.intensity())?,
        f32_field(&value, "depth_threshold", default.depth_threshold())?,
    ))
}

fn u8_field(value: &Value, field: &str, default: u8) -> Result<u8, SceneHostError> {
    let Some(value) = value.get(field) else {
        return Ok(default);
    };
    let Some(value) = value.as_u64() else {
        return Err(invalid_input(format!(
            "{field} must be an unsigned integer"
        )));
    };
    Ok(value.min(u64::from(u8::MAX)) as u8)
}

fn f32_field(value: &Value, field: &str, default: f32) -> Result<f32, SceneHostError> {
    let Some(value) = value.get(field) else {
        return Ok(default);
    };
    let Some(value) = value.as_f64() else {
        return Err(invalid_input(format!("{field} must be a number")));
    };
    Ok(value as f32)
}

fn invalid_input(message: impl Into<String>) -> SceneHostError {
    SceneHostError::new(SceneHostErrorCode::InvalidInput, message.into())
}
