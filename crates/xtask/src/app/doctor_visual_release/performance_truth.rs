use crate::app::prelude::*;

mod acceleration;
pub(crate) use acceleration::{
    check_pf06_spatial_acceleration_contracts, check_pf09_parallel_work_contracts,
    check_pf10_hot_path_contracts,
};
mod hot_path;
pub(crate) use hot_path::check_pf03_pf05_hot_path_contracts;

const RULE: &str = "PF00-PERFORMANCE-TRUTH";
const HOT_PATH_RULE: &str = "PF03-PF05-HOT-PATH-CONTRACTS";
pub(super) const PF06_RULE: &str = "PF06-SHARED-SPATIAL-ACCELERATION";
pub(super) const PF09_RULE: &str = "PF09-DETERMINISTIC-PARALLEL-WORK";
const PF10_RULE: &str = "PF10-MEASURED-HOT-PATH-WASTE";
const PF07_PF08_RULE: &str = "PF07-PF08-BOUNDED-CPU-PREPARE";
const PERFORMANCE_EVIDENCE: &str = "docs/specs/performance-evidence.json";
const REQUIRED_WORKLOADS: &[&str] = &[
    "one-node-transform-prepare-render",
    "shadow-scaling-directional-area",
    "pick-100k-triangle-deformed-undeformed",
    "cpu-texture-bake-qualifying-nonqualifying",
    "tangent-generation-static-deformed",
    "animation-many-channels-keyframes-weights",
    "environment-bake-cold-sidecar-hit",
    "native-present-capture-sync-async",
    "gpu-first-render-output-settings",
    "draw-uniform-indexing-many-unique-transforms",
    "cpu-occlusion-prepass-benefit",
];

pub(crate) fn check_pf00_performance_truth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_source_contract(
        root,
        findings,
        "tests/m5_release.rs",
        &["\"status\": \"unavailable\"", "not instrumented"],
        &["\"allocation_bytes\": 0"],
    );
    check_source_contract(
        root,
        findings,
        "tests/m9_platform_release.rs",
        &[
            "ALLOCATION_BYTES",
            "p50_allocated_bytes_per_frame",
            "p95_allocated_bytes_per_frame",
            "max_allocated_bytes_per_frame",
            "prepare_sample_count",
            "p50_prepare_ms",
            "p95_prepare_ms",
            "max_prepare_ms",
            "performance_environment",
            "SCENA_BENCHMARK_PROFILE",
            "SCENA_RUN_PF00_BENCHMARK",
            "SCENA_REAGGREGATE_PF00",
            "sidecar_cache_state",
            "distribution-only",
            "fn m9_pf00_representative_performance_artifact",
            "pick_with_assets_profiled",
            "benchmark_profiled_native_capture_workload",
            "RenderReadbackMode::PresentOnly",
            "RenderReadbackMode::Synchronous",
            "blocking_waits",
            "gpu_bind_group_creations",
            "async_readback_submissions",
            "peak_readbacks_in_flight",
            "asset_storage_lock_acquisitions",
            "precompute_environment_sidecar_profiled",
            "source_texture_samples",
            "brdf_integration_samples",
            "benchmark_profiled_gpu_output_settings_workload",
            "first_prepared_render_ms",
            "PF00_REQUIRED_HARDWARE_ARTIFACTS",
            "validate_pf00_complete_hardware_summary",
            "load_pf00_complete_hardware_proof",
            "performance_environment_metadata_with_renderer",
            "pf00_release_evidence_ready",
            "classify_pf00_workload_provenance",
            "validate_pf00_existing_measurement_artifact",
        ],
        &["pf00_gpu_output_settings_requirement"],
    );
    check_source_contract(
        root,
        findings,
        "tests/pf10_cpu_occlusion.rs",
        &[
            "SCENA_RUN_PF10_OCCLUSION_BENCHMARK",
            "sample_scene_pair",
            "cpu-occlusion-prepass-benefit",
            "pf10_release_evidence_ready",
            "measurement_evidence",
            "release_provenance",
            "source_checksums",
            "performance_environment",
        ],
        &[],
    );
    check_source_contract(
        root,
        findings,
        "src/assets.rs",
        &[
            "storage_lock_acquisitions: Arc<AtomicU64>",
            "pub fn storage_lock_acquisitions",
            "fetch_add(1, Ordering::Relaxed)",
        ],
        &[],
    );
    check_source_contract(
        root,
        findings,
        "src/render/prepare/environment_baker.rs",
        &[
            "pub struct EnvironmentBakeMetrics",
            "source_texture_samples",
            "brdf_integration_samples",
            "output_bytes_written",
            "bake_environment_ibl_profiled",
        ],
        &[],
    );
    check_baseline_policy(root, findings);
    check_evidence_registry(root, findings);
    for workflow in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        check_source_contract(
            root,
            findings,
            workflow,
            &[
                "SCENA_RUN_DEDICATED_4K_BENCHMARK",
                "SCENA_BENCHMARK_PROFILE: perf-test",
                "cargo test --profile perf-test --test m9_platform_release",
            ],
            &["cargo test --release --test m9_platform_release m9_dedicated_headless_4k"],
        );
    }
    check_source_contract(
        root,
        findings,
        ".github/workflows/hardware-gpu.yml",
        &[
            "runs-on: [self-hosted, linux, x64, gpu, scena-gpu]",
            "if: inputs.run_performance",
            "SCENA_RUN_PF00_BENCHMARK: \"1\"",
            "SCENA_BENCHMARK_PROFILE: perf-test",
            "SCENA_BENCHMARK_COMMAND:",
            "m9_pf00_representative_performance_artifact",
            "windows_complete_hardware_proof_validation.js",
            "windows-complete-hardware-proof/proof-summary.json",
        ],
        &[],
    );
}

pub(crate) fn check_pf07_pf08_cpu_prepare_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, required, forbidden) in [
        (
            "src/geometry.rs",
            &[
                "struct GeneratedTangentCache",
                "cached_generated_tangents",
                "Arc<OnceLock<Arc<[[f32; 4]]>>>",
            ][..],
            &[][..],
        ),
        (
            "src/render/prepare/tangents.rs",
            &["generate_model_tangents", "transform_model_tangents"][..],
            &[][..],
        ),
        (
            "src/render/prepare/primitives.rs",
            &[
                "record_generated_tangent_cache",
                "triangle_screen_edge_pixels",
                "triangle_uv_span",
                "max_decoded_dimension",
                "subdivision_scratch",
                "transmissive && source.material.transmission_texture().is_some()",
            ][..],
            &["cpu_texture_subdivisions(source.material, backend_shaded_material)"][..],
        ),
        (
            "src/render/prepare/cpu_bake.rs",
            &[
                "CPU_TEXTURE_SUBDIVISION_HARD_CAP",
                "CPU_TEXTURE_PIXELS_PER_SUBDIVISION",
                "SubdividedCpuCorners::Single",
                "scratch.reserve",
            ][..],
            &["return vec![corners]", "subdivided_cpu_corners(corners, 48"][..],
        ),
        (
            "tests/m9_platform_release.rs",
            &[
                "generated_tangent_cache_hits",
                "static_cold",
                "texture_roles",
                "base_color",
                "clearcoat_normal",
                "transmission",
                "thickness",
            ][..],
            &["\"qualifying_subdivisions\": 48"][..],
        ),
        (
            "tests/pf08_texture_bake_parity.rs",
            &[
                "pf08_adaptive_texture_bake_preserves_seams_perspective_and_material_identity_cpu_gpu",
                "render_scene_cpu_gpu_pair_with_renderer",
                "scena.pf08.texture_bake_parity.v1",
                "shared-triangle-seam",
                "perspective-interpolation",
                "material-identity",
                "assert_shared_triangle_diagonal_has_no_gap",
            ][..],
            &[][..],
        ),
    ] {
        check_pf10_source_contract(root, findings, relative, required, forbidden);
    }
    for finding in findings
        .iter_mut()
        .filter(|finding| finding.rule == PF10_RULE)
    {
        finding.rule = PF07_PF08_RULE;
    }
}

pub(super) fn check_pf10_source_contract(
    root: &Path,
    findings: &mut Vec<Finding>,
    relative: &str,
    required: &[&str],
    forbidden: &[&str],
) {
    let Some(text) = read_source_contract_tree(root, relative) else {
        findings.push(Finding::new(
            PF10_RULE,
            format!("could not read {relative}"),
        ));
        return;
    };
    for token in required {
        if !text.contains(token) {
            findings.push(Finding::new(
                PF10_RULE,
                format!("{relative} is missing measured hot-path contract {token}"),
            ));
        }
    }
    for token in forbidden {
        if text.contains(token) {
            findings.push(Finding::new(
                PF10_RULE,
                format!("{relative} regressed to measured hot-path pattern {token}"),
            ));
        }
    }
}

pub(super) fn check_rule_source_contract(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    relative: &str,
    required: &[&str],
    forbidden: &[&str],
) {
    let Some(text) = read_source_contract_tree(root, relative) else {
        findings.push(Finding::new(rule, format!("could not read {relative}")));
        return;
    };
    for token in required {
        if !text.contains(token) {
            findings.push(Finding::new(
                rule,
                format!("{relative} is missing required source contract {token}"),
            ));
        }
    }
    for token in forbidden {
        if text.contains(token) {
            findings.push(Finding::new(
                rule,
                format!("{relative} regressed to forbidden source pattern {token}"),
            ));
        }
    }
}

fn check_hot_path_source_contract(
    root: &Path,
    findings: &mut Vec<Finding>,
    relative: &str,
    required: &[&str],
    forbidden: &[&str],
) {
    let Some(text) = read_source_contract_tree(root, relative) else {
        findings.push(Finding::new(
            HOT_PATH_RULE,
            format!("could not read {relative}"),
        ));
        return;
    };
    for token in required {
        if !text.contains(token) {
            findings.push(Finding::new(
                HOT_PATH_RULE,
                format!("{relative} is missing hot-path contract {token}"),
            ));
        }
    }
    for token in forbidden {
        if text.contains(token) {
            findings.push(Finding::new(
                HOT_PATH_RULE,
                format!("{relative} regressed to forbidden hot-path pattern {token}"),
            ));
        }
    }
}

fn check_source_contract(
    root: &Path,
    findings: &mut Vec<Finding>,
    relative: &str,
    required: &[&str],
    forbidden: &[&str],
) {
    let Some(text) = read_source_contract_tree(root, relative) else {
        findings.push(Finding::new(RULE, format!("could not read {relative}")));
        return;
    };
    for token in required {
        if !text.contains(token) {
            findings.push(Finding::new(
                RULE,
                format!("{relative} is missing performance-truth contract {token}"),
            ));
        }
    }
    for token in forbidden {
        if text.contains(token) {
            findings.push(Finding::new(
                RULE,
                format!("{relative} retains fabricated or weak contract {token}"),
            ));
        }
    }
}

fn read_source_contract_tree(root: &Path, relative: &str) -> Option<String> {
    let relative_path = Path::new(relative);
    let mut text = fs::read_to_string(root.join(relative_path)).ok()?;
    if relative_path.extension().and_then(OsStr::to_str) != Some("rs")
        || relative_path.file_name().and_then(OsStr::to_str) == Some("mod.rs")
    {
        return Some(text);
    }
    let module_dir = relative_path.with_extension("");
    for child in source_files(root)
        .into_iter()
        .filter(|path| path.starts_with(&module_dir))
    {
        if let Ok(child_text) = fs::read_to_string(root.join(child)) {
            text.push('\n');
            text.push_str(&child_text);
        }
    }
    Some(text)
}

fn check_baseline_policy(root: &Path, findings: &mut Vec<Finding>) {
    let relative = "tests/fixtures/m9-baselines.json";
    let Ok(text) = fs::read_to_string(root.join(relative)) else {
        findings.push(Finding::new(RULE, format!("could not read {relative}")));
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        findings.push(Finding::new(RULE, format!("could not parse {relative}")));
        return;
    };
    let Some(rows) = value.get("rows").and_then(Value::as_array) else {
        findings.push(Finding::new(RULE, format!("{relative} has no rows array")));
        return;
    };
    if rows.is_empty() {
        findings.push(Finding::new(
            RULE,
            format!("{relative} has no baseline rows"),
        ));
    }
    for row in rows {
        let scene = row
            .get("scene")
            .and_then(Value::as_str)
            .unwrap_or("<unknown>");
        if row
            .get("allowed_regression_percent")
            .and_then(Value::as_f64)
            != Some(5.0)
        {
            findings.push(Finding::new(
                RULE,
                format!("{relative} row {scene} must enforce exactly 5% regression"),
            ));
        }
        for field in ["p95_prepare_ms", "max_allocated_bytes_per_frame"] {
            if row.get(field).and_then(Value::as_f64).is_none() {
                findings.push(Finding::new(
                    RULE,
                    format!("{relative} row {scene} is missing {field}"),
                ));
            }
        }
    }
}

fn check_evidence_registry(root: &Path, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(root.join(PERFORMANCE_EVIDENCE)) else {
        findings.push(Finding::new(
            RULE,
            format!("could not read {PERFORMANCE_EVIDENCE}"),
        ));
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        findings.push(Finding::new(
            RULE,
            format!("could not parse {PERFORMANCE_EVIDENCE}"),
        ));
        return;
    };
    let bundle = value.get("current_measurement_bundle");
    let bundle_release_evidence = bundle
        .and_then(|bundle| bundle.get("release_evidence"))
        .and_then(Value::as_bool);
    let valid_bundle_hash = bundle
        .and_then(|bundle| bundle.get("artifact_sha256"))
        .and_then(Value::as_str)
        .is_some_and(valid_sha256);
    if bundle
        .and_then(|bundle| bundle.get("artifact"))
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
        || bundle
            .and_then(|bundle| bundle.get("status"))
            .and_then(Value::as_str)
            != Some("measured")
        || bundle
            .and_then(|bundle| bundle.get("measurement_evidence"))
            .and_then(Value::as_bool)
            != Some(true)
        || bundle
            .and_then(|bundle| bundle.get("hardware_evidence"))
            .and_then(Value::as_bool)
            != Some(true)
        || bundle_release_evidence.is_none()
        || !valid_bundle_hash
    {
        findings.push(Finding::new(
            RULE,
            format!(
                "{PERFORMANCE_EVIDENCE} current measurement bundle is missing measured/hardware/provenance fields"
            ),
        ));
    }
    if bundle_release_evidence == Some(true)
        && bundle
            .and_then(|bundle| bundle.get("release_provenance"))
            .and_then(|provenance| provenance.get("status"))
            .and_then(Value::as_str)
            != Some("exact-commit")
    {
        findings.push(Finding::new(
            RULE,
            format!(
                "{PERFORMANCE_EVIDENCE} current bundle claims release evidence without exact-commit provenance"
            ),
        ));
    }
    let Some(rows) = value.get("workloads").and_then(Value::as_array) else {
        findings.push(Finding::new(
            RULE,
            format!("{PERFORMANCE_EVIDENCE} has no workloads array"),
        ));
        return;
    };
    let by_id = rows
        .iter()
        .filter_map(|row| Some((row.get("id")?.as_str()?, row)))
        .collect::<BTreeMap<_, _>>();
    for id in REQUIRED_WORKLOADS {
        let Some(row) = by_id.get(id) else {
            findings.push(Finding::new(
                RULE,
                format!("{PERFORMANCE_EVIDENCE} is missing workload {id}"),
            ));
            continue;
        };
        for field in ["producer", "artifact", "status"] {
            if row
                .get(field)
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                findings.push(Finding::new(
                    RULE,
                    format!("{PERFORMANCE_EVIDENCE} workload {id} is missing {field}"),
                ));
            }
        }
        let measured = row
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status.starts_with("measured"));
        if measured {
            if row.get("measurement_evidence").and_then(Value::as_bool) != Some(true) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{PERFORMANCE_EVIDENCE} workload {id} is measured without measurement_evidence=true"
                    ),
                ));
            }
            let row_release_evidence = row.get("release_evidence").and_then(Value::as_bool);
            if row_release_evidence.is_none() {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{PERFORMANCE_EVIDENCE} workload {id} is measured without an explicit release-evidence classification"
                    ),
                ));
            }
            if row_release_evidence == Some(true) && bundle_release_evidence != Some(true) {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{PERFORMANCE_EVIDENCE} workload {id} claims release evidence while the current bundle is nonrelease"
                    ),
                ));
            }
            if !row
                .get("artifact_sha256")
                .and_then(Value::as_str)
                .is_some_and(valid_sha256)
            {
                findings.push(Finding::new(
                    RULE,
                    format!(
                        "{PERFORMANCE_EVIDENCE} workload {id} is measured without a valid artifact SHA-256"
                    ),
                ));
            }
        }
        if let Some(producer) = row.get("producer").and_then(Value::as_str) {
            let Some((relative, function)) = producer.split_once("::") else {
                findings.push(Finding::new(
                    RULE,
                    format!("{PERFORMANCE_EVIDENCE} workload {id} has invalid producer {producer}"),
                ));
                continue;
            };
            let declaration = format!("fn {function}");
            let producer_missing = match fs::read_to_string(root.join(relative)) {
                Ok(source) => !source.contains(&declaration),
                Err(_) => true,
            };
            if producer_missing {
                findings.push(Finding::new(
                    RULE,
                    format!("{PERFORMANCE_EVIDENCE} workload {id} producer {producer} is missing"),
                ));
            }
        }
        for field in ["required_distributions", "required_counters"] {
            if row
                .get(field)
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
            {
                findings.push(Finding::new(
                    RULE,
                    format!("{PERFORMANCE_EVIDENCE} workload {id} is missing {field}"),
                ));
            }
        }
    }
}

fn valid_sha256(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}
