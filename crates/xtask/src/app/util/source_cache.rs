use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;

use crate::app::prelude::{Path, PathBuf, fs};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SourceCacheMetrics {
    pub(crate) file_opens: u64,
    pub(crate) bytes_read: u64,
    pub(crate) source_tree_walks: u64,
    pub(crate) directory_reads: u64,
    pub(crate) cache_hits: u64,
}

#[derive(Clone)]
enum CachedText {
    Readable(Arc<str>),
    Unreadable(io::ErrorKind, Arc<str>),
}

struct SourceCache {
    root: PathBuf,
    texts: BTreeMap<PathBuf, CachedText>,
    rust_trees: BTreeMap<PathBuf, Vec<PathBuf>>,
    metrics: SourceCacheMetrics,
}

thread_local! {
    static ACTIVE_SOURCE_CACHE: RefCell<Option<SourceCache>> = const { RefCell::new(None) };
}

pub(crate) fn with_source_cache_profiled<T>(
    root: &Path,
    operation: impl FnOnce() -> T,
) -> (T, SourceCacheMetrics) {
    let previous = ACTIVE_SOURCE_CACHE.with(|active| {
        active.replace(Some(SourceCache {
            root: root.to_path_buf(),
            texts: BTreeMap::new(),
            rust_trees: BTreeMap::new(),
            metrics: SourceCacheMetrics::default(),
        }))
    });
    let result = operation();
    let completed = ACTIVE_SOURCE_CACHE.with(|active| active.replace(previous));
    let metrics = completed.map_or_else(SourceCacheMetrics::default, |cache| cache.metrics);
    (result, metrics)
}

pub(crate) fn read_source_to_string(
    root: &Path,
    relative: impl AsRef<Path>,
) -> io::Result<Arc<str>> {
    let relative = normalize_relative(root, relative.as_ref());
    ACTIVE_SOURCE_CACHE.with(|active| {
        let mut active = active.borrow_mut();
        let Some(cache) = active.as_mut().filter(|cache| cache.root == root) else {
            return fs::read_to_string(root.join(&relative)).map(Arc::<str>::from);
        };
        if let Some(cached) = cache.texts.get(&relative).cloned() {
            cache.metrics.cache_hits = cache.metrics.cache_hits.saturating_add(1);
            return cached_text_result(cached);
        }

        cache.metrics.file_opens = cache.metrics.file_opens.saturating_add(1);
        let cached = match fs::read_to_string(root.join(&relative)) {
            Ok(text) => {
                cache.metrics.bytes_read =
                    cache.metrics.bytes_read.saturating_add(text.len() as u64);
                CachedText::Readable(Arc::<str>::from(text))
            }
            Err(error) => CachedText::Unreadable(error.kind(), Arc::<str>::from(error.to_string())),
        };
        cache.texts.insert(relative, cached.clone());
        cached_text_result(cached)
    })
}

pub(crate) fn cached_rust_files_below(root: &Path, relative: &Path) -> Vec<PathBuf> {
    ACTIVE_SOURCE_CACHE.with(|active| {
        let mut active = active.borrow_mut();
        let Some(cache) = active.as_mut().filter(|cache| cache.root == root) else {
            return collect_rust_files(root, relative, None);
        };
        if let Some(files) = cache.rust_trees.get(relative).cloned() {
            cache.metrics.cache_hits = cache.metrics.cache_hits.saturating_add(1);
            return files;
        }
        cache.metrics.source_tree_walks = cache.metrics.source_tree_walks.saturating_add(1);
        let mut directory_reads = 0_u64;
        let files = collect_rust_files(root, relative, Some(&mut directory_reads));
        cache.metrics.directory_reads = cache
            .metrics
            .directory_reads
            .saturating_add(directory_reads);
        cache
            .rust_trees
            .insert(relative.to_path_buf(), files.clone());
        files
    })
}

fn collect_rust_files(
    root: &Path,
    relative: &Path,
    mut directory_reads: Option<&mut u64>,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_files_recursive(root, relative, &mut files, &mut directory_reads);
    files.sort();
    files
}

fn collect_rust_files_recursive(
    root: &Path,
    relative: &Path,
    files: &mut Vec<PathBuf>,
    directory_reads: &mut Option<&mut u64>,
) {
    if let Some(count) = directory_reads.as_deref_mut() {
        *count = count.saturating_add(1);
    }
    let Ok(entries) = fs::read_dir(root.join(relative)) else {
        return;
    };
    for entry in entries.flatten() {
        let child = relative.join(entry.file_name());
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files_recursive(root, &child, files, directory_reads);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(child);
        }
    }
}

fn normalize_relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn cached_text_result(cached: CachedText) -> io::Result<Arc<str>> {
    match cached {
        CachedText::Readable(text) => Ok(text),
        CachedText::Unreadable(kind, message) => Err(io::Error::new(kind, message.to_string())),
    }
}
