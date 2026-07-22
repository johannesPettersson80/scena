use crate::app::prelude::*;

pub(crate) fn require_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    let Ok(text) = read_source_to_string(root, rel) else {
        if let Some(retired) = retired_internal_doc(rel) {
            eprintln!(
                "scena doctor: intentionally retired document {} (owner: {}; rationale: {})",
                retired.path, retired.owner, retired.rationale
            );
            return;
        }
        findings.push(Finding::new(rule, format!("could not read {rel}")));
        return;
    };

    for needle in needles {
        if !text.contains(needle)
            && !explicit_shader_companion_contains(root, rel, needle)
            && !split_module_companion_contains(root, rel, needle)
        {
            findings.push(Finding::new(
                rule,
                format!("{rel} is missing required contract text '{}'", needle),
            ));
        }
    }
}

/// A file module may be split into `module/*.rs` without changing its owner.
/// Positive source pins follow that owned module tree; negative checks remain
/// exact-file scans so moving forbidden behavior cannot hide it.
fn split_module_companion_contains(root: &Path, rel: &str, needle: &str) -> bool {
    let rel = Path::new(rel);
    if rel.extension().and_then(OsStr::to_str) != Some("rs")
        || rel.file_name().and_then(OsStr::to_str) == Some("mod.rs")
    {
        return false;
    }
    cached_rust_files_below(root, &rel.with_extension(""))
        .into_iter()
        .any(|path| {
            read_source_to_string(root, path)
                .map(|text| text.contains(needle))
                .unwrap_or(false)
        })
}

pub(crate) fn require_rust_test_functions(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    names: &[&str],
) {
    let Ok(text) = read_source_to_string(root, rel) else {
        findings.push(Finding::new(
            rule,
            format!("could not read {rel} for Rust test-item scan"),
        ));
        return;
    };
    let declared = rust_test_function_names(&text);
    for name in names {
        if !declared.contains(*name) {
            findings.push(Finding::new(
                rule,
                format!("{rel} is missing required active (non-ignored) #[test] function '{name}'"),
            ));
        }
    }
}

fn rust_test_function_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut attributes = Vec::new();
    let mut in_block_comment = false;
    for raw_line in text.lines() {
        let line = rust_code_before_comment(raw_line, &mut in_block_comment);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("#[") {
            attributes.push(trimmed.to_owned());
            continue;
        }
        let item = trimmed
            .strip_prefix("pub(crate) ")
            .or_else(|| trimmed.strip_prefix("pub(super) "))
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        let item = item.strip_prefix("async ").unwrap_or(item);
        if let Some(rest) = item.strip_prefix("fn ")
            && attributes.iter().any(|attribute| attribute == "#[test]")
            && !attributes
                .iter()
                .any(|attribute| attribute.starts_with("#[ignore"))
        {
            let name = rest
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
                .next()
                .unwrap_or_default();
            if !name.is_empty() {
                names.insert(name.to_owned());
            }
        }
        attributes.clear();
    }
    names
}

fn rust_code_before_comment<'a>(line: &'a str, in_block_comment: &mut bool) -> &'a str {
    let cursor = if *in_block_comment {
        let Some(end) = line.find("*/") else {
            return "";
        };
        *in_block_comment = false;
        end + 2
    } else {
        0
    };
    let rest = &line[cursor..];
    let line_comment = rest.find("//");
    let block_comment = rest.find("/*");
    match (line_comment, block_comment) {
        (Some(line_at), Some(block_at)) if block_at < line_at => {
            *in_block_comment = !rest[block_at + 2..].contains("*/");
            &line[cursor..cursor + block_at]
        }
        (Some(line_at), _) => &line[cursor..cursor + line_at],
        (None, Some(block_at)) => {
            *in_block_comment = !rest[block_at + 2..].contains("*/");
            &line[cursor..cursor + block_at]
        }
        (None, None) => rest,
    }
}

/// Some shader contracts are deliberately owned by the WGSL file included by
/// `src/render/gpu/output.rs`. Only these exact shader markers may be resolved
/// through that companion. Rust test names, item names, attributes, and any
/// newly added text must remain present in the Rust owner itself.
fn explicit_shader_companion_contains(root: &Path, rel: &str, needle: &str) -> bool {
    const OUTPUT_SHADER_MARKERS: &[&str] = &[
        "@location(5) shadow_visibility: f32",
        "var shadow_map: texture_depth_2d",
        "var shadow_sampler: sampler_comparison",
        "fn directional_shadow_factor",
        "camera_position_exposure: vec4<f32>",
        "viewport_near_far: vec4<f32>",
        "color_management: vec4<f32>",
        "clip_from_world: mat4x4<f32>",
        "world_from_model: mat4x4<f32>",
        "normal_from_model: mat4x4<f32>",
        "view_from_world: mat4x4<f32>",
        "clip_from_view: mat4x4<f32>",
        "camera.clip_from_world * world_position",
        "@group(2) @binding(0)",
        "var<uniform> draw: DrawUniform",
        "light_from_world: mat4x4<f32>",
        "var environment_cubemap: texture_cube<f32>",
        "var environment_sampler: sampler",
        "fn environment_prefilter_mip",
        "ENVIRONMENT_PREFILTER_MAX_MIP",
        "let prefiltered = textureSampleLevel(environment_cubemap, environment_sampler, reflection",
        "fn physical_transmission_color",
    ];

    if rel != "src/render/gpu/output.rs" || !OUTPUT_SHADER_MARKERS.contains(&needle) {
        return false;
    }
    let Ok(owner) = fs::read_to_string(root.join(rel)) else {
        return false;
    };
    if !owner.contains("include_str!(\"output_shader.wgsl\")") {
        return false;
    }
    fs::read_to_string(root.join("src/render/gpu/output_shader.wgsl"))
        .is_ok_and(|shader| shader.contains(needle))
}

pub(crate) fn is_retired_internal_doc(rel: &str) -> bool {
    retired_internal_doc(rel).is_some()
}

struct RetiredInternalDoc {
    path: &'static str,
    owner: &'static str,
    rationale: &'static str,
}

const RETIRED_INTERNAL_DOCS: &[RetiredInternalDoc] = &[
    RetiredInternalDoc {
        path: "docs/release-notes-template.md",
        owner: "release-hygiene",
        rationale: "superseded by versioned release notes and the release checklist",
    },
    RetiredInternalDoc {
        path: "docs/assets/gltf-asset-matrix.md",
        owner: "assets",
        rationale: "superseded by docs/assets.md and the machine-readable asset matrix",
    },
];

fn retired_internal_doc(rel: &str) -> Option<&'static RetiredInternalDoc> {
    RETIRED_INTERNAL_DOCS.iter().find(|entry| entry.path == rel)
}
