use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSceneV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<SceneRecipeBackgroundV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<SceneRecipeEnvironmentV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<SceneRecipeGridV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeBackgroundV1 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeEnvironmentV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeGridV1 {
    #[serde(default = "default_grid_enabled", skip_serializing_if = "is_true")]
    pub enabled: bool,
    #[serde(default = "default_grid_under_bounds", skip_serializing_if = "is_true")]
    pub under_bounds: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_width_px: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roughness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflection: Option<SceneRecipeGridReflectionV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeGridReflectionV1 {
    #[serde(
        default = "default_grid_reflection_enabled",
        skip_serializing_if = "is_true"
    )]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strength: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeRenderV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anti_aliasing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersample: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconstruction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bloom: Option<SceneRecipeBloomV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssao: Option<SceneRecipeSsaoV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_space_reflections: Option<SceneRecipeScreenSpaceReflectionsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth_of_field: Option<SceneRecipeDepthOfFieldV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure_ev: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_exposure: Option<SceneRecipeAutoExposureV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tonemapper: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SceneRecipeAutoExposureV1 {
    Preset(String),
    Config {
        preset: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_ev: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_ev: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        highlight_percentile: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        highlight_target_luminance: Option<f64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeBloomV1 {
    pub threshold_srgb: u8,
    pub intensity: f64,
    pub radius_px: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeSsaoV1 {
    pub radius_px: u8,
    pub intensity: f64,
    pub depth_threshold: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeScreenSpaceReflectionsV1 {
    pub strength: f64,
    pub roughness: f64,
    pub horizon_fraction: f64,
    pub fade: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRecipeDepthOfFieldV1 {
    pub focus_distance: f64,
    pub aperture_f_stop: f64,
    pub radius_px: u8,
}

fn default_grid_enabled() -> bool {
    true
}

fn default_grid_under_bounds() -> bool {
    true
}

fn default_grid_reflection_enabled() -> bool {
    true
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_true(value: &bool) -> bool {
    *value
}
