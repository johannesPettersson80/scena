//! Generic host facade over `Scene`, `Assets`, and `Renderer`.

mod core;
mod error;
mod handles;
mod reporting;

#[cfg(target_arch = "wasm32")]
mod wasm;

pub use core::SceneHostCore;
pub use error::{SceneHostError, SceneHostErrorCode};

#[cfg(target_arch = "wasm32")]
pub use wasm::SceneHost;
