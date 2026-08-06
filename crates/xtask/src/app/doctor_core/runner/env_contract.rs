use crate::app::prelude::*;

use super::{collect_test_contract_sources, is_env_contract_source};

const STANDARD_EXEMPTIONS: &[&str] = &[
    "RUST_LOG",
    "RUST_BACKTRACE",
    "OUT_DIR",
    "TMPDIR",
    "HOME",
    "PATH",
    "CARGO",
    "CI",
    "TARGET",
    "GITHUB_SHA",
    "GITHUB_RUN_ID",
    "GITHUB_REPOSITORY",
    "GITHUB_ACTIONS",
];

const REGISTERED_ENV_FLAGS: &[&str] = &[
    "CHROMIUM",
    "NO_LIGHTS",
    "RUST_TOOLCHAIN",
    "SCENA_ALLOW_PARTIAL_HARDWARE_BACKENDS",
    "SCENA_A03_BIN",
    "SCENA_A04_BIN",
    "SCENA_A04_EXPECT_AGENT",
    "SCENA_A05_BIN",
    "SCENA_BROWSER_ALLOW_UNAVAILABLE",
    "SCENA_BROWSER_BACKENDS",
    "SCENA_BROWSER_COMPRESSED_ASSETS",
    "SCENA_BROWSER_EXECUTABLE",
    "SCENA_BROWSER_OVERSIZED_TEXTURE",
    "SCENA_BROWSER_REQUIRE_V3D",
    "SCENA_BROWSER_FORCE_REBUILD",
    "SCENA_BROWSER_VIEWER_ELEMENT_ONLY",
    "SCENA_BROWSER_WORKFLOWS",
    "SCENA_WEBGL2_BROWSER",
    "SCENA_WEBGPU_BROWSER",
    "SCENA_BUILD_HEARTBEAT_MS",
    "SCENA_HARDWARE_PROOF_COMMAND",
    "SCENA_HARDWARE_PROOF_ROOT",
    "SCENA_BENCHMARK_COMMAND",
    "SCENA_BENCHMARK_CPU",
    "SCENA_BENCHMARK_PROFILE",
    "SCENA_MATERIAL_PROOF_URL",
    "SCENA_M9_TIMING_POLICY",
    "SCENA_REFERENCE_DIFF",
    "SCENA_Q11_REFERENCE_CANDIDATE_DIR",
    "SCENA_REQUIRE_PARITY",
    "SCENA_REQUIRE_AA_EFFECT_PROOF",
    "SCENA_REQUIRE_HARDWARE_GPU",
    "SCENA_REQUIRE_GPU_PARITY",
    "SCENA_RELEASE_COMMIT",
    "SCENA_RELEASE_PROFILE",
    "SCENA_ROUND_E_REFERENCE_SHOWCASE",
    "SCENA_RUN_DEDICATED_4K_BENCHMARK",
    "SCENA_RUN_M9_PLATFORM_BENCHMARK",
    "SCENA_RUN_PF00_BENCHMARK",
    "SCENA_REAGGREGATE_PF00",
    "SCENA_RUN_PF03_STORAGE_BENCHMARK",
    "SCENA_RUN_PF10_OCCLUSION_BENCHMARK",
    "SCENA_RUN_CONTROLLED_P01_BENCHMARK",
    "SCENA_RUN_EXPENSIVE_CPU_RELEASE_TESTS",
    "SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS",
    "SCENA_SHOWCASE_CONNECTOR_ONLY",
    "SCENA_SHOWCASE_SECTION_BUDGET_MS",
    "SCENA_SKIP_WASM_BUILD",
    "SCENA_USE_GPU",
    "SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU",
    "SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS",
    "SCENA_DEBUG_LOG_ENVIRONMENT",
    "SCENA_EASY_SCENE_SHOWCASE_ONLY",
    "SCENA_GLTF_VALIDATOR",
    "SCENA_GPU_EVIDENCE_CLASS",
    "SCENA_RELEASE_ARTIFACT_ROOT",
    "SCENA_REQUIRE_CI_PROVENANCE",
    "VK_ICD_FILENAMES",
    "XDG_CACHE_HOME",
];

/// Ensure every non-standard test/script environment variable is registered
/// and documented in `CLAUDE.md`.
pub(crate) fn check_tests_env_flags_documented(root: &Path, findings: &mut Vec<Finding>) {
    let claude_md = match fs::read_to_string(root.join("CLAUDE.md")) {
        Ok(text) => text,
        Err(_) => {
            findings.push(Finding::new(
                "TESTS-ENV-FLAGS-DOCUMENTED",
                "CLAUDE.md must exist and list test environment flags".to_string(),
            ));
            return;
        }
    };
    let mut entries = Vec::new();
    collect_test_contract_sources(&root.join("tests"), &mut entries);
    collect_test_contract_sources(&root.join("scripts"), &mut entries);
    // E05 (`N20`): scanning only `tests/` and `scripts/` left every flag read
    // from product code, examples, and the release tooling invisible to this
    // rule — including `SCENA_REQUIRE_CI_PROVENANCE` and
    // `SCENA_GPU_EVIDENCE_CLASS`, which gate release evidence.
    collect_test_contract_sources(&root.join("src"), &mut entries);
    collect_test_contract_sources(&root.join("examples"), &mut entries);
    collect_test_contract_sources(&root.join("crates/xtask/src"), &mut entries);
    entries.sort();
    entries.dedup();
    for path in entries {
        // xtask's own `tests_NN.rs` unit tests embed synthetic sources such as
        // `env::var("MY_OTHER_FLAG")` as fixtures for `find_env_var_names`.
        // Those are test data, not flags this repository reads.
        if path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(|name| name.starts_with("tests_"))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        // Doc comments describe the scanner's own call shapes
        // (`env::var("FOO")`); they are documentation, not reads.
        let text = strip_comment_lines(&text);
        let display = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        for capture in find_env_var_names(&text) {
            if STANDARD_EXEMPTIONS.contains(&capture.as_str()) || capture.starts_with("CARGO_") {
                continue;
            }
            if !REGISTERED_ENV_FLAGS.contains(&capture.as_str()) {
                findings.push(Finding::new(
                    "TESTS-ENV-FLAGS-DOCUMENTED",
                    format!(
                        "{display} reads env var '{capture}' that is absent from the shared test/script env registry"
                    ),
                ));
            }
            if !claude_md.contains(&capture) {
                findings.push(Finding::new(
                    "TESTS-ENV-FLAGS-DOCUMENTED",
                    format!(
                        "{display} reads env var '{capture}' that is not listed in CLAUDE.md's test environment flags table"
                    ),
                ));
            }
        }
    }
    for name in REGISTERED_ENV_FLAGS {
        if !claude_md.contains(&format!("`{name}`")) {
            findings.push(Finding::new(
                "TESTS-ENV-FLAGS-DOCUMENTED",
                format!(
                    "shared env registry entry '{name}' is missing from CLAUDE.md's test environment flags table"
                ),
            ));
        }
    }
}

/// Drops whole-line `//` comments so documentation of the scanner's own call
/// shapes is never mistaken for a real environment read.
fn strip_comment_lines(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn find_env_var_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for marker in &["env::var(\"", "env::var_os(\""] {
        let mut cursor = 0;
        while let Some(start) = source[cursor..].find(marker) {
            let head = cursor + start + marker.len();
            if let Some(end) = source[head..].find('"') {
                let name = source[head..head + end].to_string();
                if !name.is_empty() && !names.contains(&name) {
                    names.push(name);
                }
                cursor = head + end + 1;
            } else {
                break;
            }
        }
    }
    for marker in &["process.env.", "process.env[\"", "process.env['"] {
        let mut cursor = 0;
        while let Some(start) = source[cursor..].find(marker) {
            let head = cursor + start + marker.len();
            let end = source[head..]
                .find(|ch: char| !(ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_'))
                .unwrap_or(source.len() - head);
            let name = source[head..head + end].to_string();
            if !name.is_empty() && !names.contains(&name) {
                names.push(name);
            }
            cursor = (head + end).max(head + 1);
        }
    }
    names
}
