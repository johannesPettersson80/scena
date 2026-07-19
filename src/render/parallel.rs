//! Bounded native CPU parallelism shared by deterministic renderer work.
//!
//! Rayon owns the native worker pool, so `RAYON_NUM_THREADS` remains the
//! process-level override. Scena additionally caps each operation at eight
//! workers, never starts nested parallel work from a Rayon worker, and always
//! resolves to one worker on WASM.

#[cfg(not(target_arch = "wasm32"))]
const MAX_RENDER_WORKERS: usize = 8;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn worker_count(task_count: usize) -> usize {
    if task_count <= 1 || rayon::current_thread_index().is_some() {
        return 1;
    }
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(rayon::current_num_threads())
        .min(MAX_RENDER_WORKERS)
        .min(task_count)
        .max(1)
}

#[cfg(target_arch = "wasm32")]
pub(super) const fn worker_count(_task_count: usize) -> usize {
    1
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn pf09_worker_policy_is_bounded_and_disables_nested_parallelism() {
        assert_eq!(worker_count(0), 1);
        assert_eq!(worker_count(1), 1);
        assert!(worker_count(usize::MAX) <= MAX_RENDER_WORKERS);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .expect("focused nested-worker pool builds");
        assert_eq!(pool.install(|| worker_count(64)), 1);
    }
}
