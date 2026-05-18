use crate::app::prelude::*;

pub(crate) fn run_architecture_map() -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("ARCHITECTURE-MAP", message)])?;
    let artifact_dir = root.join("target/gate-artifacts/architecture");
    fs::create_dir_all(&artifact_dir).map_err(|error| {
        vec![Finding::new(
            "ARCHITECTURE-MAP",
            format!("failed to create {}: {error}", artifact_dir.display()),
        )]
    })?;

    let modules = architecture_modules_json(&root);
    let public_api = public_api_ownership_json(&root)
        .map_err(|message| vec![Finding::new("ARCH-PUBLIC-API-OWNERSHIP", message)])?;
    let dot = architecture_dependencies_dot(&root);

    let modules_path = artifact_dir.join("modules.json");
    let ownership_path = artifact_dir.join("public-api-ownership.json");
    let dot_path = artifact_dir.join("dependencies.dot");

    write_pretty_json_artifact(&modules_path, &modules)?;
    write_pretty_json_artifact(&ownership_path, &public_api)?;
    fs::write(&dot_path, dot).map_err(|error| {
        vec![Finding::new(
            "ARCHITECTURE-MAP",
            format!("failed to write {}: {error}", dot_path.display()),
        )]
    })?;

    println!("{}", modules_path.display());
    println!("{}", dot_path.display());
    println!("{}", ownership_path.display());
    Ok(())
}

pub(crate) fn write_pretty_json_artifact(
    path: &Path,
    value: &serde_json::Value,
) -> Result<(), Vec<Finding>> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|error| vec![Finding::new("ARCHITECTURE-MAP", error.to_string())])?;
    fs::write(path, format!("{body}\n")).map_err(|error| {
        vec![Finding::new(
            "ARCHITECTURE-MAP",
            format!("failed to write {}: {error}", path.display()),
        )]
    })
}

pub(crate) const ARCHITECTURE_OWNER_MODULES: &[&str] = &[
    "scene",
    "assets",
    "geometry",
    "material",
    "render",
    "animation",
    "controls",
    "picking",
    "diagnostics",
    "platform",
    "viewer",
    "browser_probe",
    "crate-root",
    "tools",
];

#[derive(Debug, Clone)]
pub(crate) struct PublicApiOwnershipEntry {
    pub(crate) type_name: String,
    pub(crate) owner: String,
    pub(crate) path: String,
    pub(crate) boundary: Option<String>,
}

pub(crate) fn architecture_modules_json(root: &Path) -> serde_json::Value {
    let modules = source_files(root)
        .into_iter()
        .map(|rel| {
            let text = fs::read_to_string(root.join(&rel)).unwrap_or_default();
            let owner = architecture_owner_for_source_path(&rel);
            let dependencies = architecture_dependency_owners(&text)
                .into_iter()
                .filter(|dependency| *dependency != owner)
                .collect::<Vec<_>>();
            json!({
                "path": path_to_forward_slash(&rel),
                "owner": owner,
                "significant_lines": significant_line_count(&text),
                "public_items": declared_public_type_names(&text),
                "dependency_owners": dependencies,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "schema": "scena.architecture.modules.v1",
        "commit": release_artifact_commit_label(root),
        "generated_at_unix_seconds": current_unix_seconds(),
        "contract": "docs/api.md",
        "modules": modules,
    })
}

pub(crate) fn public_api_ownership_json(root: &Path) -> Result<serde_json::Value, String> {
    let entries = read_public_api_ownership(root)?;
    let values = entries
        .iter()
        .map(|entry| {
            json!({
                "type": entry.type_name,
                "owner": entry.owner,
                "path": entry.path,
                "boundary": entry.boundary,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema": "scena.architecture.public_api_ownership.v1",
        "commit": release_artifact_commit_label(root),
        "generated_at_unix_seconds": current_unix_seconds(),
        "source": "src/lib.rs",
        "types": values,
    }))
}

pub(crate) fn architecture_dependencies_dot(root: &Path) -> String {
    let mut edges = BTreeSet::new();
    for rel in source_files(root) {
        let text = fs::read_to_string(root.join(&rel)).unwrap_or_default();
        let owner = architecture_owner_for_source_path(&rel);
        for dependency in architecture_dependency_owners(&text) {
            if dependency != owner {
                edges.insert((owner.to_string(), dependency.to_string()));
            }
        }
    }

    let mut dot = String::from("digraph scena_architecture {\n  rankdir=LR;\n");
    for owner in ARCHITECTURE_OWNER_MODULES {
        dot.push_str(&format!("  \"{owner}\";\n"));
    }
    for (from, to) in edges {
        dot.push_str(&format!("  \"{from}\" -> \"{to}\";\n"));
    }
    dot.push_str("}\n");
    dot
}

pub(crate) fn architecture_owner_for_source_path(rel: &Path) -> &'static str {
    let path = path_to_forward_slash(rel);
    if path == "src/lib.rs" {
        "crate-root"
    } else if path.starts_with("src/assets") {
        "assets"
    } else if path.starts_with("src/scene") {
        "scene"
    } else if path.starts_with("src/geometry") {
        "geometry"
    } else if path.starts_with("src/material") {
        "material"
    } else if path.starts_with("src/render") {
        "render"
    } else if path.starts_with("src/animation") {
        "animation"
    } else if path.starts_with("src/controls") {
        "controls"
    } else if path.starts_with("src/picking") {
        "picking"
    } else if path.starts_with("src/diagnostics") {
        "diagnostics"
    } else if path.starts_with("src/platform") {
        "platform"
    } else if path.starts_with("src/viewer")
        || path == "src/demo_page.rs"
        || path.starts_with("src/demo_page/")
    {
        "viewer"
    } else if path.starts_with("src/browser_probe") {
        "browser_probe"
    } else if path.starts_with("src/bin") {
        "tools"
    } else {
        "crate-root"
    }
}

pub(crate) fn architecture_dependency_owners(text: &str) -> BTreeSet<&'static str> {
    let mut dependencies = BTreeSet::new();
    for owner in ARCHITECTURE_OWNER_MODULES {
        if matches!(*owner, "crate-root" | "tools") {
            continue;
        }
        let direct = format!("crate::{owner}");
        if text.contains(&direct) {
            dependencies.insert(*owner);
        }
    }
    dependencies
}

pub(crate) fn declared_public_type_names(text: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    for line in text.lines() {
        if let Some(name) = public_definition_name(line) {
            names.insert(name);
        }
    }
    names.into_iter().collect()
}

pub(crate) fn read_public_api_ownership(
    root: &Path,
) -> Result<Vec<PublicApiOwnershipEntry>, String> {
    let mut entries = Vec::new();
    for type_name in public_reexported_type_names(root) {
        let definition_path = find_public_api_definition_path(root, &type_name)
            .unwrap_or_else(|| PathBuf::from("src/lib.rs"));
        let owner = architecture_owner_for_source_path(&definition_path).to_string();
        entries.push(PublicApiOwnershipEntry {
            type_name,
            owner,
            path: path_to_forward_slash(&definition_path),
            boundary: None,
        });
    }
    Ok(entries)
}
