#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::process::Command;

#[test]
fn task_cache_status_reports_exact_read_only_paths_as_json() {
    let temp = std::env::temp_dir().join(format!("scena-h01-{}", std::process::id()));
    let cache = temp.join("cache");
    let worktree = cache.join("codex-worktrees/scena-proof-task");
    let target = cache.join("codex-targets/scena-proof-task");
    fs::create_dir_all(&worktree).expect("test worktree creates");
    fs::create_dir_all(target.join("tmp")).expect("test target creates");
    fs::write(worktree.join("source.txt"), b"source").expect("test source writes");
    fs::write(target.join("artifact.txt"), b"artifact").expect("test artifact writes");

    let output = Command::new("bash")
        .arg("scripts/scena_task_cache_status.sh")
        .arg("proof-task")
        .env("SCENA_TASK_CACHE_ROOT", &cache)
        .output()
        .expect("cache status script executes");
    assert!(
        output.status.success(),
        "cache status failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cache status is JSON");
    assert_eq!(report["schema"], "scena.task_cache_status.v1");
    assert_eq!(report["task_slug"], "proof-task");
    assert_eq!(report["read_only"], true);
    let entries = report["entries"].as_array().expect("entries are an array");
    assert_eq!(entries.len(), 3);
    for entry in entries {
        let path = entry["path"].as_str().expect("entry path is text");
        assert!(path.starts_with(cache.to_str().expect("cache path is UTF-8")));
        assert_eq!(entry["exists"], true);
        assert!(entry["size_bytes"].as_u64().is_some());
        assert!(entry["modified_unix_seconds"].as_u64().is_some());
        assert_eq!(entry["reproducible"], true);
    }
    assert!(
        report.get("deleted").is_none(),
        "status must never delete data"
    );

    fs::remove_dir_all(&temp).expect("test cache removes");
}

#[test]
fn task_cache_status_rejects_broad_or_ambiguous_targets() {
    for slug in ["", ".", "..", "/", "a/b", "a b", "*"] {
        let output = Command::new("bash")
            .arg("scripts/scena_task_cache_status.sh")
            .arg(slug)
            .output()
            .expect("cache status script executes");
        assert!(!output.status.success(), "unsafe task slug {slug:?} passed");
    }
}
