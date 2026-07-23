use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicExampleTarget {
    path: PathBuf,
    required_features: BTreeSet<String>,
}

fn public_example_targets(metadata: &Value, root: &Path) -> BTreeMap<String, PublicExampleTarget> {
    let package = metadata["packages"]
        .as_array()
        .expect("cargo metadata packages")
        .iter()
        .find(|package| {
            package["name"] == "scena"
                && Path::new(
                    package["manifest_path"]
                        .as_str()
                        .expect("package manifest path"),
                ) == root.join("Cargo.toml")
        })
        .expect("workspace scena package");

    package["targets"]
        .as_array()
        .expect("scena targets")
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "example"))
        })
        .map(|target| {
            let name = target["name"].as_str().expect("example target name");
            let path = Path::new(
                target["src_path"]
                    .as_str()
                    .expect("example target source path"),
            )
            .strip_prefix(root)
            .expect("example source belongs to workspace")
            .to_path_buf();
            let required_features = target
                .get("required-features")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|feature| {
                    feature
                        .as_str()
                        .expect("required feature must be a string")
                        .to_string()
                })
                .collect();
            (
                name.to_string(),
                PublicExampleTarget {
                    path,
                    required_features,
                },
            )
        })
        .collect()
}

fn public_example_files(root: &Path) -> BTreeSet<PathBuf> {
    std::fs::read_dir(root.join("examples"))
        .expect("examples directory")
        .map(|entry| entry.expect("example directory entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .map(|path| {
            path.strip_prefix(root)
                .expect("example source belongs to workspace")
                .to_path_buf()
        })
        .collect()
}

fn validate_manifest_coverage(
    files: &BTreeSet<PathBuf>,
    targets: &BTreeMap<String, PublicExampleTarget>,
) -> Result<(), String> {
    let target_paths = targets
        .values()
        .map(|target| target.path.clone())
        .collect::<BTreeSet<_>>();
    if files != &target_paths {
        return Err(format!(
            "public example manifest mismatch: source files={files:?}, cargo targets={target_paths:?}"
        ));
    }
    Ok(())
}

#[test]
fn cargo_metadata_covers_every_public_example_and_required_feature_combination() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("cargo metadata must execute");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("cargo metadata JSON");
    let targets = public_example_targets(&metadata, root);
    let files = public_example_files(root);
    validate_manifest_coverage(&files, &targets).unwrap();

    for (name, expected) in [
        ("scene_inspection", &["inspection"][..]),
        ("scene_host_contracts", &["scene-host"][..]),
        ("scene_host_release_1_7", &["scene-host"][..]),
        ("asset_catalog_picker", &["scene-host"][..]),
        ("product_configurator", &["scene-host"][..]),
        ("application_builder_lab", &["scene-host"][..]),
    ] {
        let actual = &targets
            .get(name)
            .unwrap_or_else(|| panic!("missing public example target {name}"))
            .required_features;
        assert_eq!(
            actual,
            &expected
                .iter()
                .map(|feature| (*feature).to_string())
                .collect(),
            "wrong required-feature manifest for {name}"
        );
    }

    let mut omitted = targets.clone();
    omitted.remove("application_builder_lab");
    assert!(
        validate_manifest_coverage(&files, &omitted).is_err(),
        "omitting a future required-feature example must fail manifest coverage"
    );
}
