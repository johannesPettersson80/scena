use crate::app::prelude::*;

pub(crate) fn check_xtask_module_split(root: &Path, findings: &mut Vec<Finding>) {
    const REQUIRED_FILES: &[&str] = &["crates/xtask/src/main.rs", "crates/xtask/src/app.rs"];
    const REQUIRED_APP_MODULES: &[&str] = &[
        "core",
        "visual_proof",
        "architecture_map",
        "release",
        "visual_artifacts",
        "doctor_core",
        "doctor_docs",
        "doctor_architecture",
        "doctor_render",
        "doctor_scene_platform",
        "doctor_visual_release",
        "doctor_m7_m8_assets",
    ];

    for rel in REQUIRED_FILES {
        if !root.join(rel).is_file() {
            findings.push(Finding::new(
                "ARCH-XTASK-SPLIT",
                format!("missing required xtask split file `{rel}`"),
            ));
        }
    }

    let main_text = match fs::read_to_string(root.join("crates/xtask/src/main.rs")) {
        Ok(text) => text,
        Err(error) => {
            findings.push(Finding::new(
                "ARCH-XTASK-SPLIT",
                format!("could not read crates/xtask/src/main.rs: {error}"),
            ));
            return;
        }
    };
    let significant_lines = significant_line_count(&main_text);
    if significant_lines > 20 {
        findings.push(Finding::new(
            "ARCH-XTASK-SPLIT",
            format!(
                "crates/xtask/src/main.rs must remain a tiny entrypoint; found {significant_lines} significant lines"
            ),
        ));
    }
    for needle in ["mod app;", "app::run();"] {
        if !main_text.contains(needle) {
            findings.push(Finding::new(
                "ARCH-XTASK-SPLIT",
                format!("crates/xtask/src/main.rs missing `{needle}`"),
            ));
        }
    }

    let app_text = match fs::read_to_string(root.join("crates/xtask/src/app.rs")) {
        Ok(text) => text,
        Err(error) => {
            findings.push(Finding::new(
                "ARCH-XTASK-SPLIT",
                format!("could not read crates/xtask/src/app.rs: {error}"),
            ));
            return;
        }
    };
    if contains_xtask_include_macro(&app_text) {
        findings.push(Finding::new(
            "ARCH-XTASK-SPLIT",
            "crates/xtask/src/app.rs must use real `mod` declarations, not include! glue",
        ));
    }
    for module in REQUIRED_APP_MODULES {
        let declaration = format!("mod {module};");
        if !app_text.contains(&declaration) {
            findings.push(Finding::new(
                "ARCH-XTASK-SPLIT",
                format!("crates/xtask/src/app.rs missing `{declaration}`"),
            ));
        }
    }
    if !app_text.contains("#[cfg(test)]\nmod tests_") {
        findings.push(Finding::new(
            "ARCH-XTASK-SPLIT",
            "crates/xtask/src/app.rs must keep split test modules behind #[cfg(test)]",
        ));
    }

    for rel in xtask_source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let import_policy_exempt = rel
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("tests_"));
        let file_name = rel.file_name().and_then(|name| name.to_str()).unwrap_or("");
        if file_name.starts_with("part_") && file_name.ends_with(".rs") {
            findings.push(Finding::new(
                "ARCH-XTASK-NAMING",
                format!(
                    "{} uses numeric part_NN naming; xtask modules must be named by responsibility",
                    rel.display()
                ),
            ));
        }
        for (line_index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("mod part_") || trimmed.starts_with("pub(crate) use part_") {
                findings.push(Finding::new(
                    "ARCH-XTASK-NAMING",
                    format!(
                        "{}:{} uses numeric part_NN module wiring; name the module by responsibility",
                        rel.display(),
                        line_index + 1
                    ),
                ));
            }
            if !import_policy_exempt && xtask_cross_module_glob_import(trimmed) {
                findings.push(Finding::new(
                    "ARCH-XTASK-NO-CROSS-GLOB",
                    format!(
                        "{}:{} imports another xtask module with a glob; import explicit items instead",
                        rel.display(),
                        line_index + 1
                    ),
                ));
            }
        }
        if contains_xtask_include_macro(&text) {
            findings.push(Finding::new(
                "ARCH-XTASK-SPLIT",
                format!(
                    "{} contains include!; xtask must use real modules",
                    rel.display()
                ),
            ));
        }
        let significant_lines = significant_line_count(&text);
        if significant_lines > MAX_SIGNIFICANT_LINES_PER_XTASK_MODULE {
            findings.push(Finding::new(
                "ARCH-XTASK-SPLIT",
                format!(
                    "{} has {significant_lines} significant lines; xtask trust-base modules must stay below {MAX_SIGNIFICANT_LINES_PER_XTASK_MODULE}",
                    rel.display()
                ),
            ));
        }
    }
}

pub(crate) fn xtask_cross_module_glob_import(trimmed: &str) -> bool {
    if trimmed == "use crate::app::prelude::*;" {
        return false;
    }
    (trimmed.starts_with("use super::") && trimmed.contains("::*"))
        || (trimmed.starts_with("pub(crate) use super::") && trimmed.contains("::*"))
        || (trimmed.starts_with("use crate::app::") && trimmed.contains("::*"))
        || (trimmed.starts_with("pub(crate) use crate::app::") && trimmed.contains("::*"))
}

pub(crate) fn contains_xtask_include_macro(text: &str) -> bool {
    text.lines()
        .any(|line| line.trim_start().starts_with("include!("))
}

pub(crate) fn xtask_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_xtask_source_files(
        &root.join("crates/xtask/src"),
        Path::new("crates/xtask/src"),
        &mut files,
    );
    files.sort();
    files
}

pub(crate) fn collect_xtask_source_files(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if path.is_dir() {
            collect_xtask_source_files(&path, &rel, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(rel);
        }
    }
}
