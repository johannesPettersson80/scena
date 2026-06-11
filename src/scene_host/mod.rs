//! Generic host facade over `Scene`, `Assets`, and `Renderer`.

mod assets;
mod camera;
mod capture;
mod core;
mod error;
mod handles;
mod inputs;
mod post;
mod product;
mod reporting;
mod subtree;
mod transforms;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
mod wasm_assets;
#[cfg(target_arch = "wasm32")]
mod wasm_camera;
#[cfg(target_arch = "wasm32")]
mod wasm_post;
#[cfg(target_arch = "wasm32")]
mod wasm_product;
#[cfg(target_arch = "wasm32")]
mod wasm_readback;
#[cfg(target_arch = "wasm32")]
mod wasm_subtree;
#[cfg(target_arch = "wasm32")]
mod wasm_transforms;

pub use camera::SceneHostCameraState;
pub use core::SceneHostCore;
pub use error::{SceneHostError, SceneHostErrorCode};
pub use reporting::{
    SCENE_HOST_ASSET_IMPORT_SCHEMA_V1, SCENE_HOST_SUBTREE_SCHEMA_V1, SceneHostAssetImportReportV1,
    SceneHostSubtreeNodeV1, SceneHostSubtreeReportV1,
};

#[cfg(target_arch = "wasm32")]
pub use wasm::SceneHost;
