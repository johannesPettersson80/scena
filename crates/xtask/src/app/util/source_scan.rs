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
        "docs/specs/module-boundaries.md",
        &[
            "## SOLID/KISS Gate",
            "Every public feature must name exactly one owner module",
            "No catch-all `Manager`, `Engine`, `World`, or broad `Context` type",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SOLID-KISS-DOCS",
        "docs/specs/architecture-contract.md",
        &[
            "No catch-all `Manager`, `Engine`, `World`, broad `Context`, `Registry`, or `ServiceLocator`",
            "Source modules should stay small enough to review",
            "Abstractions are allowed only when they remove real duplication or enforce a current contract",
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

pub(crate) fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_source_files(&root.join("src"), Path::new("src"), &mut files);
    files.sort();
    files
}

pub(crate) fn collect_source_files(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
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
