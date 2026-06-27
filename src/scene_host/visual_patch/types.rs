use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::{SceneHostCameraState, SceneHostEasing, SceneHostError, SceneHostErrorCode};
use crate::{Color, Transform};

pub const VISUAL_PATCH_SCHEMA_V1: &str = "scena.visual_patch.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchV1 {
    pub schema: String,
    #[serde(default)]
    pub transforms: Vec<VisualPatchTransformV1>,
    #[serde(default)]
    pub tints: Vec<VisualPatchTintV1>,
    #[serde(default)]
    pub visibility: Vec<VisualPatchVisibilityV1>,
    #[serde(default)]
    pub camera: Option<SceneHostCameraState>,
    #[serde(default)]
    pub transforms_eased: Vec<VisualPatchTransformEasedV1>,
    #[serde(default)]
    pub tints_eased: Vec<VisualPatchTintEasedV1>,
    #[serde(default)]
    pub camera_eased: Option<VisualPatchCameraEasedV1>,
    #[serde(default)]
    pub animation_time: Vec<VisualPatchAnimationTimeV1>,
    #[serde(default)]
    pub selection: Option<VisualPatchSelectionV1>,
    #[serde(default)]
    pub hover: Option<VisualPatchHoverV1>,
    #[serde(default)]
    pub material_variants: Vec<VisualPatchMaterialVariantV1>,
    #[serde(default)]
    pub labels: Vec<VisualPatchLabelV1>,
    #[serde(default)]
    pub section_box: Option<VisualPatchSectionBoxV1>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub echo_metadata: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchTransformV1 {
    pub node: u64,
    pub transform: Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchTintV1 {
    pub node: u64,
    pub tint: Option<Color>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchVisibilityV1 {
    pub node: u64,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchTransformEasedV1 {
    pub node: u64,
    pub transform: Transform,
    pub duration_seconds: f64,
    pub easing: SceneHostEasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchTintEasedV1 {
    pub node: u64,
    pub tint: Option<Color>,
    pub duration_seconds: f64,
    pub easing: SceneHostEasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchCameraEasedV1 {
    pub camera: SceneHostCameraState,
    pub duration_seconds: f64,
    pub easing: SceneHostEasing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchAnimationTimeV1 {
    pub mixer: u64,
    pub mode: VisualPatchAnimationTimeModeV1,
    pub seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchSelectionV1 {
    pub node: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchHoverV1 {
    pub node: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchMaterialVariantV1 {
    pub import: u64,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualPatchLabelV1 {
    pub id: String,
    pub target: VisualPatchLabelTargetV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualPatchLabelTargetV1 {
    Node { node: u64, local_offset: [f32; 3] },
    World { position: [f32; 3] },
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum VisualPatchSectionBoxV1 {
    Set {
        min: [f32; 3],
        max: [f32; 3],
        #[serde(default)]
        margin: f32,
        #[serde(default)]
        inverted: bool,
        #[serde(default)]
        helper_wireframe: bool,
    },
    Invert {
        inverted: bool,
    },
    Disable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualPatchAnimationTimeModeV1 {
    Seek,
    Advance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchResultV1 {
    pub schema: String,
    pub applied: VisualPatchAppliedCountsV1,
    pub failed: Vec<VisualPatchEntryErrorV1>,
    pub revisions: VisualPatchRevisionDeltaV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchAppliedCountsV1 {
    pub transforms: u64,
    pub tints: u64,
    pub visibility: u64,
    pub camera: u64,
    #[serde(default)]
    pub transforms_eased: u64,
    #[serde(default)]
    pub tints_eased: u64,
    #[serde(default)]
    pub camera_eased: u64,
    #[serde(default)]
    pub animation_time: u64,
    #[serde(default)]
    pub selection: u64,
    #[serde(default)]
    pub hover: u64,
    #[serde(default)]
    pub material_variants: u64,
    #[serde(default)]
    pub labels: u64,
    #[serde(default)]
    pub section_box: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchEntryErrorV1 {
    pub channel: String,
    pub index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<u64>,
    pub code: SceneHostErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualPatchRevisionDeltaV1 {
    pub structure: u64,
    pub transform: u64,
    pub appearance: u64,
    pub visibility: u64,
    pub interaction: u64,
}

impl Default for VisualPatchV1 {
    fn default() -> Self {
        Self {
            schema: VISUAL_PATCH_SCHEMA_V1.to_owned(),
            transforms: Vec::new(),
            tints: Vec::new(),
            visibility: Vec::new(),
            camera: None,
            transforms_eased: Vec::new(),
            tints_eased: Vec::new(),
            camera_eased: None,
            animation_time: Vec::new(),
            selection: None,
            hover: None,
            material_variants: Vec::new(),
            labels: Vec::new(),
            section_box: None,
            metadata: None,
            echo_metadata: false,
        }
    }
}

impl VisualPatchResultV1 {
    pub(super) fn new() -> Self {
        Self {
            schema: VISUAL_PATCH_SCHEMA_V1.to_owned(),
            applied: VisualPatchAppliedCountsV1::default(),
            failed: Vec::new(),
            revisions: VisualPatchRevisionDeltaV1::default(),
            metadata: None,
        }
    }
}

impl VisualPatchEntryErrorV1 {
    pub(super) fn from_error(
        channel: &'static str,
        index: usize,
        handle: Option<u64>,
        error: SceneHostError,
    ) -> Self {
        Self {
            channel: channel.to_owned(),
            index,
            handle,
            code: error.code(),
            message: error.message().to_owned(),
        }
    }
}

impl VisualPatchLabelTargetV1 {
    pub(super) fn handle(&self) -> Option<u64> {
        match self {
            Self::Node { node, .. } => Some(*node),
            Self::World { .. } | Self::Clear => None,
        }
    }
}
