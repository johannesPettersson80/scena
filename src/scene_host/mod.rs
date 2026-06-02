//! Generic host facade over `Scene`, `Assets`, and `Renderer`.

mod assets;
mod camera;
mod capture;
mod core;
mod error;
mod handles;
mod reporting;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
mod wasm_assets;
#[cfg(target_arch = "wasm32")]
mod wasm_camera;
#[cfg(target_arch = "wasm32")]
mod wasm_inputs;
#[cfg(target_arch = "wasm32")]
mod wasm_readback;

pub use camera::SceneHostCameraState;
pub use core::SceneHostCore;
pub use error::{SceneHostError, SceneHostErrorCode};
pub use reporting::{SCENE_HOST_ASSET_IMPORT_SCHEMA_V1, SceneHostAssetImportReportV1};

#[cfg(target_arch = "wasm32")]
pub use wasm::SceneHost;
