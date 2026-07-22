use crate::app::prelude::*;

#[test]
fn q06_required_test_pins_reject_ignored_test_items() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q06-ignored-required-test");
    let source = fixture_root.join("tests/proof.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source.parent().expect("test fixture parent"))
        .expect("test fixture directory creates");
    fs::write(
        source,
        "#[test]\n#[ignore = \"optional on this host\"]\nfn required_rendered_output_proof() {}\n",
    )
    .expect("ignored test fixture writes");
    let mut findings = Vec::new();

    require_rust_test_functions(
        &fixture_root,
        &mut findings,
        "Q06-ACTIVE-TEST-PIN",
        "tests/proof.rs",
        &["required_rendered_output_proof"],
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q06-ACTIVE-TEST-PIN"
                && finding.message.contains("required_rendered_output_proof")
                && finding.message.contains("active")
        }),
        "an ignored test must not satisfy an active required-test pin: {findings:?}",
    );
}

#[test]
fn q06_marker_words_do_not_bypass_unregistered_early_return() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q06-marker-bypass");
    let source = fixture_root.join("tests/proof.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source.parent().expect("test fixture parent"))
        .expect("test fixture directory creates");
    fs::write(
        source,
        "// fail_closed release_evidence\n#[test]\nfn required_proof() { if adapter_unavailable() { return; } }\n",
    )
    .expect("marker-bypass fixture writes");
    let mut findings = Vec::new();

    check_test_control_flow_policy(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "TESTS-CONTROL-FLOW-POLICY"
                && finding.message.contains("tests/proof.rs")
        }),
        "marker words must not exempt an unregistered early return: {findings:?}",
    );
}

#[test]
fn q06_cross_owner_guard_rejects_each_known_silent_failure_mutation() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q06-cross-owner-guards");
    let _ = fs::remove_dir_all(&fixture_root);
    let files = [
        (
            "Cargo.toml",
            "[package]\nname = \"scena\"\nversion = \"1.8.0\"\n",
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
            "demo/index.html",
            "<title>scena 1.8.0 live showcase</title>\nmain.js?v=1.8.0-public-fixture\n",
        ),
        ("demo/main.js", "pkg/scena.js?v=1.8.0-public-fixture\n"),
        ("demo/proof/index.html", "proof.js?v=1.8.0-proof-fixture\n"),
        (
            "demo/proof.js",
            "scena 1.8.0 — pick a name, not a number\nproof/pkg/scena.js?v=1.8.0-proof-fixture\n",
        ),
        (
            "scripts/build_demo_wasm.js",
            "function crateVersion() {}\nfunction validateGeneratedPackageVersion() {}\nfunction stampPublicVersionText() {}\nvalidateGeneratedPackageVersion();\n",
        ),
        (
            "src/render/cpu_render/parallel_pass.rs",
            ".reduce(CpuGeometryPassResult::default, |mut aggregate, result| {\n    aggregate.oit_passes = aggregate.oit_passes.max(result.oit_passes);\n    aggregate\n})\n",
        ),
        (
            "src/bin/scena/input.rs",
            "pub(crate) enum ResolvedRecipeBuild {}\nscene_host_build_from_resolved_recipe\n",
        ),
        (
            "tests/c09_gpu_resource_lifecycle.rs",
            "const REQUIRED_LIFECYCLE_ENV: &str = \"SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE\";\n#[test]\nfn required_hardware_gpu_resource_lifecycle_executes_complete_cycle() { write_lifecycle_artifact(\"required-skip.json\", &()); }\n",
        ),
        (
            "crates/xtask/src/app/release/review_artifacts.rs",
            "pub(crate) const REQUIRED_RELEASE_ARTIFACT_SUFFIXES: &[&str] = &[\n    \"m9-platform/linux-native-vulkan/rendered-output.json\",\n];\n",
        ),
        (
            "crates/xtask/src/app/tests_36.rs",
            "#[test]\nfn c01_doctor_rejects_short_circuit_parallel_band_consumption() {}\n#[test]\nfn c03_doctor_rejects_first_import_recipe_command_routing() {}\n",
        ),
        (
            "crates/xtask/src/app/tests_41.rs",
            "#[test]\nfn c04_every_specialized_release_artifact_is_required_for_existence() {}\n",
        ),
        (
            "crates/xtask/src/app/tests_69.rs",
            "#[test]\nfn q06_required_test_pins_reject_ignored_test_items() {}\n#[test]\nfn q06_marker_words_do_not_bypass_unregistered_early_return() {}\n#[test]\nfn q06_cross_owner_guard_rejects_each_known_silent_failure_mutation() {}\n",
        ),
        ("crates/xtask/src/app.rs", "#[cfg(test)]\nmod tests_69;\n"),
        (
            "docs/specs/release-gates.md",
            "Static doctor guards enforce ownership and wiring; runtime correctness remains owned by executed focused tests and rendered evidence.\n",
        ),
    ];
    for (relative, contents) in files {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("Q06 fixture parent"))
            .expect("Q06 fixture directory creates");
        fs::write(destination, contents).expect("Q06 fixture writes");
    }

    let mut findings = Vec::new();
    check_full_review_q06_silent_failure_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new(), "strong Q06 fixture must pass");

    let mutations = [
        (
            "src/render/cpu_render/parallel_pass.rs",
            ".reduce(CpuGeometryPassResult::default, |mut aggregate, result| {\n    aggregate.oit_passes = aggregate.oit_passes.max(result.oit_passes);\n    aggregate\n})\n",
            ".any(|result| result.oit_passes > 0)\n",
            "short-circuit",
        ),
        (
            "src/bin/scena/input.rs",
            "scene_host_build_from_resolved_recipe",
            "recipe.imports.first()",
            "first-import",
        ),
        (
            "tests/c09_gpu_resource_lifecycle.rs",
            "write_lifecycle_artifact(\"required-skip.json\", &());",
            "return;",
            "write_lifecycle_artifact",
        ),
        (
            "crates/xtask/src/app/release/review_artifacts.rs",
            "    \"m9-platform/linux-native-vulkan/rendered-output.json\",\n",
            "",
            "existence-required",
        ),
        ("demo/main.js", "1.8.0-public", "1.7.1-public", "version"),
    ];
    for (relative, old, new, expected) in mutations {
        let path = fixture_root.join(relative);
        let source = fs::read_to_string(&path).expect("Q06 mutation source reads");
        let mutated = source.replace(old, new);
        assert_ne!(source, mutated, "Q06 mutation must alter {relative}");
        fs::write(&path, &mutated).expect("Q06 mutation writes");
        findings.clear();
        check_full_review_q06_silent_failure_contracts(&fixture_root, &mut findings);
        assert!(
            findings
                .iter()
                .any(|finding| finding.message.contains(expected)),
            "Q06 {relative} mutation must be rejected with {expected}: {findings:?}",
        );
        fs::write(path, source).expect("Q06 fixture restores");
    }
}
