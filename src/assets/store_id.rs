use super::AssetStoreId;

impl AssetStoreId {
    pub(super) fn next() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let raw = COUNTER.fetch_add(1, Ordering::Relaxed);
        let value = std::num::NonZeroU64::new(raw)
            .expect("AssetStoreId counter never returns zero before saturation");
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for AssetStoreId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Assets#{}", self.0.get())
    }
}
