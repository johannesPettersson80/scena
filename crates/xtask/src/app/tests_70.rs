use crate::app::prelude::*;

#[test]
fn d01_version_alignment_rejects_each_public_drift_surface() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/d01-version-alignment");
    let _ = fs::remove_dir_all(&fixture_root);
    let files = [
        (
            "Cargo.toml",
            "[package]\nname = \"scena\"\nversion = \"1.8.0\"\n",
        ),
        (
            "Cargo.lock",
            "[[package]]\nname = \"scena\"\nversion = \"1.8.0\"\n",
        ),
        (
            "package.json",
            "{\"name\":\"scena-release-gates\",\"private\":true}\n",
        ),
        (
            "package-lock.json",
            "{\"name\":\"scena-release-gates\",\"packages\":{\"\":{\"name\":\"scena-release-gates\"}}}\n",
        ),
        (
            "demo/pkg/package.json",
            "{\"name\":\"scena\",\"version\":\"1.8.0\"}\n",
        ),
        (
            "demo/proof/pkg/package.json",
            "{\"name\":\"scena\",\"version\":\"1.8.0\"}\n",
        ),
        (
            "demo/pkg/scena_bg.wasm.size.json",
            "{\"bundle\":\"public\",\"crate_version\":\"1.8.0\"}\n",
        ),
        (
            "demo/proof/pkg/scena_bg.wasm.size.json",
            "{\"bundle\":\"proof\",\"crate_version\":\"1.8.0\"}\n",
        ),
        (
            "demo/index.html",
            "<title>scena 1.8.0 live showcase</title>\n?v=1.8.0-public-fixture\n",
        ),
        ("demo/main.js", "?v=1.8.0-public-fixture\n"),
        ("demo/proof/index.html", "?v=1.8.0-proof-fixture\n"),
        (
            "demo/proof.js",
            "scena 1.8.0 — pick a name, not a number\n?v=1.8.0-proof-fixture\n",
        ),
        (
            "scripts/build_demo_wasm.js",
            "function crateVersion() {}\nfunction validateGeneratedPackageVersion() {}\nfunction stampPublicVersionText() {}\ncrate_version: crateVersion()\nCARGO_PROFILE_RELEASE_OPT_LEVEL\nstampCacheBusters(writeSizeManifest());\n",
        ),
        (
            "README.md",
            "cargo add scena\nhttps://docs.rs/scena/latest/scena/\n",
        ),
        ("docs/README.md", "https://docs.rs/scena/latest/scena/\n"),
        ("docs/api.md", "https://docs.rs/scena/latest/scena/\n"),
        ("docs/getting-started.md", "cargo add scena\n"),
        ("docs/feature-flags.md", "cargo add scena\n"),
        ("docs/examples.md", "cargo add scena\n"),
        ("examples/version_policy.rs", "use scena::Scene;\n"),
        ("crates/xtask/src/app.rs", "#[cfg(test)]\nmod tests_70;\n"),
        (
            "crates/xtask/src/app/tests_70.rs",
            "#[test]\nfn d01_version_alignment_rejects_each_public_drift_surface() {}\n",
        ),
        (
            "docs/specs/release-gates.md",
            "HISTORICAL_VERSION_PATH_PREFIXES\nCargo.toml is the canonical public version source.\n",
        ),
    ];
    for (relative, contents) in files {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("D01 fixture parent"))
            .expect("D01 fixture directory creates");
        fs::write(destination, contents).expect("D01 fixture writes");
    }

    let mut findings = Vec::new();
    check_d01_public_version_alignment(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new(), "aligned D01 fixture must pass");

    for (relative, old, new, expected) in [
        ("Cargo.lock", "1.8.0", "1.7.1", "Cargo.lock"),
        ("package.json", "true", "false", "private"),
        ("demo/pkg/package.json", "1.8.0", "1.7.1", "demo/pkg"),
        (
            "demo/main.js",
            "1.8.0-public",
            "1.7.1-public",
            "cache-buster",
        ),
        (
            "demo/index.html",
            "scena 1.8.0 live",
            "scena 1.5 live",
            "title",
        ),
        (
            "README.md",
            "docs.rs/scena/latest",
            "docs.rs/scena/1.7.1",
            "docs.rs",
        ),
        (
            "examples/version_policy.rs",
            "use scena::Scene;",
            "scena = \"1.7.1\"",
            "example",
        ),
        (
            "scripts/build_demo_wasm.js",
            "validateGeneratedPackageVersion",
            "validatePackage",
            "validateGeneratedPackageVersion",
        ),
    ] {
        let path = fixture_root.join(relative);
        let source = fs::read_to_string(&path).expect("D01 mutation source reads");
        let mutated = source.replace(old, new);
        assert_ne!(source, mutated, "D01 mutation alters {relative}");
        fs::write(&path, mutated).expect("D01 mutation writes");
        findings.clear();
        check_d01_public_version_alignment(&fixture_root, &mut findings);
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains(expected)),
            "D01 {relative} mutation must report {expected}: {findings:?}",
        );
        fs::write(path, source).expect("D01 fixture restores");
    }
}
