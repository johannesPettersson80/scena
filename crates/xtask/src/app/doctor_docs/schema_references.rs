use crate::app::prelude::*;

pub(crate) fn check_schema_doc_references_listed_in_catalog(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    let catalog_rel = "tests/assets/stable-contracts/schema_catalog.v1.json";
    let Ok(catalog_text) = fs::read_to_string(root.join(catalog_rel)) else {
        findings.push(Finding::new(
            "STABLE-CONTRACT-EVIDENCE",
            format!("{catalog_rel} must be readable for schema reference checks"),
        ));
        return;
    };
    let Ok(catalog_json) = serde_json::from_str::<Value>(&catalog_text) else {
        findings.push(Finding::new(
            "STABLE-CONTRACT-EVIDENCE",
            format!("{catalog_rel} must be valid JSON for schema reference checks"),
        ));
        return;
    };
    let catalog_schemas = catalog_json
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("schema").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();

    for rel in markdown_files(root) {
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        for schema in schema_references_in_text(&text) {
            if !schema_reference_requires_catalog(&schema) {
                continue;
            }
            if !catalog_schemas.contains(schema.as_str()) {
                findings.push(Finding::new(
                    "STABLE-CONTRACT-EVIDENCE",
                    format!(
                        "{} references schema {schema}, but {catalog_rel} does not list it",
                        rel.display()
                    ),
                ));
            }
        }
    }
}

pub(crate) fn check_public_cli_schemas_listed_in_catalog(root: &Path, findings: &mut Vec<Finding>) {
    let catalog_rel = "tests/assets/stable-contracts/schema_catalog.v1.json";
    let Ok(catalog_text) = fs::read_to_string(root.join(catalog_rel)) else {
        findings.push(Finding::new(
            "PUBLIC-SCHEMA-DISCOVERY",
            format!("{catalog_rel} must be readable for public contract discovery"),
        ));
        return;
    };
    let Ok(catalog_json) = serde_json::from_str::<Value>(&catalog_text) else {
        findings.push(Finding::new(
            "PUBLIC-SCHEMA-DISCOVERY",
            format!("{catalog_rel} must be valid JSON for public contract discovery"),
        ));
        return;
    };
    let catalog_schemas = catalog_json
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("schema").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();

    let mut sources = vec![PathBuf::from("src/bin/scena.rs")];
    collect_rust_sources(
        &root.join("src/bin/scena"),
        Path::new("src/bin/scena"),
        &mut sources,
    );
    for rel in sources {
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            findings.push(Finding::new(
                "PUBLIC-SCHEMA-DISCOVERY",
                format!("could not read public CLI source {}", rel.display()),
            ));
            continue;
        };
        for schema in schema_references_in_text(&text) {
            if !catalog_schemas.contains(schema.as_str()) {
                findings.push(Finding::new(
                    "PUBLIC-SCHEMA-DISCOVERY",
                    format!(
                        "{} exposes public schema {schema}, but {catalog_rel} does not list it",
                        rel.display()
                    ),
                ));
            }
        }
    }
}

fn collect_rust_sources(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if path.is_dir() {
            collect_rust_sources(&path, &rel, files);
        } else if path.extension().and_then(OsStr::to_str) == Some("rs") {
            files.push(rel);
        }
    }
}

pub(crate) fn check_schema_catalog_covers_stable_fixtures(
    root: &Path,
    findings: &mut Vec<Finding>,
    fixtures: &[(&str, &str)],
) {
    let rel = "tests/assets/stable-contracts/schema_catalog.v1.json";
    let Ok(text) = fs::read_to_string(root.join(rel)) else {
        return;
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    let Some(entries) = json.get("entries").and_then(Value::as_array) else {
        findings.push(Finding::new(
            "STABLE-CONTRACT-EVIDENCE",
            format!("{rel} must contain an entries array"),
        ));
        return;
    };
    let schemas = entries
        .iter()
        .filter_map(|entry| entry.get("schema").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();

    for (_, expected_schema) in fixtures {
        if !schemas.contains(expected_schema) {
            findings.push(Finding::new(
                "STABLE-CONTRACT-EVIDENCE",
                format!("{rel} must list stable fixture schema {expected_schema}"),
            ));
        }
    }

    for entry in entries {
        let schema = entry.get("schema").and_then(Value::as_str);
        let fixture_path = entry.get("fixture_path").and_then(Value::as_str);
        let (Some(schema), Some(fixture_path)) = (schema, fixture_path) else {
            continue;
        };
        let pinned = fixtures
            .iter()
            .find(|(rel, _)| *rel == fixture_path)
            .map(|(_, expected_schema)| *expected_schema);
        match pinned {
            Some(expected_schema) if expected_schema == schema => {}
            Some(expected_schema) => findings.push(Finding::new(
                "STABLE-CONTRACT-EVIDENCE",
                format!(
                    "{rel} lists fixture {fixture_path} as {schema}, but doctor FIXTURES pins {expected_schema}"
                ),
            )),
            None => findings.push(Finding::new(
                "STABLE-CONTRACT-EVIDENCE",
                format!(
                    "{rel} lists fixture {fixture_path} for schema {schema}, but doctor FIXTURES does not pin it"
                ),
            )),
        }
    }
}

fn schema_references_in_text(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'))
        .filter_map(|candidate| {
            let candidate = candidate.trim_matches('.');
            (candidate.starts_with("scena.")
                && !candidate.contains('*')
                && versioned_schema_suffix(candidate))
            .then(|| candidate.to_owned())
        })
        .collect()
}

fn schema_reference_requires_catalog(schema: &str) -> bool {
    !schema.contains("_proof.")
        && !schema.contains(".m6.")
        && schema != "scena.scena_viewer_inspector_snapshot.v1"
}

fn versioned_schema_suffix(candidate: &str) -> bool {
    let Some((_, suffix)) = candidate.rsplit_once(".v") else {
        return false;
    };
    !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
}
