use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::diagnostics::Backend;
use crate::{AssetFetcher, RawSemanticAovCapture, RawSemanticAovError, RawSemanticAovExclusions};

pub const SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1: &str = "scena.semantic_aov_capture.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostSemanticAovLegendEntryV1 {
    pub palette_index: u32,
    pub rgba8: [u8; 4],
    pub node_handle: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_handle: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostSemanticAovExclusionsV1 {
    pub transparent_triangle_count: usize,
    pub overlay_triangle_count: usize,
    pub unattributed_triangle_count: usize,
    pub stroke_segment_count: usize,
    pub label_quad_count: usize,
    pub gpu_instance_record_count: usize,
}

impl From<RawSemanticAovExclusions> for SceneHostSemanticAovExclusionsV1 {
    fn from(value: RawSemanticAovExclusions) -> Self {
        Self {
            transparent_triangle_count: value.transparent_triangle_count,
            overlay_triangle_count: value.overlay_triangle_count,
            unattributed_triangle_count: value.unattributed_triangle_count,
            stroke_segment_count: value.stroke_segment_count,
            label_quad_count: value.label_quad_count,
            gpu_instance_record_count: value.gpu_instance_record_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneHostSemanticAovCaptureV1 {
    pub schema: String,
    pub width: u32,
    pub height: u32,
    pub identity_scope: String,
    pub sample_pattern: String,
    pub depth_convention: String,
    pub normal_space: String,
    pub near: f32,
    pub far: f32,
    pub id_indices: Vec<u32>,
    pub depth_meters: Vec<f32>,
    pub world_normals: Vec<[f32; 3]>,
    pub legend: Vec<SceneHostSemanticAovLegendEntryV1>,
    pub exclusions: SceneHostSemanticAovExclusionsV1,
}

impl SceneHostSemanticAovCaptureV1 {
    pub fn id_rgba8(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.id_indices.len().saturating_mul(4));
        for index in &self.id_indices {
            output.extend_from_slice(&palette_rgba8(*index));
        }
        output
    }

    pub fn normal_rgba8(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.world_normals.len().saturating_mul(4));
        for (index, normal) in self.world_normals.iter().enumerate() {
            if self.id_indices.get(index).copied().unwrap_or(0) == 0 {
                output.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            output.extend_from_slice(&[
                encode_normal_component(normal[0]),
                encode_normal_component(normal[1]),
                encode_normal_component(normal[2]),
                255,
            ]);
        }
        output
    }

    /// Encodes background as zero and finite near/far camera distance as
    /// `1..=65535`. The caller writes these samples as a 16-bit grayscale PNG.
    pub fn depth_u16(&self) -> Vec<u16> {
        let range = self.far - self.near;
        self.depth_meters
            .iter()
            .map(|depth| {
                if !depth.is_finite() || !range.is_finite() || range <= 0.0 {
                    return 0;
                }
                let normalized = ((*depth - self.near) / range).clamp(0.0, 1.0);
                1 + (normalized * 65_534.0).round() as u16
            })
            .collect()
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    /// Opts GPU-backed hosts into lifecycle-owned semantic AOV resources.
    /// Call before `prepare()`; changing it invalidates the current prepared
    /// output-resource generation.
    pub fn set_semantic_aov_capture_enabled(&mut self, enabled: bool) {
        self.renderer.set_semantic_aov_capture_enabled(enabled);
    }

    pub fn semantic_aov_capture_enabled(&self) -> bool {
        self.renderer.semantic_aov_capture_enabled()
    }

    pub fn capture_semantic_aovs(&self) -> Result<SceneHostSemanticAovCaptureV1, SceneHostError> {
        self.ensure_active_camera()?;
        if self.backend() != Backend::Headless {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "semantic AOV CPU v1 requires the Headless backend; {:?} GPU target/readback support is pending FR06 parity proof",
                    self.backend()
                ),
            ));
        }
        let raw = self
            .renderer
            .semantic_aov_raw(&self.scene, self.active_camera)
            .map_err(semantic_error)?;
        self.map_semantic_capture(raw)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn capture_semantic_aovs_gpu(
        &mut self,
    ) -> Result<SceneHostSemanticAovCaptureV1, SceneHostError> {
        self.ensure_active_camera()?;
        if !matches!(
            self.backend(),
            Backend::HeadlessGpu | Backend::NativeSurface
        ) {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "semantic AOV GPU capture requires HeadlessGpu or NativeSurface, got {:?}",
                    self.backend()
                ),
            ));
        }
        if !self.semantic_aov_capture_enabled() {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                "semantic AOV GPU resources are disabled; enable them before prepare",
            ));
        }
        let raw = self
            .renderer
            .semantic_aov_gpu_raw(&self.scene, self.active_camera)
            .map_err(semantic_error)?;
        self.map_semantic_capture(raw)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn capture_semantic_aovs_gpu_async(
        &mut self,
    ) -> Result<SceneHostSemanticAovCaptureV1, SceneHostError> {
        self.ensure_active_camera()?;
        if !matches!(self.backend(), Backend::WebGpu | Backend::WebGl2) {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "semantic AOV browser capture requires WebGpu or WebGl2, got {:?}",
                    self.backend()
                ),
            ));
        }
        if !self.semantic_aov_capture_enabled() {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                "semantic AOV GPU resources are disabled; enable them before prepare",
            ));
        }
        let raw = self
            .renderer
            .semantic_aov_gpu_raw(&self.scene, self.active_camera)
            .await
            .map_err(semantic_error)?;
        self.map_semantic_capture(raw)
    }

    fn map_semantic_capture(
        &self,
        raw: RawSemanticAovCapture,
    ) -> Result<SceneHostSemanticAovCaptureV1, SceneHostError> {
        let mut legend = Vec::with_capacity(raw.legend.len());
        for entry in raw.legend {
            let node_handle = self
                .node_handle_map
                .get(&entry.identity.node)
                .copied()
                .ok_or_else(|| {
                    SceneHostError::new(
                        SceneHostErrorCode::Inspect,
                        "prepared semantic AOV node has no registered SceneHost handle",
                    )
                })?;
            let instance_handle = entry.identity.instance.and_then(|instance| {
                self.instance_handle_map
                    .get(&(entry.identity.node, instance))
                    .copied()
            });
            legend.push(SceneHostSemanticAovLegendEntryV1 {
                palette_index: entry.palette_index,
                rgba8: palette_rgba8(entry.palette_index),
                node_handle,
                instance_handle,
                instance_id: entry.identity.instance.map(|instance| instance.as_u64()),
            });
        }
        Ok(SceneHostSemanticAovCaptureV1 {
            schema: SCENE_HOST_SEMANTIC_AOV_SCHEMA_V1.to_owned(),
            width: raw.width,
            height: raw.height,
            identity_scope: "runtime_scoped".to_owned(),
            sample_pattern: "single_center_sample".to_owned(),
            depth_convention: "linear_camera_distance_scene_meters".to_owned(),
            normal_space: "world".to_owned(),
            near: raw.near,
            far: raw.far,
            id_indices: raw.id_indices,
            depth_meters: raw.depth_meters,
            world_normals: raw.world_normals,
            legend,
            exclusions: raw.exclusions.into(),
        })
    }
}

pub const fn palette_rgba8(index: u32) -> [u8; 4] {
    if index == 0 {
        [0, 0, 0, 0]
    } else {
        [
            (index & 0xff) as u8,
            ((index >> 8) & 0xff) as u8,
            ((index >> 16) & 0xff) as u8,
            255,
        ]
    }
}

fn encode_normal_component(value: f32) -> u8 {
    if value.is_finite() {
        ((value.clamp(-1.0, 1.0) * 0.5 + 0.5) * 255.0).round() as u8
    } else {
        0
    }
}

fn semantic_error(error: RawSemanticAovError) -> SceneHostError {
    match error {
        RawSemanticAovError::Render(error) => error.into(),
        RawSemanticAovError::UnsupportedBackend(backend) => SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("semantic AOV CPU v1 does not support backend {backend:?}"),
        ),
        RawSemanticAovError::PaletteExhausted { entries } => SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!(
                "semantic AOV requires {entries} palette entries, exceeding the 24-bit limit of 16777215"
            ),
        ),
    }
}
