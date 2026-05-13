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
            "extras.scena.connectors[]",
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
            "cargo run -p xtask -- architecture-map",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-ARCHITECTURE-CONTRACT",
        "docs/specs/architecture-contract.md",
        &[
            "Every production feature must have exactly one owner module",
            "Dependency Direction",
            "Public API Ownership",
            "SOLID/KISS Gate",
            "Architecture Evidence",
            "cargo run -p xtask -- architecture-map",
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

pub(crate) fn require_contains(
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
