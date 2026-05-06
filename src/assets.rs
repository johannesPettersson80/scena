//! Asset fetchers, caches, glTF/GLB parsing, texture decoding, and asset handles.

/// CPU-side retention behavior for asset data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainPolicy {
    Never,
    OnContextLossOnly,
    Always,
}

/// Asset source and cache owner.
#[derive(Debug, Clone)]
pub struct Assets<F = ()> {
    fetcher: F,
    retain_policy: RetainPolicy,
}

impl Assets<()> {
    pub fn new() -> Self {
        Self::with_fetcher(())
    }
}

impl Default for Assets<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> Assets<F> {
    pub fn with_fetcher(fetcher: F) -> Self {
        Self {
            fetcher,
            retain_policy: RetainPolicy::OnContextLossOnly,
        }
    }

    pub fn fetcher(&self) -> &F {
        &self.fetcher
    }

    pub fn retain_policy(&self) -> RetainPolicy {
        self.retain_policy
    }

    pub fn set_retain_policy(&mut self, policy: RetainPolicy) {
        self.retain_policy = policy;
    }
}
