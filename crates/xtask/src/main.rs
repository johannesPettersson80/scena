use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let outcome = match parse_command(env::args().skip(1).collect()) {
        Ok(Command::Doctor(mode)) => run_doctor(mode),
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

    if args.first().map(String::as_str) != Some("doctor") {
        return Err(format!(
            "unknown command '{}'; expected 'doctor'",
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
        "Usage:\n  cargo run -p xtask -- doctor --docs\n  cargo run -p xtask -- doctor --architecture\n  cargo run -p xtask -- doctor --full"
    );
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
}

fn run_architecture_doctor(root: &Path, findings: &mut Vec<Finding>) {
    require_files(root, findings, "ARCH-REQUIRED", REQUIRED_SOURCE_MODULES);
    check_source_scope(root, findings);
    check_module_boundaries(root, findings);
    check_solid_kiss(root, findings);
    check_backend_vocabulary(root, findings);
    check_unit_test_first_governance(root, findings);
    check_agent_validation(root, findings);
}

const REQUIRED_DOCS: &[&str] = &[
    "AGENTS.md",
    "README.md",
    "docs/RFC-rust-3d-renderer.md",
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
    "src/assets.rs",
    "src/geometry.rs",
    "src/material.rs",
    "src/render.rs",
    "src/animation.rs",
    "src/controls.rs",
    "src/picking.rs",
    "src/diagnostics.rs",
    "src/platform.rs",
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
        "src/diagnostics.rs",
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
    }

    #[test]
    fn rejects_unknown_command() {
        assert!(parse_command(vec!["check".into()]).is_err());
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
}
