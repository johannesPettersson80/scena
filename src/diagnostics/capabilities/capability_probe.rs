use serde::{Deserialize, Serialize};

use super::{AdapterLimitsReport, Backend};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuDeviceReport {
    pub features: String,
    pub limits: AdapterLimitsReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityProbeModeV1 {
    Static,
    LiveAdapter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityProbeStatusV1 {
    StaticNoDevice,
    Measured,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityConstraintStatusV1 {
    Supported,
    NotProbed,
    NotApplicable,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityTargetProbeV1 {
    pub format: String,
    pub source: String,
    pub measured: bool,
    pub allowed_usages: Option<String>,
    pub sample_counts: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityConstraintProbeV1 {
    pub status: CapabilityConstraintStatusV1,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityProbeUnavailableV1 {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityProbeV1 {
    pub mode: CapabilityProbeModeV1,
    pub status: CapabilityProbeStatusV1,
    pub source: String,
    pub probed_at_unix_ms: Option<u64>,
    pub requested_backend: Backend,
    pub selected_backend: Option<Backend>,
    pub device: Option<GpuDeviceReport>,
    pub color_target: CapabilityTargetProbeV1,
    pub depth_target: CapabilityTargetProbeV1,
    pub readback: CapabilityConstraintProbeV1,
    pub presentation: CapabilityConstraintProbeV1,
    pub unavailable: Option<CapabilityProbeUnavailableV1>,
}
