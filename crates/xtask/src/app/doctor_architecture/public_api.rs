use crate::app::prelude::*;

pub(crate) fn check_public_api_ownership(root: &Path, findings: &mut Vec<Finding>) {
    let entries = match read_public_api_ownership(root) {
        Ok(entries) => entries,
        Err(message) => {
            findings.push(Finding::new("ARCH-PUBLIC-API-OWNERSHIP", message));
            return;
        }
    };
    let mut by_type = BTreeMap::new();
    for entry in &entries {
        if !ARCHITECTURE_OWNER_MODULES.contains(&entry.owner.as_str()) {
            findings.push(Finding::new(
                "ARCH-PUBLIC-API-OWNERSHIP",
                format!(
                    "{} has unknown owner '{}'; expected one of {:?}",
                    entry.type_name, entry.owner, ARCHITECTURE_OWNER_MODULES
                ),
            ));
        }
        if by_type
            .insert(entry.type_name.clone(), entry.owner.clone())
            .is_some()
        {
            findings.push(Finding::new(
                "ARCH-PUBLIC-API-OWNERSHIP",
                format!("{} has duplicate ownership entries", entry.type_name),
            ));
        }
        let source_path = root.join(&entry.path);
        let Ok(source_text) = fs::read_to_string(&source_path) else {
            findings.push(Finding::new(
                "ARCH-PUBLIC-API-OWNERSHIP",
                format!(
                    "{} ownership points at unreadable path {}",
                    entry.type_name, entry.path
                ),
            ));
            continue;
        };
        if !public_api_definition_exists(&source_text, &entry.type_name) {
            let actual_path = find_public_api_definition_path(root, &entry.type_name);
            findings.push(Finding::new(
                "ARCH-PUBLIC-API-OWNERSHIP",
                if let Some(actual_path) = actual_path {
                    format!(
                        "{} ownership path {} does not contain its public definition; actual definition appears in {}",
                        entry.type_name,
                        entry.path,
                        actual_path.display()
                    )
                } else {
                    format!(
                        "{} ownership path {} does not contain a public struct/enum/trait/type/const definition",
                        entry.type_name, entry.path
                    )
                },
            ));
        }
        let inferred_owner = architecture_owner_for_source_path(Path::new(&entry.path));
        if inferred_owner != entry.owner && entry.boundary.is_none() {
            findings.push(Finding::new(
                "ARCH-PUBLIC-API-OWNERSHIP",
                format!(
                    "{} is listed as owner '{}' but {} is inferred as owner '{}'; add a boundary note or move ownership",
                    entry.type_name, entry.owner, entry.path, inferred_owner
                ),
            ));
        }
    }

    for required in [
        "Scene",
        "Assets",
        "Renderer",
        "SceneImport",
        "MaterialDesc",
        "GeometryDesc",
        "Capabilities",
        "SurfaceEvent",
        "FirstRender",
        "HeadlessGltfViewer",
        "HeadlessGltfViewerBuilder",
        "InteractiveGltfViewer",
        "InteractiveGltfViewerBuilder",
    ] {
        if !by_type.contains_key(required) {
            findings.push(Finding::new(
                "ARCH-PUBLIC-API-OWNERSHIP",
                format!("core public API type {required} is missing from docs/api/public-api-ownership.toml"),
            ));
        }
    }
    for public_type in public_reexported_type_names(root) {
        if !by_type.contains_key(&public_type) {
            findings.push(Finding::new(
                "ARCH-PUBLIC-API-OWNERSHIP",
                format!(
                    "crate-root public type re-export {public_type} is missing from docs/api/public-api-ownership.toml"
                ),
            ));
        }
    }
}

pub(crate) fn public_reexported_type_names(root: &Path) -> Vec<String> {
    let text = fs::read_to_string(root.join("src/lib.rs")).unwrap_or_default();
    let mut names = BTreeSet::new();
    let mut current_pub_use = String::new();
    let mut collecting = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub use ") && trimmed.contains("::{") {
            current_pub_use.clear();
            current_pub_use.push_str(trimmed);
            collecting = !trimmed.ends_with("};");
        } else if collecting {
            current_pub_use.push(' ');
            current_pub_use.push_str(trimmed);
            collecting = !trimmed.ends_with("};");
        } else if let Some(name) = trimmed
            .strip_prefix("pub use ")
            .and_then(|rest| rest.split("::").nth(1))
            .and_then(|rest| rest.strip_suffix(';'))
            .map(str::trim)
            .filter(|name| name.starts_with(|c: char| c.is_ascii_uppercase()))
        {
            names.insert(name.to_string());
        } else {
            continue;
        }

        if !collecting
            && let Some(body) = current_pub_use
                .split_once("::{")
                .and_then(|(_, rest)| rest.rsplit_once("};").map(|(body, _)| body))
        {
            for name in body.split(',').map(str::trim) {
                let name = name.split(" as ").next().unwrap_or_default().trim();
                if name
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_uppercase())
                {
                    names.insert(name.to_string());
                }
            }
        }
    }
    names.into_iter().collect()
}

pub(crate) fn public_api_definition_exists(text: &str, type_name: &str) -> bool {
    text.lines().any(|line| {
        public_definition_name(line).as_deref() == Some(type_name)
            || public_use_exports_name(line, type_name)
    })
}

pub(crate) fn find_public_api_definition_path(root: &Path, type_name: &str) -> Option<PathBuf> {
    source_files(root).into_iter().find(|rel| {
        fs::read_to_string(root.join(rel))
            .is_ok_and(|text| public_api_definition_exists(&text, type_name))
    })
}

pub(crate) fn public_definition_name(line: &str) -> Option<String> {
    let line = line.trim_start();
    let line = line.strip_prefix("pub ")?;
    let line = line
        .strip_prefix("struct ")
        .or_else(|| line.strip_prefix("enum "))
        .or_else(|| line.strip_prefix("trait "))
        .or_else(|| line.strip_prefix("type "))
        .or_else(|| line.strip_prefix("const "))?;
    let name = line
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .next()
        .unwrap_or_default();
    (!name.is_empty()).then(|| name.to_string())
}

pub(crate) fn public_use_exports_name(line: &str, type_name: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix("pub use ") else {
        return false;
    };
    if let Some((_, grouped)) = rest.split_once("::{") {
        let Some((names, _)) = grouped.split_once('}') else {
            return false;
        };
        return names
            .split(',')
            .map(str::trim)
            .any(|name| name == type_name);
    }
    rest.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '_' || character == ':')
    })
    .filter_map(|segment| segment.rsplit("::").next())
    .any(|name| name == type_name)
}

pub(crate) fn forbid_contains_required_path(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &Path,
    needles: &[&str],
) {
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        findings.push(Finding::new(
            rule,
            format!(
                "{} is required for this architecture check; missing files must fail closed",
                rel.display()
            ),
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
