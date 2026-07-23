use crate::app::prelude::*;
use crate::app::tests_11::{STAGE_TEST_COMMIT, write_stage_test_json};

pub(crate) fn stamp_stage_json_fixtures(dir: &Path, commit: &str, timestamp: u64) {
    for entry in fs::read_dir(dir).expect("stage fixture directory reads") {
        let path = entry.expect("stage fixture entry").path();
        if path.is_dir() {
            stamp_stage_json_fixtures(&path, commit, timestamp);
            continue;
        }
        if path.extension().and_then(OsStr::to_str) != Some("json")
            || path
                .components()
                .any(|component| component.as_os_str() == OsStr::new("reviews"))
        {
            continue;
        }
        let text = fs::read_to_string(&path).expect("JSON stage fixture reads");
        let mut value = serde_json::from_str::<Value>(&text).expect("JSON stage fixture parses");
        let object = value
            .as_object_mut()
            .expect("JSON stage fixture is an object");
        object
            .entry("schema".to_string())
            .or_insert_with(|| json!("scena.xtask.stage_fixture.v1"));
        object.insert(
            "producer".to_string(),
            json!("cargo test -p xtask stage release fixture"),
        );
        object.insert(
            "producing_command".to_string(),
            json!("cargo test -p xtask stage release fixture"),
        );
        object.insert("toolchain".to_string(), json!("rustc fixture-toolchain"));
        object.insert("profile".to_string(), json!("test-unoptimized"));
        object.insert("sample_count".to_string(), json!(1));
        object.insert(
            "source_checksums".to_string(),
            json!([{
                "path": "fixture-source",
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            }]),
        );
        object.insert("commit_sha".to_string(), json!(commit));
        object.insert("timestamp_unix_seconds".to_string(), json!(timestamp));
        let payload_sha256 = release_payload_sha256(&value);
        value
            .as_object_mut()
            .expect("JSON stage fixture is an object")
            .insert("payload_sha256".to_string(), json!(payload_sha256));
        fs::write(
            &path,
            format!(
                "{}\n",
                serde_json::to_string_pretty(&value).expect("JSON stage fixture serializes")
            ),
        )
        .expect("stamped JSON stage fixture writes");
    }
}

pub(crate) fn release_payload_sha256(value: &Value) -> String {
    let mut payload = value.clone();
    payload
        .as_object_mut()
        .expect("release payload is a JSON object")
        .remove("payload_sha256");
    let serialized = serde_json::to_vec(&payload).expect("release payload serializes");
    let normalized =
        serde_json::from_slice::<Value>(&serialized).expect("release payload normalizes");
    let bytes = serde_json::to_vec(&normalized).expect("normalized release payload serializes");
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn assert_m5_provenance_mutations_rejected(
    fixture_root: &Path,
    output_root: &Path,
    benchmark_source: &Path,
    commit: &str,
) {
    let mut missing_producer = serde_json::from_str::<Value>(
        &fs::read_to_string(benchmark_source).expect("benchmark fixture reads"),
    )
    .expect("benchmark fixture parses");
    {
        let object = missing_producer
            .as_object_mut()
            .expect("benchmark fixture is an object");
        object.remove("producer");
        object.remove("producing_command");
    }
    write_stage_test_json(benchmark_source, &missing_producer);
    let error = stage_release_artifacts_for_commit(fixture_root, output_root, commit)
        .expect_err("every source JSON must identify its producing command or test");
    assert!(
        error.contains("RELEASE-SOURCE-EVIDENCE")
            && error.contains("m5-benchmarks.json")
            && error.contains("producer"),
        "missing source producer must fail explicitly: {error}",
    );
    stamp_stage_json_fixtures(fixture_root, commit, current_unix_seconds());

    for field in [
        "toolchain",
        "profile",
        "producing_command",
        "sample_count",
        "payload_sha256",
    ] {
        let mut missing_field = serde_json::from_str::<Value>(
            &fs::read_to_string(benchmark_source).expect("benchmark fixture reads"),
        )
        .expect("benchmark fixture parses");
        missing_field
            .as_object_mut()
            .expect("benchmark fixture is an object")
            .remove(field);
        write_stage_test_json(benchmark_source, &missing_field);
        let error = stage_release_artifacts_for_commit(fixture_root, output_root, commit)
            .expect_err("M5 release artifacts must include complete measurement provenance");
        assert!(
            error.contains("RELEASE-SOURCE-EVIDENCE")
                && error.contains("m5-benchmarks.json")
                && error.contains(field),
            "missing M5 {field} must fail explicitly: {error}",
        );
        stamp_stage_json_fixtures(fixture_root, commit, current_unix_seconds());
    }

    let mut corrupted_payload = serde_json::from_str::<Value>(
        &fs::read_to_string(benchmark_source).expect("benchmark fixture reads"),
    )
    .expect("benchmark fixture parses");
    corrupted_payload["status"] = json!("corrupted-after-hash");
    assert_ne!(
        corrupted_payload["payload_sha256"],
        release_payload_sha256(&corrupted_payload),
        "mutation must invalidate the fixture payload hash",
    );
    write_stage_test_json(benchmark_source, &corrupted_payload);
    let error = stage_release_artifacts_for_commit(fixture_root, output_root, commit)
        .expect_err("M5 release artifacts must reject content changed after hashing");
    assert!(
        error.contains("RELEASE-SOURCE-EVIDENCE")
            && error.contains("m5-benchmarks.json")
            && error.contains("payload_sha256")
            && error.contains("does not match"),
        "corrupt M5 payload hash must fail explicitly: {error}",
    );
    stamp_stage_json_fixtures(fixture_root, commit, current_unix_seconds());
}

pub(crate) fn write_stage_waterbottle_result(fixture_root: &Path, png_path: &Path) {
    let png_sha256 = sha256_hex(png_path).expect("WaterBottle fixture hash");
    let diff_path = fixture_root.join("release-macos-metal/m8-real-asset/waterbottle_diff.png");
    fs::copy(png_path, &diff_path).expect("WaterBottle diff fixture writes");
    let diff_sha256 = sha256_hex(&diff_path).expect("WaterBottle diff fixture hash");
    let reference_sha256 = "4db449cdacf2340f8fa53937c28e5c4b5e2c7deaea73cbe0987dcd51eb93c751";
    let path = fixture_root.join("release-macos-metal/m8-real-asset/waterbottle_gpu_result.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("WaterBottle result fixture parent");
    }
    let value = json!({
        "schema": "scena.m8.waterbottle_gpu_result.v1",
        "status": "passed",
        "release_evidence": true,
        "test_name": "m8_real_asset_waterbottle_gpu_headline",
        "producer": "cargo test --test m8_real_asset_proof m8_real_asset_waterbottle_gpu_headline -- --exact",
        "commit_sha": STAGE_TEST_COMMIT,
        "timestamp_unix_seconds": current_unix_seconds(),
        "backend": "Metal",
        "adapter": "Apple Paravirtual device (Metal)",
        "adapter_key": {
            "schema": "scena.gpu_adapter_key.v1",
            "backend": "Metal",
            "vendor": 0,
            "device": 0,
            "device_type": "IntegratedGpu",
            "driver": "",
            "driver_info": ""
        },
        "adapter_expectation": waterbottle_macos_adapter_expectation_fixture(),
        "software_adapter": false,
        "skip_marker_observed": false,
        "fallback_observed": false,
        "rust_test_output_observed": true,
        "png_path": "m8-real-asset/waterbottle_gpu.png",
        "png_sha256": png_sha256,
        "reference_path": "tests/assets/gltf/khronos/WaterBottle/reference_512.png",
        "reference_sha256": reference_sha256,
        "diff_path": "m8-real-asset/waterbottle_diff.png",
        "diff_sha256": diff_sha256,
        "command_record_sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "metrics": {
            "nonblack_passed": true,
            "region_checks_passed": true,
            "color_family_histograms_passed": true,
            "reference_diff": "passed",
            "full_frame": {
                "compared_pixels": 262144,
                "within_tolerance_fraction": 1.0,
                "max_channel_delta": 0,
                "worst_region_bbox": [0, 0, 0, 0]
            },
            "thresholds": {
                "rgb_chebyshev_max": 16,
                "within_tolerance_fraction_min": 0.95
            }
        },
        "known_bad_mutations": [{
            "name": "horizontal_mirror",
            "rejected": true,
            "oracle": "full-frame-rgba8-chebyshev"
        }],
        "source_checksums": [
            {"path":"m8-real-asset/waterbottle_gpu.png", "sha256":png_sha256},
            {"path":"m8-real-asset/waterbottle_diff.png", "sha256":diff_sha256},
            {"path":"tests/assets/gltf/khronos/WaterBottle/reference_512.png", "sha256":reference_sha256},
            {"path":"release-lanes/macos-metal.commands.jsonl", "sha256":"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"}
        ]
    });
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("WaterBottle result fixture serializes")
        ),
    )
    .expect("WaterBottle result fixture writes");
}

fn waterbottle_macos_adapter_expectation_fixture() -> Value {
    json!({
        "schema": "scena.m8.waterbottle_adapter_expectation.v1",
        "profile_id": "github-macos-14-apple-paravirtual-metal-v1",
        "match_key": {
            "backend": "Metal",
            "vendor": 0,
            "device": 0,
            "device_type": "IntegratedGpu",
            "driver": "",
            "driver_info": ""
        },
        "owner": "scena-renderer-quality",
        "reviewed_at": "2026-07-22",
        "expires_at": "2026-10-31",
        "evidence_sha256": "2239bbb25313877e32dd5431fdae14660608257a4c11c60c383804fecbf6285f",
        "regions": [
            {"name":"cap_dome", "x":250, "y":70, "expected":[76,28,12], "tolerance":25},
            {"name":"cap_dome_left", "x":240, "y":70, "expected":[76,28,12], "tolerance":25},
            {"name":"upper_body", "x":249, "y":130, "expected":[145,126,43], "tolerance":25},
            {"name":"body_olive_mid", "x":249, "y":270, "expected":[150,131,44], "tolerance":25},
            {"name":"body_olive_low", "x":249, "y":330, "expected":[121,104,26], "tolerance":25},
            {"name":"label_metal_r", "x":270, "y":380, "expected":[30,20,6], "tolerance":25},
            {"name":"label_metal_l", "x":255, "y":380, "expected":[28,19,5], "tolerance":25}
        ]
    })
}

pub(crate) fn write_stage_q07_antialiasing_fixture(fixture_root: &Path) {
    let artifact_root = fixture_root.join("release-macos-metal/q07-antialiasing-effect");
    fs::create_dir_all(&artifact_root).expect("Q07 fixture directory");
    for mode in ["none", "fxaa", "msaa4"] {
        let mut ppm = b"P6\n2 2\n255\n".to_vec();
        ppm.extend_from_slice(&[0, 0, 0, 255, 255, 255, 96, 96, 96, 192, 192, 192]);
        fs::write(artifact_root.join(format!("{mode}.ppm")), ppm).expect("Q07 PPM fixture writes");
    }
    let baseline = json!({
        "intermediate_luma_pixels": 4_554,
        "hard_transition_count": 762,
        "squared_edge_energy": 1_000_000,
        "luma_range": 200,
    });
    let candidate = |intermediate, hard, energy| {
        json!({
            "intermediate_luma_pixels": intermediate,
            "hard_transition_count": hard,
            "squared_edge_energy": energy,
            "luma_range": 190,
        })
    };
    let source_checksums = ["none.ppm", "fxaa.ppm", "msaa4.ppm"]
        .map(|name| {
            json!({
                "path": format!("q07-antialiasing-effect/{name}"),
                "sha256": sha256_hex(&artifact_root.join(name)).expect("Q07 fixture hash"),
            })
        })
        .to_vec();
    write_stage_test_json(
        &artifact_root.join("result.json"),
        &json!({
            "schema": "scena.q07.antialiasing_effect.v1",
            "status": "passed",
            "release_evidence": true,
            "producer": "cargo test --test q07_antialiasing_effect q07_required_native_antialiasing_modes_have_pixel_effect -- --exact",
            "commit_sha": crate::app::tests_11::STAGE_TEST_COMMIT,
            "timestamp_unix_seconds": current_unix_seconds(),
            "fixture": "high-contrast-asymmetric-diagonal-v1",
            "adapter": {
                "name": "Apple GPU",
                "backend": "Metal",
                "device_type": "IntegratedGpu",
                "vendor": 0,
                "device": 0,
                "driver": "",
                "driver_info": ""
            },
            "baseline": {"mode":"none", "metrics":baseline},
            "modes": {
                "fxaa": {"status":"passed", "metrics":candidate(4_831, 500, 700_000)},
                "msaa4": {"status":"passed", "metrics":candidate(4_800, 550, 800_000)},
                "msaa8": {
                    "status":"degraded",
                    "reason_code":"UNSUPPORTED_SAMPLE_COUNT",
                    "requested":8,
                    "maximum":4
                }
            },
            "known_bad_mutations": [
                {"name":"no_op", "rejected":true},
                {"name":"blur_everything", "rejected":true}
            ],
            "source_checksums": source_checksums
        }),
    );
}

pub(crate) fn write_stage_q08_physical_parity_fixtures(fixture_root: &Path) {
    let artifact_root = fixture_root.join("release-macos-metal");
    for (suffix, test_name, source) in crate::app::release::REQUIRED_Q08_PARITY_RESULTS {
        let source_sha256 = sha256_hex(&repo_root().expect("workspace root").join(source))
            .expect("Q08 source fixture hash");
        write_stage_test_json(
            &artifact_root.join(suffix),
            &json!({
                "schema": "scena.q08.required_cpu_gpu_parity.v1",
                "status": "passed",
                "release_evidence": true,
                "proof_class": "physical-hardware-required",
                "test_name": test_name,
                "producer": format!("cargo test --test q08-fixture {test_name} -- --exact"),
                "commit_sha": STAGE_TEST_COMMIT,
                "timestamp_unix_seconds": current_unix_seconds(),
                "assertions_executed": 8,
                "adapter": {
                    "name": "Apple GPU",
                    "backend": "Metal",
                    "device_type": "IntegratedGpu",
                    "vendor": 0,
                    "device": 0,
                    "driver": "fixture",
                    "driver_info": "fixture"
                },
                "source_checksums": [{
                    "path": source,
                    "sha256": source_sha256
                }]
            }),
        );
    }
}

pub(crate) fn write_stage_q11_reference_stability_fixtures(fixture_root: &Path) {
    for (lane, os, arch) in [
        ("linux-native-vulkan", "linux", "x86_64"),
        ("macos-metal", "macos", "aarch64"),
        ("windows-dx12", "windows", "x86_64"),
    ] {
        write_q11_reference_stability_fixture(
            &fixture_root.join(format!("release-{lane}")),
            STAGE_TEST_COMMIT,
            os,
            arch,
        );
    }
}

pub(crate) fn write_q11_reference_stability_fixture(
    artifact_root: &Path,
    commit: &str,
    os: &str,
    arch: &str,
) {
    let render_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let mut result = json!({
        "schema": "scena.q11.reference_stability.v1",
        "status": "passed",
        "release_evidence": true,
        "test_name": "q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison",
        "producer": "cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
        "producing_command": "cargo test --test q01_waterbottle_cpu_reference q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison -- --exact",
        "toolchain": "rustc fixture-toolchain",
        "profile": "test-unoptimized",
        "sample_count": 2,
        "commit_sha": commit,
        "timestamp_unix_seconds": current_unix_seconds(),
        "os": os,
        "arch": arch,
        "backend": "Headless",
        "adapter": "software-rasterizer",
        "width": 256,
        "height": 256,
        "comparison_order": "independent-render-before-committed-reference",
        "repeat_count": 2,
        "byte_identical": true,
        "rgba8_sha256": [render_sha, render_sha],
        "metric_distribution": [
            {"passed":true,"within_tolerance_fraction":1.0,"rgb_rmse":0.0,"alpha_mismatch_pixels":0},
            {"passed":true,"within_tolerance_fraction":1.0,"rgb_rmse":0.0,"alpha_mismatch_pixels":0}
        ],
        "reference": {
            "sha256":"922cc35e0c6420d2b3f8e533891291a9d4f9396697ae366f0b93de3c15973da4"
        },
        "source_asset": {
            "sha256":"0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547"
        },
        "source_checksums": [{
            "path":"tests/assets/gltf/khronos/WaterBottle/WaterBottle.gltf",
            "sha256":"0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547"
        }]
    });
    result["payload_sha256"] = json!(release_payload_sha256(&result));
    write_stage_test_json(
        &artifact_root.join(format!("q11-reference-stability/{os}-{arch}.json")),
        &result,
    );
}

#[test]
pub(crate) fn waterbottle_result_finalizer_rejects_wrong_command_record() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-readiness-test/waterbottle-finalizer");
    let _ = fs::remove_dir_all(&fixture_root);
    let result_path =
        fixture_root.join("target/gate-artifacts/m8-real-asset/waterbottle_gpu_result.json");
    let png_path = fixture_root.join("target/gate-artifacts/m8-real-asset/waterbottle_gpu.png");
    let diff_path = fixture_root.join("target/gate-artifacts/m8-real-asset/waterbottle_diff.png");
    let reference_path =
        fixture_root.join("tests/assets/gltf/khronos/WaterBottle/reference_512.png");
    let command_path =
        fixture_root.join("target/gate-artifacts/release-lanes/macos-metal.commands.jsonl");
    let log_path = fixture_root.join("target/gate-artifacts/release-lanes/macos-metal.log");
    fs::create_dir_all(result_path.parent().expect("result parent")).expect("result directory");
    fs::create_dir_all(command_path.parent().expect("command parent")).expect("command directory");
    fs::create_dir_all(reference_path.parent().expect("reference parent"))
        .expect("reference directory");
    fs::write(&png_path, b"measured PNG bytes").expect("PNG fixture writes");
    fs::write(&diff_path, b"measured diff bytes").expect("diff fixture writes");
    fs::write(&reference_path, b"committed reference bytes").expect("reference fixture writes");
    let png_sha256 = sha256_hex(&png_path).expect("PNG fixture hash");
    let diff_sha256 = sha256_hex(&diff_path).expect("diff fixture hash");
    let reference_sha256 = sha256_hex(&reference_path).expect("reference fixture hash");
    fs::write(
        &result_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "status": "passed",
                "release_evidence": true,
                "adapter_key": {
                    "schema": "scena.gpu_adapter_key.v1",
                    "backend": "Metal",
                    "vendor": 0,
                    "device": 0,
                    "device_type": "IntegratedGpu",
                    "driver": "",
                    "driver_info": ""
                },
                "adapter_expectation": waterbottle_macos_adapter_expectation_fixture(),
                "png_sha256": png_sha256,
                "diff_sha256": diff_sha256,
                "reference_sha256": reference_sha256,
                "metrics": {"reference_diff": "passed"},
                "known_bad_mutations": [{"name":"horizontal_mirror", "rejected":true}]
            }))
            .expect("result fixture serializes")
        ),
    )
    .expect("result fixture writes");
    fs::write(
        &command_path,
        "{\"command\":\"cargo test --test unrelated\",\"status\":\"passed\"}\n",
    )
    .expect("wrong command fixture writes");
    fs::write(
        &log_path,
        "running 1 test\ntest m8_real_asset_waterbottle_gpu_headline ... ok\n\
         test result: ok. 1 passed; 0 failed\n",
    )
    .expect("command log fixture writes");

    fs::remove_file(&command_path).expect("command fixture removes");
    let missing_error = finalize_waterbottle_gpu_result(&fixture_root)
        .expect_err("missing command record must not finalize WaterBottle proof");
    assert!(
        missing_error.contains("missing") && missing_error.contains("commands"),
        "missing command record must fail explicitly: {missing_error}"
    );
    fs::write(
        &command_path,
        "{\"command\":\"cargo test --test unrelated\",\"status\":\"passed\"}\n",
    )
    .expect("wrong command fixture rewrites");

    let error = finalize_waterbottle_gpu_result(&fixture_root)
        .expect_err("wrong command record must not finalize WaterBottle proof");
    assert!(
        error.contains("exact") && error.contains("command"),
        "wrong command must fail for exact command identity: {error}",
    );

    fs::write(
        &command_path,
        "{\"command\":\"cargo test --test m8_real_asset_proof m8_real_asset_waterbottle_gpu_headline -- --exact\",\"status\":\"passed\",\"duration_ms\":1,\"failure_log_path\":\"target/gate-artifacts/release-lanes/macos-metal.log\"}\n",
    )
    .expect("exact command fixture writes");
    finalize_waterbottle_gpu_result(&fixture_root).expect("exact command finalizes proof");
    let finalized = fs::read_to_string(result_path).expect("finalized result reads");
    assert!(finalized.contains("\"rust_test_output_observed\": true"));
    assert!(finalized.contains("\"command_record_sha256\""));
}
