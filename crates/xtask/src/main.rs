use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use serde_json::json;
use sha2::{Digest, Sha256};

fn main() {
    let outcome = match parse_command(env::args().skip(1).collect()) {
        Ok(Command::Doctor(mode)) => run_doctor(mode),
        Ok(Command::ClaimAudit) => run_claim_audit(),
        Ok(Command::ReleaseLaneArtifact(lane)) => run_release_lane_artifact(&lane),
        Ok(Command::Help) => {
            print_usage();
            Ok(())
        }
        Err(error) => Err(vec![Finding::new("CLI", error)]),
    };

    match outcome {
        Ok(()) => {}
        Err(findings) => {
            eprintln!("scena doctor failed with {} finding(s):", findings.len());
            for finding in findings {
                eprintln!("- {}: {}", finding.rule, finding.message);
            }
            process::exit(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoctorMode {
    Docs,
    Architecture,
    Full,
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Doctor(DoctorMode),
    ClaimAudit,
    ReleaseLaneArtifact(String),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    rule: &'static str,
    message: String,
}

impl Finding {
    fn new(rule: &'static str, message: impl Into<String>) -> Self {
        Self {
            rule,
            message: message.into(),
        }
    }
}

fn parse_command(args: Vec<String>) -> Result<Command, String> {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(Command::Help);
    }

    if args.first().map(String::as_str) == Some("claim-audit") {
        if args.len() == 1 {
            return Ok(Command::ClaimAudit);
        }
        return Err("claim-audit accepts no arguments".to_string());
    }

    if args.first().map(String::as_str) == Some("release-lane-artifact") {
        if args.len() == 2 {
            return Ok(Command::ReleaseLaneArtifact(args[1].clone()));
        }
        return Err("release-lane-artifact expects exactly one lane argument".to_string());
    }

    if args.first().map(String::as_str) != Some("doctor") {
        return Err(format!(
            "unknown command '{}'; expected 'doctor', 'claim-audit', or 'release-lane-artifact'",
            args.first().map(String::as_str).unwrap_or("")
        ));
    }

    let mode = match args.get(1).map(String::as_str) {
        None | Some("--full") => DoctorMode::Full,
        Some("--docs") => DoctorMode::Docs,
        Some("--architecture") => DoctorMode::Architecture,
        Some("--help") | Some("-h") => return Ok(Command::Help),
        Some(other) => {
            return Err(format!(
                "unknown doctor mode '{other}'; expected --docs, --architecture, or --full"
            ));
        }
    };

    if args.len() > 2 {
        return Err("doctor accepts at most one mode flag".to_string());
    }

    Ok(Command::Doctor(mode))
}

fn print_usage() {
    println!(
        "Usage:\n  cargo run -p xtask -- doctor --docs\n  cargo run -p xtask -- doctor --architecture\n  cargo run -p xtask -- doctor --full\n  cargo run -p xtask -- claim-audit\n  cargo run -p xtask -- release-lane-artifact <lane>"
    );
}

fn run_release_lane_artifact(lane: &str) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("RELEASE-LANE-ROOT", message)])?;
    let artifact = release_lane_artifact(lane)
        .map_err(|message| vec![Finding::new("RELEASE-LANE", message)])?;
    let artifact_dir = root.join("target/gate-artifacts/release-lanes");
    if let Err(error) = fs::create_dir_all(&artifact_dir) {
        return Err(vec![Finding::new(
            "RELEASE-LANE",
            format!("failed to create {}: {error}", artifact_dir.display()),
        )]);
    }
    let artifact_path = artifact_dir.join(format!("{lane}.json"));
    let body = serde_json::to_string_pretty(&artifact)
        .map_err(|error| vec![Finding::new("RELEASE-LANE", error.to_string())])?;
    if let Err(error) = fs::write(&artifact_path, format!("{body}\n")) {
        return Err(vec![Finding::new(
            "RELEASE-LANE",
            format!("failed to write {}: {error}", artifact_path.display()),
        )]);
    }
    println!("{}", artifact_path.display());
    Ok(())
}

fn release_lane_artifact(lane: &str) -> Result<serde_json::Value, String> {
    let (os, backend) = match lane {
        "linux-native-vulkan" => ("ubuntu-24.04", "NativeSurface"),
        "linux-webgl2-chromium" => ("ubuntu-24.04", "WebGl2"),
        "linux-webgpu-chromium" => ("ubuntu-24.04", "WebGpu"),
        "macos-metal" => ("macos-15", "Metal"),
        "windows-dx12" => ("windows-2025", "Dx12"),
        "wasm32-unknown-unknown" => ("ubuntu-24.04", "Wasm"),
        _ => return Err(format!("unknown release lane '{lane}'")),
    };
    Ok(json!({
        "schema": "scena.release_lane.v1",
        "lane": lane,
        "os": os,
        "backend": backend,
        "rustc": "1.93.1",
        "status": "command-recorded",
        "artifacts": [
            "target/gate-artifacts"
        ],
        "note": "This schema artifact records lane execution. Rendered-output proof is required separately for visual release gates."
    }))
}

fn run_claim_audit() -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("CLAIM-AUDIT-ROOT", message)])?;
    let artifact = match build_claim_audit(&root) {
        Ok(artifact) => artifact,
        Err(error) => return Err(vec![Finding::new("CLAIM-AUDIT", error)]),
    };
    let artifact_dir = root.join("target/gate-artifacts");
    if let Err(error) = fs::create_dir_all(&artifact_dir) {
        return Err(vec![Finding::new(
            "CLAIM-AUDIT",
            format!("failed to create target/gate-artifacts: {error}"),
        )]);
    }
    let artifact_path = artifact_dir.join("m10-claim-audit.json");
    let body = serde_json::to_string_pretty(&artifact)
        .map_err(|error| vec![Finding::new("CLAIM-AUDIT", error.to_string())])?;
    if let Err(error) = fs::write(&artifact_path, format!("{body}\n")) {
        return Err(vec![Finding::new(
            "CLAIM-AUDIT",
            format!("failed to write {}: {error}", artifact_path.display()),
        )]);
    }
    println!("{}", artifact_path.display());
    Ok(())
}

fn build_claim_audit(root: &Path) -> Result<serde_json::Value, String> {
    let mut files = Vec::new();
    for path in claim_audit_paths(root)? {
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).map_err(|error| error.to_string())?;
        let categories = claim_categories(&source);
        files.push(json!({
            "path": relative,
            "sha256": sha256_hex(&path).map_err(|error| error.to_string())?,
            "evidence_categories": categories,
            "evidence": categories
                .iter()
                .map(|category| json!({
                    "category": category,
                    "links": evidence_links_for_category(category),
                }))
                .collect::<Vec<_>>(),
        }));
    }
    Ok(json!({
        "schema": "scena.m10.claim_audit.v1",
        "status": "generated-with-evidence-index",
        "scope": [
            "repo-root markdown",
            "docs markdown",
            "examples rust"
        ],
        "files": files,
        "required_final_gates": [
            "cargo fmt --check",
            "cargo clippy --all-targets -- -D warnings",
            "cargo test",
            "cargo check --examples",
            "cargo run -p xtask -- doctor --full",
            "RUSTDOCFLAGS=\"-D warnings\" cargo doc --no-deps --all-features",
            "browser WebGPU/WebGL2 rendered-output proof",
            "native platform rendered-output proof",
            "clean cargo publish --dry-run",
            "external review reports",
            "named maintainer sign-off"
        ]
    }))
}

fn claim_audit_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for name in ["README.md", "CHANGELOG.md", "AGENTS.md"] {
        let path = root.join(name);
        if path.is_file() {
            paths.push(path);
        }
    }
    collect_files_with_extensions(&root.join("docs"), &["md"], &mut paths)?;
    collect_files_with_extensions(&root.join("examples"), &["rs"], &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_files_with_extensions(
    dir: &Path,
    extensions: &[&str],
    paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_extensions(&path, extensions, paths)?;
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            paths.push(path);
        }
    }
    Ok(())
}

fn claim_categories(source: &str) -> Vec<&'static str> {
    let mut categories = Vec::new();
    for (needle, category) in [
        ("Scene", "public-api"),
        ("Renderer", "public-api"),
        ("glTF", "assets-gltf"),
        ("WebGPU", "browser-platform"),
        ("WebGL2", "browser-platform"),
        ("Metal", "native-platform"),
        ("DX12", "native-platform"),
        ("screenshot", "visual-proof"),
        ("benchmark", "performance"),
        ("doctor", "doctor"),
        ("non-goal", "scope-non-goal"),
        ("physics", "scope-non-goal"),
        ("simulation", "scope-non-goal"),
        ("prepare", "render-lifecycle"),
        ("render", "render-lifecycle"),
    ] {
        if source.contains(needle) && !categories.contains(&category) {
            categories.push(category);
        }
    }
    categories
}

fn evidence_links_for_category(category: &str) -> Vec<&'static str> {
    match category {
        "public-api" => vec![
            "docs/specs/public-api.md",
            "tests/m0_foundation.rs",
            "tests/m5_release.rs",
        ],
        "assets-gltf" => vec![
            "docs/specs/asset-gltf-contract.md",
            "tests/m3a_app_features.rs",
            "tests/m3b_gltf_animation.rs",
            "tests/m8_assets_materials_ecosystem.rs",
        ],
        "browser-platform" => vec![
            "docs/checklists/m6-browser-renderer-parity.md",
            "tests/m6_browser_renderer_parity.rs",
            "tests/browser/m6_rust_wasm_renderer_probe.js",
        ],
        "native-platform" => vec![
            "docs/specs/platform-capabilities.md",
            "docs/decisions/ADR-0005-local-release-candidate-deferrals.md",
            ".github/workflows/ci.yml",
            ".github/workflows/release.yml",
        ],
        "visual-proof" => vec![
            "docs/specs/visual-quality-contract.md",
            "tests/m1_visual_proof.rs",
            "tests/m2_visual_proof.rs",
            "tests/m3a_visual_proof.rs",
            "tests/m3b_visual_proof.rs",
        ],
        "performance" => vec![
            "docs/specs/release-gates.md",
            "tests/m4_performance_platform.rs",
            "tests/m5_release.rs",
        ],
        "doctor" => vec!["docs/specs/doctor-contract.md", "crates/xtask/src/main.rs"],
        "scope-non-goal" => vec![
            "docs/decisions/ADR-0001-renderer-not-engine.md",
            "docs/specs/module-boundaries.md",
            "AGENTS.md",
        ],
        "render-lifecycle" => vec![
            "docs/specs/render-lifecycle.md",
            "docs/decisions/ADR-0002-explicit-prepare-lifecycle.md",
            "tests/m0_foundation.rs",
        ],
        _ => Vec::new(),
    }
}

fn run_doctor(mode: DoctorMode) -> Result<(), Vec<Finding>> {
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

fn repo_root() -> Result<PathBuf, String> {
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

fn run_docs_doctor(root: &Path, findings: &mut Vec<Finding>) {
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
    check_m10_claim_audit_contract(root, findings);
}

fn run_architecture_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "ARCH-REQUIRED", REQUIRED_SOURCE_MODULES);
    check_source_scope(root, findings);
    check_module_boundaries(root, findings);
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
    check_render_alpha_contracts(root, findings);
    check_output_stage_contracts(root, findings);
    check_fxaa_output_contracts(root, findings);
    check_diagnostics_contracts(root, findings);
    check_renderer_stats_contracts(root, findings);
    check_solid_kiss(root, findings);
    check_backend_vocabulary(root, findings);
    check_unit_test_first_governance(root, findings);
    check_agent_validation(root, findings);
}

const REQUIRED_DOCS: &[&str] = &[
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
    "docs/specs/module-boundaries.md",
    "docs/specs/render-lifecycle.md",
    "docs/specs/asset-gltf-contract.md",
    "docs/specs/visual-quality-contract.md",
    "docs/specs/platform-capabilities.md",
    "docs/specs/release-gates.md",
    "docs/specs/doctor-contract.md",
    "docs/checklists/acceptance-index.md",
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

const REQUIRED_SOURCE_MODULES: &[&str] = &[
    "src/lib.rs",
    "src/scene.rs",
    "src/scene/camera.rs",
    "src/scene/dirty.rs",
    "src/scene/inspection.rs",
    "src/scene/lights.rs",
    "src/scene/origin.rs",
    "src/scene/skinning.rs",
    "src/diagnostics/capabilities.rs",
    "src/assets.rs",
    "src/assets/environment.rs",
    "src/assets/load.rs",
    "src/assets/gltf/accessor/skin.rs",
    "src/assets/gltf/skins.rs",
    "src/assets/gltf/transform.rs",
    "src/geometry.rs",
    "src/geometry/bounds.rs",
    "src/geometry/primitive.rs",
    "src/geometry/skinning.rs",
    "src/geometry/static_batch.rs",
    "src/material.rs",
    "src/render.rs",
    "src/render/build.rs",
    "src/render/culling.rs",
    "src/render/surface.rs",
    "src/render/gpu/build.rs",
    "src/render/gpu/depth.rs",
    "src/render/gpu/culling.rs",
    "src/render/gpu/shadow.rs",
    "src/render/gpu/vertices.rs",
    "src/render/prepare/strokes.rs",
    "src/animation.rs",
    "src/animation/sampling.rs",
    "src/controls.rs",
    "src/picking.rs",
    "src/diagnostics.rs",
    "src/platform.rs",
    "src/bin/scena-convert.rs",
];

const STALE_DOC_TERMS: &[&str] = &[
    "TBD",
    "TODO",
    "FIXME",
    "not final API",
    "complete working example",
    "Renderer::prepare(&mut self, &mut scene)",
    "Renderer::render(&mut self, &scene",
    "RenderError::BackendCapabilityMismatch",
    "MutationQueueFull",
    "HardwareTier::Low / Mid",
    "rotation_quat",
    "gpu_memory_mb",
    "frame_time_ms",
    "render_on_change_skips",
    "texture_count",
    "Assets owns all GPU",
    "Load error unless feature enabled",
    "load error unless feature enabled",
    "Scene::replace_import(import, new_scene_asset)",
    "instantiate(scene_asset)",
    "instantiate_with(scene_asset",
    "Color::from_rgb(",
];

const SOURCE_SCOPE_TERMS: &[&str] = &[
    "plc",
    "robotics",
    "robot",
    "physics",
    "simulation",
    "process semantics",
    "game engine",
    "game loop",
];

const MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE: usize = 500;

const CATCH_ALL_TYPE_NAMES: &[&str] = &["World", "Engine", "Manager", "Registry", "ServiceLocator"];

const CATCH_ALL_TYPE_SUFFIXES: &[&str] = &["Manager", "Engine"];

const ALLOWED_CONTEXT_TYPES: &[&str] = &[
    "InteractionContext",
    "RenderContext",
    "PrepareContext",
    "DiagnosticContext",
];

fn require_files(root: &Path, findings: &mut Vec<Finding>, rule: &'static str, paths: &[&str]) {
    for rel in paths {
        if !root.join(rel).is_file() {
            findings.push(Finding::new(rule, format!("missing required file {rel}")));
        }
    }
}

fn check_markdown_links(root: &Path, findings: &mut Vec<Finding>) {
    for rel in markdown_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            findings.push(Finding::new(
                "DOCS-LINKS",
                format!("could not read {}", rel.display()),
            ));
            continue;
        };

        for target in markdown_link_targets(&text) {
            if is_external_link(&target) || target.starts_with('#') {
                continue;
            }

            let without_fragment = target.split('#').next().unwrap_or_default();
            if without_fragment.is_empty() {
                continue;
            }

            let target_path = path
                .parent()
                .unwrap_or(root)
                .join(without_fragment.trim_matches(['<', '>']));
            if !target_path.exists() {
                findings.push(Finding::new(
                    "DOCS-LINKS",
                    format!("{} links to missing {}", rel.display(), target),
                ));
            }
        }
    }
}

fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from("README.md"), PathBuf::from("AGENTS.md")];
    collect_markdown(&root.join("docs"), Path::new("docs"), &mut files);
    collect_markdown(
        &root.join(".codex/skills"),
        Path::new(".codex/skills"),
        &mut files,
    );
    files
}

fn collect_markdown(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if path.is_dir() {
            collect_markdown(&path, &rel, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(rel);
        }
    }
}

fn markdown_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if bytes[index] == b']' && bytes[index + 1] == b'(' {
            let start = index + 2;
            if let Some(end_offset) = text[start..].find(')') {
                let target = text[start..start + end_offset].trim();
                if !target.is_empty() {
                    targets.push(target.to_string());
                }
                index = start + end_offset + 1;
                continue;
            }
        }
        index += 1;
    }
    targets
}

fn is_external_link(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("app://")
}

fn check_for_stale_doc_terms(root: &Path, findings: &mut Vec<Finding>) {
    for rel in markdown_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for term in STALE_DOC_TERMS {
            if text.contains(term) {
                findings.push(Finding::new(
                    "DOCS-STALE-TERM",
                    format!("{} contains stale term '{}'", rel.display(), term),
                ));
            }
        }
    }
}

fn check_required_doc_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DOCS-PUBLIC-API",
        "docs/specs/public-api.md",
        &[
            "pub fn prepare(&mut self, scene: &mut Scene) -> Result<(), PrepareError>;",
            "pub struct RendererStats",
            "pub enum Error",
            "pub enum SurfaceEvent",
            "Color::from_linear_rgb",
            "`MaterialDesc` is an immutable descriptor value",
            "Texture slots store `TextureHandle` values only",
            "MaterialDesc::unlit(base_color);",
            "MaterialDesc::pbr_metallic_roughness(base_color, metallic, roughness);",
            "material.with_base_color_texture(texture);",
            "material.with_normal_texture(texture);",
            "material.with_metallic_roughness_texture(texture);",
            "material.with_occlusion_texture(texture);",
            "material.with_emissive_texture(texture);",
            "material.with_alpha_mode(alpha_mode);",
            "material.with_emissive(color);",
            "material.with_emissive_strength(strength);",
            "material.with_double_sided(true);",
            "MaterialDesc::line(base_color, width_px);",
            "MaterialDesc::wireframe(base_color, width_px);",
            "MaterialDesc::edge(base_color, width_px);",
            "material.with_stroke_width_px(width_px);",
            "material.with_edge_angle_threshold_degrees(angle_threshold_degrees);",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-LIFECYCLE",
        "docs/specs/render-lifecycle.md",
        &[
            "warning watermark is 1024",
            "Retain policy is global and prospective",
            "`mixer.seek()` while paused",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-GLTF",
        "docs/specs/asset-gltf-contract.md",
        &[
            "Coordinate conversion must preserve visible winding",
            "LookupError::StaleImport",
            "cubic-spline quaternion output is normalized",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-VISUAL",
        "docs/specs/visual-quality-contract.md",
        &[
            "Rgba8UnormSrgb",
            "source SHA-256",
            "Screenshot determinism is scoped to a pinned backend profile",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-DOCTOR",
        "docs/specs/doctor-contract.md",
        &[
            "cargo run -p xtask -- doctor --docs",
            "cargo run -p xtask -- doctor --architecture",
            "cargo run -p xtask -- doctor --full",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-RELEASE-GATES",
        "docs/specs/release-gates.md",
        &["Doctor", "cargo run -p xtask -- doctor --full"],
    );
}

fn require_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        findings.push(Finding::new(rule, format!("could not read {rel}")));
        return;
    };

    for needle in needles {
        if !text.contains(needle) {
            findings.push(Finding::new(
                rule,
                format!("{rel} is missing required contract text '{}'", needle),
            ));
        }
    }
}

fn check_source_scope(root: &Path, findings: &mut Vec<Finding>) {
    for rel in source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        for term in SOURCE_SCOPE_TERMS {
            if contains_scope_term(&lower, term) {
                findings.push(Finding::new(
                    "ARCH-SCOPE",
                    format!(
                        "{} contains renderer-forbidden term '{}'",
                        rel.display(),
                        term
                    ),
                ));
            }
        }
    }
}

fn check_module_boundaries(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-MODULES",
        "docs/specs/module-boundaries.md",
        &[
            "`scene`",
            "`assets`",
            "`geometry`",
            "`material`",
            "`render`",
            "`animation`",
            "`controls`",
            "`picking`",
            "`diagnostics`",
            "`platform`",
            "No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`",
        ],
    );

    forbid_contains(
        root,
        findings,
        "ARCH-PLATFORM",
        "src/platform.rs",
        &["wgpu::", "ForwardPass", "ShadowPass", "PostProcessPass"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ASSETS",
        "src/assets.rs",
        &["wgpu::", "RenderPass", "Surface"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-RENDER",
        "src/render.rs",
        &["fetch(", "load_scene", "load_texture", "gltf::"],
    );
    for rel in source_files(root)
        .into_iter()
        .filter(|path| path.starts_with("src/render"))
    {
        forbid_contains_path(
            root,
            findings,
            "ARCH-RENDER",
            &rel,
            &["fetch(", "load_scene", "load_texture", "gltf::"],
        );
    }
}

fn check_asset_api_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/assets.rs",
        &[
            "pub async fn load_texture",
            "&self,",
            "color_space: TextureColorSpace",
            "Result<TextureHandle, AssetError>",
            "pub fn create_material(&self, material: impl Into<MaterialDesc>) -> MaterialHandle",
            "pub fn default_environment(&self) -> EnvironmentHandle",
            "pub async fn load_environment",
            "pub fn environment(&self, handle: EnvironmentHandle) -> Option<EnvironmentDesc>",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/assets/environment.rs",
        &[
            "pub struct EnvironmentDesc",
            "pub struct EnvironmentDerivative",
            "pub enum EnvironmentSourceKind",
            "pub enum WasmEnvironmentDelivery",
            "pub const fn source_kind(&self) -> EnvironmentSourceKind",
            "pub const fn source_dimensions(&self) -> Option<(u32, u32)>",
            "pub const fn is_equirectangular_hdr(&self) -> bool",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/diagnostics.rs",
        &[
            "pub enum AssetError",
            "UnsupportedRequiredExtension",
            "UnsupportedEnvironmentFormat",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/material.rs",
        &[
            "pub struct MaterialDesc",
            "pub const DEFAULT_STROKE_WIDTH_PX",
            "pub const DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES",
            "pub enum MaterialKind",
            "Unlit",
            "PbrMetallicRoughness",
            "Line",
            "Wireframe",
            "Edge",
            "pub const fn unlit",
            "pub const fn pbr_metallic_roughness",
            "pub const fn line",
            "pub const fn wireframe",
            "pub const fn edge",
            "pub const fn with_stroke_width_px",
            "pub const fn with_edge_angle_threshold_degrees",
            "pub const fn with_base_color_texture",
            "pub const fn with_normal_texture",
            "pub const fn with_metallic_roughness_texture",
            "pub const fn with_occlusion_texture",
            "pub const fn with_emissive_texture",
            "pub const fn with_alpha_mode",
            "pub const fn with_emissive(",
            "pub const fn with_emissive_strength",
            "pub const fn with_double_sided",
            "pub const fn kind(&self) -> MaterialKind",
            "pub const fn base_color(&self) -> Color",
            "pub const fn base_color_texture(&self) -> Option<TextureHandle>",
            "pub const fn normal_texture(&self) -> Option<TextureHandle>",
            "pub const fn metallic_roughness_texture(&self) -> Option<TextureHandle>",
            "pub const fn occlusion_texture(&self) -> Option<TextureHandle>",
            "pub const fn emissive_texture(&self) -> Option<TextureHandle>",
            "pub const fn alpha_mode(&self) -> AlphaMode",
            "pub const fn emissive(&self) -> Color",
            "pub const fn emissive_strength(&self) -> f32",
            "pub const fn metallic_factor(&self) -> f32",
            "pub const fn roughness_factor(&self) -> f32",
            "pub const fn double_sided(&self) -> bool",
            "pub const fn stroke_width_px(&self) -> Option<f32>",
            "pub const fn edge_angle_threshold_degrees(&self) -> Option<f32>",
            "metallic_factor: clamp_unit_or",
            "roughness_factor: clamp_unit_or",
            "cutoff: clamp_unit_or",
            "self.emissive_strength = non_negative_or",
            "DEFAULT_STROKE_WIDTH_PX",
            "DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES",
            "stroke_width_px: Some(positive_or",
            "Some(clamp_degrees_or",
        ],
    );
    check_material_desc_fields_private(root, findings);
    forbid_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/material.rs",
        &[
            "pub kind:",
            "pub base_color:",
            "pub base_color_texture:",
            "pub normal_texture:",
            "pub metallic_roughness_texture:",
            "pub occlusion_texture:",
            "pub emissive_texture:",
            "pub alpha_mode:",
            "pub emissive:",
            "pub emissive_strength:",
            "pub metallic_factor:",
            "pub roughness_factor:",
            "pub double_sided:",
            "pub struct MaterialTexture",
            "pub enum MaterialTexture",
            "pub type MaterialTexture",
            "pub trait MaterialTexture",
            "pub fn basic(",
            "pub const fn basic(",
            "Basic",
            "basic(",
            "Basic,",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/lib.rs",
        &["MaterialTexture", "Basic", "basic"],
    );
}

fn check_render_alpha_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum AlphaPipelineStatus",
            "LinearSourceOver",
            "BackendPassthrough",
            "pub alpha_pipeline: AlphaPipelineStatus",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/cpu.rs",
        &[
            "fn blend_source_over(source: Color, destination: Color) -> Color",
            "let blended = blend_source_over(color, linear_frame[pixel_index])",
            "linear_frame[pixel_index] = blended",
            "&output.encode_rgba8(blended)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render.rs",
        &[
            "linear_frame: Option<Vec<Color>>",
            "cpu::clear_cpu",
            "cpu::draw_primitive_cpu",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/build.rs",
        &["linear_frame: (!has_gpu).then(|| vec![Color::BLACK; target.pixel_len()])"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/gpu/pipeline.rs",
        &["blend: Some(wgpu::BlendState::ALPHA_BLENDING)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "tests/m1_geometry_materials.rs",
        &[
            "headless_alpha_blends_in_linear_before_output_encoding",
            "headless_gpu_alpha_blends_sorted_asset_meshes_when_available",
            "AlphaPipelineStatus::LinearSourceOver",
        ],
    );
}

fn check_output_stage_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/output.rs",
        &[
            "fn aces_tonemap",
            "fn rrt_and_odt_fit",
            "ACES_INPUT_MATRIX",
            "ACES_OUTPUT_MATRIX",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/gpu/output.rs",
        &[
            "fn aces_tonemap(color: vec3<f32>) -> vec3<f32>",
            "fn rrt_and_odt_fit(value: f32) -> f32",
            "exposure_multiplier: f32",
            "fn encode_output_uniform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/gpu/pipeline.rs",
        &[
            "GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb",
            "pass.set_bind_group(0, output_bind_group, &[])",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum OutputStageStatus",
            "output_stage: OutputStageStatus::AcesSrgb",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "tests/m1_geometry_materials.rs",
        &["headless_gpu_output_stage_applies_aces_srgb_for_pinned_white_fixture"],
    );
}

fn check_fxaa_output_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/diagnostics.rs",
        &["pub fxaa_passes: u64"],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/render.rs",
        &[
            "fxaa_scratch: Vec<u8>",
            "output::apply_fxaa_rgba8(self.target, &mut self.frame, &mut self.fxaa_scratch)",
            "self.stats.fxaa_passes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/render/output.rs",
        &[
            "pub(super) fn apply_fxaa_rgba8",
            "luma_from_srgb8",
            "FXAA_LUMA_THRESHOLD",
            "fn aces_tonemap",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "fxaa_pass_runs_after_aces_without_second_tonemap",
            "stats.fxaa_passes",
            "[206, 206, 206, 255]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "docs/specs/public-api.md",
        &["pub fxaa_passes: u64", "tonemapper again"],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["FXAA pass attached", "ARCH-FXAA-OUTPUT"],
    );
}

fn check_diagnostics_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/diagnostics.rs",
        &[
            "pub struct Diagnostic",
            "pub code: DiagnosticCode",
            "pub severity: DiagnosticSeverity",
            "pub message: String",
            "pub help: Option<String>",
            "pub enum DiagnosticCode",
            "LargeScenePrecisionRisk",
            "DepthPrecisionRisk",
            "WebGl2DepthCompatibility",
            "pub enum DiagnosticSeverity",
            "pub fn warning",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/lib.rs",
        &["Diagnostic", "DiagnosticCode", "DiagnosticSeverity"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render.rs",
        &[
            "diagnostics: Vec<Diagnostic>",
            "self.diagnostics.clear()",
            "prepare::collect_precision_diagnostics(scene, self.target.backend)",
            "pub fn diagnostics(&self) -> &[Diagnostic]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/render/prepare.rs",
        &[
            "pub(super) fn collect_precision_diagnostics",
            "LARGE_SCENE_TRANSLATION_WARNING: f32 = 10_000.0",
            "DEPTH_RANGE_RATIO_WARNING: f32 = 100_000.0",
            "DiagnosticCode::LargeScenePrecisionRisk",
            "DiagnosticCode::DepthPrecisionRisk",
            "DiagnosticCode::WebGl2DepthCompatibility",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "src/scene.rs",
        &[
            "pub(crate) fn node_transforms",
            "pub(crate) fn camera_nodes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "prepare_emits_structured_depth_precision_warnings",
            "DiagnosticCode::DepthPrecisionRisk",
            "DiagnosticCode::LargeScenePrecisionRisk",
            "DiagnosticSeverity::Warning",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "docs/specs/public-api.md",
        &[
            "pub struct Diagnostic",
            "LargeScenePrecisionRisk",
            "DepthPrecisionRisk",
            "far/near ratio greater than",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIAGNOSTICS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Large-scene precision diagnostics", "ARCH-DIAGNOSTICS"],
    );
}

fn check_renderer_stats_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/diagnostics.rs",
        &[
            "pub struct RendererStats",
            "pub buffers: u64",
            "pub textures: u64",
            "pub materials: u64",
            "pub render_targets: u64",
            "pub pipelines: u64",
            "pub bind_groups: u64",
            "pub shader_modules: u64",
            "pub environments: u64",
            "pub environment_cubemaps: u64",
            "pub environment_prefilter_passes: u64",
            "pub environment_brdf_luts: u64",
            "pub scene_imports: u64",
            "pub shadow_maps: u64",
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
            "pub fxaa_passes: u64",
            "pub live_logical_handles: u64",
            "pub pending_destructions: u64",
            "pub approximate_gpu_memory_bytes: Option<u64>",
            "pub gpu_frame_ms: Option<f32>",
            "pub directional_shadow_map_resolution: Option<u32>",
            "pub directional_shadow_pcf_kernel: Option<u8>",
            "pub struct DevicePoll",
            "pub destroyed_resources: u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/prepare.rs",
        &[
            "pub(super) struct PreparedEnvironmentStats",
            "pub(super) struct PreparedDepthStats",
            "pub(super) fn collect_environment_prepare_stats",
            "pub(super) fn collect_depth_prepass_stats",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/prepare/resources.rs",
        &[
            "pub(in crate::render) struct PreparedLogicalResourceStats",
            "pub(in crate::render) fn collect_logical_resource_stats",
            "material.base_color_texture()",
            "live_logical_handles",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/stats.rs",
        &[
            "pub(in crate::render) struct GpuResourceStats",
            "fn estimate_prepared_resource_stats",
            "approximate_gpu_memory_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu.rs",
        &[
            "mod lifecycle;",
            "pub(super) fn prepared_resource_stats(&self) -> GpuResourceStats",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/lifecycle.rs",
        &[
            "pub(in crate::render) fn pending_destructions(&self) -> u64",
            "pub(in crate::render) fn poll_device(&mut self) -> (u64, bool)",
            "pub(in crate::render) fn release_prepared_resources(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render.rs",
        &[
            "pub fn poll_device(&mut self) -> DevicePoll",
            "self.stats.live_logical_handles = logical_stats.live_logical_handles",
            "self.stats.shadow_maps = lighting_stats.shadow_maps",
            "self.stats.depth_prepass_passes = depth_stats.passes",
            "self.stats.depth_prepass_draws = depth_stats.draws",
            "self.stats.textures = logical_stats.textures",
            "self.stats.environment_cubemaps = environment_prepare_stats.cubemaps",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "tests/m1_geometry_materials.rs",
        &[
            "m1_cpu_resource_lifetime_counters_return_to_baseline",
            "m1_logical_asset_resource_counters_return_to_baseline_after_empty_prepare",
            "m1_headless_gpu_resource_counters_return_to_baseline_after_empty_reprepare",
            "poll.pending_destructions_before",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "src/render/gpu/stats.rs",
        &[
            "estimates_prepared_headless_gpu_resource_counters",
            "estimates_empty_headless_gpu_resource_counters_at_baseline",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STATS",
        "docs/specs/public-api.md",
        &[
            "pub struct RendererStats",
            "pub struct DevicePoll",
            "shadow_maps",
            "depth_prepass_passes",
            "depth_prepass_draws",
            "fxaa_passes",
            "live_logical_handles",
            "pub buffers: u64",
            "pub target_height: u32",
            "logical `TextureHandle` values only",
        ],
    );
}

fn check_prepare_asset_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render.rs",
        &[
            "pub fn prepare_with_assets",
            "prepare::collect_prepared_primitives",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare.rs",
        &[
            "fn collect_prepared_primitives",
            "PrepareError::AssetsRequired",
            "fn append_geometry_primitives",
            "fn material_pass",
            "TransparentPrimitive",
            "total_cmp",
            "fn average_depth",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare/strokes.rs",
        &[
            "fn append_line_primitives",
            "fn append_wireframe_primitives",
            "fn append_edge_primitives",
            "struct EdgeCandidate",
            "fn append_line_segment",
            "fn screen_x_to_ndc",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/diagnostics.rs",
        &[
            "AssetsRequired",
            "GeometryNotFound",
            "MaterialNotFound",
            "UnsupportedGeometryTopology",
            "UnsupportedMaterialKind",
            "UnsupportedAlphaMode",
            "UnsupportedModelNode",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/scene.rs",
        &["pub(crate) fn mesh_nodes", "pub(crate) fn model_nodes"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "tests/m1_geometry_materials.rs",
        &[
            "prepare_with_assets_renders_scene_mesh_unlit_geometry",
            "prepare_without_assets_rejects_asset_backed_mesh_nodes",
            "prepare_with_assets_sorts_blend_meshes_back_to_front_before_render",
            "prepare_with_assets_renders_line_material_as_screen_space_stroke",
            "prepare_with_assets_renders_wireframe_material_triangle_edges",
            "prepare_with_assets_renders_edge_material_without_coplanar_internal_edges",
            "headless_gpu_renders_technical_material_primitives_when_available",
            "prepare_with_assets_rejects_unsupported_mesh_inputs_structurally",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "docs/specs/public-api.md",
        &["pub fn prepare_with_assets<F>"],
    );
}

fn check_environment_lifecycle_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render.rs",
        &[
            "environment: Option<EnvironmentHandle>",
            "environment_revision: u64",
            "PrepareError::EnvironmentAssetsRequired",
            "PrepareError::EnvironmentNotFound",
            "NotPreparedReason::EnvironmentChanged",
            "ChangeKind::Environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render/settings.rs",
        &[
            "pub fn environment(&self) -> Option<EnvironmentHandle>",
            "pub fn set_environment(&mut self, environment: EnvironmentHandle)",
            "pub fn clear_environment(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/diagnostics.rs",
        &[
            "EnvironmentAssetsRequired",
            "EnvironmentNotFound",
            "EnvironmentChanged",
            "Environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "tests/m1_geometry_materials.rs",
        &[
            "renderer_environment_is_structural_and_validated_during_prepare",
            "m1_logical_asset_resource_counters_return_to_baseline_after_empty_prepare",
            "renderer.clear_environment()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "tests/m1_visual_proof.rs",
        &[
            "render_default_cube_with_default_environment",
            "validate_default_cube_luminance_and_silhouette",
        ],
    );
}

fn check_equirectangular_hdr_environment_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets/environment.rs",
        &[
            "EnvironmentSourceKind::EquirectangularHdr",
            "pub fn from_equirectangular_hdr_path",
            "is_equirectangular_hdr_path",
            "parse_equirectangular_hdr_dimensions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets.rs",
        &["AssetError::UnsupportedEnvironmentFormat"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/lib.rs",
        &["EnvironmentSourceKind"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "equirectangular_hdr_environment_loading_records_source_contract",
            "EnvironmentSourceKind::EquirectangularHdr",
            "UnsupportedEnvironmentFormat",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Equirectangular HDR environment loading",
            "EnvironmentSourceKind",
        ],
    );
}

fn check_environment_ibl_prepare_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare.rs",
        &[
            "pub(super) struct PreparedEnvironmentStats",
            "cubemaps: 1",
            "prefilter_passes: 1",
            "brdf_luts: 1",
            "environment.is_equirectangular_hdr()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render.rs",
        &[
            "prepare::collect_environment_prepare_stats(Some(&environment_desc))",
            "self.stats.environment_cubemaps = environment_prepare_stats.cubemaps",
            "self.stats.environment_prefilter_passes = environment_prepare_stats.prefilter_passes",
            "self.stats.environment_brdf_luts = environment_prepare_stats.brdf_luts",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "equirectangular_environment_prepare_generates_ibl_resources",
            "environment_cubemaps",
            "environment_prefilter_passes",
            "environment_brdf_luts",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Cubemap conversion", "ARCH-ENV-IBL-PREP"],
    );
}

fn check_scene_light_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene.rs",
        &[
            "pub struct LightKey",
            "mod lights;",
            "pub use lights::{DirectionalLight, Light, LightBuilder, PointLight, SpotLight}",
            "NodeKind::Light",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene/lights.rs",
        &[
            "pub enum Light",
            "pub struct DirectionalLight",
            "pub struct PointLight",
            "pub struct SpotLight",
            "casts_shadows: bool",
            "pub fn directional_light(&mut self, light: DirectionalLight) -> LightBuilder<'_>",
            "pub fn point_light(&mut self, light: PointLight) -> LightBuilder<'_>",
            "pub fn spot_light(&mut self, light: SpotLight) -> LightBuilder<'_>",
            "pub fn light(&self, light: LightKey) -> Option<&Light>",
            "pub const fn casts_shadows",
            "pub const fn with_shadows",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/lib.rs",
        &[
            "DirectionalLight",
            "LightBuilder",
            "LightKey",
            "PointLight",
            "SpotLight",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "scene_light_components_are_typed_and_node_owned",
            ".directional_light",
            ".point_light",
            ".spot_light",
            "NodeKind::Light",
        ],
    );
}

fn check_direct_light_shading_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/scene.rs",
        &[
            "impl Iterator<Item = (NodeKey, LightKey, Light, Transform)>",
            "map(|light| (node_key, light_key, light, node.transform))",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare.rs",
        &[
            "mod lighting;",
            "use self::lighting::{PreparedLights, material_color}",
            "let lights = PreparedLights::from_scene(scene, origin_shift)",
            "material_color(material, position_a, normal_a, params.lights)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare/lighting.rs",
        &[
            "pub(super) struct PreparedLights",
            "pub(super) fn from_scene(scene: &Scene, origin_shift: Vec3) -> Self",
            "MaterialKind::PbrMetallicRoughness if lights.has_direct_lights()",
            "shade_pbr_base_color",
            "light_direction(transform)",
            "light.illuminance_lux()",
            "light.intensity_candela()",
            "spot_cone_attenuation",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "direct_lights_tint_pbr_mesh_output",
            "MaterialDesc::pbr_metallic_roughness",
            "with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))",
            "[216, 0, 9, 255]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "direct_lights_tint_pbr_mesh_output",
            "ARCH-DIRECT-LIGHT-SHADING",
        ],
    );
}

fn check_directional_shadow_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/render/prepare.rs",
        &[
            "pub(super) fn collect_lighting_stats(",
            "backend: Backend",
            "Capabilities::for_backend(backend)",
            "scene.light_nodes()",
            "light.casts_shadows()",
            "PrepareError::MultipleShadowedDirectionalLights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/diagnostics.rs",
        &["MultipleShadowedDirectionalLights"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "src/diagnostics/display.rs",
        &["only one shadowed directional light"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "shadowed_directional_light_is_opt_in_and_single_owner",
            "with_shadows(true)",
            "MultipleShadowedDirectionalLights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECTIONAL-SHADOW",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "One opt-in shadowed directional light",
            "with_shadows(true)",
        ],
    );
}

fn check_shadow_map_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/diagnostics.rs",
        &[
            "pub shadow_maps: u64",
            "pub directional_shadow_map_resolution: Option<u32>",
            "pub directional_shadow_pcf_kernel: Option<u8>",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/diagnostics/capabilities.rs",
        &[
            "pub directional_shadow_map_default_size: u32",
            "pub directional_shadow_map_max_size: u32",
            "pub directional_shadow_pcf_kernel: u8",
            "pub reversed_z_depth: CapabilityStatus",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/prepare.rs",
        &[
            "Capabilities::for_backend(backend)",
            "capabilities.directional_shadow_map_default_size",
            "DIRECTIONAL_SHADOW_PCF_KERNEL: u8 = 3",
            "pub(super) struct PreparedLightingStats",
            "shadow_maps: 1",
            "directional_shadow_pcf_kernel: Some(DIRECTIONAL_SHADOW_PCF_KERNEL)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu.rs",
        &[
            "shadow_texture: Option<wgpu::Texture>",
            "shadow_view: Option<wgpu::TextureView>",
            "ARCH-SHADOW-MAP: M2 allocates shadow resources before the shadow render pass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/shadow.rs",
        &[
            "pub(super) fn create_shadow_texture",
            "wgpu::TextureFormat::Depth32Float",
            "wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING",
            "scena.m2.directional_shadow_map",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "src/render/gpu/stats.rs",
        &[
            "shadow_maps: u64",
            "shadow_map_resolution: Option<u32>",
            "depth_prepass_passes: u64",
            "textures: 1 + shadow_maps + depth_prepass_passes",
            "render_targets: 1 + shadow_maps + depth_prepass_passes",
            "estimates_single_shadow_map_resource_counters",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "single_shadow_map_records_pcf3x3_prepare_stats",
            "directional_shadow_map_default_size",
            "stats.shadow_maps",
            "directional_shadow_pcf_kernel",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SHADOW-MAP",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Single shadow map with PCF 3x3", "ARCH-SHADOW-MAP"],
    );
}

fn check_depth_prepass_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/diagnostics.rs",
        &[
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/prepare.rs",
        &[
            "pub(super) struct PreparedDepthStats",
            "pub(super) fn collect_depth_prepass_stats(",
            "backend: Backend",
            "passes: 1",
            "draws: primitives.len() as u64",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render.rs",
        &[
            "let depth_stats = prepare::collect_depth_prepass_stats(&primitives, self.target.backend)",
            "self.stats.depth_prepass_passes = depth_stats.passes",
            "self.stats.depth_prepass_draws = depth_stats.draws",
            "gpu.prepare(self.target, &primitives, lighting_stats, depth_stats)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu.rs",
        &[
            "mod depth;",
            "PreparedDepthStats",
            "depth_prepass: Option<depth::DepthPrepassResources>",
            "depth::create_depth_prepass_resources",
            "depth::encode_depth_prepass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/depth.rs",
        &[
            "pub(super) struct DepthPrepassResources",
            "wgpu::TextureFormat::Depth32Float",
            "scena.m2.depth_prepass",
            "clear_depth",
            "pub(super) fn encode_depth_prepass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "src/render/gpu/stats.rs",
        &[
            "depth_prepass_passes: u64",
            "depth_prepass_bytes",
            "estimates_depth_prepass_resource_counters",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "depth_prepass_is_prepared_for_opaque_scene_geometry",
            "depth_prepass_passes",
            "depth_prepass_draws",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Depth pre-pass", "ARCH-DEPTH-PREPASS"],
    );
    require_contains(
        root,
        findings,
        "ARCH-DEPTH-PREPASS",
        "docs/specs/public-api.md",
        &[
            "pub depth_prepass_passes: u64",
            "pub depth_prepass_draws: u64",
            "M2 also prepares a depth pre-pass",
        ],
    );
}

fn check_reversed_z_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum CapabilityStatus",
            "Supported",
            "FeatureDisabled",
            "pub reversed_z_depth: CapabilityStatus",
            "const fn reversed_z_depth_status",
            "Backend::WebGl2",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/lib.rs",
        &["CapabilityStatus"],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/render/prepare.rs",
        &[
            "reversed_z: bool",
            "capabilities.reversed_z_depth == CapabilityStatus::Supported",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "src/render/gpu/depth.rs",
        &[
            "reversed_z: bool",
            "wgpu::CompareFunction::GreaterEqual",
            "clear_depth: if reversed_z { 0.0 } else { 1.0 }",
            "wgpu::LoadOp::Clear(resources.clear_depth)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "capability_matrix_reports_reversed_z_depth_support_and_webgl2_fallback",
            "CapabilityStatus::Supported",
            "CapabilityStatus::FeatureDisabled",
            "Backend::WebGl2",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "docs/specs/public-api.md",
        &[
            "pub reversed_z_depth: CapabilityStatus",
            "Capabilities::reversed_z_depth",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-REVERSED-Z",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Reversed-Z support", "ARCH-REVERSED-Z"],
    );
}

fn check_webgl2_depth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "src/diagnostics/capabilities.rs",
        &[
            "pub fn diagnostics(self) -> Vec<Diagnostic>",
            "self.backend == Backend::WebGl2",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "near/far ranges",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "src/diagnostics.rs",
        &["WebGl2DepthCompatibility"],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "webgl2_depth_capability_reports_structured_compatibility_warning",
            "Capabilities::for_attached_gpu_backend(Backend::WebGl2).diagnostics()",
            "DiagnosticCode::WebGl2DepthCompatibility",
            "near/far",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "docs/specs/public-api.md",
        &[
            "Capabilities::diagnostics()",
            "DiagnosticCode::WebGl2DepthCompatibility",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-WEBGL2-DEPTH",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["WebGL2 depth compatibility warnings", "ARCH-WEBGL2-DEPTH"],
    );
}

fn check_m2_leak_stats_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M2-LEAK-STATS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "m2_resource_counters_return_to_baseline_after_empty_prepare",
            "environment_cubemaps",
            "environment_prefilter_passes",
            "environment_brdf_luts",
            "shadow_maps",
            "depth_prepass_passes",
            "depth_prepass_draws",
            "released.textures, baseline.textures",
            "released.pending_destructions, baseline.pending_destructions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M2-LEAK-STATS",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "m2_resource_counters_return_to_baseline_after_empty_prepare",
            "ARCH-M2-LEAK-STATS",
        ],
    );
}

fn check_camera_depth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene.rs",
        &[
            "mod camera;",
            "pub use camera::{Camera, DepthRange, OrthographicCamera, PerspectiveCamera}",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/scene/camera.rs",
        &[
            "pub enum Camera",
            "pub struct PerspectiveCamera",
            "pub struct OrthographicCamera",
            "pub struct DepthRange",
            "pub const fn new(near: f32, far: f32) -> Self",
            "pub const fn fit_sphere(center_distance: f32, radius: f32) -> Self",
            "pub const fn contains_interval(self, near: f32, far: f32) -> bool",
            "pub const fn with_depth_range(mut self, range: DepthRange) -> Self",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "src/lib.rs",
        &["DepthRange", "PerspectiveCamera", "OrthographicCamera"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "camera_depth_fit_helpers_cover_unit_cube_reference_distances",
            "DepthRange::fit_sphere",
            "with_depth_range",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CAMERA-DEPTH",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Camera depth-range and depth-fit helpers",
            "DepthRange::fit_sphere",
        ],
    );
}

fn check_clipping_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/scene.rs",
        &[
            "pub struct ClippingPlaneKey",
            "pub struct ClippingPlane",
            "pub struct ClippingPlaneSet",
            "pub fn add_clipping_plane",
            "pub fn set_clipping_planes",
            "pub(crate) fn active_clipping_plane_values",
            "pub fn contains(self, point: Vec3) -> bool",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/diagnostics.rs",
        &["ClippingPlaneNotFound"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/lib.rs",
        &["ClippingPlane", "ClippingPlaneKey", "ClippingPlaneSet"],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/render.rs",
        &[
            "clipping_planes: Vec<ClippingPlane>",
            "scene.active_clipping_plane_values().collect()",
            "let clipping_planes = self.prepared_state(scene)?.clipping_planes.clone()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "src/render/cpu.rs",
        &[
            "clipping_planes: &[ClippingPlane]",
            "mix_position",
            "is_clipped",
            "plane.contains(position)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "clipping_plane_set_clips_rendered_output_half_space",
            "ClippingPlane::new",
            "ClippingPlaneSet::new().with_plane",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "docs/specs/public-api.md",
        &[
            "pub struct ClippingPlaneKey",
            "dot(normal, position) + distance >= 0",
            "ClippingPlaneNotFound",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-CLIPPING",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["ClippingPlane", "ARCH-CLIPPING"],
    );
}

fn check_origin_shift_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/scene.rs",
        &["origin_shift: Vec3"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/scene/origin.rs",
        &[
            "pub fn set_origin_shift",
            "pub fn origin_shift(&self) -> Vec3",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "src/render/prepare.rs",
        &[
            "let origin_shift = scene.origin_shift()",
            "transform_primitive",
            "transform_position",
            "subtract_vec3",
            "relative_translation",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "origin_shift_keeps_large_offset_renderable_visible_without_precision_warning",
            "scene.set_origin_shift",
            "DiagnosticCode::LargeScenePrecisionRisk",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "docs/specs/public-api.md",
        &["pub fn set_origin_shift", "large-world"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ORIGIN-SHIFT",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Camera-relative rendering or origin-shift support",
            "ARCH-ORIGIN-SHIFT",
        ],
    );
}

fn check_m3a_scene_import_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "Cargo.toml",
        &[
            "base64",
            "serde_json",
            "wasm-bindgen-futures",
            "Response",
            "obj = []",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets.rs",
        &[
            "mod fetch;",
            "mod gltf;",
            "mod obj;",
            "pub use fetch::{AssetFetcher, DefaultAssetFetcher}",
            "pub use gltf::{",
            "SceneAssetMesh",
            "scene_lookup: BTreeMap<AssetPath, SceneAsset>",
            "pub async fn load_scene",
            "pub async fn reload_scene",
            "RetainPolicy::Always",
            "ReloadRequiresRetain",
            "SceneAsset::from_gltf_bytes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/fetch.rs",
        &[
            "pub trait AssetFetcher",
            "pub type DefaultAssetFetcher",
            "pub struct FileAssetFetcher",
            "pub struct BrowserAssetFetcher",
            "window.fetch_with_str",
            "wasm_bindgen_futures::JsFuture",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/obj.rs",
        &[
            "pub async fn load_geometry",
            "parse_obj_geometry",
            "mtllib",
            "GeometryTopology::Triangles",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf.rs",
        &[
            "pub struct SceneAsset",
            "pub struct SceneAssetAnchor",
            "pub struct SceneAssetClip",
            "pub struct SceneAssetLight",
            "pub struct SceneAssetNode",
            "pub struct SceneAssetMesh",
            "pub(super) fn from_gltf_bytes",
            "pub(super) fn from_gltf_bytes_with_external_buffers",
            "pub(super) fn external_buffer_paths",
            "pub(super) fn from_gltf_source",
            "parse_glb",
            "pub fn mesh_count",
            "pub fn transform(&self)",
            "pub fn mesh(&self)",
            "pub fn meshes(&self)",
            "pub fn anchors(&self)",
            "pub(crate) fn invalid_reason",
            "pub fn clips(&self)",
            "pub fn light(&self)",
            "pub const fn bounds",
            "pub const fn uses_vertex_colors",
            "parse_punctual_lights",
            "parse_gltf_clips",
            "parse_node_anchors",
            "parse_node_transform",
            "UnsupportedRequiredExtension",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/extensions.rs",
        &[
            "pub enum GltfExtensionStatus",
            "pub struct GltfExtensionDiagnostic",
            "pub(super) fn is_v1_required_gltf_extension",
            "pub(super) fn collect_extension_diagnostics",
            "KHR_lights_punctual",
            "KHR_materials_unlit",
            "KHR_materials_emissive_strength",
            "KHR_texture_transform",
            "KHR_mesh_quantization",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/transform.rs",
        &[
            "pub(super) fn parse_node_transform",
            "\"matrix\"",
            "matrix_transform",
            "quat_from_rotation_columns",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/anchors.rs",
        &[
            "pub(super) fn parse_node_anchors",
            "validate_anchor_extras",
            "validate_number_array",
            "anchor rotation quaternion must be normalized",
            "anchor scale components must not be zero",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/glb.rs",
        &["pub(super) fn parse_glb", "GLB_BIN_CHUNK", "GLB_JSON_CHUNK"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/external.rs",
        &[
            "pub(super) fn external_buffer_paths",
            "resolve_relative_path",
            "!uri.starts_with(\"data:\")",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/accessor.rs",
        &[
            "parse_buffers",
            "binary_chunk",
            "external_buffers",
            "parse_buffer_views",
            "parse_accessors",
            "read_color_accessor",
            "read_normalized_components",
            "normalized",
            "GL_SHORT",
            "GL_UNSIGNED_SHORT",
            "base64::engine::general_purpose::STANDARD",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/assets/gltf/read.rs",
        &[
            "parse_materials",
            "parse_meshes",
            "let primitives = mesh",
            "parse_mesh_primitive",
            "parse_texture_transform",
            "TextureTransform::new",
            "GeometryDesc::try_new_with_vertex_colors",
            "TextureColorSpace::Srgb",
            "KHR_materials_unlit",
            "KHR_materials_emissive_strength",
            "KHR_texture_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/material.rs",
        &[
            "pub struct TextureTransform",
            "pub const fn base_color_texture_transform",
            "pub const fn with_base_color_texture_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/geometry.rs",
        &["try_new_with_vertex_colors", "pub fn vertex_colors"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene.rs",
        &[
            "mod import;",
            "mod instances;",
            "mod labels;",
            "mod materials;",
            "mod picking;",
            "mod view;",
            "ImportOptions",
            "InstanceSetKey",
            "LabelKey",
            "SceneImport",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/materials.rs",
        &[
            "pub fn set_mesh_material",
            "NodeIsNotMesh",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/picking.rs",
        &[
            "pub fn pick(",
            "pickable_renderables",
            "pick_scene",
            "pub fn interaction(&self)",
            "pub fn interaction_mut(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/instances.rs",
        &[
            "pub struct InstanceId",
            "pub enum InstanceCullingPolicy",
            "CpuBoundingBoxFallback",
            "pub struct InstanceSet",
            "pub fn add_instance_set",
            "pub fn reserve_instances",
            "pub fn push_instance",
            "pub fn remove_instance",
            "pub fn clear_instances",
            "pub fn instances(&self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/labels.rs",
        &[
            "pub struct LabelDesc",
            "pub enum LabelRasterization",
            "pub enum LabelBillboard",
            "pub fn sdf",
            "pub fn msdf",
            "pub fn add_label",
            "pub fn set_label_text",
            "LabelNotFound",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/picking.rs",
        &[
            "pub struct CursorPosition",
            "pub struct Viewport",
            "pub enum HitTarget",
            "pub struct Hit",
            "pub struct InteractionContext",
            "pub struct InteractionStyle",
            "set_hover",
            "set_primary_selection",
            "pub(crate) const fn revision",
            "pub(crate) fn pick_scene",
            "HitTarget::Node",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/prepare.rs",
        &[
            "scene.instance_set_nodes()",
            "labels::append_label_primitives",
            "compose_transform",
            "instance_set.geometry()",
            "instance_set.material()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/prepare/labels.rs",
        &[
            "pub(super) fn append_label_primitives",
            "scene.label_nodes()",
            "LabelBillboard::ScreenAligned",
            "Primitive::triangle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render.rs",
        &[
            "mod offscreen;",
            "hover_style: InteractionStyle",
            "selection_style: InteractionStyle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/offscreen.rs",
        &[
            "pub struct OffscreenTarget",
            "pub struct PixelReadback",
            "pub fn offscreen",
            "pub fn read_pixels",
            "pub fn into_rgba8",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/render/settings.rs",
        &["pub fn set_hover_style", "pub fn set_selection_style"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/view.rs",
        &[
            "pub fn camera_node",
            "pub fn frame(&mut self, camera: CameraKey, bounds: Aabb)",
            "pub fn look_at(&mut self, camera: CameraKey, target: NodeKey)",
            "DepthRange::fit_sphere",
            "set_node_transform_and_mark_changed",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import.rs",
        &[
            "pub struct ImportOptions",
            "pub enum SourceUnits",
            "pub enum SourceCoordinateSystem",
            "Centimeters",
            "ZUpRightHanded",
            "pub struct SceneImport",
            "pub struct ImportAnchor",
            "pub struct ImportClip",
            "pub struct ImportPivot",
            "pub fn instantiate(",
            "pub fn instantiate_with(",
            "pub async fn import<",
            "pub async fn import_with<",
            "pub fn replace_import(",
            "mark_stale",
            "NodeKind::Mesh",
            "source_node.meshes()",
            "mesh_node_kind",
            "scene_asset: &SceneAsset",
            "InvalidAnchorExtras",
            "ImportDiagnosticOverlayKind::Origin",
            "ImportDiagnosticOverlayKind::Axes",
            "ImportDiagnosticOverlayKind::Bounds",
            "ImportDiagnosticOverlayKind::Anchor",
            "ImportDiagnosticOverlayKind::Pivot",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/options.rs",
        &[
            "pub const fn gltf_default() -> Self",
            "pub const fn with_source_units",
            "pub const fn with_source_coordinate_system",
            "pub(super) fn convert_transform",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/accessors.rs",
        &["pub fn channels(&self)", "pub const fn duration_seconds"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/scene/import/lookups.rs",
        &[
            "LookupError::StaleImport",
            "pub fn node(&self, name: &str)",
            "pub fn first_node(&self, name: &str)",
            "pub fn nodes_named",
            "pub fn path(&self, path: &str)",
            "pub fn bounds_local",
            "pub fn bounds_world",
            "pub fn pivot(&self",
            "pub fn diagnostic_overlays",
            "pub fn anchor(&self",
            "pub fn first_anchor",
            "pub fn anchors_named",
            "pub fn clip(&self",
            "pub fn first_clip",
            "pub fn clips_named",
            "AmbiguousAnchorName",
            "AmbiguousClipName",
            "fn path_segments(path: &str)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/animation.rs",
        &["pub struct AnimationClipKey", "pub(crate) fn fresh"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "src/diagnostics.rs",
        &[
            "InstantiateError",
            "ImportError",
            "StaleImport",
            "NodeNameNotFound",
            "AmbiguousNodeName",
            "AnchorNotFound",
            "AmbiguousAnchorName",
            "ClipNotFound",
            "AmbiguousClipName",
            "PathNotFound",
            "NodeIsNotMesh",
            "pub struct ImportDiagnosticOverlay",
            "pub enum ImportDiagnosticOverlayKind",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "tests/m3a_app_features.rs",
        &[
            "assets_load_scene_caches_gltf_asset_and_rejects_required_extensions",
            "scene_instantiate_creates_import_hierarchy_and_name_lookups",
            "scene_import_convenience_uses_gltf_default_options",
            "replace_import_returns_fresh_import_and_stales_old_lookups",
            "scene_import_reports_duplicate_names_and_escaped_paths",
            "camera_frame_and_look_at_helpers_update_view_and_require_prepare",
            "assets_load_scene_uses_fetcher_trait_and_deduplicates_by_asset_path",
            "gltf_loader_creates_geometry_material_texture_and_vertex_color_contracts",
            "glb_loader_reads_binary_chunk_mesh_materials_and_instantiates",
            "gltf_loader_fetches_external_buffers_relative_to_scene_path",
            "gltf_loader_preserves_multi_primitive_meshes_as_child_mesh_nodes",
            "reload_scene_requires_retain_and_reprepare_after_replace_import",
            "import_options_apply_gltf_node_transforms_and_source_units",
            "source_coordinate_conversion_preserves_right_handed_basis_for_winding",
            "scene_import_exposes_named_pivots_and_diagnostic_overlays",
            "prepared_render_requires_reprepare_after_transform_changes",
            "prepared_render_requires_reprepare_after_material_and_interaction_changes",
            "scene_import_reports_local_and_world_bounds_for_imported_meshes",
            "scene_import_anchor_lookups_parse_gltf_extras_and_stale",
            "scene_import_rejects_duplicate_anchor_names_on_same_host",
            "scene_import_rejects_invalid_anchor_extras_data",
            "scene_import_clip_lookups_are_import_local_and_stale",
            "gltf_required_punctual_lights_instantiate_as_scene_lights",
            "gltf_required_texture_transform_and_mesh_quantization_are_realized",
            "obj_feature_load_geometry_parses_triangle_faces",
            "scene_pick_returns_typed_hit_target_for_renderable_triangle",
            "interaction_context_and_renderer_styles_are_explicit",
            "instance_sets_have_stable_ids_mutations_and_cpu_fallback",
            "offscreen_target_readback_is_explicit_and_owned",
            "m3a_resource_lifetime_counters_return_to_baseline_for_imports_targets_and_instances",
            "labels_use_sdf_msdf_descriptors_and_billboard_render_path",
            "InstanceCullingPolicy::CpuBoundingBoxFallback",
            "LabelRasterization::Msdf",
            "mesh_material_vertex_color_scene.gltf",
            "transform_options_scene.gltf",
            "ImportOptions::gltf_default",
            "ImportDiagnosticOverlayKind::Pivot",
            "Root/A\\\\/B",
            "UnsupportedRequiredExtension",
            "import.path(\"Root/Child\")",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "tests/m3a_visual_proof.rs",
        &[
            "m3a_headless_visual_artifacts_cover_import_interaction_instances_labels_and_readback",
            "target/gate-artifacts/m3a-visual",
            "m3a-glb-model-viewer",
            "m3a-picking-selection",
            "m3a-instancing",
            "m3a-labels",
            "m3a-offscreen-readback",
            "write_ppm_artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "tests/m3a_browser_rendered_output.rs",
        &[
            "wasm_bindgen_test_configure!(run_in_browser)",
            "m3a_browser_wasm_renders_import_and_interaction_paths_to_canvas",
            "render_glb_import_frame",
            "render_interaction_frame",
            "browser_canvas_roundtrip",
            "minimal_glb_triangle_scene",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3A-SCENE-IMPORT",
        "docs/checklists/m3a-app-features.md",
        &[
            "assets_load_scene_caches_gltf_asset_and_rejects_required_extensions",
            "scene_instantiate_creates_import_hierarchy_and_name_lookups",
            "ARCH-M3A-SCENE-IMPORT",
        ],
    );
}

fn check_m3b_animation_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/animation.rs",
        &[
            "pub struct AnimationMixerKey",
            "pub enum AnimationPlaybackState",
            "pub enum AnimationLoopMode",
            "pub enum AnimationTarget",
            "pub enum AnimationInterpolation",
            "pub struct AnimationClip",
            "pub struct AnimationSourceClip",
            "pub struct AnimationChannel",
            "pub struct AnimationSourceChannel",
            "pub struct AnimationMixer",
            "pub enum AnimationOutput",
            "pub fn rebind",
            "pub fn sample_vec3",
            "pub fn sample_quat",
            "pub fn sample_weights",
            "pub(crate) fn play",
            "pub(crate) fn pause",
            "pub(crate) fn stop",
            "pub(crate) fn seek",
            "pub(crate) fn advance",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/animation/sampling.rs",
        &[
            "sample_cubic_vec3",
            "sample_cubic_quat",
            "sample_cubic_weights",
            "slerp_quat",
            "cubic_scalar",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/animation.rs",
        &[
            "pub(super) fn parse_gltf_clips",
            "parse_animation_channel",
            "\"translation\"",
            "\"rotation\"",
            "\"scale\"",
            "\"weights\"",
            "read_f32_accessor",
            "read_vec3_accessor",
            "read_vec4_accessor",
            "AnimationInterpolation::CubicSpline",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import.rs",
        &[
            "clip.clip().rebind",
            "resolve_import_skin_bindings",
            "SceneSkinBinding::new",
            "convert_animation_vec3",
            "pub(crate) fn live_flag",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/options.rs",
        &[
            "convert_animation_vec3",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/accessors.rs",
        &["pub fn channels(&self)", "pub const fn duration_seconds"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/mixers.rs",
        &[
            "pub fn create_animation_mixer",
            "pub fn animation_mixer",
            "pub fn play_animation",
            "pub fn pause_animation",
            "pub fn stop_animation",
            "pub fn seek_animation",
            "pub fn set_animation_speed",
            "pub fn set_animation_loop_mode",
            "pub fn update_animation",
            "AnimationError::StaleMixer",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/skinning.rs",
        &[
            "pub struct SceneSkinBinding",
            "pub fn skin_binding",
            "pub fn skin_matrices",
            "set_initial_skin_binding",
            "world_transform",
            "SkinningMatrix::inverse_from_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/morphs.rs",
        &[
            "pub fn morph_weights",
            "pub fn set_morph_weights",
            "set_initial_morph_weights",
            "set_morph_weights_unchecked",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry.rs",
        &[
            "InvalidMorphTargetVertexCount",
            "InvalidSkinJointVertexCount",
            "InvalidSkinWeightVertexCount",
            "InvalidSkinJointIndex",
            "GeometryMorphTarget",
            "GeometrySkin",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry/morph.rs",
        &[
            "pub struct GeometryMorphTarget",
            "pub fn with_morph_targets",
            "pub fn morphed_vertices",
            "InvalidMorphTargetVertexCount",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry/skinning.rs",
        &[
            "pub struct GeometrySkin",
            "pub struct SkinningMatrix",
            "pub fn with_skin",
            "pub fn skinned_vertices",
            "from_gltf_column_major",
            "inverse_from_transform",
            "pub fn then",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/read.rs",
        &[
            "\"targets\"",
            "GeometryMorphTarget::new",
            "\"weights\"",
            "\"JOINTS_0\"",
            "\"WEIGHTS_0\"",
            "GeometrySkin::new",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/accessor/skin.rs",
        &[
            "read_mat4_accessor",
            "read_joints_accessor",
            "read_weights_accessor",
            "SkinningMatrix::from_gltf_column_major",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf.rs",
        &[
            "pub use self::skins::SceneAssetSkin",
            "parse_skins",
            "pub fn skins(&self)",
            "pub const fn skin(&self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/skins.rs",
        &[
            "pub struct SceneAssetSkin",
            "parse_skins",
            "inverseBindMatrices",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/render/prepare.rs",
        &[
            "scene.skin_matrices(node)",
            "skinned_vertices",
            "InvalidSkinGeometry",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/diagnostics.rs",
        &[
            "pub enum AnimationError",
            "StaleMixer",
            "InvalidSkinIndex",
            "InvalidSkinJointIndex",
            "InvalidSkinGeometry",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_gltf_animation.rs",
        &[
            "mixer_controls_rebind_translation_channels_to_import_local_nodes",
            "playing_paused_and_seek_animation_dirty_prepared_render_state",
            "replace_import_invalidates_animation_mixers_with_stale_error",
            "gltf_animation_supports_rotation_scale_weights_and_normalizes_quaternions",
            "morph_target_weights_channel_updates_scene_morph_weights",
            "skinning_rebinds_joints_and_deforms_vertices_from_skeleton_hierarchy",
            "combined_morph_and_skinning_deforms_morphed_vertices_through_joint_matrices",
            "interpolation_handles_step_cubic_spline_and_quaternion_slerp",
            "khronos_sample_assets_load_instantiate_and_cover_animation_contracts",
            "steady_animation_update_reprepare_keeps_resource_counts_stable",
            "AnimationLoopMode::Repeat",
            "AnimationPlaybackState::Playing",
            "AnimationError::StaleMixer",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/assets/gltf/khronos/manifest.toml",
        &[
            "https://github.com/KhronosGroup/glTF-Sample-Assets",
            "2bac6f8c57bf471df0d2a1e8a8ec023c7801dddf",
            "RiggedSimple",
            "SimpleSkin",
            "SimpleMorph",
            "MorphCube",
            "RiggedFigure",
            "BrainStem",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_visual_proof.rs",
        &[
            "m3b_headless_visual_artifacts_cover_khronos_skin_morph_and_animation",
            "m3b-khronos-simple-skin",
            "m3b-khronos-simple-morph",
            "m3b-khronos-rigged-simple",
            "write_ppm_artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_browser_rendered_output.rs",
        &[
            "m3b_browser_wasm_renders_morph_animation_to_canvas",
            "browser_canvas_roundtrip",
            "render_morph_animation_frame",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/visual/fixtures/m3b-headless-animation.toml",
        &[
            "m3b-headless-animation",
            "max_abs_diff = 0",
            "m3b-khronos-simple-skin",
            "m3b-khronos-simple-morph",
            "m3b-khronos-rigged-simple",
        ],
    );
}

fn check_material_desc_fields_private(root: &Path, findings: &mut Vec<Finding>) {
    let path = root.join("src/material.rs");
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };

    for field in public_fields_in_struct(&text, "MaterialDesc") {
        findings.push(Finding::new(
            "ARCH-ASSET-API",
            format!("src/material.rs MaterialDesc exposes public field '{field}'"),
        ));
    }
}

fn check_m4_platform_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/scene/dirty.rs",
        &[
            "pub struct SceneDirtyState",
            "transform_revision",
            "pub fn dirty_state",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum HardwareTier",
            "pub hardware_tier: HardwareTier",
            "pub gpu_frustum_culling: CapabilityStatus",
            "pub per_instance_culling: CapabilityStatus",
            "pub compute_shaders: CapabilityStatus",
            "pub storage_buffers: CapabilityStatus",
            "HardwareTier::Medium",
            "Backend::WebGl2 => 128",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/settings.rs",
        &[
            "pub enum Profile",
            "pub enum Quality",
            "pub enum RenderMode",
            "pub struct RendererOptions",
            "OnChange",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render.rs",
        &[
            "render_generation",
            "skipped_frames",
            "culling::cull_cpu_frustum",
            "gpu_culling_dispatches",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/build.rs",
        &[
            "headless_with_options",
            "from_surface_with_options",
            "RenderMode::OnChange",
            "resolve_quality",
            "resolve_render_mode",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/surface.rs",
        &[
            "handle_surface_event",
            "recover_surface",
            "recover_context",
            "RetainPolicy::Never",
            "loss_error",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/platform.rs",
        &[
            "ScaleFactorChanged",
            "Occluded",
            "Lost",
            "ContextLost",
            "ContextRestored",
            "DeviceLost",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/culling.rs",
        &["cull_cpu_frustum", "outside_clip_box", "culled"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/render/gpu/culling.rs",
        &[
            "create_culling_pipeline",
            "encode_culling_dispatch",
            "@compute @workgroup_size(64)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "src/controls.rs",
        &[
            "pub struct OrbitControls",
            "pub struct PointerEvent",
            "pub enum PointerButton",
            "pub enum OrbitControlAction",
            "handle_pointer",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "Cargo.toml",
        &[
            "controls = []",
            "controls-winit = [\"controls\"]",
            "controls-web = [\"controls\"]",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/m4_performance_platform.rs",
        &[
            "capability_matrix_reports_hardware_tier_and_backend_feature_states",
            "transform_dirty_state_propagates_through_world_transform_queries",
            "renderer_options_apply_profile_quality_and_render_mode_precedence",
            "on_change_render_static_idle_records_skipped_frame_stats",
            "render_on_change_static_idle_skip_has_zero_allocations",
            "cpu_frustum_culling_drops_offscreen_renderables_before_draw",
            "per_instance_cpu_culling_keeps_visible_instances_and_counts_culled_ones",
            "gpu_capable_renderer_records_compute_culling_dispatch_when_available",
            "surface_loss_requires_recovery_and_prepare_before_render",
            "dpr_change_marks_surface_state_dirty_until_prepare",
            "context_recovery_rejects_assets_without_retained_cpu_data",
            "public_threading_contract_is_statically_enforced",
            "orbit_controls_are_platform_neutral_pointer_actions",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m4_platform_smoke.html",
        &[
            "scena.capabilities.v1",
            "linux-webgpu-chromium",
            "linux-webgl2-chromium",
            "gpu_frustum_culling",
            "per_instance_culling",
            "event_sequence",
            "recover_context",
            "webglcontextlost",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "tests/browser/m4_platform_smoke.js",
        &[
            "m4-platform-browser-smoke",
            "webgl2",
            "webgpu",
            "capabilities",
            "loss",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M4-PLATFORM",
        "docs/checklists/m4-performance-platform.md",
        &[
            "m4_performance_platform",
            "m4-platform-browser-smoke.json",
            "m4-wasm-size.json",
            "brotli_q11_bytes",
            "ARCH-M4-PLATFORM",
        ],
    );
}

fn check_m5_release_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "ARCH-M5-RELEASE", REQUIRED_EXAMPLES);
    require_files(
        root,
        findings,
        "ARCH-M5-RELEASE",
        REQUIRED_M5_GATE_ARTIFACTS,
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "Cargo.toml",
        &[
            "version = \"1.0.0\"",
            "rust-version = ",
            "documentation = \"https://docs.rs/scena\"",
            "keywords = [",
            "categories = [",
            "include = [",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/diagnostics.rs",
        &[
            "pub enum DebugOverlay",
            "RendererChanged",
            "DebugOverlay",
            "pub struct RendererStats",
            "pub enum BuildError",
            "pub enum AssetError",
            "pub enum ImportError",
            "pub enum InstantiateError",
            "pub enum PrepareError",
            "pub enum RenderError",
            "pub enum LookupError",
            "pub enum AnimationError",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/render/settings.rs",
        &["pub fn debug_overlay", "pub fn set_debug", "debug_revision"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/render.rs",
        &[
            "debug_revision",
            "NotPreparedReason::RendererChanged",
            "ChangeKind::DebugOverlay",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "src/bin/scena-convert.rs",
        &[
            "scena-convert",
            "FBX to glTF",
            "FBX2glTF",
            "--dry-run",
            "planned",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/api/m5-public-api-baseline.txt",
        &[
            "Renderer::prepare",
            "Renderer::render",
            "Renderer::set_debug",
            "DebugOverlay",
            "RendererStats",
            "BuildError",
            "RenderError",
            "SceneImport",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/api/m5-semver-baseline.toml",
        &[
            "version = \"1.0.0\"",
            "api_baseline = \"cargo run -p xtask -- doctor --full\"",
            "BuildError",
            "AnimationError",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "tests/m5_release.rs",
        &[
            "m5_debug_overlay_api_is_public_and_requires_prepare_after_change",
            "m5_public_api_baseline_names_frozen_contracts",
            "m5_benchmark_report_writes_required_scene_rows",
            "scena_convert_cli_reports_fbx_to_gltf_plan",
            "m5-benchmarks",
            "m5-public-api-freeze",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/specs/release-gates.md",
        &[
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/checklists/m5-v1-release.md",
        &[
            "m5_release",
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
            "cargo publish --dry-run",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "docs/checklists/acceptance-index.md",
        &[
            "m5-benchmarks.json",
            "m5-public-api-freeze.json",
            "cargo check --examples",
            "cargo publish --dry-run",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "target/gate-artifacts/m5-benchmarks.json",
        &[
            "\"gate\": \"m5-benchmarks\"",
            "\"status\": \"passed\"",
            "static-viewer",
            "standard-model-viewer-gltf",
            "larger-industrial-gltf",
            "high-instance",
            "headless-4k",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M5-RELEASE",
        "target/gate-artifacts/m5-public-api-freeze.json",
        &[
            "\"gate\":\"m5-public-api-freeze\"",
            "\"status\":\"passed\"",
            "docs/api/m5-public-api-baseline.txt",
        ],
    );
}

const REQUIRED_EXAMPLES: &[&str] = &[
    "examples/primitive_shapes.rs",
    "examples/glb_model_viewer.rs",
    "examples/picking_selection_hover.rs",
    "examples/instancing.rs",
    "examples/labels_helpers.rs",
    "examples/animation.rs",
    "examples/native_window.rs",
    "examples/browser_canvas.rs",
    "examples/headless_ci.rs",
    "examples/industrial_static_scene.rs",
];

const REQUIRED_M5_GATE_ARTIFACTS: &[&str] = &[
    "target/gate-artifacts/m5-benchmarks.json",
    "target/gate-artifacts/m5-public-api-freeze.json",
];

fn public_fields_in_struct(text: &str, struct_name: &str) -> Vec<String> {
    let Some(body) = braced_body_after(text, &format!("struct {struct_name}")) else {
        return Vec::new();
    };

    body.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub ") || line.starts_with("pub("))
        .map(|line| line.trim_end_matches(',').to_string())
        .collect()
}

fn braced_body_after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let marker_start = text.find(marker)?;
    let search_start = marker_start + marker.len();
    let brace_start = text[search_start..].find('{')? + search_start;
    let mut depth = 0usize;

    for (offset, character) in text[brace_start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&text[brace_start + 1..brace_start + offset]);
                }
            }
            _ => {}
        }
    }

    None
}

fn check_solid_kiss(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SOLID-KISS-DOCS",
        "docs/specs/module-boundaries.md",
        &[
            "## SOLID/KISS Gate",
            "Every public feature must name exactly one owner module",
            "No catch-all `Manager`, `Engine`, `World`, or broad `Context` type",
        ],
    );

    for rel in source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        let significant_lines = significant_line_count(&text);
        if significant_lines > MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE {
            findings.push(Finding::new(
                "ARCH-KISS-SIZE",
                format!(
                    "{} has {significant_lines} significant lines; split before exceeding {MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE}",
                    rel.display()
                ),
            ));
        }

        for (line_index, type_name) in declared_type_names(&text) {
            if is_catch_all_type_name(&type_name) {
                findings.push(Finding::new(
                    "ARCH-SOLID-CATCH-ALL",
                    format!(
                        "{}:{} declares catch-all type '{}'; use an owner-specific type or add an ADR-backed doctor allowlist",
                        rel.display(),
                        line_index + 1,
                        type_name
                    ),
                ));
            }
        }
    }
}

fn significant_line_count(text: &str) -> usize {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("//"))
        .count()
}

fn declared_type_names(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| declared_type_name(line).map(|name| (index, name)))
        .collect()
}

fn declared_type_name(line: &str) -> Option<String> {
    let line = line.trim_start();
    let line = line.strip_prefix("pub ").unwrap_or(line);
    let line = line.strip_prefix("crate ").unwrap_or(line);
    let line = line
        .strip_prefix("struct ")
        .or_else(|| line.strip_prefix("enum "))
        .or_else(|| line.strip_prefix("type "))
        .or_else(|| line.strip_prefix("trait "))?;
    let name = line
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .next()
        .unwrap_or_default();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn is_catch_all_type_name(name: &str) -> bool {
    if ALLOWED_CONTEXT_TYPES.contains(&name) {
        return false;
    }
    CATCH_ALL_TYPE_NAMES.contains(&name)
        || CATCH_ALL_TYPE_SUFFIXES
            .iter()
            .any(|suffix| name.ends_with(suffix))
        || name == "Context"
        || (name.ends_with("Context") && name.len() > "Context".len())
}

fn forbid_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    forbid_contains_path(root, findings, rule, Path::new(rel), needles);
}

fn forbid_contains_path(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &Path,
    needles: &[&str],
) {
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };

    for needle in needles {
        if text.contains(needle) {
            findings.push(Finding::new(
                rule,
                format!(
                    "{} contains forbidden boundary text '{}'",
                    rel.display(),
                    needle
                ),
            ));
        }
    }
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_source_files(&root.join("src"), Path::new("src"), &mut files);
    files.sort();
    files
}

fn collect_source_files(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if path.is_dir() {
            collect_source_files(&path, &rel, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(rel);
        }
    }
}

fn contains_scope_term(lower_text: &str, term: &str) -> bool {
    if term.contains(' ') {
        return lower_text.contains(term);
    }

    lower_text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|token| token == term)
}

fn check_unit_test_first_governance(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "TEST-FIRST-AGENTS",
        "AGENTS.md",
        &[
            "## Unit Test First Rule",
            "Run the focused test and confirm it fails for the expected reason",
            "Do not mark a checklist implementation item complete without naming the test-first proof",
        ],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-SKILL-QUALITY",
        ".codex/skills/scena-renderer-quality/SKILL.md",
        &[
            "## Unit Test First Workflow",
            "Run the focused test and verify the failure is the expected failure",
        ],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-SKILL-ARCH",
        ".codex/skills/scena-renderer-architecture/SKILL.md",
        &["Before production implementation"],
    );
    require_contains(
        root,
        findings,
        "TEST-FIRST-DOCTOR-CONTRACT",
        "docs/specs/doctor-contract.md",
        &["unit-test-first governance"],
    );

    for rel in MILESTONE_CHECKLISTS {
        require_contains(
            root,
            findings,
            "TEST-FIRST-CHECKLIST",
            rel,
            &["Unit-test-first evidence"],
        );
    }
}

const MILESTONE_CHECKLISTS: &[&str] = &[
    "docs/checklists/m0-foundation.md",
    "docs/checklists/m1-geometry-materials.md",
    "docs/checklists/m2-lighting-depth-clipping.md",
    "docs/checklists/m3a-app-features.md",
    "docs/checklists/m3b-gltf-animation.md",
    "docs/checklists/m4-performance-platform.md",
    "docs/checklists/m5-v1-release.md",
    "docs/checklists/acceptance-index.md",
];

fn check_backend_vocabulary(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/platform.rs",
        &["browser_webgpu_canvas", "browser_webgl2_canvas"],
    );
    require_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/diagnostics/capabilities.rs",
        &["WebGpu", "WebGl2"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-BACKEND-VOCAB",
        "src/diagnostics.rs",
        &["BrowserSurface"],
    );
}

fn check_agent_validation(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "AGENTS-VALIDATION",
        "AGENTS.md",
        &["cargo run -p xtask -- doctor --full", "Use `scena-doctor`"],
    );
    require_contains(
        root,
        findings,
        "SKILL-DOCTOR",
        ".codex/skills/scena-doctor/SKILL.md",
        &["cargo run -p xtask -- doctor --full"],
    );
}

fn check_default_environment_manifest(root: &Path, findings: &mut Vec<Finding>) {
    let manifest_rel = "tests/assets/environment/default-environment.toml";
    let manifest_path = root.join(manifest_rel);
    let Ok(text) = fs::read_to_string(&manifest_path) else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("missing required default environment manifest {manifest_rel}"),
        ));
        return;
    };

    require_manifest_value(findings, manifest_rel, &text, "name", "neutral-studio");
    require_manifest_value(findings, manifest_rel, &text, "license", "CC0-1.0");
    require_manifest_value(findings, manifest_rel, &text, "wasm_delivery", "bundled");
    require_manifest_value(findings, manifest_rel, &text, "status", "generated-fixture");
    require_manifest_u32(findings, manifest_rel, &text, "cubemap_resolution", 256);
    require_manifest_u32(findings, manifest_rel, &text, "brdf_lut_size", 256);

    let Some(source_path) = quoted_manifest_assignment(&text, "source_path") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing source_path"),
        ));
        return;
    };
    let Some(source_sha256) = quoted_manifest_assignment(&text, "source_sha256") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing source_sha256"),
        ));
        return;
    };
    let Some(generator) = quoted_manifest_assignment(&text, "generator") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing generator"),
        ));
        return;
    };
    if !generator.contains(&source_path) {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} generator does not reference source_path"),
        ));
    }
    check_manifest_file_hash(root, findings, manifest_rel, &source_path, &source_sha256);

    let derivatives = derivative_manifest_entries(&text);
    if derivatives.len() < 2 {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} must declare at least cubemap and BRDF LUT derivatives"),
        ));
    }
    for (path, sha256) in derivatives {
        if path.contains("placeholder") {
            findings.push(Finding::new(
                "VISUAL-DEFAULT-ENV",
                format!("{manifest_rel} derivative {path} still points at a placeholder file"),
            ));
        }
        check_default_environment_derivative_payload(root, findings, manifest_rel, &path);
        check_manifest_file_hash(root, findings, manifest_rel, &path, &sha256);
    }
}

fn check_default_environment_derivative_payload(
    root: &Path,
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    path: &str,
) {
    let Ok(text) = fs::read_to_string(root.join(path)) else {
        return;
    };
    if text.contains("not a renderer-consumable") {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} derivative {path} declares itself non-consumable"),
        ));
    }
    let valid_magic =
        text.starts_with("SCENA_CUBEMAP_V1\n") || text.starts_with("SCENA_BRDF_LUT_V1\n");
    if !valid_magic {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} derivative {path} is missing a scena environment magic header"),
        ));
    }
}

fn check_visual_fixture_metadata(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/visual/fixtures/m1-headless-core.toml",
        &[
            "[suite]",
            "name = \"m1-headless-core\"",
            "format = \"ppm\"",
            "encoding = \"srgb8\"",
            "artifact_dir = \"target/gate-artifacts/m1-visual\"",
            "reference = \"tests/visual/references/m1-headless-core.toml\"",
            "reference_mode = \"sampled-rgba\"",
            "max_abs_diff = 0",
            "name = \"primitive-fullscreen\"",
            "name = \"unlit-asset-mesh\"",
            "name = \"pbr-asset-mesh\"",
            "name = \"transparent-blend\"",
            "name = \"line-material\"",
            "name = \"wire-edge-materials\"",
            "name = \"default-cube\"",
            "luminance_gate = \"center-nonblack\"",
            "silhouette_gate = \"corner-black\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/visual/references/m1-headless-core.toml",
        &[
            "[suite]",
            "status = \"reference\"",
            "max_abs_diff = 0",
            "center_rgba = [119, 177, 204, 255]",
            "nonblack_pixels = 109",
            "rgba_hash = \"fnv1a64:1b305a55001a2b13\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/m1_visual_proof.rs",
        &[
            "m1_headless_visual_artifacts_cover_core_material_paths",
            "m1_headless_reference_tolerances_match_current_fixtures",
            "write_ppm_artifact",
            "target/gate-artifacts/m1-visual",
            "include_str!(\"visual/fixtures/m1-headless-core.toml\")",
            "include_str!(\"visual/references/m1-headless-core.toml\")",
            "rgba_within_tolerance",
            "rgba_fnv1a64",
        ],
    );
}

fn check_m2_visual_fixture_metadata(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/visual/fixtures/m2-headless-core.toml",
        &[
            "[suite]",
            "name = \"m2-headless-core\"",
            "format = \"ppm\"",
            "encoding = \"srgb8\"",
            "artifact_dir = \"target/gate-artifacts/m2-visual\"",
            "reference = \"tests/visual/references/m2-headless-core.toml\"",
            "reference_mode = \"sampled-rgba\"",
            "max_abs_diff = 0",
            "name = \"direct-lights-pbr\"",
            "name = \"shadowed-directional-light\"",
            "name = \"ibl-environment\"",
            "name = \"fxaa-edge\"",
            "name = \"clipping-half-space\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/visual/references/m2-headless-core.toml",
        &[
            "[suite]",
            "status = \"reference\"",
            "max_abs_diff = 0",
            "center_rgba = [216, 0, 9, 255]",
            "center_rgba = [68, 68, 68, 255]",
            "nonblack_pixels = 141",
            "rgba_hash = \"fnv1a64:53e497bce0ce2aed\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/m2_visual_proof.rs",
        &[
            "m2_headless_visual_artifacts_cover_lighting_depth_and_clipping",
            "m2_headless_reference_tolerances_match_current_fixtures",
            "write_ppm_artifact",
            "target/gate-artifacts/m2-visual",
            "include_str!(\"visual/fixtures/m2-headless-core.toml\")",
            "include_str!(\"visual/references/m2-headless-core.toml\")",
            "validate_shadowed_directional_light",
            "validate_ibl_environment",
            "validate_clipping_half_space",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "m2_headless_visual_artifacts_cover_lighting_depth_and_clipping",
            "m2-headless-core.toml",
            "VISUAL-M2-FIXTURE-METADATA",
        ],
    );
}

fn check_m1_browser_rendered_output(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "Cargo.toml",
        &[
            "wasm-bindgen",
            "wasm-bindgen-test",
            "CanvasRenderingContext2d",
            "ImageData",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "tests/m1_browser_rendered_output.rs",
        &[
            "wasm_bindgen_test_configure!(run_in_browser)",
            "fn m1_browser_wasm_renders_color_and_alpha_to_canvas",
            "fn m1_browser_wasm_renders_technical_materials_to_canvas",
            "Renderer::headless(4, 4)",
            "MaterialDesc::line",
            "MaterialDesc::wireframe",
            "MaterialDesc::edge",
            "put_image_data",
            "get_image_data",
            "[158, 0, 159, 255]",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "docs/checklists/m1-geometry-materials.md",
        &[
            "m1_browser_rendered_output",
            "Rust/WASM browser rendered-output proof",
        ],
    );
}

fn check_m2_browser_rendered_output(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "tests/browser/m2_browser_lighting_clipping_smoke.js",
        &[
            "m2_browser_lighting_clipping_smoke.html",
            "scenaM2BrowserLightingClippingSmoke",
            "webgl2",
            "webgpu",
            "m2-browser-lighting-clipping-smoke.json",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "tests/browser/m2_browser_lighting_clipping_smoke.html",
        &[
            "runWebGpuScene",
            "runWebGl2Scene",
            "directLightPassed",
            "clippingPassed",
            "vec4<f32>(1.0, 0.0, 0.0, 1.0)",
            "vec4(1.0, 0.0, 0.0, 1.0)",
            "directCenter",
            "clippingLeft",
            "clippingRight",
            "clippingNonBlackPixels",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "node tests/browser/m2_browser_lighting_clipping_smoke.js",
            "m2-browser-lighting-clipping-smoke.json",
            "VISUAL-BROWSER-M2",
        ],
    );
}

fn check_m6_browser_renderer_probe(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "Cargo.toml",
        &[
            "browser-probe",
            "WebGl2RenderingContext",
            "WebGlProgram",
            "WebGlShader",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe.rs",
        &[
            "m6RenderWebgl2Probe",
            "m6RenderWebgpuProbe",
            "m6RenderWorkflowProbe",
            "m6RenderSurfaceLifecycleProbe",
            "m6RenderBenchmarkProbe",
            "Renderer::from_surface_async",
            "prepare_with_assets",
            "Renderer::render",
            "scena.m6.browser_renderer_probe.v1",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/probes.rs",
        &[
            "scena.m6.browser_surface_lifecycle_probe.v1",
            "scena.m6.browser_benchmark_probe.v1",
            "surface-context-lifecycle",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/browser_probe/workflows.rs",
        &[
            "model-viewer",
            "instancing",
            "picking-selection",
            "animation",
            "labels-helpers",
            "industrial-static-scene",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "src/render/gpu/build.rs",
        &[
            "create_browser_canvas_surface",
            "WebCanvasWindowHandle",
            "WebDisplayHandle",
            "raw_display_handle: Some(raw_display_handle)",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "m6-rust-wasm-renderer-probe",
            "scenaM6RustWasmRendererProbe",
            "scenaM6RustWasmWorkflowProbe",
            "scenaM6RustWasmLifecycleProbe",
            "scenaM6RustWasmBenchmarkProbe",
            "/fixtures/",
            "webgl2",
            "webgpu",
            "m6-rust-wasm-renderer-probe.json",
            "model-viewer",
            "instancing",
            "picking-selection",
            "animation",
            "labels-helpers",
            "industrial-static-scene",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "tests/browser/m6_rust_wasm_renderer_probe_page.js",
        &[
            "m6RenderWebgl2Probe",
            "m6RenderWebgpuProbe",
            "m6RenderWorkflowProbe",
            "m6RenderSurfaceLifecycleProbe",
            "m6RenderBenchmarkProbe",
            "readWebGl2Pixels",
            "scenaM6RustWasmWorkflowProbe",
            "scenaM6RustWasmLifecycleProbe",
            "scenaM6RustWasmBenchmarkProbe",
            "nonblack",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M6",
        "docs/checklists/m6-browser-renderer-parity.md",
        &[
            "wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe",
            "node tests/browser/m6_rust_wasm_renderer_probe.js",
            "VISUAL-BROWSER-M6",
        ],
    );
}

fn check_m9_ci_release_lanes(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        ".github/workflows/ci.yml",
        &[
            "linux-native-vulkan",
            "linux-browser-webgl2",
            "linux-browser-webgpu",
            "wasm32",
            "macos-metal",
            "windows-dx12",
            "dtolnay/rust-toolchain@1.93.1",
            "node-version: \"20.20.0\"",
            "PLAYWRIGHT_VERSION: \"1.59.1\"",
            "BINARYEN_VERSION: \"129.0.0\"",
            "BROTLI_CLI_VERSION: \"2.1.1\"",
            "npm ci",
            "npx playwright install chromium --with-deps",
            "cargo install wasm-pack --version 0.14.0",
            "npm run wasm:size",
            "cargo run -p xtask -- doctor --full",
            "release-lane-artifact",
            "target/gate-artifacts/**",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        ".github/workflows/release.yml",
        &[
            "linux-native-vulkan",
            "linux-browser-webgl2",
            "linux-browser-webgpu",
            "wasm32-unknown-unknown",
            "macos-metal",
            "windows-dx12",
            "cargo publish --dry-run",
            "cargo publish",
            "gh release create",
            "needs:",
            "release-lane-artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "package.json",
        &[
            "\"node\": \"20.20.0\"",
            "\"npm\": \"10.8.2\"",
            "\"playwright\": \"1.59.1\"",
            "\"binaryen\": \"129.0.0\"",
            "\"brotli-cli\": \"2.1.1\"",
            "\"browser:m6\"",
            "\"wasm:size\"",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "package-lock.json",
        &["\"playwright\": \"1.59.1\"", "\"version\": \"1.59.1\""],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "Cargo.toml",
        &[
            "repository = \"https://github.com/johannesPettersson80/scena\"",
            "authors = [\"Johannes Pettersson <johannes_salomon@hotmail.com>\"]",
            "documentation = \"https://docs.rs/scena\"",
            "license = \"MIT OR Apache-2.0\"",
            "readme = \"README.md\"",
            "keywords = [\"renderer\", \"scene-graph\", \"gltf\", \"webgpu\", \"wasm\"]",
            "categories = [\"graphics\", \"rendering\", \"wasm\"]",
            "CHANGELOG.md",
            "docs/api/m5-semver-baseline.toml",
            "[package.metadata.docs.rs]",
            "wasm32-unknown-unknown",
        ],
    );
    require_contains(
        root,
        findings,
        "RELEASE-CI-M9",
        "crates/xtask/src/main.rs",
        &["release-lane-artifact", "scena.release_lane.v1"],
    );
}

fn check_m10_claim_audit_contract(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "crates/xtask/src/main.rs",
        &[
            "claim-audit",
            "m10-claim-audit.json",
            "scena.m10.claim_audit.v1",
            "required_final_gates",
        ],
    );
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "docs/checklists/m10-threejs-replacement-acceptance.md",
        &["m10-claim-audit.json", "claim audit"],
    );
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "docs/api/m10-public-api-diff.md",
        &[
            "M10 Public API Diff From M5 Baseline",
            "Renderer::diagnose_scene",
            "AssetLoadControl",
            "AssetError::UnsupportedTextureFormat",
            "Semver Decision",
        ],
    );
    require_contains(
        root,
        findings,
        "CLAIM-AUDIT-M10",
        "docs/release-notes/v1.0.0-rc.md",
        &[
            "Release Candidate Notes",
            "Remaining Release Blockers",
            "does not claim to replace game engines",
        ],
    );
}

fn check_m7_ergonomics_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/controls.rs",
        &[
            "with_damping",
            "focus",
            "apply_to_scene",
            "damping_factor",
            "TouchEvent",
            "pub const fn wheel",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/platform.rs",
        &["SurfaceViewport", "ViewportChanged", "device_pixel_ratio"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/picking.rs",
        &["pick_pointer", "pick_and_select", "InvalidViewport"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/visibility.rs",
        &[
            "set_camera_layer_mask",
            "camera_layer_mask",
            "visible_for_active_camera",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/import.rs",
        &[
            "ImportAnchorDebugMetadata",
            "YUpLeftHanded",
            "ZUpLeftHanded",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/import/options.rs",
        &["meters_per_unit", "convert_position"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene/inspection.rs",
        &[
            "pub struct SceneInspectionReport",
            "pub struct SceneNodeInspection",
            "pub fn inspect(&self)",
            "visible_drawable_count",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/assets.rs",
        &[
            "create_static_batch",
            "create_static_batch_with_report",
            "assets.material(texture)",
            "assets.geometry(material)",
            "assets.texture(material)",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/scene.rs",
        &["scene.mesh(geometry, texture)"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry.rs",
        &["pub fn static_batch", "transform_point"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/static_batch.rs",
        &[
            "pub struct StaticBatchReport",
            "pub fn static_batch_report",
            "requires_prepare_after_rebuild",
            "picking_debug_instances",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/geometry/helpers.rs",
        &[
            "pub fn bounding_box",
            "pub fn camera_frustum",
            "pub fn light_helper",
            "pub fn anchor_marker",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/diagnostics/help.rs",
        &["add_default_camera", "anchors_named", "recover_context"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "src/render.rs",
        &[
            "pub fn diagnose_scene",
            "MissingActiveCamera",
            "InvisibleScene",
            "MissingLightingOrEnvironment",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "README.md",
        &[
            "## Happy Path",
            "examples/camera_framing.rs",
            "examples/anchor_alignment.rs",
            "examples/coordinate_units.rs",
            "examples/static_batching.rs",
            "examples/layers_visibility.rs",
            "examples/beginner_diagnostics.rs",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/first_visible_render.rs",
        &["add_default_camera", "render_active"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls.rs",
        &[
            "OrbitControls",
            "with_damping",
            "apply_to_scene",
            "TouchEvent",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls_native_adapter.rs",
        &[
            "NativeMouseButton",
            "native_press",
            "PointerEvent::released",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/orbit_controls_browser_adapter.rs",
        &["browser_pointer_drag", "browser_wheel", "browser_pinch"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/camera_framing.rs",
        &["scene.frame(", "scene.look_at(", "render_active"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/anchor_alignment.rs",
        &[
            "snap_anchor",
            "anchor(\"inspection\")",
            "anchor_debug_metadata",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/coordinate_units.rs",
        &[
            "SourceUnits::Millimeters",
            "SourceCoordinateSystem::ZUpRightHanded",
            "meters_per_unit",
            "convert_position",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/static_batching.rs",
        &[
            "create_static_batch_with_report",
            "requires_prepare_after_rebuild",
            "picking_debug_instances",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/layers_visibility.rs",
        &[
            "set_layer_mask",
            "set_camera_layer_mask",
            "set_render_group",
            "set_helper_on_top",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/beginner_diagnostics.rs",
        &[
            "diagnose_scene",
            "DiagnosticSeverity::Error",
            "add_default_camera",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/glb_model_viewer.rs",
        &["load_scene", "frame_import", "prepare_with_assets"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "examples/scene_inspection.rs",
        &["scene.inspect", "visible_drawable_count", "node.kind()"],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_threejs_ergonomics.rs",
        &[
            "create_static_batch",
            "pick_and_select",
            "set_camera_layer_mask",
            "SurfaceViewport",
            "ImportAnchorDebugMetadata",
            "with_damping",
            "m7_beginner_scene_diagnostics_explain_invisible_setups",
            "m7_error_display_snapshots_cover_beginner_recovery_paths",
            "m7_viewer_operations_dirty_prepare_without_persistent_resource_growth",
            "m7_benchmark_artifact_writes_required_viewer_workflow_rows",
            "m7-workflow-benchmarks.json",
            "scena.m7.workflow_benchmarks.v1",
            "create_static_batch_with_report",
            "picking_debug_instances",
            "m7_scene_inspection_feature_reports_reproducible_metadata",
        ],
    );
    require_contains(
        root,
        findings,
        "ERGONOMICS-M7",
        "tests/m7_visual_proof.rs",
        &[
            "m7_headless_visual_artifacts_cover_ergonomics_workflows",
            "target/gate-artifacts/m7-visual",
            "m7-first-render",
            "m7-first-glb",
            "m7-camera-frame",
            "m7-picking-selection",
            "m7-helpers",
            "m7-labels",
            "m7-controls",
            "m7-layers-helper-on-top",
            "m7-static-batching",
            "m7-anchor-alignment",
            "m7-coordinate-units",
            "m7-industrial-static-scene",
        ],
    );
}

fn check_m8_assets_materials_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/read.rs",
        &[
            "normalTexture",
            "metallicRoughnessTexture",
            "occlusionTexture",
            "emissiveTexture",
            "with_normal_texture_transform",
            "with_metallic_roughness_texture_transform",
            "with_occlusion_texture_transform",
            "with_emissive_texture_transform",
            "TextureColorSpace::Linear",
            "TextureColorSpace::Srgb",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/read/textures.rs",
        &[
            "AssetError::MissingTexture",
            "validate_texture_source_format",
            "basisu_texture_source_format",
            "KHR_texture_basisu",
            "TextureSourceFormat::Ktx2Basisu",
            "parse_sampler",
            "TextureWrap::MirroredRepeat",
            "TextureFilter::LinearMipmapLinear",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets.rs",
        &[
            "validate_texture_source_format",
            "UnsupportedTextureFormat",
            "TextureSourceFormat",
            "source_format",
            "load_scene_with_progress",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/load.rs",
        &[
            "pub struct AssetLoadControl",
            "pub struct AssetLoadReport",
            "pub enum AssetLoadProgress",
            "progress_events",
            "emit_progress",
            "AssetError::Cancelled",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets.rs",
        &["load_scene_with_report", "load_scene_controlled"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/extensions.rs",
        &[
            "pub enum GltfDecoderPolicy",
            "decoder_policy",
            "basis-universal",
            "feature: \"ktx2\"",
            "KHR_materials_clearcoat",
            "KHR_materials_transmission",
            "KHR_materials_ior",
            "KHR_materials_volume",
            "KHR_texture_basisu",
            "KHR_draco_mesh_compression",
            "EXT_meshopt_compression",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_missing_texture_slots_fail_with_actionable_asset_error",
            "GltfDecoderPolicy::FeatureFlag",
            "GltfDecoderPolicy::External",
            "normal_texture",
            "normal_texture_transform",
            "metallic_roughness_texture",
            "occlusion_texture",
            "emissive_texture",
            "emissive_texture_transform",
            "TextureWrap::MirroredRepeat",
            "TextureFilter::LinearMipmapLinear",
            "GltfExtensionStatus::Degraded",
            "m8_unsupported_texture_formats_fail_before_silent_handles_are_created",
            "m8_scene_load_reports_cache_fetch_and_external_buffer_metadata",
            "m8_cancelled_scene_load_does_not_cache_partial_asset_state",
            "m8_scene_load_progress_reports_fetch_parse_cache_and_external_buffers",
            "m8_ktx2_basisu_texture_requires_feature_or_explicit_decoder_policy",
            "m8_ktx2_basisu_feature_loads_compressed_texture_descriptor",
        ],
    );
}

fn check_gltf_asset_matrix_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ASSET-MATRIX-M8";
    let matrix_rel = "docs/assets/gltf-asset-matrix.md";
    let manifest_rel = "tests/assets/gltf/khronos/manifest.toml";

    let Ok(matrix) = fs::read_to_string(root.join(matrix_rel)) else {
        findings.push(Finding::new(RULE, format!("could not read {matrix_rel}")));
        return;
    };

    for required in [
        "Source/License",
        "Expected Diagnostics",
        "Rendered Output Reference",
        "fail with a structured error",
        "silent fallback",
    ] {
        if !matrix.contains(required) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} is missing asset-matrix contract text '{required}'"),
            ));
        }
    }

    let rows = gltf_asset_matrix_rows(&matrix);
    if rows.is_empty() {
        findings.push(Finding::new(
            RULE,
            format!("{matrix_rel} has no asset rows"),
        ));
        return;
    }

    let mut listed_local_fixtures = BTreeSet::new();
    let mut listed_khronos_assets = BTreeSet::new();

    for row in rows {
        if row.len() != 7 {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{}' must have 7 columns: asset, source/license, features, expected result, expected diagnostics, rendered-output reference, evidence",
                    row.join(" | ")
                ),
            ));
            continue;
        }

        let asset = row[0].trim();
        let source_license = row[1].trim();
        let features = row[2].trim();
        let expected = row[3].trim();
        let diagnostics = row[4].trim();
        let rendered_output = row[5].trim();
        let evidence = row[6].trim();
        let row_text = row.join(" | ");

        if contains_placeholder(&row_text) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row contains placeholder text: {row_text}"),
            ));
        }
        if source_license.is_empty()
            || !(source_license.to_ascii_lowercase().contains("license")
                || source_license.to_ascii_lowercase().contains("test-only"))
        {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row '{asset}' must name source and license/test-only status"),
            ));
        }
        if features.is_empty() {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row '{asset}' must name covered features"),
            ));
        }
        if !expected_result_is_explicit(expected) {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{asset}' must expect pass, degrade, fail, or defer explicitly"
                ),
            ));
        }
        if diagnostics.is_empty() || diagnostics.eq_ignore_ascii_case("none") {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{asset}' must record expected diagnostics or the explicit 'none expected' contract"
                ),
            ));
        }
        if rendered_output.is_empty()
            || rendered_output.eq_ignore_ascii_case("n/a")
            || contains_placeholder(rendered_output)
        {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{matrix_rel} row '{asset}' must name rendered-output proof or an explicit structured non-visual/deferred reason"
                ),
            ));
        }
        if evidence.is_empty() || contains_placeholder(evidence) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} row '{asset}' must link executable evidence"),
            ));
        }

        for evidence_path in backtick_values(evidence) {
            if is_local_evidence_path(&evidence_path) && !root.join(&evidence_path).is_file() {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} row '{asset}' links missing evidence {evidence_path}"),
                ));
            }
        }

        for rendered_path in backtick_values(rendered_output) {
            if is_local_evidence_path(&rendered_path) && !root.join(&rendered_path).exists() {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{matrix_rel} row '{asset}' links missing rendered-output reference {rendered_path}"
                    ),
                ));
            }
        }

        if let Some(local_fixture) = first_backtick_value(asset)
            && local_fixture.starts_with("tests/assets/gltf/")
            && !local_fixture.contains("/khronos/")
        {
            listed_local_fixtures.insert(local_fixture.clone());
            if !root.join(&local_fixture).is_file() {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} lists missing fixture {local_fixture}"),
                ));
            }
        }

        if let Some(memory_fixture) = first_backtick_value(asset)
            && memory_fixture.starts_with("memory://")
        {
            let source_lower = source_license.to_ascii_lowercase();
            if !(source_lower.contains("generated") && source_lower.contains("test-only")) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{matrix_rel} memory fixture '{memory_fixture}' must be marked generated test-only"
                    ),
                ));
            }
        }

        if asset.starts_with("Khronos ") {
            if let Some(name) = first_backtick_value(asset) {
                listed_khronos_assets.insert(name);
            } else {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} Khronos row '{asset}' must backtick the asset name"),
                ));
            }
            let source_lower = source_license.to_ascii_lowercase();
            if !(source_lower.contains("khronos") && source_lower.contains("license")) {
                findings.push(Finding::new(
                    RULE,
                    format!("{matrix_rel} Khronos row '{asset}' must name Khronos license source"),
                ));
            }
        }
    }

    for fixture in direct_gltf_fixture_paths(root) {
        if !listed_local_fixtures.contains(&fixture) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} is missing direct glTF fixture row for {fixture}"),
            ));
        }
    }

    let Ok(manifest) = fs::read_to_string(root.join(manifest_rel)) else {
        findings.push(Finding::new(RULE, format!("could not read {manifest_rel}")));
        return;
    };

    for required in ["repository = ", "commit = ", "license_reference = "] {
        if !manifest.contains(required) {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} is missing required source metadata '{required}'"),
            ));
        }
    }

    for name in khronos_manifest_asset_names(&manifest) {
        if !listed_khronos_assets.contains(&name) {
            findings.push(Finding::new(
                RULE,
                format!("{matrix_rel} is missing Khronos asset row for {name}"),
            ));
        }
    }

    for rel in khronos_manifest_file_paths(&manifest) {
        let full_rel = format!("tests/assets/gltf/khronos/{rel}");
        if !root.join(&full_rel).is_file() {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} references missing Khronos fixture file {full_rel}"),
            ));
        }
    }
}

fn gltf_asset_matrix_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|') && line.ends_with('|'))
        .filter_map(|line| {
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect::<Vec<_>>();
            match cells.first().map(String::as_str) {
                Some("Asset/Fixture") => None,
                Some(value) if value.chars().all(|ch| ch == '-') => None,
                Some(_) => Some(cells),
                None => None,
            }
        })
        .collect()
}

fn direct_gltf_fixture_paths(root: &Path) -> Vec<String> {
    let dir = root.join("tests/assets/gltf");
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut fixtures = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("gltf") {
                return None;
            }
            Some(format!(
                "tests/assets/gltf/{}",
                entry.file_name().to_string_lossy()
            ))
        })
        .collect::<Vec<_>>();
    fixtures.sort();
    fixtures
}

fn khronos_manifest_asset_names(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| quoted_assignment(line, "name"))
        .collect()
}

fn khronos_manifest_file_paths(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| quoted_assignment(line, "path").or_else(|| quoted_array_item(line)))
        .collect()
}

fn quoted_array_item(line: &str) -> Option<String> {
    line.strip_prefix('"')
        .and_then(|value| value.trim_end_matches(',').strip_suffix('"'))
        .map(str::to_string)
}

fn expected_result_is_explicit(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("pass")
        || lower.starts_with("degrade")
        || lower.starts_with("fail")
        || lower.starts_with("defer")
}

fn contains_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("todo")
        || lower.contains("tbd")
        || lower.contains("placeholder")
        || lower.contains("unknown")
}

fn is_local_evidence_path(value: &str) -> bool {
    value.starts_with("tests/")
        || value.starts_with("docs/")
        || value.starts_with("examples/")
        || value.starts_with("target/gate-artifacts/")
}

fn first_backtick_value(value: &str) -> Option<String> {
    backtick_values(value).into_iter().next()
}

fn backtick_values(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remainder = value;
    while let Some(start) = remainder.find('`') {
        let after_start = &remainder[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        values.push(after_start[..end].to_string());
        remainder = &after_start[end + 1..];
    }
    values
}

fn require_manifest_value(
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    text: &str,
    key: &str,
    expected: &str,
) {
    match quoted_manifest_assignment(text, key) {
        Some(value) if value == expected => {}
        Some(value) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} {key} is '{value}', expected '{expected}'"),
        )),
        None => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing {key}"),
        )),
    }
}

fn require_manifest_u32(
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    text: &str,
    key: &str,
    expected: u32,
) {
    match u32_manifest_assignment(text, key) {
        Some(value) if value == expected => {}
        Some(value) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} {key} is {value}, expected {expected}"),
        )),
        None => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing {key}"),
        )),
    }
}

fn check_manifest_file_hash(
    root: &Path,
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    rel: &str,
    expected_sha256: &str,
) {
    if !is_lower_hex_sha256(expected_sha256) {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} records invalid SHA-256 for {rel}"),
        ));
        return;
    }

    let path = root.join(rel);
    if !path.is_file() {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} references missing file {rel}"),
        ));
        return;
    }

    match sha256_hex(&path) {
        Ok(actual) if actual == expected_sha256 => {}
        Ok(actual) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} SHA-256 mismatch for {rel}: got {actual}"),
        )),
        Err(error) => findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("could not hash {rel}: {error}"),
        )),
    }
}

fn derivative_manifest_entries(text: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut current_path = None;
    for line in text.lines().map(str::trim) {
        if line == "[[derivative]]" {
            current_path = None;
            continue;
        }
        if let Some(path) = quoted_assignment(line, "path") {
            current_path = Some(path);
            continue;
        }
        if let Some(sha256) = quoted_assignment(line, "sha256")
            && let Some(path) = current_path.take()
        {
            entries.push((path, sha256));
        }
    }
    entries
}

fn quoted_manifest_assignment(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find_map(|line| quoted_assignment(line, key))
}

fn u32_manifest_assignment(text: &str, key: &str) -> Option<u32> {
    let prefix = format!("{key} = ");
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))
        .and_then(|value| value.parse().ok())
}

fn quoted_assignment(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    line.strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_string)
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(path: &Path) -> std::io::Result<String> {
    let digest = Sha256::digest(fs::read(path)?);
    Ok(format!("{digest:x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_doctor_modes() {
        assert_eq!(
            parse_command(vec!["doctor".into(), "--docs".into()]),
            Ok(Command::Doctor(DoctorMode::Docs))
        );
        assert_eq!(
            parse_command(vec!["doctor".into(), "--architecture".into()]),
            Ok(Command::Doctor(DoctorMode::Architecture))
        );
        assert_eq!(
            parse_command(vec!["doctor".into(), "--full".into()]),
            Ok(Command::Doctor(DoctorMode::Full))
        );
        assert_eq!(
            parse_command(vec!["doctor".into()]),
            Ok(Command::Doctor(DoctorMode::Full))
        );
        assert_eq!(
            parse_command(vec!["claim-audit".into()]),
            Ok(Command::ClaimAudit)
        );
        assert_eq!(
            parse_command(vec!["release-lane-artifact".into(), "macos-metal".into()]),
            Ok(Command::ReleaseLaneArtifact("macos-metal".into()))
        );
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse_command(vec!["check".into()]).is_err());
    }

    #[test]
    fn release_lane_artifacts_use_release_schema() {
        let artifact =
            release_lane_artifact("linux-webgpu-chromium").expect("known lane is accepted");

        assert_eq!(artifact["schema"], "scena.release_lane.v1");
        assert_eq!(artifact["lane"], "linux-webgpu-chromium");
        assert_eq!(artifact["backend"], "WebGpu");
        assert!(release_lane_artifact("unknown").is_err());
    }

    #[test]
    fn claim_categories_map_to_evidence_links() {
        let categories = claim_categories("Scene Renderer glTF WebGPU benchmark doctor");

        assert!(categories.contains(&"public-api"));
        assert!(categories.contains(&"assets-gltf"));
        assert!(categories.contains(&"browser-platform"));
        assert!(categories.contains(&"performance"));
        assert!(categories.contains(&"doctor"));
        assert!(
            evidence_links_for_category("assets-gltf")
                .iter()
                .any(|link| link.ends_with("m8_assets_materials_ecosystem.rs"))
        );
    }

    #[test]
    fn extracts_markdown_links() {
        let links = markdown_link_targets("A [doc](docs/a.md) and [external](https://x.test).");
        assert_eq!(links, vec!["docs/a.md", "https://x.test"]);
    }

    #[test]
    fn extracts_declared_type_names() {
        assert_eq!(
            declared_type_name("pub struct Renderer;"),
            Some("Renderer".into())
        );
        assert_eq!(declared_type_name("enum Backend {"), Some("Backend".into()));
        assert_eq!(declared_type_name("// pub struct Ignored;"), None);
    }

    #[test]
    fn catches_broad_type_names() {
        assert!(is_catch_all_type_name("SceneManager"));
        assert!(is_catch_all_type_name("World"));
        assert!(is_catch_all_type_name("AppContext"));
        assert!(!is_catch_all_type_name("InteractionContext"));
        assert!(!is_catch_all_type_name("Renderer"));
    }

    #[test]
    fn unit_test_first_governance_is_declared() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_unit_test_first_governance(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn browser_backend_vocabulary_is_explicit() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_backend_vocabulary(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn source_files_include_renderer_submodules() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let files = source_files(&root);

        assert!(
            files
                .iter()
                .any(|path| path == Path::new("src/render/gpu.rs"))
        );
    }

    #[test]
    fn source_scope_terms_match_whole_tokens() {
        assert!(contains_scope_term("a robot module", "robot"));
        assert!(!contains_scope_term("a roboticist module", "robot"));
    }

    #[test]
    fn asset_api_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_asset_api_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn render_alpha_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_render_alpha_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn output_stage_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_output_stage_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn fxaa_output_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_fxaa_output_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn diagnostics_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_diagnostics_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn renderer_stats_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_renderer_stats_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn prepare_asset_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_prepare_asset_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn environment_lifecycle_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_environment_lifecycle_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn equirectangular_hdr_environment_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_equirectangular_hdr_environment_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn environment_ibl_prepare_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_environment_ibl_prepare_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn scene_light_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_scene_light_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn direct_light_shading_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_direct_light_shading_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn directional_shadow_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_directional_shadow_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn shadow_map_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_shadow_map_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn depth_prepass_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_depth_prepass_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn reversed_z_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_reversed_z_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn webgl2_depth_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_webgl2_depth_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m2_leak_stats_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m2_leak_stats_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn camera_depth_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_camera_depth_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn origin_shift_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_origin_shift_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn clipping_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_clipping_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m3a_scene_import_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m3a_scene_import_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn default_environment_manifest_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_default_environment_manifest(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn visual_fixture_metadata_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_visual_fixture_metadata(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m2_visual_fixture_metadata_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m2_visual_fixture_metadata(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m1_browser_rendered_output_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m1_browser_rendered_output(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m2_browser_rendered_output_is_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m2_browser_rendered_output(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m9_release_metadata_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m9_ci_release_lanes(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m8_gltf_asset_matrix_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_gltf_asset_matrix_contract(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn m7_ergonomics_contracts_are_source_enforced() {
        let root = repo_root().expect("test runs inside the scena workspace");
        let mut findings = Vec::new();

        check_m7_ergonomics_contracts(&root, &mut findings);

        assert_eq!(findings, Vec::new());
    }

    #[test]
    fn public_fields_in_struct_detects_material_desc_visibility_regressions() {
        let source = r#"
            pub struct MaterialDesc {
                kind: MaterialKind,
                pub base_color: Color,
                pub(crate) roughness_factor: f32,
            }
        "#;

        assert_eq!(
            public_fields_in_struct(source, "MaterialDesc"),
            vec!["pub base_color: Color", "pub(crate) roughness_factor: f32"]
        );
    }
}
