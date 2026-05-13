use crate::app::prelude::*;

pub(crate) fn run_doctor(mode: DoctorMode) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("DOCTOR-ROOT", message)])?;
    let mut findings = Vec::new();

    match mode {
        DoctorMode::Docs => run_docs_doctor(&root, &mut findings),
        DoctorMode::Architecture => run_architecture_doctor(&root, &mut findings),
        DoctorMode::Full => {
            run_docs_doctor(&root, &mut findings);
            run_architecture_doctor(&root, &mut findings);
        }
    }

    if findings.is_empty() {
        println!("scena doctor: mode={mode:?} status=pass");
        Ok(())
    } else {
        Err(findings)
    }
}

pub(crate) fn repo_root() -> Result<PathBuf, String> {
    let mut dir = env::current_dir().map_err(|error| error.to_string())?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs/RFC-rust-3d-renderer.md").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not find scena repo root".to_string());
        }
    }
}

pub(crate) fn run_docs_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "DOCS-REQUIRED", REQUIRED_DOCS);
    check_markdown_links(root, findings);
    check_for_stale_doc_terms(root, findings);
    check_required_doc_contracts(root, findings);
    check_default_environment_manifest(root, findings);
    check_visual_fixture_metadata(root, findings);
    check_m2_visual_fixture_metadata(root, findings);
    check_m1_browser_rendered_output(root, findings);
    check_m2_browser_rendered_output(root, findings);
    check_m6_browser_renderer_probe(root, findings);
    check_gltf_asset_matrix_contract(root, findings);
    check_m9_ci_release_lanes(root, findings);
    check_release_readiness_ci_fail_closed(root, findings);
    check_release_publish_dry_run_helper(root, findings);
    check_m10_claim_audit_contract(root, findings);
    check_state_of_art_checklist_links(root, findings);
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
    check_environment_lifecycle_contracts(root, findings);
    check_equirectangular_hdr_environment_contracts(root, findings);
    check_environment_ibl_prepare_contracts(root, findings);
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
    check_m7_ergonomics_contracts(root, findings);
    check_m8_assets_materials_contracts(root, findings);
    check_tangent_generation_dependency_contracts(root, findings);
    check_binary_render_asset_contracts(root, findings);
    check_render_alpha_contracts(root, findings);
    check_output_stage_contracts(root, findings);
    check_fxaa_output_contracts(root, findings);
    check_diagnostics_contracts(root, findings);
    check_renderer_stats_contracts(root, findings);
    check_renderer_truth_contracts(root, findings);
    check_render_world_bake_contracts(root, findings);
    check_solid_kiss(root, findings);
    check_backend_vocabulary(root, findings);
    check_unit_test_first_governance(root, findings);
    check_agent_validation(root, findings);
    check_tests_env_flags_documented(root, findings);
    check_no_ignored_release_tests(root, findings);
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

/// `CPU-IBL-GAP-DOCUMENTED`: the CPU rasterizer's IBL contract must stay
/// explicit. Earlier releases documented scalar approximation as a known gap;
/// current releases document the split-sum CPU path and keep renderer metadata
/// exposing `ibl_specular_path` so reviewers can tell which path ran.
pub(crate) fn check_cpu_ibl_gap_documented(root: &Path, findings: &mut Vec<Finding>) {
    let spec_path = root.join("docs/specs/cpu-rasterizer-ibl-gap.md");
    let Ok(spec_text) = fs::read_to_string(&spec_path) else {
        findings.push(Finding::new(
            "CPU-IBL-GAP-DOCUMENTED",
            "docs/specs/cpu-rasterizer-ibl-gap.md must exist and \
             describe the CPU vs GPU IBL specular gap"
                .to_string(),
        ));
        return;
    };
    for needle in [
        "split_sum",
        "CPU split-sum",
        "renderer_path",
        "Renderer::headless",
        "Renderer::headless_gpu",
    ] {
        if !spec_text.contains(needle) {
            findings.push(Finding::new(
                "CPU-IBL-GAP-DOCUMENTED",
                format!("docs/specs/cpu-rasterizer-ibl-gap.md missing required text '{needle}'"),
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
        "ARTIFACT_CPU_PNG",
        "Renderer::headless_gpu",
        "Renderer::headless(",
        "build_waterbottle_scene",
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

/// `TESTS-ENV-FLAGS-DOCUMENTED`: every non-standard env var that a test under
/// `tests/` reads must be listed in `CLAUDE.md`'s "Test environment flags"
/// section so contributors can discover them without grep. Standard cargo /
/// rust vars (`RUST_LOG`, `RUST_BACKTRACE`, `CARGO_*`, `OUT_DIR`, `TMPDIR`)
/// are exempt.
pub(crate) fn check_tests_env_flags_documented(root: &Path, findings: &mut Vec<Finding>) {
    const STANDARD_EXEMPTIONS: &[&str] = &[
        "RUST_LOG",
        "RUST_BACKTRACE",
        "OUT_DIR",
        "TMPDIR",
        "HOME",
        "PATH",
        "CARGO",
        "CI",
        "TARGET",
        "GITHUB_SHA",
        "GITHUB_RUN_ID",
        "GITHUB_REPOSITORY",
    ];
    let claude_md = match fs::read_to_string(root.join("CLAUDE.md")) {
        Ok(text) => text,
        Err(_) => {
            findings.push(Finding::new(
                "TESTS-ENV-FLAGS-DOCUMENTED",
                "CLAUDE.md must exist and list test environment flags".to_string(),
            ));
            return;
        }
    };
    let Ok(read_dir) = fs::read_dir(root.join("tests")) else {
        return;
    };
    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        entries.push(entry.path());
    }
    entries.sort();
    for path in entries {
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let display = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        for capture in find_env_var_names(&text) {
            if STANDARD_EXEMPTIONS
                .iter()
                .any(|prefix| capture.starts_with(prefix))
            {
                continue;
            }
            if !claude_md.contains(&capture) {
                findings.push(Finding::new(
                    "TESTS-ENV-FLAGS-DOCUMENTED",
                    format!(
                        "{display} reads env var '{capture}' that is not listed in \
                         CLAUDE.md's 'Test environment flags' table; either document it \
                         or remove the read",
                    ),
                ));
            }
        }
    }
}

/// `TESTS-NO-IGNORED-RELEASE-PROOF`: release-relevant evidence must not be
/// hidden behind `#[ignore]`. Adapter-sensitive lanes should run by explicit
/// env var and otherwise write fail-closed `release_evidence=false` metadata.
pub(crate) fn check_no_ignored_release_tests(root: &Path, findings: &mut Vec<Finding>) {
    let Ok(read_dir) = fs::read_dir(root.join("tests")) else {
        return;
    };
    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        entries.push(entry.path());
    }
    entries.sort();
    for path in entries {
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if text.contains("#[ignore") {
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

/// Scan a Rust source for `std::env::var("NAME")` / `env::var("NAME")` reads
/// and return the literal NAME strings. Best-effort: handles the common
/// `env::var("FOO")` and `std::env::var("FOO")` call shapes; macro-built
/// names are not detected.
pub(crate) fn find_env_var_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for marker in &["env::var(\"", "env::var_os(\""] {
        let mut cursor = 0;
        while let Some(start) = source[cursor..].find(marker) {
            let head = cursor + start + marker.len();
            if let Some(end) = source[head..].find('"') {
                let name = source[head..head + end].to_string();
                if !name.is_empty() && !names.contains(&name) {
                    names.push(name);
                }
                cursor = head + end + 1;
            } else {
                break;
            }
        }
    }
    names
}

pub(crate) const REQUIRED_DOCS: &[&str] = &[
    "AGENTS.md",
    "README.md",
    "CHANGELOG.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "docs/RFC-rust-3d-renderer.md",
    "docs/api/m5-public-api-baseline.txt",
    "docs/api/m5-semver-baseline.toml",
    "docs/agents/subagents.md",
    "docs/specs/public-api.md",
    "docs/specs/architecture-contract.md",
    "docs/specs/module-boundaries.md",
    "docs/specs/render-lifecycle.md",
    "docs/specs/asset-gltf-contract.md",
    "docs/specs/visual-quality-contract.md",
    "docs/specs/platform-capabilities.md",
    "docs/specs/release-gates.md",
    "docs/specs/doctor-contract.md",
    "docs/specs/release-reviews.md",
    "docs/api/public-api-ownership.toml",
    "docs/checklists/acceptance-index.md",
    "docs/checklists/architecture-perfection-checklist.md",
    "docs/checklists/m0-foundation.md",
    "docs/checklists/m1-geometry-materials.md",
    "docs/checklists/m2-lighting-depth-clipping.md",
    "docs/checklists/m3a-app-features.md",
    "docs/checklists/m3b-gltf-animation.md",
    "docs/checklists/m4-performance-platform.md",
    "docs/checklists/m5-v1-release.md",
    "docs/decisions/ADR-0001-renderer-not-engine.md",
    "docs/decisions/ADR-0002-explicit-prepare-lifecycle.md",
    "docs/decisions/ADR-0003-gltf-primary-format.md",
    "docs/decisions/ADR-0004-visual-evidence-policy.md",
    ".codex/skills/scena-doctor/SKILL.md",
    ".codex/skills/scena-git-github/SKILL.md",
    ".codex/skills/scena-gltf-assets/SKILL.md",
    ".codex/skills/scena-release-hygiene/SKILL.md",
    ".codex/skills/scena-renderer-architecture/SKILL.md",
    ".codex/skills/scena-renderer-quality/SKILL.md",
    ".codex/skills/scena-rfc-governance/SKILL.md",
    ".claude/agents/scena-doctor-reviewer.md",
];
