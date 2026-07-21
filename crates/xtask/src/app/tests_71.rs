use crate::app::prelude::*;

#[test]
fn p07_source_cache_reads_each_file_once_and_is_immutable_per_doctor_run() {
    let root = unique_fixture_root("source-cache");
    fs::create_dir_all(root.join("src/nested")).expect("fixture source directory creates");
    fs::write(root.join("src/lib.rs"), "first\n").expect("fixture source writes");
    fs::write(root.join("src/nested/child.rs"), "child\n").expect("fixture child writes");

    let (_, metrics) = with_source_cache_profiled(&root, || {
        assert_eq!(
            &*read_source_to_string(&root, "src/lib.rs").unwrap(),
            "first\n"
        );
        fs::write(root.join("src/lib.rs"), "second\n").expect("fixture mutation writes");
        assert_eq!(
            &*read_source_to_string(&root, "src/lib.rs").unwrap(),
            "first\n",
            "one doctor run observes one immutable source snapshot"
        );
        assert_eq!(source_files(&root), source_files(&root));
    });
    assert_eq!(metrics.file_opens, 1, "the repeated source read is cached");
    assert!(metrics.cache_hits >= 2);
    assert_eq!(metrics.source_tree_walks, 1);

    with_source_cache_profiled(&root, || {
        assert_eq!(
            &*read_source_to_string(&root, "src/lib.rs").unwrap(),
            "second\n",
            "the next doctor run receives a fresh snapshot"
        );
    });
    fs::remove_dir_all(&root).expect("fixture removes");
}

#[test]
fn p07_source_cache_keeps_missing_files_fail_closed_without_reopening() {
    let root = unique_fixture_root("source-cache-missing");
    fs::create_dir_all(root.join("src")).expect("fixture source directory creates");

    let (_, metrics) = with_source_cache_profiled(&root, || {
        assert!(read_source_to_string(&root, "src/missing.rs").is_err());
        assert!(read_source_to_string(&root, "src/missing.rs").is_err());
    });
    assert_eq!(metrics.file_opens, 1);
    assert_eq!(metrics.cache_hits, 1);

    fs::remove_dir_all(&root).expect("fixture removes");
}

fn unique_fixture_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock follows epoch")
        .as_nanos();
    env::temp_dir().join(format!("scena-{label}-{}-{nonce}", process::id()))
}
