use crate::app::prelude::*;

pub(crate) fn public_fields_in_struct(text: &str, struct_name: &str) -> Vec<String> {
    let Some(body) = braced_body_after(text, &format!("struct {struct_name}")) else {
        return Vec::new();
    };

    body.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("pub ") || line.starts_with("pub("))
        .map(|line| line.trim_end_matches(',').to_string())
        .collect()
}

pub(crate) fn braced_body_after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
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

pub(crate) fn check_solid_kiss(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SOLID-KISS-DOCS",
        "docs/api.md",
        &["Scene", "Assets", "Renderer", "SceneImport"],
    );

    let cfg_test_only = cfg_test_only_module_roots(root);
    for rel in source_files(root) {
        if cfg_test_only.iter().any(|test_root| {
            rel.as_path() == test_root.as_path()
                || rel.starts_with(test_module_child_directory(test_root))
        }) {
            continue;
        }
        let Ok(text) = read_source_to_string(root, &rel) else {
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

fn cfg_test_only_module_roots(root: &Path) -> BTreeSet<PathBuf> {
    let sources = source_files(root);
    let mut roots = BTreeSet::new();
    for owner in &sources {
        let Ok(text) = read_source_to_string(root, owner) else {
            continue;
        };
        let module_dir = if owner.file_name().and_then(OsStr::to_str) == Some("mod.rs") {
            owner.parent().unwrap_or(Path::new("src")).to_path_buf()
        } else {
            owner.with_extension("")
        };
        for name in rust_cfg_test_module_names(&text) {
            let flat = module_dir.join(format!("{name}.rs"));
            if root.join(&flat).is_file() {
                roots.insert(flat);
                continue;
            }
            let nested = module_dir.join(name).join("mod.rs");
            if root.join(&nested).is_file() {
                roots.insert(nested);
            }
        }
    }
    roots
}

fn test_module_child_directory(test_root: &Path) -> PathBuf {
    if test_root.file_name().and_then(OsStr::to_str) == Some("mod.rs") {
        test_root.parent().unwrap_or(test_root).to_path_buf()
    } else {
        test_root.with_extension("")
    }
}

pub(crate) fn significant_line_count(text: &str) -> usize {
    let mut count = 0;
    let mut brace_depth = 0i32;
    let mut pending_test_cfg = false;
    let mut skip_test_block_at_depth: Option<i32> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        let trimmed_start = line.trim_start();

        if let Some(skip_depth) = skip_test_block_at_depth {
            brace_depth += brace_delta(line);
            if brace_depth <= skip_depth {
                skip_test_block_at_depth = None;
            }
            continue;
        }

        if trimmed_start.starts_with("#[cfg(test")
            || trimmed_start.starts_with("#[cfg(all(test")
            || trimmed_start.starts_with("#[cfg(any(test")
        {
            pending_test_cfg = true;
            continue;
        }

        if pending_test_cfg {
            if trimmed_start.starts_with("mod ") && trimmed_start.contains('{') {
                let skip_depth = brace_depth;
                brace_depth += brace_delta(line);
                if brace_depth > skip_depth {
                    skip_test_block_at_depth = Some(skip_depth);
                }
                pending_test_cfg = false;
                continue;
            }
            pending_test_cfg = false;
        }

        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            count += 1;
        }
        brace_depth += brace_delta(line);
    }

    count
}

pub(crate) fn brace_delta(line: &str) -> i32 {
    line.matches('{').count() as i32 - line.matches('}').count() as i32
}

pub(crate) fn declared_type_names(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| declared_type_name(line).map(|name| (index, name)))
        .collect()
}

pub(crate) fn declared_type_name(line: &str) -> Option<String> {
    let line = strip_rust_visibility(line.trim_start());
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

pub(crate) fn strip_rust_visibility(line: &str) -> &str {
    let line = line.trim_start();
    let Some(rest) = line.strip_prefix("pub") else {
        return line;
    };
    let rest = rest.trim_start();
    if let Some(rest) = rest.strip_prefix('(') {
        let Some((_, after_visibility)) = rest.split_once(')') else {
            return line;
        };
        return after_visibility.trim_start();
    }
    rest
}

pub(crate) fn rust_cfg_test_module_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut cfg_test = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed == "#[cfg(test)]" {
            cfg_test = true;
            continue;
        }
        if cfg_test {
            let item = strip_rust_visibility(trimmed);
            if let Some(rest) = item.strip_prefix("mod ") {
                let name = rest
                    .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                    .next()
                    .unwrap_or_default();
                if !name.is_empty() {
                    names.insert(name.to_owned());
                }
            }
        }
        cfg_test = false;
    }
    names
}

pub(crate) fn is_catch_all_type_name(name: &str) -> bool {
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

pub(crate) fn forbid_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    forbid_contains_path(root, findings, rule, Path::new(rel), needles);
}

pub(crate) fn forbid_contains_path(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &Path,
    needles: &[&str],
) {
    let Ok(text) = read_source_to_string(root, rel) else {
        if rel
            .to_str()
            .is_some_and(crate::app::doctor_docs::is_retired_internal_doc)
        {
            return;
        }
        findings.push(Finding::new(
            rule,
            format!("could not read {} for forbidden-text scan", rel.display()),
        ));
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

pub(crate) fn source_files(root: &Path) -> Vec<PathBuf> {
    cached_rust_files_below(root, Path::new("src"))
}
