use crate::app::prelude::*;

pub(crate) fn check_viewer_facade_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-VIEWER-FACADE",
        "docs/specs/module-boundaries.md",
        &[
            "HeadlessGltfViewer",
            "InteractiveGltfViewer",
            "host-owned convenience facade exceptions",
            "Mutable accessors remain explicit escape hatches",
        ],
    );
    let text = match fs::read_to_string(root.join("src/viewer.rs")) {
        Ok(text) => text,
        Err(error) => {
            findings.push(Finding::new(
                "ARCH-VIEWER-FACADE",
                format!("could not read src/viewer.rs: {error}"),
            ));
            return;
        }
    };
    for type_name in public_struct_names(&text) {
        for field in public_fields_in_struct(&text, type_name) {
            findings.push(Finding::new(
                "ARCH-VIEWER-FACADE",
                format!("src/viewer.rs {type_name} exposes public field '{field}'; use explicit accessor methods"),
            ));
        }
    }
}

pub(crate) fn public_struct_names(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix("pub struct ")?;
            let name = rest
                .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
                .next()
                .unwrap_or_default();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

pub(crate) fn check_render_singleton_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for rel in source_files(root)
        .into_iter()
        .filter(|path| path.starts_with("src/render"))
    {
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            let singleton = trimmed.starts_with("thread_local!")
                || trimmed.starts_with("lazy_static!")
                || trimmed.starts_with("static ")
                || trimmed.starts_with("pub static ")
                || trimmed.starts_with("pub(crate) static ")
                || trimmed.starts_with("pub(super) static ")
                || (trimmed.starts_with("pub(in ") && trimmed.contains(" static "))
                || trimmed.contains("OnceCell")
                || trimmed.contains("OnceLock")
                || trimmed.contains("Lazy::new")
                || trimmed.contains("Mutex::new")
                || trimmed.contains("RwLock::new")
                || trimmed.contains("RefCell::new");
            if singleton {
                findings.push(Finding::new(
                    "ARCH-RENDER-SINGLETON",
                    format!(
                        "{}:{} declares or initializes singleton-style render state; render caches must be renderer-owned or explicitly allowlisted",
                        rel.display(),
                        line_index + 1
                    ),
                ));
            }
        }
    }
}
