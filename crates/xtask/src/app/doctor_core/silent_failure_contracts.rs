use crate::app::prelude::*;

const RULE: &str = "FULL-REVIEW-Q06-SILENT-FAILURE-GUARDS";

pub(crate) fn check_full_review_q06_silent_failure_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    check_parallel_completion(root, findings);
    check_recipe_assembly_owner(root, findings);
    check_required_gpu_skip_evidence(root, findings);
    check_required_artifact_inventory(root, findings);
    check_public_version_alignment(root, findings);
    check_active_mutation_tests(root, findings);
    require_contains(
        root,
        findings,
        RULE,
        "docs/specs/release-gates.md",
        &[
            "Static doctor guards enforce ownership and wiring",
            "runtime correctness remains",
            "executed focused tests and rendered evidence",
        ],
    );
}

fn check_parallel_completion(root: &Path, findings: &mut Vec<Finding>) {
    let relative = "src/render/cpu_render/parallel_pass.rs";
    require_contains(
        root,
        findings,
        RULE,
        relative,
        &[
            ".reduce(CpuGeometryPassResult::default",
            "aggregate.oit_passes.max(result.oit_passes)",
        ],
    );
    for owner in ["src/render/cpu_render.rs", relative] {
        if read_optional(root, owner)
            .is_some_and(|source| source.contains(".any(|result| result.oit_passes > 0)"))
        {
            findings.push(Finding::new(
                RULE,
                format!("{owner} restores a side-effectful short-circuit parallel consumer"),
            ));
        }
    }
}

fn check_recipe_assembly_owner(root: &Path, findings: &mut Vec<Finding>) {
    for relative in [
        "src/bin/scena/input.rs",
        "src/bin/scena/scene_commands.rs",
        "src/bin/scena/verify.rs",
        "src/bin/scena/verify_animation.rs",
        "src/bin/scena/verify_interaction.rs",
        "src/bin/scena/doctor.rs",
    ] {
        let Some(source) = read_optional(root, relative) else {
            continue;
        };
        if source.contains(".imports.first()") || source.contains("recipe.imports.first()") {
            findings.push(Finding::new(
                RULE,
                format!("{relative} contains forbidden first-import recipe assembly"),
            ));
        }
    }
    require_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/input.rs",
        &[
            "ResolvedRecipeBuild",
            "scene_host_build_from_resolved_recipe",
        ],
    );
}

fn check_required_gpu_skip_evidence(root: &Path, findings: &mut Vec<Finding>) {
    let relative = "tests/c09_gpu_resource_lifecycle.rs";
    require_contains(
        root,
        findings,
        RULE,
        relative,
        &[
            "SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE",
            "required_hardware_gpu_resource_lifecycle_executes_complete_cycle",
            "write_lifecycle_artifact",
            "required-skip.json",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        relative,
        &["required_hardware_gpu_resource_lifecycle_executes_complete_cycle"],
    );
}

fn check_required_artifact_inventory(root: &Path, findings: &mut Vec<Finding>) {
    let relative = "crates/xtask/src/app/release/review_artifacts.rs";
    let Some(source) = read_required(root, findings, relative) else {
        return;
    };
    let required = source
        .split_once("pub(crate) const REQUIRED_RELEASE_ARTIFACT_SUFFIXES")
        .and_then(|(_, tail)| tail.split_once("];").map(|(block, _)| block));
    if required
        .is_none_or(|block| !block.contains("m9-platform/linux-native-vulkan/rendered-output.json"))
    {
        findings.push(Finding::new(
            RULE,
            format!("{relative} must keep Linux native rendered output existence-required"),
        ));
    }
}

fn check_public_version_alignment(root: &Path, findings: &mut Vec<Finding>) {
    let Some(cargo) = read_required(root, findings, "Cargo.toml") else {
        return;
    };
    let Some(version) = cargo_package_version(&cargo) else {
        findings.push(Finding::new(
            RULE,
            "Cargo.toml is missing the canonical [package] version",
        ));
        return;
    };

    for relative in ["demo/pkg/package.json", "demo/proof/pkg/package.json"] {
        let Some(source) = read_optional(root, relative) else {
            continue;
        };
        match serde_json::from_str::<Value>(&source)
            .ok()
            .and_then(|value| value.get("version")?.as_str().map(str::to_owned))
        {
            Some(generated) if generated == version => {}
            Some(generated) => findings.push(Finding::new(
                RULE,
                format!(
                    "{relative} version {generated} does not match canonical version {version}"
                ),
            )),
            None => findings.push(Finding::new(
                RULE,
                format!("{relative} is missing a string version"),
            )),
        }
    }

    for (relative, bundle) in [
        ("demo/index.html", "public"),
        ("demo/main.js", "public"),
        ("demo/proof/index.html", "proof"),
        ("demo/proof.js", "proof"),
    ] {
        let Some(source) = read_required(root, findings, relative) else {
            continue;
        };
        let expected = format!("?v={version}-{bundle}-");
        if !source.contains(&expected) {
            findings.push(Finding::new(
                RULE,
                format!("{relative} contains stale public version metadata; expected {expected}"),
            ));
        }
    }

    for (relative, expected) in [
        (
            "demo/index.html",
            format!("<title>scena {version} live showcase</title>"),
        ),
        (
            "demo/proof.js",
            format!("scena {version} — pick a name, not a number"),
        ),
    ] {
        if read_required(root, findings, relative).is_some_and(|source| !source.contains(&expected))
        {
            findings.push(Finding::new(
                RULE,
                format!("{relative} contains stale user-visible version text; expected {expected}"),
            ));
        }
    }

    require_contains(
        root,
        findings,
        RULE,
        "scripts/build_demo_wasm.js",
        &[
            "function crateVersion()",
            "function validateGeneratedPackageVersion()",
            "function stampPublicVersionText()",
            "validateGeneratedPackageVersion()",
        ],
    );
}

fn check_active_mutation_tests(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, names) in [
        (
            "crates/xtask/src/app/tests_36.rs",
            &[
                "c01_doctor_rejects_short_circuit_parallel_band_consumption",
                "c03_doctor_rejects_first_import_recipe_command_routing",
            ][..],
        ),
        (
            "crates/xtask/src/app/tests_41.rs",
            &["c04_every_specialized_release_artifact_is_required_for_existence"][..],
        ),
        (
            "crates/xtask/src/app/tests_69.rs",
            &[
                "q06_required_test_pins_reject_ignored_test_items",
                "q06_marker_words_do_not_bypass_unregistered_early_return",
                "q06_cross_owner_guard_rejects_each_known_silent_failure_mutation",
            ][..],
        ),
    ] {
        require_rust_test_functions(root, findings, RULE, relative, names);
    }
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app.rs",
        &["mod tests_69;"],
    );
}

fn cargo_package_version(source: &str) -> Option<String> {
    let package = source.split_once("[package]")?.1;
    let package = package
        .split_once("\n[")
        .map_or(package, |(block, _)| block);
    package.lines().find_map(|line| {
        let value = line.trim().strip_prefix("version")?.trim();
        let value = value.strip_prefix('=')?.trim();
        value
            .strip_prefix('"')?
            .strip_suffix('"')
            .map(str::to_owned)
    })
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
