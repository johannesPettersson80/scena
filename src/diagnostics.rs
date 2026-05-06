//! Structured errors, debug overlays, capability reports, and renderer stats.

/// Minimal placeholder error until the RFC error hierarchy is implemented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenaError {
    pub message: &'static str,
}
