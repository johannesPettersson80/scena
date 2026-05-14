use crate::app::prelude::*;

pub(crate) fn check_tangent_generation_dependency_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    require_contains(
        root,
        findings,
        "ARCH-TANGENT-DEPENDENCY",
        "Cargo.toml",
        &["bevy_mikktspace"],
    );

    for (rel, stale_needles) in [
        (
            "Cargo.toml",
            &["nalgebra = \"0.26", "nalgebra = { version = \"0.26"][..],
        ),
        (
            "Cargo.lock",
            &["name = \"mikktspace\"", "version = \"0.26."][..],
        ),
    ] {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for needle in stale_needles {
            if text.contains(needle) {
                findings.push(Finding::new(
                    "ARCH-TANGENT-DEPENDENCY",
                    format!(
                        "{rel} contains stale tangent/math dependency marker '{needle}'; use maintained bevy_mikktspace and do not reintroduce nalgebra 0.26"
                    ),
                ));
            }
        }
        if rel == "Cargo.toml" {
            for line in text.lines() {
                if line.trim_start().starts_with("mikktspace =") {
                    findings.push(Finding::new(
                        "ARCH-TANGENT-DEPENDENCY",
                        "Cargo.toml contains stale tangent dependency 'mikktspace'; use maintained bevy_mikktspace".to_string(),
                    ));
                }
            }
        }
    }
}

pub(crate) fn check_binary_render_asset_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "BINARY-ASSET-TRUTH-P9";
    for rel in [
        Path::new("tests/assets"),
        Path::new("docs/assets"),
        Path::new("examples/assets"),
    ] {
        collect_text_binary_asset_findings(root, rel, findings, RULE);
    }
}

pub(crate) fn collect_text_binary_asset_findings(
    root: &Path,
    rel: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
) {
    let full = root.join(rel);
    let Ok(metadata) = fs::metadata(&full) else {
        return;
    };
    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(&full) else {
            return;
        };
        for entry in entries.flatten() {
            collect_text_binary_asset_findings(root, &rel.join(entry.file_name()), findings, rule);
        }
        return;
    }
    if !binary_render_asset_extension(rel) {
        return;
    }
    let Ok(bytes) = fs::read(&full) else {
        findings.push(Finding::new(
            rule,
            format!("could not read binary render asset {}", rel.display()),
        ));
        return;
    };
    if looks_like_text_fixture(&bytes) {
        findings.push(Finding::new(
            rule,
            format!(
                "{} uses a binary render asset extension but contains text fixture data; rename it to a fixture format or replace it with real binary bytes",
                rel.display()
            ),
        ));
    }
}

pub(crate) fn binary_render_asset_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "hdr" | "ktx2" | "rgba16f" | "glb"
            )
        })
        .unwrap_or(false)
}

pub(crate) fn looks_like_text_fixture(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && std::str::from_utf8(bytes)
            .map(|text| {
                text.contains("placeholder")
                    || text.contains("text-fixture")
                    || text
                        .chars()
                        .all(|ch| ch == '\n' || ch == '\r' || ch == '\t' || !ch.is_control())
            })
            .unwrap_or(false)
}

pub(crate) fn check_gltf_asset_matrix_contract(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ASSET-MATRIX-M8";
    let matrix_rel = "docs/assets/gltf-asset-matrix.md";
    let manifest_rel = "tests/assets/gltf/khronos/manifest.toml";

    let Ok(matrix) = fs::read_to_string(root.join(matrix_rel)) else {
        require_contains(
            root,
            findings,
            RULE,
            "docs/assets.md",
            &["glTF/GLB", "KTX2", "meshopt", "Supported asset features"],
        );
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

    let file_hashes = khronos_manifest_file_hashes(&manifest);
    for rel in khronos_manifest_file_paths(&manifest) {
        let full_rel = format!("tests/assets/gltf/khronos/{rel}");
        if !root.join(&full_rel).is_file() {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} references missing Khronos fixture file {full_rel}"),
            ));
            continue;
        }

        let Some(expected_sha256) = file_hashes.get(&rel) else {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} must record a SHA-256 hash for {full_rel}"),
            ));
            continue;
        };

        if !is_lower_hex_sha256(expected_sha256) {
            findings.push(Finding::new(
                RULE,
                format!("{manifest_rel} records invalid SHA-256 for {full_rel}"),
            ));
            continue;
        }

        match sha256_hex(&root.join(&full_rel)) {
            Ok(actual) if actual == *expected_sha256 => {}
            Ok(actual) => findings.push(Finding::new(
                RULE,
                format!(
                    "{manifest_rel} SHA-256 mismatch for {full_rel}: got {actual}, expected {expected_sha256}"
                ),
            )),
            Err(error) => findings.push(Finding::new(
                RULE,
                format!("could not hash {full_rel}: {error}"),
            )),
        }
    }

    for rel in file_hashes.keys() {
        let full_rel = format!("tests/assets/gltf/khronos/{rel}");
        if !root.join(&full_rel).is_file() {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{manifest_rel} records a hash for missing Khronos fixture file {full_rel}"
                ),
            ));
        }
    }
}

pub(crate) fn gltf_asset_matrix_rows(text: &str) -> Vec<Vec<String>> {
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

pub(crate) fn direct_gltf_fixture_paths(root: &Path) -> Vec<String> {
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

pub(crate) fn khronos_manifest_asset_names(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| quoted_assignment(line, "name"))
        .collect()
}

pub(crate) fn khronos_manifest_file_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut in_file_hashes = false;

    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_file_hashes = line == "[file_hashes]";
            continue;
        }
        if in_file_hashes {
            continue;
        }
        if let Some(path) = quoted_assignment(line, "path").or_else(|| quoted_array_item(line)) {
            paths.push(path);
        }
    }

    paths
}
