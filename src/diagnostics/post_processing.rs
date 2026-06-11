use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostProcessingReportV1 {
    #[serde(default)]
    pub active_passes: Vec<PostProcessingPassV1>,
    #[serde(default)]
    pub anti_aliasing: bool,
    #[serde(default)]
    pub bloom: bool,
    #[serde(default)]
    pub screen_space_ambient_occlusion: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssao_depth_source: Option<PostProcessingDepthSourceV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostProcessingPassV1 {
    ScreenSpaceAmbientOcclusion,
    Bloom,
    Fxaa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostProcessingDepthSourceV1 {
    CpuDepthFrame,
    SampleableDepthTexture,
    DepthColorTarget,
}
