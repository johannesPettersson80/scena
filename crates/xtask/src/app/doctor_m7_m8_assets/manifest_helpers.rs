use crate::app::prelude::*;

pub(crate) fn khronos_manifest_file_hashes(text: &str) -> BTreeMap<String, String> {
    let mut hashes = BTreeMap::new();
    let mut in_file_hashes = false;

    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_file_hashes = line == "[file_hashes]";
            continue;
        }
        if !in_file_hashes || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        let Some(path) = key.strip_prefix('"').and_then(|key| key.strip_suffix('"')) else {
            continue;
        };
        let Some(sha256) = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        else {
            continue;
        };
        hashes.insert(path.to_string(), sha256.to_string());
    }

    hashes
}

pub(crate) fn quoted_array_item(line: &str) -> Option<String> {
    line.strip_prefix('"')
        .and_then(|value| value.trim_end_matches(',').strip_suffix('"'))
        .map(str::to_string)
}

pub(crate) fn expected_result_is_explicit(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("pass")
        || lower.starts_with("degrade")
        || lower.starts_with("fail")
        || lower.starts_with("defer")
}

pub(crate) fn contains_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("todo")
        || lower.contains("tbd")
        || lower.contains("placeholder")
        || lower.contains("unknown")
}

pub(crate) fn is_local_evidence_path(value: &str) -> bool {
    value.starts_with("tests/")
        || value.starts_with("docs/")
        || value.starts_with("examples/")
        || value.starts_with("target/gate-artifacts/")
}

pub(crate) fn first_backtick_value(value: &str) -> Option<String> {
    backtick_values(value).into_iter().next()
}

pub(crate) fn backtick_values(value: &str) -> Vec<String> {
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

pub(crate) fn require_manifest_value(
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

pub(crate) fn require_manifest_u32(
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

pub(crate) fn check_manifest_file_hash(
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

pub(crate) fn derivative_manifest_entries(text: &str) -> Vec<(String, String)> {
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

pub(crate) fn quoted_manifest_assignment(text: &str, key: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find_map(|line| quoted_assignment(line, key))
}

pub(crate) fn u32_manifest_assignment(text: &str, key: &str) -> Option<u32> {
    let prefix = format!("{key} = ");
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))
        .and_then(|value| value.parse().ok())
}

pub(crate) fn quoted_assignment(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = \"");
    line.strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix('"'))
        .map(str::to_string)
}

pub(crate) fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn sha256_hex(path: &Path) -> std::io::Result<String> {
    let digest = Sha256::digest(fs::read(path)?);
    Ok(digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>())
}
