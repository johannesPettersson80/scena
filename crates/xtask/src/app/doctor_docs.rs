use crate::app::prelude::*;

pub(crate) fn check_markdown_links(root: &Path, findings: &mut Vec<Finding>) {
    for rel in markdown_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            findings.push(Finding::new(
                "DOCS-LINKS",
                format!("could not read {}", rel.display()),
            ));
            continue;
        };

        for target in markdown_link_targets(&text) {
            if is_external_link(&target) || target.starts_with('#') {
                continue;
            }

            let without_fragment = target.split('#').next().unwrap_or_default();
            if without_fragment.is_empty() {
                continue;
            }

            let target_path = path
                .parent()
                .unwrap_or(root)
                .join(without_fragment.trim_matches(['<', '>']));
            if !target_path.exists() {
                findings.push(Finding::new(
                    "DOCS-LINKS",
                    format!("{} links to missing {}", rel.display(), target),
                ));
            }
        }
    }
}

pub(crate) fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from("README.md"), PathBuf::from("AGENTS.md")];
    collect_markdown(&root.join("docs"), Path::new("docs"), &mut files);
    collect_markdown(
        &root.join(".codex/skills"),
        Path::new(".codex/skills"),
        &mut files,
    );
    files
}

pub(crate) fn collect_markdown(dir: &Path, rel_dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let rel = rel_dir.join(entry.file_name());
        if path.is_dir() {
            collect_markdown(&path, &rel, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(rel);
        }
    }
}

pub(crate) fn markdown_link_targets(text: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let bytes = text.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if bytes[index] == b']' && bytes[index + 1] == b'(' {
            let start = index + 2;
            if let Some(end_offset) = text[start..].find(')') {
                let target = text[start..start + end_offset].trim();
                if !target.is_empty() {
                    targets.push(target.to_string());
                }
                index = start + end_offset + 1;
                continue;
            }
        }
        index += 1;
    }
    targets
}

pub(crate) fn is_external_link(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("app://")
}

pub(crate) fn check_for_stale_doc_terms(root: &Path, findings: &mut Vec<Finding>) {
    for rel in markdown_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for term in STALE_DOC_TERMS {
            if text.contains(term) {
                findings.push(Finding::new(
                    "DOCS-STALE-TERM",
                    format!("{} contains stale term '{}'", rel.display(), term),
                ));
            }
        }
    }
}

pub(crate) fn check_required_doc_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DOCS-PUBLIC-API",
        "docs/api.md",
        &[
            "Scene",
            "Assets",
            "Renderer",
            "SceneImport",
            "Typed handles",
            "Errors and diagnostics",
            "Stats and capabilities",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-LIFECYCLE",
        "docs/lifecycle.md",
        &["prepare", "render", "When to prepare again"],
    );
    require_contains(
        root,
        findings,
        "DOCS-GLTF",
        "docs/assets.md",
        &[
            "glTF/GLB",
            "External buffers and textures",
            "Units, axes, and handedness",
            "Anchors and connectors",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-VISUAL",
        "docs/headless-rendering.md",
        &["Headless rendering", "CI snapshots", "Renderer::headless"],
    );
    require_contains(
        root,
        findings,
        "DOCS-PLATFORM",
        "docs/platforms.md",
        &["WebGPU", "WebGL2", "wasm32-unknown-unknown"],
    );
    require_contains(
        root,
        findings,
        "DOCS-ERRORS",
        "docs/errors.md",
        &["AssetError", "RenderError", "PrepareError"],
    );
    require_contains(
        root,
        findings,
        "DOCS-README",
        "docs/README.md",
        &["Getting started", "Examples", "Troubleshooting"],
    );
}

pub(crate) fn check_stable_contract_release_evidence(root: &Path, findings: &mut Vec<Finding>) {
    const FIXTURES: &[(&str, &str)] = &[
        (
            "tests/assets/stable-contracts/capability_report.v1.json",
            "scena.capability_report.v1",
        ),
        (
            "tests/assets/stable-contracts/schema_catalog.v1.json",
            "scena.schema_catalog.v1",
        ),
        (
            "tests/assets/stable-contracts/schema_entry.v1.json",
            "scena.schema_entry.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_inspection.v1.json",
            "scena.scene_inspection.v1",
        ),
        (
            "tests/assets/stable-contracts/capture.v1.json",
            "scena.capture.v1",
        ),
        (
            "tests/assets/stable-contracts/capture_baseline.v1.json",
            "scena.capture_baseline.v1",
        ),
        (
            "tests/assets/stable-contracts/render_introspection.v1.json",
            "scena.render_introspection.v1",
        ),
        (
            "tests/assets/stable-contracts/visibility_diagnosis.v1.json",
            "scena.visibility_diagnosis.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_recipe.v1.json",
            "scena.scene_recipe.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_recipe_validation.v1.json",
            "scena.scene_recipe_validation.v1",
        ),
        (
            "tests/assets/stable-contracts/placement_result.v1.json",
            "scena.placement_result.v1",
        ),
        (
            "tests/assets/stable-contracts/annotation_projection.v1.json",
            "scena.annotation_projection.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_geometry_summary.v1.json",
            "scena.asset_geometry_summary.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_load_report.v1.json",
            "scena.asset_load_report.v1",
        ),
        (
            "tests/assets/stable-contracts/asset_doctor.v1.json",
            "scena.asset_doctor.v1",
        ),
        (
            "tests/assets/stable-contracts/connector_browser.v1.json",
            "scena.connector_browser.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_asset_import.v1.json",
            "scena.scene_host_asset_import.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_subtree.v1.json",
            "scena.subtree.v1",
        ),
        (
            "tests/assets/stable-contracts/scene_host_animation_inventory.v1.json",
            "scena.animation_inventory.v1",
        ),
        (
            "tests/assets/stable-contracts/visual_patch.v1.json",
            "scena.visual_patch.v1",
        ),
        (
            "tests/assets/stable-contracts/visual_patch_result.v1.json",
            "scena.visual_patch.v1",
        ),
        (
            "tests/assets/stable-contracts/host_event.v1.json",
            "scena.host_event.v1",
        ),
    ];
    const REQUIRED_FILES: &[&str] = &[
        "examples/scene_host_contracts.rs",
        "examples/scene_host_browser_contracts.js",
        "tests/assets/stable-contracts/asset_provenance.json",
        "docs/checklists/scene-host-browser-gpu-proof.md",
    ];

    require_files(root, findings, "STABLE-CONTRACT-EVIDENCE", REQUIRED_FILES);
    for (rel, expected_schema) in FIXTURES {
        require_files(root, findings, "STABLE-CONTRACT-EVIDENCE", &[*rel]);
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(&text) else {
            findings.push(Finding::new(
                "STABLE-CONTRACT-EVIDENCE",
                format!("{rel} must be valid JSON"),
            ));
            continue;
        };
        if json.get("schema").and_then(Value::as_str) != Some(*expected_schema) {
            findings.push(Finding::new(
                "STABLE-CONTRACT-EVIDENCE",
                format!("{rel} must carry schema {expected_schema}"),
            ));
        }
    }
    check_schema_catalog_covers_stable_fixtures(root, findings, FIXTURES);

    require_contains(
        root,
        findings,
        "STABLE-CONTRACT-EVIDENCE",
        "tests/assets/stable-contracts/asset_provenance.json",
        &[
            "source_path",
            "source_sha256",
            "license",
            "generator",
            "derivatives",
        ],
    );
    require_contains(
        root,
        findings,
        "STABLE-CONTRACT-EVIDENCE",
        "Cargo.toml",
        &[
            "name = \"scene_host_contracts\"",
            "path = \"examples/scene_host_contracts.rs\"",
            "required-features = [\"scene-host\"]",
            "name = \"scene_host_release_1_7\"",
            "path = \"examples/scene_host_release_1_7.rs\"",
        ],
    );
    require_contains(
        root,
        findings,
        "STABLE-CONTRACT-EVIDENCE",
        "docs/schema-contracts.md",
        &[
            "tests/assets/stable-contracts",
            "scena.scene_host_asset_import.v1",
            "scena.animation_inventory.v1",
            "scena.visual_patch.v1",
            "scena.host_event.v1",
            "transforms_eased",
            "tints_eased",
            "camera_eased",
            "animation_time",
            "selection",
            "hover",
            "material_variants",
            "labels",
            "echo_metadata",
            "visual_patch_result.v1.json",
            "host_event.v1.json",
            "drainEventsJson",
            "AssetProvenance",
        ],
    );
    require_contains(
        root,
        findings,
        "STABLE-CONTRACT-EVIDENCE",
        "docs/examples.md",
        &[
            "scene_host_contracts.rs",
            "scene_host_release_1_7.rs",
            "scene_host_browser_contracts.js",
            "scena.visual_patch.v1",
            "scena.host_event.v1",
            "drainEventsJson",
            "tests/assets/stable-contracts",
        ],
    );
    require_contains(
        root,
        findings,
        "STABLE-CONTRACT-EVIDENCE",
        "docs/browser.md",
        &[
            "examples/scene_host_browser_contracts.js",
            "Real browser/GPU proof is separate from CPU builder validation",
            "scene-host-browser-gpu-proof.md",
        ],
    );
    require_contains(
        root,
        findings,
        "STABLE-CONTRACT-EVIDENCE",
        "docs/checklists/scene-host-browser-gpu-proof.md",
        &[
            "Status: passed for SceneHost contracts",
            "Real browser/GPU machine required",
            "scena.scene_host_browser_proof.v1",
            "SceneHost.capture()",
            "annotationProjectionsJson()",
            "inspectJson()",
            "renderer-fidelity-dependencies.md",
            "target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json",
        ],
    );
}

fn check_schema_catalog_covers_stable_fixtures(
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
}

pub(crate) fn check_demo_build_heartbeat_contract(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DEMO-BUILD-HEARTBEAT",
        "package.json",
        &["\"demo:build\": \"node scripts/build_demo_wasm.js\""],
    );
    require_contains(
        root,
        findings,
        "DEMO-BUILD-HEARTBEAT",
        "scripts/build_demo_wasm.js",
        &[
            "wasm-pack",
            "SCENA_BUILD_HEARTBEAT_MS",
            "still running",
            "process.exit(code ?? 1)",
        ],
    );
}

pub(crate) fn require_contains(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
    needles: &[&str],
) {
    let path = root.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        if is_retired_internal_doc(rel) {
            return;
        }
        findings.push(Finding::new(rule, format!("could not read {rel}")));
        return;
    };

    // Phase 5.4 follow-up: extracted shader text lives in a sibling
    // `<module>_shader.wgsl` file. When checking a `.rs` module, fall
    // back to that sibling so doctor pins that name shader-text
    // strings (e.g. `var brdf_lut: texture_2d<f32>`) continue to
    // resolve after the extraction.
    let sibling = if rel.ends_with(".rs") {
        let stripped = rel.trim_end_matches(".rs");
        let sibling_rel = format!("{stripped}_shader.wgsl");
        fs::read_to_string(root.join(&sibling_rel)).ok()
    } else {
        None
    };

    for needle in needles {
        let found_in_primary = text.contains(needle);
        let found_in_sibling = sibling
            .as_deref()
            .is_some_and(|sibling_text| sibling_text.contains(needle));
        if !found_in_primary && !found_in_sibling {
            findings.push(Finding::new(
                rule,
                format!("{rel} is missing required contract text '{}'", needle),
            ));
        }
    }
}

pub(crate) fn is_retired_internal_doc(rel: &str) -> bool {
    rel == "docs/RFC-rust-3d-renderer.md"
        || rel == "docs/release-notes-template.md"
        || rel.starts_with("docs/specs/")
        || rel.starts_with("docs/checklists/")
        || rel.starts_with("docs/decisions/")
        || rel.starts_with("docs/api/")
        || rel.starts_with("docs/benchmarks/")
        || rel == "docs/assets/gltf-asset-matrix.md"
}

pub(crate) fn check_source_scope(root: &Path, findings: &mut Vec<Finding>) {
    for rel in source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        for term in SOURCE_SCOPE_TERMS {
            if contains_scope_term(&lower, term) {
                findings.push(Finding::new(
                    "ARCH-SCOPE",
                    format!(
                        "{} contains renderer-forbidden term '{}'",
                        rel.display(),
                        term
                    ),
                ));
            }
        }
    }

    check_public_contract_vocabulary(root, findings);
}

const PUBLIC_CONTRACT_VOCABULARY_FILES: &[&str] = &[
    "docs/README.md",
    "docs/schema-contracts.md",
    "docs/feature-flags.md",
    "src/scene_host.rs",
    "src/capture.rs",
];

const PUBLIC_CONTRACT_VOCABULARY_DIRS: &[&str] =
    &["src/scene_host", "src/capture", "examples/scene_host"];

const PUBLIC_CONTRACT_EXAMPLE_PREFIXES: &[&str] = &["scene_host", "wasm_scene_host"];

const PUBLIC_CONTRACT_FORBIDDEN_TERMS: &[&str] = &[
    "robot",
    "joint",
    "urdf",
    "plc",
    "gripper",
    "workpiece",
    "weld",
    "motion",
    "trajectory",
    "twin",
    "simulation",
    "controller",
];

fn check_public_contract_vocabulary(root: &Path, findings: &mut Vec<Finding>) {
    for rel in public_contract_vocabulary_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let lower = text.to_ascii_lowercase();
        for term in PUBLIC_CONTRACT_FORBIDDEN_TERMS {
            if contains_scope_term(&lower, term) {
                findings.push(Finding::new(
                    "ARCH-PUBLIC-CONTRACT-VOCAB",
                    format!(
                        "{} contains public-contract forbidden term '{term}'",
                        rel.display()
                    ),
                ));
            }
        }
    }
}

fn public_contract_vocabulary_files(root: &Path) -> Vec<PathBuf> {
    let mut files = BTreeSet::new();

    for rel in PUBLIC_CONTRACT_VOCABULARY_FILES {
        files.insert(PathBuf::from(rel));
    }

    for rel_dir in PUBLIC_CONTRACT_VOCABULARY_DIRS {
        collect_public_contract_text_files(root, Path::new(rel_dir), &mut files);
    }

    collect_public_contract_example_files(root, &mut files);

    files.into_iter().collect()
}

fn collect_public_contract_text_files(root: &Path, rel_dir: &Path, files: &mut BTreeSet<PathBuf>) {
    let dir = root.join(rel_dir);
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let rel = rel_dir.join(entry.file_name());
        let path = entry.path();
        if path.is_dir() {
            collect_public_contract_text_files(root, &rel, files);
        } else if is_public_contract_text_file(&rel) {
            files.insert(rel);
        }
    }
}

fn collect_public_contract_example_files(root: &Path, files: &mut BTreeSet<PathBuf>) {
    let examples_dir = root.join("examples");
    let Ok(entries) = fs::read_dir(examples_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let rel = Path::new("examples").join(entry.file_name());
        let path = entry.path();
        if path.is_dir() {
            if is_public_contract_example_path(&rel) {
                collect_public_contract_text_files(root, &rel, files);
            }
        } else if is_public_contract_example_path(&rel) && is_public_contract_text_file(&rel) {
            files.insert(rel);
        }
    }
}

fn is_public_contract_example_path(rel: &Path) -> bool {
    rel.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| {
            PUBLIC_CONTRACT_EXAMPLE_PREFIXES.iter().any(|prefix| {
                stem == *prefix
                    || stem
                        .strip_prefix(prefix)
                        .is_some_and(|suffix| suffix.starts_with('_'))
            })
        })
}

fn is_public_contract_text_file(rel: &Path) -> bool {
    rel.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "rs" | "md" | "html" | "js" | "ts" | "toml" | "json"))
}
