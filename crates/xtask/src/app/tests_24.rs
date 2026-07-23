use crate::app::prelude::*;

#[test]
pub(crate) fn q01_headless_lane_requires_bound_cpu_waterbottle_evidence() {
    let required = release_lane_required_artifacts("headless-cpu");
    for expected in [
        "target/gate-artifacts/q01-waterbottle-cpu/live.png",
        "target/gate-artifacts/q01-waterbottle-cpu/known_bad_flattened_chrome.png",
        "target/gate-artifacts/q01-waterbottle-cpu/known_bad_wrong_material.png",
        "target/gate-artifacts/q01-waterbottle-cpu/known_bad_wrong_camera.png",
        "target/gate-artifacts/q01-waterbottle-cpu/result.json",
    ] {
        assert!(
            required.iter().any(|artifact| artifact == expected),
            "Q01 release lane must require {expected}; got {required:?}"
        );
    }

    let commands = release_lane_expected_commands("headless-cpu");
    assert!(
        commands.iter().any(|command| {
            command
                == &"cargo test --test q01_waterbottle_cpu_reference \
q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact"
        }),
        "Q01 release lane must require the exact live CPU producer command; got {commands:?}"
    );
}

#[test]
pub(crate) fn q01_typed_visual_proof_rejects_unbound_or_passing_mutations() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q01-typed-visual-proof");
    let visual_root = fixture_root.join("visual-proof");
    let source = fixture_root.join("q01-waterbottle-cpu/live.png");
    let result = fixture_root.join("q01-waterbottle-cpu/result.json");
    let proof_path = visual_root.join("waterbottle-cpu.json");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(source.parent().expect("Q01 source parent")).expect("Q01 source dir");
    fs::create_dir_all(&visual_root).expect("Q01 visual proof dir");
    fs::write(&source, b"not a PNG").expect("Q01 source fixture writes");
    fs::write(&result, b"{}\n").expect("Q01 result fixture writes");
    fs::write(
        &proof_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.visual_proof.v1",
            "producer": "cargo run -p xtask -- stage-release-artifacts",
            "lane": "waterbottle-cpu",
            "status": "passed",
            "preview_only": false,
            "rust_test_command": true,
            "rust_test_output_observed": false,
            "skip_marker_observed": false,
            "release_evidence": true,
            "proof_class": "cpu-waterbottle-reference",
            "commit_sha": "0123456789abcdef0123456789abcdef01234567",
            "timestamp_unix_seconds": current_unix_seconds(),
            "source_artifact_path": "q01-waterbottle-cpu/live.png",
            "source_artifact_sha256": "0".repeat(64),
            "result_artifact": "q01-waterbottle-cpu/result.json",
            "result_sha256": "0".repeat(64),
            "width": 512,
            "height": 512,
            "color_type": "rgb8",
            "color_space": "unknown",
            "row_orientation": "bottom-to-top",
            "alpha_contract": "variable",
            "backend": "Metal",
            "adapter": "hardware",
            "command_record_sha256": "0".repeat(64),
            "metrics": {"passed": false},
            "mutations": [
                {"name":"flattened_chrome", "oracle_rejected":false},
                {"name":"wrong_material", "oracle_rejected":false},
                {"name":"wrong_camera", "oracle_rejected":false}
            ]
        }))
        .expect("Q01 visual proof fixture serializes"),
    )
    .expect("Q01 visual proof fixture writes");

    let mut findings = Vec::new();
    require_visual_proof_artifact_file(
        &proof_path,
        "visual-proof/waterbottle-cpu.json",
        &mut findings,
    );
    for expected in [
        "source artifact hash",
        "CPU dimensions/type",
        "CPU color/orientation contract",
        "CPU backend/adapter",
        "CPU comparison metrics",
        "CPU mutation oracle",
        "exact Rust test output",
        "command-record hash",
        "result artifact hash",
    ] {
        assert!(
            findings.iter().any(|finding| {
                finding.rule == "VISUAL-PROOF" && finding.message.contains(expected)
            }),
            "Q01 typed visual proof must reject {expected}: {findings:?}"
        );
    }
}

#[test]
pub(crate) fn q01_cpu_finalizer_binds_command_log_and_rejects_tampered_live_png() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q01-cpu-finalizer");
    let artifact_root = fixture_root.join("target/gate-artifacts");
    let result_path = artifact_root.join("q01-waterbottle-cpu/result.json");
    let command_path = artifact_root.join("release-lanes/headless-cpu.commands.jsonl");
    let log_path = artifact_root.join("release-lanes/headless-cpu.log");
    let _ = fs::remove_dir_all(&fixture_root);
    fs::create_dir_all(result_path.parent().expect("Q01 result parent")).expect("Q01 result dir");
    fs::create_dir_all(command_path.parent().expect("Q01 command parent"))
        .expect("Q01 command dir");

    let live_path = artifact_root.join("q01-waterbottle-cpu/live.png");
    fs::write(&live_path, b"live PNG fixture").expect("Q01 live fixture writes");
    let mutations = [
        (
            "flattened_chrome",
            "q01-waterbottle-cpu/known_bad_flattened_chrome.png",
        ),
        (
            "wrong_material",
            "q01-waterbottle-cpu/known_bad_wrong_material.png",
        ),
        (
            "wrong_camera",
            "q01-waterbottle-cpu/known_bad_wrong_camera.png",
        ),
    ]
    .map(|(name, relative)| {
        let path = artifact_root.join(relative);
        fs::write(&path, format!("{name} PNG fixture")).expect("Q01 mutation fixture writes");
        let (mutation_kind, mutation_stage, render_count, pipeline_coverage) =
            q01_mutation_provenance(name);
        json!({
            "name": name,
            "path": relative,
            "sha256": sha256_hex(&path).expect("Q01 mutation hash"),
            "oracle_rejected": true,
            "mutation_kind": mutation_kind,
            "mutation_stage": mutation_stage,
            "render_count": render_count,
            "pipeline_coverage": pipeline_coverage,
            "metrics": {"passed": false}
        })
    });
    let expected_command = "cargo test --test q01_waterbottle_cpu_reference \
                            q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact";
    fs::write(
        &command_path,
        format!(
            "{}\n",
            json!({
                "command": expected_command,
                "status": "passed",
                "duration_ms": 42,
                "failure_log_path": "target/gate-artifacts/release-lanes/headless-cpu.log"
            })
        ),
    )
    .expect("Q01 command fixture writes");
    fs::write(
        &log_path,
        "running 1 test\n\
test q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders ... ok\n\
test result: ok. 1 passed; 0 failed\n",
    )
    .expect("Q01 log fixture writes");
    fs::write(
        &result_path,
        serde_json::to_string_pretty(&json!({
            "schema": "scena.q01.waterbottle_cpu_reference.v1",
            "status": "passed",
            "release_evidence": true,
            "test_name": "q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders",
            "commit_sha": "0123456789abcdef0123456789abcdef01234567",
            "timestamp_unix_seconds": current_unix_seconds(),
            "backend": "Headless",
            "adapter": "software-rasterizer",
            "width": 256,
            "height": 256,
            "color_type": "rgba8",
            "color_space": "srgb-output",
            "row_orientation": "top-to-bottom",
            "alpha_contract": "opaque",
            "live_png_sha256": sha256_hex(&live_path).expect("Q01 live hash"),
            "determinism": {
                "comparison_order": "independent-render-before-committed-reference",
                "repeat_count": 2,
                "byte_identical": true,
                "rgba8_sha256": [
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                ]
            },
            "metrics": {"passed": true},
            "mutations": mutations,
            "source_checksums": []
        }))
        .expect("Q01 result fixture serializes"),
    )
    .expect("Q01 result fixture writes");

    finalize_waterbottle_cpu_result(&fixture_root).expect("complete Q01 proof finalizes");
    let finalized: Value = serde_json::from_str(
        &fs::read_to_string(&result_path).expect("finalized Q01 result reads"),
    )
    .expect("finalized Q01 result parses");
    assert_eq!(
        finalized.get("rust_test_output_observed"),
        Some(&Value::Bool(true))
    );
    let command_hash = sha256_hex(&command_path).expect("Q01 command hash");
    assert_eq!(
        finalized
            .get("command_record_sha256")
            .and_then(Value::as_str),
        Some(command_hash.as_str())
    );

    let mut post_hoc_material = finalized.clone();
    let wrong_material = post_hoc_material["mutations"]
        .as_array_mut()
        .expect("Q01 mutations array")
        .iter_mut()
        .find(|mutation| mutation["name"] == "wrong_material")
        .expect("Q01 wrong-material mutation");
    wrong_material["mutation_kind"] = json!("post-hoc-pixel");
    wrong_material["render_count"] = json!(0);
    fs::write(
        &result_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&post_hoc_material)
                .expect("mutated Q01 result serializes")
        ),
    )
    .expect("mutated Q01 result writes");
    let provenance_error = finalize_waterbottle_cpu_result(&fixture_root)
        .expect_err("post-hoc wrong-material output must not finalize");
    assert!(
        provenance_error.contains("wrong_material") && provenance_error.contains("rendered-scene"),
        "unexpected Q10 provenance error: {provenance_error}"
    );
    fs::write(
        &result_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&finalized).expect("restored Q01 result serializes")
        ),
    )
    .expect("restored Q01 result writes");

    fs::write(&live_path, b"tampered live PNG").expect("Q01 tampered live fixture writes");
    let error = finalize_waterbottle_cpu_result(&fixture_root)
        .expect_err("tampered Q01 live PNG must fail finalization");
    assert!(error.contains("live PNG hash"), "unexpected error: {error}");
}

#[test]
pub(crate) fn q01_doctor_rejects_missing_default_lane_producer_command() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q01-waterbottle-policy");
    let required_files = [
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        "CLAUDE.md",
        "tests/q01_waterbottle_cpu_reference.rs",
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
        "crates/xtask/src/app/release/lane_artifacts.rs",
        "crates/xtask/src/app/release/waterbottle_results.rs",
        "crates/xtask/src/app/release/stage_visual_proofs.rs",
        "crates/xtask/src/app/visual_artifacts/typed_visual_proof.rs",
        "crates/xtask/src/app/release/review_artifacts.rs",
    ];
    let _ = fs::remove_dir_all(&fixture_root);
    for relative in required_files {
        let source = root.join(relative);
        let target = fixture_root.join(relative);
        fs::create_dir_all(target.parent().expect("Q01 doctor fixture parent"))
            .expect("Q01 doctor fixture dir");
        fs::copy(source, target).expect("Q01 doctor fixture copies");
    }
    let ci = fixture_root.join(".github/workflows/ci.yml");
    let text = fs::read_to_string(&ci).expect("Q01 CI fixture reads");
    let exact = "          bash scripts/release_lane_command.sh headless-cpu cargo test --test \
q01_waterbottle_cpu_reference q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact\n";
    assert!(text.contains(exact), "Q01 exact CI producer command exists");
    fs::write(&ci, text.replacen(exact, "", 1)).expect("Q01 CI mutation writes");

    let mut findings = Vec::new();
    check_q01_waterbottle_cpu_proof(&fixture_root, &mut findings);
    assert!(
        findings.iter().any(|finding| {
            finding.rule == "Q01-WATERBOTTLE"
                && finding.message.contains(".github/workflows/ci.yml")
                && finding.message.contains("q01_waterbottle_cpu_reference")
        }),
        "Q01 doctor must reject a missing exact CI producer command: {findings:?}"
    );
}

pub(crate) fn write_q01_cpu_proof_fixture(artifact_root: &Path, commit: &str) {
    let q01_root = artifact_root.join("q01-waterbottle-cpu");
    let command_root = artifact_root.join("release-lanes");
    fs::create_dir_all(&q01_root).expect("Q01 fixture artifact dir");
    fs::create_dir_all(&command_root).expect("Q01 fixture command dir");
    let live_path = q01_root.join("live.png");
    write_q01_fixture_png(&live_path, 0);
    let mutation_specs = [
        ("flattened_chrome", "known_bad_flattened_chrome.png", 37),
        ("wrong_material", "known_bad_wrong_material.png", 83),
        ("wrong_camera", "known_bad_wrong_camera.png", 151),
    ];
    let mutations = mutation_specs.map(|(name, file, offset)| {
        let path = q01_root.join(file);
        write_q01_fixture_png(&path, offset);
        let (mutation_kind, mutation_stage, render_count, pipeline_coverage) =
            q01_mutation_provenance(name);
        json!({
            "name": name,
            "path": format!("q01-waterbottle-cpu/{file}"),
            "sha256": sha256_hex(&path).expect("Q01 fixture mutation hash"),
            "oracle_rejected": true,
            "mutation_kind": mutation_kind,
            "mutation_stage": mutation_stage,
            "render_count": render_count,
            "pipeline_coverage": pipeline_coverage,
            "metrics": {"passed": false}
        })
    });
    let command_path = command_root.join("headless-cpu.commands.jsonl");
    let log_path = command_root.join("headless-cpu.log");
    let q01_command = "cargo test --test q01_waterbottle_cpu_reference \
                       q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders -- --exact";
    fs::write(
        &command_path,
        format!(
            "{}\n{}\n",
            json!({
                "command": "cargo test --test m9_platform_release",
                "status": "passed",
                "duration_ms": 1,
                "failure_log_path": "target/gate-artifacts/release-lanes/headless-cpu.log"
            }),
            json!({
                "command": q01_command,
                "status": "passed",
                "duration_ms": 1,
                "failure_log_path": "target/gate-artifacts/release-lanes/headless-cpu.log"
            })
        ),
    )
    .expect("Q01 fixture command writes");
    fs::write(
        &log_path,
        "running 1 test\n\
test q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders ... ok\n\
test result: ok. 1 passed; 0 failed\n",
    )
    .expect("Q01 fixture log writes");
    let live_sha = sha256_hex(&live_path).expect("Q01 fixture live hash");
    let command_sha = sha256_hex(&command_path).expect("Q01 fixture command hash");
    let log_sha = sha256_hex(&log_path).expect("Q01 fixture log hash");
    let mut result = json!({
        "schema": "scena.q01.waterbottle_cpu_reference.v1",
        "status": "passed",
        "release_evidence": true,
        "test_name": "q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders",
        "producer": q01_command,
        "producing_command": q01_command,
        "toolchain": "rustc fixture-toolchain",
        "profile": "test-unoptimized",
        "sample_count": 1,
        "commit_sha": commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "backend": "Headless",
        "adapter": "software-rasterizer",
        "width": 256,
        "height": 256,
        "color_type": "rgba8",
        "color_space": "srgb-output",
        "row_orientation": "top-to-bottom",
        "alpha_contract": "opaque",
        "live_png_path": "q01-waterbottle-cpu/live.png",
        "live_png_sha256": live_sha,
        "reference_path": "tests/assets/gltf/khronos/WaterBottle/reference_cpu_256.png",
        "reference_sha256": "922cc35e0c6420d2b3f8e533891291a9d4f9396697ae366f0b93de3c15973da4",
        "determinism": {
            "comparison_order": "independent-render-before-committed-reference",
            "repeat_count": 2,
            "byte_identical": true,
            "rgba8_sha256": [
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ]
        },
        "metrics": {
            "passed": true,
            "alpha_mismatch_pixels": 0,
            "rgb_chebyshev_tolerance": 4,
            "within_tolerance_fraction": 1.0,
            "rgb_rmse": 0.0
        },
        "mutations": mutations,
        "rust_test_output_observed": true,
        "command_record_path": "release-lanes/headless-cpu.commands.jsonl",
        "command_record_sha256": command_sha,
        "source_checksums": [
            {"path":"q01-waterbottle-cpu/live.png", "sha256":live_sha},
            {"path":"release-lanes/headless-cpu.commands.jsonl", "sha256":command_sha},
            {"path":"release-lanes/headless-cpu.log", "sha256":log_sha}
        ]
    });
    let payload_sha256 = crate::app::tests_19::release_payload_sha256(&result);
    result["payload_sha256"] = json!(payload_sha256);
    fs::write(
        q01_root.join("result.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&result).expect("Q01 fixture result serializes")
        ),
    )
    .expect("Q01 fixture result writes");
}

fn q01_mutation_provenance(name: &str) -> (&'static str, &'static str, u64, Vec<&'static str>) {
    match name {
        "flattened_chrome" => (
            "post-hoc-pixel",
            "output-rgba8",
            0,
            vec!["oracle-evaluator"],
        ),
        "wrong_material" => (
            "rendered-scene",
            "scene-mesh-material-before-prepare",
            1,
            vec![
                "gltf-import",
                "texture-resources-loaded",
                "scene-material-override",
                "cpu-material-resolution",
                "prepare",
                "render",
                "pbr-neutral-tonemap",
                "srgb8-output",
            ],
        ),
        "wrong_camera" => (
            "rendered-scene",
            "active-camera-transform-before-prepare",
            1,
            vec![
                "gltf-import",
                "texture-resources-loaded",
                "active-camera",
                "prepare",
                "render",
                "pbr-neutral-tonemap",
                "srgb8-output",
            ],
        ),
        _ => panic!("unknown Q01 mutation {name}"),
    }
}

fn write_q01_fixture_png(path: &Path, offset: u8) {
    let image = image::RgbaImage::from_fn(256, 256, |x, y| {
        image::Rgba([
            (x as u8).wrapping_add(offset),
            (y as u8).wrapping_add(offset),
            (x as u8).wrapping_add(y as u8).wrapping_add(offset),
            255,
        ])
    });
    image
        .save_with_format(path, image::ImageFormat::Png)
        .expect("Q01 fixture PNG writes");
}
