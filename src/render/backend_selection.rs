use crate::{Backend, BuildError};

/// Typed outcome for an explicitly fallback-capable headless GPU request.
///
/// Strict `headless_gpu` constructors do not return this report because their
/// successful backend is guaranteed to be [`Backend::HeadlessGpu`]. The report
/// belongs only to explicitly named `headless_prefer_gpu` construction, where
/// callers opted into a CPU fallback and must be able to observe why it
/// happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlessBackendSelectionReport {
    requested_backend: Backend,
    selected_backend: Backend,
    gpu_error: Option<BuildError>,
}

impl HeadlessBackendSelectionReport {
    pub(crate) const fn gpu() -> Self {
        Self {
            requested_backend: Backend::HeadlessGpu,
            selected_backend: Backend::HeadlessGpu,
            gpu_error: None,
        }
    }

    pub(crate) const fn cpu_fallback(gpu_error: BuildError) -> Self {
        Self {
            requested_backend: Backend::HeadlessGpu,
            selected_backend: Backend::Headless,
            gpu_error: Some(gpu_error),
        }
    }

    pub const fn requested_backend(&self) -> Backend {
        self.requested_backend
    }

    pub const fn selected_backend(&self) -> Backend {
        self.selected_backend
    }

    pub const fn fallback_used(&self) -> bool {
        self.gpu_error.is_some()
    }

    pub const fn gpu_error(&self) -> Option<&BuildError> {
        self.gpu_error.as_ref()
    }
}
