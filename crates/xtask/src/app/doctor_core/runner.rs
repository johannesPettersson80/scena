use crate::app::prelude::*;

use super::control_flow_allowlist::DIAGNOSTIC_EARLY_RETURNS;

mod env_contract;
pub(crate) use env_contract::{check_tests_env_flags_documented, find_env_var_names};

pub(crate) fn repo_root() -> Result<PathBuf, String> {
    let mut dir = env::current_dir().map_err(|error| error.to_string())?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs/README.md").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not find scena repo root".to_string());
        }
    }
}

pub(crate) fn run_docs_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "DOCS-REQUIRED", REQUIRED_DOCS);
    check_current_release_document_version(root, findings);
    check_markdown_links(root, findings);
    check_for_stale_doc_terms(root, findings);
    check_shipped_feature_status_drift(root, findings);
    check_review_provenance_contracts(root, findings);
    check_c11_onboarding_contracts(root, findings);
    check_d01_public_version_alignment(root, findings);
    check_fr01_fr04_contract_discovery(root, findings);
    check_required_doc_contracts(root, findings);
    check_stable_contract_release_evidence(root, findings);
    check_easy_scene_setup_contracts(root, findings);
    check_demo_build_heartbeat_contract(root, findings);
    check_default_environment_manifest(root, findings);
    check_visual_fixture_metadata(root, findings);
    check_m2_visual_fixture_metadata(root, findings);
    check_m1_browser_rendered_output(root, findings);
    check_m2_browser_rendered_output(root, findings);
    check_m6_browser_renderer_probe(root, findings);
    check_q01_required_webgpu_pixel_parity(root, findings);
    check_q04_browser_evidence_classification(root, findings);
    check_q03_m2_local_structure(root, findings);
    check_q04_cpu_webgl2_parity_contracts(root, findings);
    check_q05_effect_footprint_contracts(root, findings);
    check_q06_required_gpu_lane_contracts(root, findings);
    check_q07_antialiasing_effect_contract(root, findings);
    check_q08_required_physical_parity(root, findings);
    check_q09_structured_adapter_expectations(root, findings);
    check_q10_rendered_waterbottle_mutations(root, findings);
    check_q11_reference_stability(root, findings);
    check_q12_semantic_doctor_contracts(root, findings);
    check_full_review_q06_silent_failure_contracts(root, findings);
    check_gltf_asset_matrix_contract(root, findings);
    check_m9_ci_release_lanes(root, findings);
    check_q01_waterbottle_cpu_proof(root, findings);
    check_q02_round_e_material_proof(root, findings);
    check_feature_specific_visual_oracles(root, findings);
    check_pf00_performance_truth_contracts(root, findings);
    check_pf03_pf05_hot_path_contracts(root, findings);
    check_pf06_spatial_acceleration_contracts(root, findings);
    check_pf07_pf08_cpu_prepare_contracts(root, findings);
    check_pf09_parallel_work_contracts(root, findings);
    check_pf10_hot_path_contracts(root, findings);
    check_release_readiness_ci_fail_closed(root, findings);
    check_c04_release_readiness_contract(root, findings);
    check_release_publish_dry_run_helper(root, findings);
    check_m10_claim_audit_contract(root, findings);
    check_state_of_art_checklist_links(root, findings);
}

pub(crate) fn check_current_release_document_version(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "DOCS-CURRENT-RELEASE-VERSION";
    let manifest_path = root.join("Cargo.toml");
    let Ok(manifest) = fs::read_to_string(&manifest_path) else {
        findings.push(Finding::new(RULE, "could not read root Cargo.toml"));
        return;
    };
    let package_version = manifest
        .lines()
        .skip_while(|line| line.trim() != "[package]")
        .skip(1)
        .take_while(|line| !line.trim().starts_with('['))
        .find_map(|line| {
            line.trim()
                .strip_prefix("version = ")
                .map(|value| value.trim_matches('"'))
        });
    if package_version != Some(CURRENT_RELEASE_VERSION) {
        findings.push(Finding::new(
            RULE,
            format!(
                "current release document version {CURRENT_RELEASE_VERSION} does not match root package version {package_version:?}"
            ),
        ));
    }
    for relative in [
        CURRENT_RELEASE_NOTES,
        CURRENT_REVIEW_REPORT,
        CURRENT_REMEDIATION_CHECKLIST,
    ] {
        if !root.join(relative).is_file() {
            findings.push(Finding::new(
                RULE,
                format!("current versioned document is missing: {relative}"),
            ));
        }
        if !relative.contains(CURRENT_RELEASE_VERSION) {
            findings.push(Finding::new(
                RULE,
                format!(
                    "current versioned document {relative} does not use {CURRENT_RELEASE_VERSION}"
                ),
            ));
        }
    }
}

pub(crate) fn run_architecture_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "ARCH-REQUIRED", REQUIRED_SOURCE_MODULES);
    check_source_scope(root, findings);
    check_architecture_contract(root, findings);
    check_xtask_module_split(root, findings);
    check_module_boundaries(root, findings);
    check_architecture_dependency_direction(root, findings);
    check_public_api_ownership(root, findings);
    check_viewer_facade_contracts(root, findings);
    check_render_singleton_contracts(root, findings);
    check_asset_api_contracts(root, findings);
    check_prepare_asset_contracts(root, findings);
    check_c05_primitive_winding_contract(root, findings);
    check_c07_target_transfer_contract(root, findings);
    check_particle_prepare_allocation_contract(root, findings);
    check_environment_lifecycle_contracts(root, findings);
    check_equirectangular_hdr_environment_contracts(root, findings);
    check_environment_ibl_prepare_contracts(root, findings);
    check_calibration_oracles_pair_parity_sweeps(root, findings);
    check_scene_light_contracts(root, findings);
    check_direct_light_shading_contracts(root, findings);
    check_directional_shadow_contracts(root, findings);
    check_shadow_map_contracts(root, findings);
    check_depth_prepass_contracts(root, findings);
    check_reversed_z_contracts(root, findings);
    check_webgl2_depth_contracts(root, findings);
    check_m2_leak_stats_contracts(root, findings);
    check_camera_depth_contracts(root, findings);
    check_origin_shift_contracts(root, findings);
    check_clipping_contracts(root, findings);
    check_m3a_scene_import_contracts(root, findings);
    check_m3b_animation_contracts(root, findings);
    check_m4_platform_contracts(root, findings);
    check_m5_release_contracts(root, findings);
    check_cli_output_contracts(root, findings);
    check_agent_contracts(root, findings);
    check_m7_ergonomics_contracts(root, findings);
    check_m8_assets_materials_contracts(root, findings);
    check_tangent_generation_dependency_contracts(root, findings);
    check_binary_render_asset_contracts(root, findings);
    check_render_alpha_contracts(root, findings);
    check_output_stage_contracts(root, findings);
    check_fxaa_output_contracts(root, findings);
    check_headless_gpu_test_guard_contracts(root, findings);
    check_diagnostics_contracts(root, findings);
    check_renderer_stats_contracts(root, findings);
    check_renderer_truth_contracts(root, findings);
    check_render_movement_contracts(root, findings);
    check_render_quality_contracts(root, findings);
    check_render_quality_reflection_contracts(root, findings);
    check_material_reflection_quality_contracts(root, findings);
    check_render_world_bake_contracts(root, findings);
    check_solid_kiss(root, findings);
    check_backend_vocabulary(root, findings);
    check_unit_test_first_governance(root, findings);
    check_agent_validation(root, findings);
    check_remote_builder_bootstrap_contracts(root, findings);
    check_recipe_build_policy_boundary(root, findings);
    check_fr05_capture_sequence_contracts(root, findings);
    check_fr06_semantic_aov_contracts(root, findings);
    check_fr07_recipe_diff_contracts(root, findings);
    check_fr08_recipe_spatial_state_contracts(root, findings);
    check_tests_env_flags_documented(root, findings);
    check_feature_gated_contract_tests_documented(root, findings);
    check_feature_ownership_contracts(root, findings);
    check_q07_claim_truth_contracts(root, findings);
    check_no_ignored_release_tests(root, findings);
    check_test_control_flow_policy(root, findings);
    check_m8_real_asset_dual_lane(root, findings);
    check_cpu_ibl_gap_documented(root, findings);
    check_waterbottle_third_party_reference(root, findings);
}

/// `M8-WATERBOTTLE-THIRD-PARTY-REFERENCE`: the m8 WaterBottle proof
/// must ship a third-party PBR reference (Blender Cycles render) in
/// addition to the scena-gold regression baseline. The scena-gold
/// reference catches future drift; the Blender reference is the
/// answer to "is scena's output canonically correct".
pub(crate) fn check_waterbottle_third_party_reference(root: &Path, findings: &mut Vec<Finding>) {
    let blender_png =
        root.join("tests/assets/gltf/khronos/WaterBottle/reference_blender_cycles_512.png");
    let blender_script =
        root.join("tests/assets/gltf/khronos/WaterBottle/render_blender_reference.py");
    let metadata = root.join("tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml");
    if !blender_png.is_file() {
        findings.push(Finding::new(
            "M8-WATERBOTTLE-THIRD-PARTY-REFERENCE",
            "tests/assets/gltf/khronos/WaterBottle/reference_blender_cycles_512.png \
             must exist (Blender Cycles third-party reference render)"
                .to_string(),
        ));
    }
    if !blender_script.is_file() {
        findings.push(Finding::new(
            "M8-WATERBOTTLE-THIRD-PARTY-REFERENCE",
            "tests/assets/gltf/khronos/WaterBottle/render_blender_reference.py \
             must exist so the Blender reference is reproducible"
                .to_string(),
        ));
    }
    let Ok(metadata_text) = fs::read_to_string(&metadata) else {
        findings.push(Finding::new(
            "M8-WATERBOTTLE-THIRD-PARTY-REFERENCE",
            "reference_metadata.toml must exist and document both \
             scena-gold and blender_cycles references"
                .to_string(),
        ));
        return;
    };
    for needle in [
        "[scena_gold]",
        "[blender_cycles]",
        "third-party PBR validation",
    ] {
        if !metadata_text.contains(needle) {
            findings.push(Finding::new(
                "M8-WATERBOTTLE-THIRD-PARTY-REFERENCE",
                format!("reference_metadata.toml missing required marker '{needle}'"),
            ));
        }
    }
    let test_path = root.join("tests/m8_real_asset_proof.rs");
    let Ok(test_text) = fs::read_to_string(&test_path) else {
        findings.push(Finding::new(
            "M8-WATERBOTTLE-THIRD-PARTY-REFERENCE",
            "tests/m8_real_asset_proof.rs must contain the WaterBottle third-party \
             comparison test"
                .to_string(),
        ));
        return;
    };
    for needle in [
        "PngImage::read(WATERBOTTLE_BLENDER_REFERENCE_PNG)",
        "PngImage::read(WATERBOTTLE_REFERENCE_PNG)",
        "assert_olive_yellow(\"scena body\"",
        "assert_dark_burgundy(\"scena cap\"",
    ] {
        if !test_text.contains(needle) {
            findings.push(Finding::new(
                "M8-WATERBOTTLE-THIRD-PARTY-REFERENCE",
                format!("tests/m8_real_asset_proof.rs missing required marker '{needle}'"),
            ));
        }
    }
}

/// `CPU-IBL-GAP-DOCUMENTED`: the public headless-rendering docs must keep
/// CPU/GPU rendered-output paths explicit so reviewers can tell which path
/// produced an artifact.
pub(crate) fn check_cpu_ibl_gap_documented(root: &Path, findings: &mut Vec<Finding>) {
    let spec_path = root.join("docs/headless-rendering.md");
    let Ok(spec_text) = fs::read_to_string(&spec_path) else {
        findings.push(Finding::new(
            "CPU-IBL-GAP-DOCUMENTED",
            "docs/headless-rendering.md must exist and describe CPU/GPU headless output",
        ));
        return;
    };
    for needle in [
        "Headless rendering",
        "CPU",
        "GPU",
        "Renderer::headless",
        "metadata",
    ] {
        if !spec_text.contains(needle) {
            findings.push(Finding::new(
                "CPU-IBL-GAP-DOCUMENTED",
                format!("docs/headless-rendering.md missing required text '{needle}'"),
            ));
        }
    }
    let test_text = match fs::read_to_string(root.join("tests/m8_real_asset_proof.rs")) {
        Ok(t) => t,
        Err(_) => return,
    };
    if !test_text.contains("ibl_specular_path") {
        findings.push(Finding::new(
            "CPU-IBL-GAP-DOCUMENTED",
            "tests/m8_real_asset_proof.rs must emit ibl_specular_path \
             in the renderer metadata so reviewers can tell which \
             IBL path produced the artifact"
                .to_string(),
        ));
    }
}

/// `M8-REAL-ASSET-DUAL-LANE`: the m8 WaterBottle proof must be split into
/// a hard-required GPU headline lane (region asserts + diff) and a
/// CPU release-quality lane. Both must produce their own artifact under
/// `target/gate-artifacts/m8-real-asset/`. Catches regressions where
/// someone collapses the two lanes back into one and silently passes
/// either by the loose bar or by the CPU lane masking GPU breakage.
pub(crate) fn check_m8_real_asset_dual_lane(root: &Path, findings: &mut Vec<Finding>) {
    let test_path = root.join("tests/m8_real_asset_proof.rs");
    let Ok(text) = fs::read_to_string(&test_path) else {
        findings.push(Finding::new(
            "M8-REAL-ASSET-DUAL-LANE",
            "could not read tests/m8_real_asset_proof.rs".to_string(),
        ));
        return;
    };
    let required = [
        "fn m8_real_asset_waterbottle_gpu_headline",
        "fn m8_real_asset_waterbottle_cpu_release_quality",
        "ARTIFACT_GPU_PNG",
        "ARTIFACT_GPU_RESULT_JSON",
        "ARTIFACT_CPU_PNG",
        "scena.m8.waterbottle_gpu_result.v1",
        "write_gpu_release_result",
        "source_checksums",
        "region_checks_passed",
        "evaluate_waterbottle_reference_diff",
        "horizontal_mirror",
        "FULL_FRAME_REFERENCE_DIFF_NOT_RUN",
        "waterbottle_diff.png",
        "worst_region_bbox",
        "scena.gpu_adapter_key.v1",
        "Renderer::headless_gpu",
        "Renderer::headless(",
        "build_waterbottle_scene",
        "Microsoft Basic Render Driver",
        "software-dx12",
        "SCENA_REFERENCE_DIFF",
    ];
    for needle in required {
        if !text.contains(needle) {
            findings.push(Finding::new(
                "M8-REAL-ASSET-DUAL-LANE",
                format!(
                    "tests/m8_real_asset_proof.rs missing required contract text '{needle}'; \
             the m8 WaterBottle proof must keep its GPU-headline + CPU-release split",
                ),
            ));
        }
    }
    // Reject the old combined test name — if it comes back, the split was
    // undone.
    if text.contains("fn m8_real_asset_waterbottle_imports_and_renders") {
        findings.push(Finding::new(
            "M8-REAL-ASSET-DUAL-LANE",
            "tests/m8_real_asset_proof.rs contains the legacy combined test name \
             `m8_real_asset_waterbottle_imports_and_renders`; the Phase 3 split \
             replaced it with gpu_headline + cpu_release_quality lanes"
                .to_string(),
        ));
    }
}

/// `TESTS-NO-IGNORED-RELEASE-PROOF`: release-relevant evidence must not be
/// hidden behind `#[ignore]`. Adapter-sensitive lanes should run by explicit
/// env var and otherwise write fail-closed `release_evidence=false` metadata.
pub(crate) fn check_no_ignored_release_tests(root: &Path, findings: &mut Vec<Finding>) {
    let mut entries = Vec::new();
    collect_test_contract_sources(&root.join("tests"), &mut entries);
    entries.sort();
    for path in entries {
        if !is_env_contract_source(&path) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");
        let ignored = if extension == "rs" {
            text.contains("#[ignore")
        } else {
            ["test.skip(", "it.skip(", "describe.skip(", ".skip("]
                .iter()
                .any(|marker| text.contains(marker))
        };
        if ignored {
            let display = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            findings.push(Finding::new(
                "TESTS-NO-IGNORED-RELEASE-PROOF",
                format!(
                    "{display} contains #[ignore]; replace ignored proof with env-gated \
                     fail-closed release_evidence=false metadata or move it out of the \
                     release proof suite",
                ),
            ));
        }
    }
}

/// Recursively audits test/script control-flow escape hatches. Exact legacy
/// diagnostic paths remain visible with an owner/rationale while required
/// release workflows are checked separately and may not opt into unavailable
/// success. New early-return files or new allow-unavailable readers fail.
pub(crate) fn check_test_control_flow_policy(root: &Path, findings: &mut Vec<Finding>) {
    const DIAGNOSTIC_ALLOW_UNAVAILABLE: &str = "tests/browser/m6_rust_wasm_renderer_probe.js";

    let mut entries = Vec::new();
    collect_test_contract_sources(&root.join("tests"), &mut entries);
    collect_test_contract_sources(&root.join("scripts"), &mut entries);
    entries.sort();
    for path in entries {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let display = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if text.contains("SCENA_BROWSER_ALLOW_UNAVAILABLE") {
            if display == DIAGNOSTIC_ALLOW_UNAVAILABLE {
                eprintln!(
                    "scena doctor: diagnostic-only allow-unavailable reader {} (required workflows forbid the flag)",
                    display
                );
            } else {
                findings.push(Finding::new(
                    "TESTS-CONTROL-FLOW-POLICY",
                    format!(
                        "{display} reads SCENA_BROWSER_ALLOW_UNAVAILABLE outside the one diagnostic-only browser probe"
                    ),
                ));
            }
        }
        if path.extension().and_then(OsStr::to_str) != Some("rs") || !text.contains("return;") {
            continue;
        }
        if let Some(entry) = DIAGNOSTIC_EARLY_RETURNS
            .iter()
            .find(|entry| entry.path == display)
        {
            eprintln!(
                "scena doctor: diagnostic early-return path {} (owner: {}; rationale: {})",
                entry.path, entry.owner, entry.rationale
            );
            continue;
        }
        findings.push(Finding::new(
            "TESTS-CONTROL-FLOW-POLICY",
            format!(
                "{display} contains an unregistered early return; make it fail closed or add an exact owner/rationale diagnostic policy"
            ),
        ));
    }
}

/// Scan a Rust source for `std::env::var("NAME")` / `env::var("NAME")` reads
/// and return the literal NAME strings. Best-effort: handles the common
/// `env::var("FOO")` and `std::env::var("FOO")` call shapes; macro-built
/// names are not detected.
pub(super) fn collect_test_contract_sources(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_test_contract_sources(&path, files);
        } else if is_env_contract_source(&path) {
            files.push(path);
        }
    }
}

pub(super) fn is_env_contract_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("rs" | "js" | "mjs" | "cjs" | "ts" | "tsx")
    )
}

pub(crate) const REQUIRED_DOCS: &[&str] = &[
    "AGENTS.md",
    "README.md",
    "CHANGELOG.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "docs/RFC-rust-3d-renderer.md",
    "docs/README.md",
    "docs/api.md",
    "docs/getting-started.md",
    "docs/examples.md",
    "docs/platforms.md",
    "docs/assets.md",
    "docs/rendering.md",
    "docs/headless-rendering.md",
    "docs/browser.md",
    "docs/troubleshooting.md",
    "docs/lifecycle.md",
    "docs/capabilities.md",
    "docs/errors.md",
    "docs/feature-flags.md",
    "docs/specs/release-gates.md",
    "docs/guides/authoring-gltf-anchors-connectors.md",
    "docs/guides/easy-scene-setup.md",
    "docs/guides/migrating-from-threejs.md",
    "docs/guides/place-and-connect-objects.md",
    "docs/guides/troubleshooting-misplaced-assets.md",
    "docs/guides/units-axes-handedness.md",
    CURRENT_RELEASE_NOTES,
    CURRENT_REVIEW_REPORT,
    CURRENT_REMEDIATION_CHECKLIST,
    "docs/release-notes/v1.8.0.md",
    "docs/release-notes/v1.7.2.md",
    "docs/release-notes/v1.7.1.md",
    "docs/release-notes/v1.7.0.md",
    "docs/release-notes/v1.5.0.md",
    "docs/release-notes/v1.4.0.md",
    "docs/release-notes/v1.3.0.md",
    "docs/release-notes/v1.2.0.md",
    ".codex/skills/scena-doctor/SKILL.md",
    ".codex/skills/scena-git-github/SKILL.md",
    ".codex/skills/scena-gltf-assets/SKILL.md",
    ".codex/skills/scena-release-hygiene/SKILL.md",
    ".codex/skills/scena-renderer-architecture/SKILL.md",
    ".codex/skills/scena-renderer-quality/SKILL.md",
    ".codex/skills/scena-rfc-governance/SKILL.md",
    ".claude/agents/scena-doctor-reviewer.md",
];
