use crate::app::doctor_core::check_c03_canonical_recipe_command_routing;
use crate::app::prelude::*;

#[test]
fn pf06_doctor_rejects_restored_brute_force_spatial_loops() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf06-spatial-acceleration");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/geometry.rs",
        "src/geometry/spatial.rs",
        "src/picking.rs",
        "src/picking/geometry_hit.rs",
        "src/render/prepare.rs",
        "src/render/prepare/shadows.rs",
        "src/render/prepare/shadows/cache.rs",
        "tests/pf06_spatial_acceleration.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("PF06 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("PF06 source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf06_spatial_acceleration_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let picking = fixture_root.join("src/picking.rs");
    let mut source = fs::read_to_string(&picking).expect("picking fixture reads");
    source.push_str("\n// mutation: for indices in geometry.indices().chunks_exact(3) {}\n");
    fs::write(picking, source).expect("picking mutation writes");
    findings.clear();
    check_pf06_spatial_acceleration_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF06-SHARED-SPATIAL-ACCELERATION"
                && finding.message.contains("chunks_exact(3)")
        }),
        "restoring brute-force triangle scanning must fail: {findings:?}"
    );
}

#[test]
fn pf09_doctor_rejects_restored_all_triangles_per_band_scan() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/pf09-parallel-work");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/parallel.rs",
        "src/render/prepare/environment_baker.rs",
        "src/render/prepare/environment_baker/brdf.rs",
        "src/render/cpu.rs",
        "src/render/cpu_geometry.rs",
        "src/render/cpu_render.rs",
        "src/render/cpu_render/parallel_pass.rs",
        "src/render/cpu_render/row_bands.rs",
        "src/render/cpu_render/tests.rs",
        "tests/m9_platform_release.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("PF09 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("PF09 source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf09_parallel_work_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let raster = fixture_root.join("src/render/cpu_render/parallel_pass.rs");
    let source = fs::read_to_string(&raster).expect("CPU raster fixture reads");
    let mutated = source.replace(
        "primitive_indices: Some(&row_bands.bands[chunk_index])",
        "primitive_indices: None",
    );
    assert_ne!(source, mutated, "PF09 mutation must alter the fixture");
    fs::write(raster, mutated).expect("PF09 mutation writes");
    findings.clear();
    check_pf09_parallel_work_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF09-DETERMINISTIC-PARALLEL-WORK"
                && finding.message.contains("primitive_indices")
        }),
        "restoring every-triangle band scans must fail: {findings:?}"
    );
}

#[test]
fn c01_doctor_rejects_short_circuit_parallel_band_consumption() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c01-parallel-band-completion");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/render/parallel.rs",
        "src/render/prepare/environment_baker.rs",
        "src/render/prepare/environment_baker/brdf.rs",
        "src/render/cpu.rs",
        "src/render/cpu_geometry.rs",
        "src/render/cpu_render.rs",
        "src/render/cpu_render/parallel_pass.rs",
        "src/render/cpu_render/row_bands.rs",
        "src/render/cpu_render/tests.rs",
        "tests/m9_platform_release.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C01 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C01 source fixture copies");
    }

    let mut findings = Vec::new();
    check_pf09_parallel_work_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let raster = fixture_root.join("src/render/cpu_render/parallel_pass.rs");
    let source = fs::read_to_string(&raster).expect("CPU raster fixture reads");
    let mutated = source.replace(
        ".reduce(CpuGeometryPassResult::default, |mut aggregate, result| {\n            aggregate.oit_passes = aggregate.oit_passes.max(result.oit_passes);\n            aggregate",
        ".any(|result| result.oit_passes > 0) as u64\n            .then(CpuGeometryPassResult::default)\n            .unwrap_or_default()",
    );
    assert_ne!(source, mutated, "C01 mutation must alter the fixture");
    fs::write(raster, mutated).expect("C01 mutation writes");
    findings.clear();
    check_pf09_parallel_work_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "PF09-DETERMINISTIC-PARALLEL-WORK"
                && finding.message.contains("oit_passes")
        }),
        "short-circuiting side-effectful row-band work must fail: {findings:?}"
    );
}

#[test]
fn c02_doctor_rejects_missing_packaged_template_asset_or_license() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c02-portable-agent-assets");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        ".codex/skills/scena-app-builder/SKILL.md",
        "CHANGELOG.md",
        "Cargo.toml",
        "README.md",
        "docs/assets.md",
        "docs/examples.md",
        "docs/getting-started.md",
        "docs/guides/llm-app-builder.md",
        "docs/troubleshooting.md",
        "src/assets/builtin.rs",
        "src/assets/environment_loading.rs",
        "src/assets/environment_preset.rs",
        "src/bin/scena/examples_agent.rs",
        "src/bin/scena/examples_agent/starter.rs",
        "src/scene_host/recipe/setup.rs",
        "tests/assets/environment/PRESET-LICENSES.md",
        "tests/assets/gltf/AGENT-TEMPLATE-ASSETS-LICENSE.md",
        "tests/scena_cli_agent_templates.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C02 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C02 source fixture copies");
    }

    let mut findings = Vec::new();
    crate::app::doctor_easy_scene::check_c02_portable_agent_asset_contracts(
        &fixture_root,
        &mut findings,
    );
    assert_eq!(findings, Vec::new());

    let manifest = fixture_root.join("Cargo.toml");
    let source = fs::read_to_string(&manifest).expect("C02 manifest fixture reads");
    let mutated = source.replace(
        "    \"/tests/assets/environment/PRESET-LICENSES.md\",\n",
        "",
    );
    assert_ne!(source, mutated, "C02 mutation must alter the fixture");
    fs::write(manifest, mutated).expect("C02 manifest mutation writes");
    findings.clear();
    crate::app::doctor_easy_scene::check_c02_portable_agent_asset_contracts(
        &fixture_root,
        &mut findings,
    );
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C02-PORTABLE-AGENT-ASSETS"
                && finding.message.contains("PRESET-LICENSES.md")
        }),
        "removing a bundled preset license from the package must fail: {findings:?}"
    );
}

#[test]
fn c03_doctor_rejects_first_import_recipe_command_routing() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/c03-recipe-command-routing");
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in [
        "src/bin/scena/input.rs",
        "src/bin/scena/scene_commands.rs",
        "src/bin/scena/verify.rs",
        "src/bin/scena/verify_animation.rs",
        "src/bin/scena/verify_interaction.rs",
        "src/bin/scena/doctor.rs",
        "tests/scena_cli_recipe.rs",
    ] {
        let destination = fixture_root.join(relative);
        fs::create_dir_all(destination.parent().expect("fixture file has parent"))
            .expect("C03 fixture directory creates");
        fs::copy(root.join(relative), &destination).expect("C03 source fixture copies");
    }

    let mut findings = Vec::new();
    check_c03_canonical_recipe_command_routing(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    let input = fixture_root.join("src/bin/scena/input.rs");
    let mut source = fs::read_to_string(&input).expect("C03 input fixture reads");
    source.push_str("\n// mutation: recipe.imports.first()\n");
    fs::write(input, source).expect("C03 mutation writes");
    findings.clear();
    check_c03_canonical_recipe_command_routing(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "C03-CANONICAL-RECIPE-COMMAND-ROUTING"
                && finding.message.contains(".imports.first()")
        }),
        "restoring first-import recipe assembly must fail: {findings:?}"
    );
}

#[test]
fn renderer_owned_interior_mutability_is_not_misclassified_as_a_singleton() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/render-singleton-scan");
    let source_path = fixture_root.join("src/render/cache.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source_path.parent().expect("fixture file has parent"))
        .expect("singleton fixture directory creates");
    fs::write(
        &source_path,
        "use std::cell::RefCell;\nstruct Cache { values: RefCell<Vec<u32>> }\nimpl Cache { fn new() -> Self { Self { values: RefCell::new(Vec::new()) } } }\n",
    )
    .expect("renderer-owned cache fixture writes");

    let mut findings = Vec::new();
    check_render_singleton_contracts(&fixture_root, &mut findings);
    assert_eq!(findings, Vec::new());

    fs::write(
        source_path,
        "use std::cell::RefCell;\nstatic CACHE: RefCell<Vec<u32>> = RefCell::new(Vec::new());\n",
    )
    .expect("global singleton mutation writes");
    findings.clear();
    check_render_singleton_contracts(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "ARCH-RENDER-SINGLETON"
                && finding.message.contains("src/render/cache.rs:2")
        }),
        "global render state must still fail: {findings:?}"
    );
}

#[test]
fn positive_contract_pins_follow_owned_split_module_files() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/split-module-contract-pin");
    let parent = fixture_root.join("src/render.rs");
    let child = fixture_root.join("src/render/frame.rs");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(child.parent().expect("split module parent exists"))
        .expect("split module fixture directory creates");
    fs::write(&parent, "mod frame;\n").expect("split module owner writes");
    fs::write(&child, "pub fn retained_render_contract() {}\n").expect("split module child writes");

    let mut findings = Vec::new();
    require_contains(
        &fixture_root,
        &mut findings,
        "SPLIT-MODULE-PIN",
        "src/render.rs",
        &["retained_render_contract"],
    );
    assert_eq!(findings, Vec::new());

    fs::write(child, "pub fn removed_contract() {}\n").expect("split module mutation writes");
    findings.clear();
    require_contains(
        &fixture_root,
        &mut findings,
        "SPLIT-MODULE-PIN",
        "src/render.rs",
        &["retained_render_contract"],
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SPLIT-MODULE-PIN"),
        "removing a child-module contract must still fail: {findings:?}"
    );
}
