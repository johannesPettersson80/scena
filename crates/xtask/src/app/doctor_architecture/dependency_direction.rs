use crate::app::prelude::*;

pub(crate) fn check_architecture_dependency_direction(root: &Path, findings: &mut Vec<Finding>) {
    for rel in source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let owner = architecture_owner_for_source_path(&rel);
        let dependencies = architecture_dependency_owners(&text);
        let rel_display = rel.display();
        if owner == "crate-root" && rel != Path::new("src/lib.rs") {
            findings.push(Finding::new(
                "ARCH-DEPENDENCY-DIRECTION",
                format!(
                    "{rel_display} is not mapped to an architecture owner; add it to the owner map or move it under an existing owner"
                ),
            ));
        }
        match owner {
            "assets" => {
                for forbidden in ["render", "platform"] {
                    if dependencies.contains(forbidden) {
                        findings.push(Finding::new(
                            "ARCH-DEPENDENCY-DIRECTION",
                            format!(
                                "{rel_display} is owned by assets and must not depend on {forbidden}"
                            ),
                        ));
                    }
                }
                if text.contains("wgpu::") {
                    findings.push(Finding::new(
                        "ARCH-DEPENDENCY-DIRECTION",
                        format!("{rel_display} is owned by assets and must not depend on wgpu"),
                    ));
                }
            }
            "scene" => {
                for forbidden in ["render", "platform"] {
                    if dependencies.contains(forbidden) {
                        findings.push(Finding::new(
                            "ARCH-DEPENDENCY-DIRECTION",
                            format!(
                                "{rel_display} is owned by scene and must not depend on {forbidden}"
                            ),
                        ));
                    }
                }
                if text.contains("wgpu::") {
                    findings.push(Finding::new(
                        "ARCH-DEPENDENCY-DIRECTION",
                        format!("{rel_display} is owned by scene and must not depend on wgpu"),
                    ));
                }
            }
            "platform" => {
                if dependencies.contains("render") {
                    findings.push(Finding::new(
                        "ARCH-DEPENDENCY-DIRECTION",
                        format!(
                            "{rel_display} is owned by platform and must remain a thin adapter without render dependencies"
                        ),
                    ));
                }
                for forbidden in ["wgpu::", "ForwardPass", "ShadowPass", "PostProcessPass"] {
                    if text.contains(forbidden) {
                        findings.push(Finding::new(
                            "ARCH-DEPENDENCY-DIRECTION",
                            format!(
                                "{rel_display} is owned by platform and must remain a thin adapter without {forbidden}"
                            ),
                        ));
                    }
                }
            }
            "render" => {
                for pattern in render_asset_loading_patterns(&text) {
                    findings.push(Finding::new(
                        "ARCH-DEPENDENCY-DIRECTION",
                        format!(
                            "{rel_display} is owned by render and must not fetch, parse, or load assets via {pattern}"
                        ),
                    ));
                }
            }
            "material" => {
                for forbidden in [
                    "crate::assets::Assets",
                    "AssetFetcher",
                    ".fetch(",
                    "load_scene",
                    "load_texture",
                ] {
                    if text.contains(forbidden) {
                        findings.push(Finding::new(
                            "ARCH-DEPENDENCY-DIRECTION",
                            format!(
                                "{rel_display} is owned by material and may depend on typed asset handles only, not asset stores/fetching via {forbidden}"
                            ),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn check_render_asset_loading_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for rel in source_files(root)
        .into_iter()
        .filter(|path| path.starts_with("src/render"))
    {
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        for pattern in render_asset_loading_patterns(&text) {
            findings.push(Finding::new(
                "ARCH-RENDER",
                format!(
                    "{} is render-owned and must not fetch, parse, or load assets via {pattern}",
                    rel.display()
                ),
            ));
        }
    }
}

pub(crate) fn render_asset_loading_patterns(text: &str) -> Vec<&'static str> {
    let mut patterns = BTreeSet::new();
    for raw_line in text.lines() {
        let line = raw_line.split("//").next().unwrap_or_default();
        for (pattern, label) in [
            ("AssetFetcher", "AssetFetcher"),
            (".fetch(", ".fetch("),
            ("load_scene", "load_scene"),
            ("load_texture", "load_texture"),
            ("::gltf::", "::gltf::"),
            ("gltf::Gltf", "gltf::Gltf"),
        ] {
            if line.contains(pattern) {
                patterns.insert(label);
            }
        }
    }
    patterns.into_iter().collect()
}
