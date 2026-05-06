//! `scena` is a Rust-native scene-graph renderer.
//!
//! The implementation is intentionally skeletal at repo creation time. The
//! execution plan lives in `docs/RFC-rust-3d-renderer.md`.

pub mod animation;
pub mod assets;
pub mod controls;
pub mod diagnostics;
pub mod geometry;
pub mod material;
pub mod picking;
pub mod platform;
pub mod render;
pub mod scene;

/// Crate-level result type placeholder until the public error hierarchy lands.
pub type Result<T> = std::result::Result<T, diagnostics::ScenaError>;
