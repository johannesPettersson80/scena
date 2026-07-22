use crate::app::prelude::*;

const RULE: &str = "D01-PUBLIC-VERSION-ALIGNMENT";

const HISTORICAL_VERSION_PATH_PREFIXES: &[&str] = &[
    "CHANGELOG.md",
    "docs/release-notes/",
    "docs/reviews/",
    "docs/checklists/",
    "docs/decisions/",
];

pub(crate) fn check_d01_public_version_alignment(root: &Path, findings: &mut Vec<Finding>) {
    let Some(version) = canonical_version(root, findings) else {
        return;
    };
    check_cargo_lock(root, findings, &version);
    check_node_manifests(root, findings);
    check_generated_packages(root, findings, &version);
    check_generated_size_manifests(root, findings, &version);
    check_tracked_demo_versions(root, findings, &version);
    check_builder_owner(root, findings);
    check_current_docs(root, findings);
    check_examples(root, findings);
    require_contains(
        root,
        findings,
        RULE,
        "docs/specs/release-gates.md",
        &[
            "canonical public version source",
            "HISTORICAL_VERSION_PATH_PREFIXES",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/tests_70.rs",
        &["d01_version_alignment_rejects_each_public_drift_surface"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app.rs",
        &["mod tests_70;"],
    );
    let _ = HISTORICAL_VERSION_PATH_PREFIXES;
}

fn canonical_version(root: &Path, findings: &mut Vec<Finding>) -> Option<String> {
    let source = read_required(root, findings, "Cargo.toml")?;
    let package = source.split_once("[package]")?.1;
    let package = package
        .split_once("\n[")
        .map_or(package, |(block, _)| block);
    let version = package.lines().find_map(|line| {
        line.trim()
            .strip_prefix("version")?
            .trim()
            .strip_prefix('=')?
            .trim()
            .strip_prefix('"')?
            .strip_suffix('"')
            .map(str::to_owned)
    });
    if version.is_none() {
        findings.push(Finding::new(
            RULE,
            "Cargo.toml is missing the canonical package version",
        ));
    }
    version
}

fn check_cargo_lock(root: &Path, findings: &mut Vec<Finding>, version: &str) {
    let Some(source) = read_required(root, findings, "Cargo.lock") else {
        return;
    };
    let locked = source.split("[[package]]").skip(1).find_map(|block| {
        let mut name = None;
        let mut version = None;
        for line in block.lines() {
            let line = line.trim();
            if let Some(value) = line.strip_prefix("name = \"") {
                name = value.strip_suffix('"');
            } else if let Some(value) = line.strip_prefix("version = \"") {
                version = value.strip_suffix('"');
            }
        }
        (name == Some("scena"))
            .then_some(version?)
            .map(str::to_owned)
    });
    if locked.as_deref() != Some(version) {
        findings.push(Finding::new(
            RULE,
            format!(
                "Cargo.lock scena version {} does not match Cargo.toml {version}",
                locked.as_deref().unwrap_or("<missing>")
            ),
        ));
    }
}

fn check_node_manifests(root: &Path, findings: &mut Vec<Finding>) {
    for relative in ["package.json", "package-lock.json"] {
        let Some(source) = read_required(root, findings, relative) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&source) else {
            findings.push(Finding::new(RULE, format!("{relative} is not valid JSON")));
            continue;
        };
        let root_package = if relative == "package.json" {
            &value
        } else {
            value.pointer("/packages/").unwrap_or(&Value::Null)
        };
        if root_package.get("name").and_then(Value::as_str) != Some("scena-release-gates") {
            findings.push(Finding::new(
                RULE,
                format!("{relative} root package name drifted"),
            ));
        }
        if root_package.get("version").is_some() {
            findings.push(Finding::new(
                RULE,
                format!("{relative} must remain versionless instead of duplicating crate version"),
            ));
        }
        if relative == "package.json" && value.get("private").and_then(Value::as_bool) != Some(true)
        {
            findings.push(Finding::new(
                RULE,
                "package.json release-gate tooling must remain private",
            ));
        }
    }
}

fn check_generated_packages(root: &Path, findings: &mut Vec<Finding>, version: &str) {
    for relative in ["demo/pkg/package.json", "demo/proof/pkg/package.json"] {
        let Some(source) = read_optional(root, relative) else {
            continue;
        };
        let generated = serde_json::from_str::<Value>(&source)
            .ok()
            .and_then(|value| value.get("version")?.as_str().map(str::to_owned));
        if generated.as_deref() != Some(version) {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{relative} generated version {} does not match Cargo.toml {version}",
                    generated.as_deref().unwrap_or("<missing>")
                ),
            ));
        }
    }
}

fn check_generated_size_manifests(root: &Path, findings: &mut Vec<Finding>, version: &str) {
    for relative in [
        "demo/pkg/scena_bg.wasm.size.json",
        "demo/proof/pkg/scena_bg.wasm.size.json",
    ] {
        let Some(source) = read_optional(root, relative) else {
            continue;
        };
        let generated = serde_json::from_str::<Value>(&source)
            .ok()
            .and_then(|value| value.get("crate_version")?.as_str().map(str::to_owned));
        if generated.as_deref() != Some(version) {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{relative} generated metadata version {} does not match Cargo.toml {version}",
                    generated.as_deref().unwrap_or("<missing>")
                ),
            ));
        }
    }
}

fn check_tracked_demo_versions(root: &Path, findings: &mut Vec<Finding>, version: &str) {
    for (relative, bundle) in [
        ("demo/index.html", "public"),
        ("demo/main.js", "public"),
        ("demo/proof/index.html", "proof"),
        ("demo/proof.js", "proof"),
    ] {
        let Some(source) = read_required(root, findings, relative) else {
            continue;
        };
        if !source.contains(&format!("?v={version}-{bundle}-")) {
            findings.push(Finding::new(
                RULE,
                format!("{relative} cache-buster does not use canonical version {version}"),
            ));
        }
    }
    for (relative, expected, label) in [
        (
            "demo/index.html",
            format!("<title>scena {version} live showcase</title>"),
            "title",
        ),
        (
            "demo/proof.js",
            format!("scena {version} — pick a name, not a number"),
            "proof subtitle",
        ),
    ] {
        if read_required(root, findings, relative).is_some_and(|source| !source.contains(&expected))
        {
            findings.push(Finding::new(
                RULE,
                format!("{relative} {label} does not use canonical version {version}"),
            ));
        }
    }
}

fn check_builder_owner(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        RULE,
        "scripts/build_demo_wasm.js",
        &[
            "function crateVersion()",
            "function validateGeneratedPackageVersion()",
            "function stampPublicVersionText()",
            "crate_version: crateVersion()",
            "CARGO_PROFILE_RELEASE_OPT_LEVEL",
            "stampCacheBusters(writeSizeManifest())",
        ],
    );
}

fn check_current_docs(root: &Path, findings: &mut Vec<Finding>) {
    for relative in ["README.md", "docs/README.md", "docs/api.md"] {
        let Some(source) = read_required(root, findings, relative) else {
            continue;
        };
        for line in source.lines() {
            if let Some(index) = line.find("docs.rs/scena/") {
                let suffix = &line[index + "docs.rs/scena/".len()..];
                if suffix.starts_with(|character: char| character.is_ascii_digit()) {
                    findings.push(Finding::new(
                        RULE,
                        format!("{relative} has a numeric docs.rs version outside the historical allowlist"),
                    ));
                }
            }
        }
    }
    for relative in ["docs/README.md", "docs/api.md"] {
        require_contains(root, findings, RULE, relative, &["docs.rs/scena/latest"]);
    }
    for relative in [
        "README.md",
        "docs/getting-started.md",
        "docs/feature-flags.md",
        "docs/examples.md",
    ] {
        if let Some(source) = read_required(root, findings, relative) {
            for (index, line) in source.lines().enumerate() {
                if numeric_scena_pin(line) {
                    findings.push(Finding::new(
                        RULE,
                        format!(
                            "{relative}:{} has a numeric scena dependency pin",
                            index + 1
                        ),
                    ));
                }
            }
        }
    }
}

fn check_examples(root: &Path, findings: &mut Vec<Finding>) {
    let mut files = Vec::new();
    collect_files(&root.join("examples"), &mut files);
    for path in files {
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in source.lines().enumerate() {
            if numeric_scena_pin(line) {
                let relative = path.strip_prefix(root).unwrap_or(&path).display();
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "example {relative}:{} has a numeric scena dependency pin",
                        index + 1
                    ),
                ));
            }
        }
    }
}

fn numeric_scena_pin(line: &str) -> bool {
    let compact = line.trim();
    (compact.contains("scena =")
        && compact.contains('"')
        && compact.chars().any(|c| c.is_ascii_digit()))
        || compact.contains("cargo add scena@")
}

fn collect_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
    files.sort();
}

fn read_required(root: &Path, findings: &mut Vec<Finding>, relative: &str) -> Option<String> {
    match fs::read_to_string(root.join(relative)) {
        Ok(source) => Some(source),
        Err(error) => {
            findings.push(Finding::new(
                RULE,
                format!("could not read {relative}: {error}"),
            ));
            None
        }
    }
}

fn read_optional(root: &Path, relative: &str) -> Option<String> {
    fs::read_to_string(root.join(relative)).ok()
}
