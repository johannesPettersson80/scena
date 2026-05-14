use crate::app::prelude::*;

pub(crate) fn check_markdown_links(root: &Path, findings: &mut Vec<Finding>) {
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

pub(crate) fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from("README.md"), PathBuf::from("AGENTS.md")];
    collect_markdown(&root.join("docs"), Path::new("docs"), &mut files);
    collect_markdown(
        &root.join(".codex/skills"),
        Path::new(".codex/skills"),
        &mut files,
    );
    files
}

pub(crate) fn collect_markdown(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
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

pub(crate) fn markdown_link_targets(text: &str) -> Vec<String> {
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

pub(crate) fn is_external_link(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("app://")
}

pub(crate) fn check_for_stale_doc_terms(root: &Path, findings: &mut Vec<Finding>) {
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

pub(crate) fn check_required_doc_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DOCS-PUBLIC-API",
        "docs/api.md",
        &[
            "Scene",
            "Assets",
            "Renderer",
            "SceneImport",
            "Typed handles",
            "Errors and diagnostics",
            "Stats and capabilities",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-LIFECYCLE",
        "docs/lifecycle.md",
        &["prepare", "render", "When to prepare again"],
    );
    require_contains(
        root,
        findings,
        "DOCS-GLTF",
        "docs/assets.md",
        &[
            "glTF/GLB",
            "External buffers and textures",
            "Units, axes, and handedness",
            "Anchors and connectors",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-VISUAL",
        "docs/headless-rendering.md",
        &["Headless rendering", "CI snapshots", "Renderer::headless"],
    );
    require_contains(
        root,
        findings,
        "DOCS-PLATFORM",
        "docs/platforms.md",
        &["WebGPU", "WebGL2", "wasm32-unknown-unknown"],
    );
    require_contains(
        root,
        findings,
        "DOCS-ERRORS",
        "docs/errors.md",
        &["AssetError", "RenderError", "PrepareError"],
    );
    require_contains(
        root,
        findings,
        "DOCS-README",
        "docs/README.md",
        &["Getting started", "Examples", "Troubleshooting"],
    );
}

pub(crate) fn require_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        if is_retired_internal_doc(rel) {
            return;
        }
        findings.push(Finding::new(rule, format!("could not read {rel}")));
        return;
    };

    // Phase 5.4 follow-up: extracted shader text lives in a sibling
    // `<module>_shader.wgsl` file. When checking a `.rs` module, fall
    // back to that sibling so doctor pins that name shader-text
    // strings (e.g. `var brdf_lut: texture_2d<f32>`) continue to
    // resolve after the extraction.
    let sibling = if rel.ends_with(".rs") {
        let stripped = rel.trim_end_matches(".rs");
        let sibling_rel = format!("{stripped}_shader.wgsl");
        fs::read_to_string(root.join(&sibling_rel)).ok()
    } else {
        None
    };

    for needle in needles {
        let found_in_primary = text.contains(needle);
        let found_in_sibling = sibling
            .as_deref()
            .is_some_and(|sibling_text| sibling_text.contains(needle));
        if !found_in_primary && !found_in_sibling {
            findings.push(Finding::new(
                rule,
                format!("{rel} is missing required contract text '{}'", needle),
            ));
        }
    }
}

pub(crate) fn is_retired_internal_doc(rel: &str) -> bool {
    rel == "docs/RFC-rust-3d-renderer.md"
        || rel == "docs/release-notes-template.md"
        || rel.starts_with("docs/specs/")
        || rel.starts_with("docs/checklists/")
        || rel.starts_with("docs/decisions/")
        || rel.starts_with("docs/api/")
        || rel.starts_with("docs/benchmarks/")
        || rel == "docs/assets/gltf-asset-matrix.md"
}

pub(crate) fn check_source_scope(root: &Path, findings: &mut Vec<Finding>) {
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
