use crate::app::prelude::*;
use crate::app::tests_11::{STAGE_TEST_COMMIT, write_stage_test_json};

pub(crate) fn write_stage_review_fixture(
    fixture_root: &Path,
    reviewed_commit: &str,
) -> Vec<(PathBuf, Vec<u8>)> {
    let reviews_root = fixture_root.join("release-review-evidence/reviews");
    let mut expected = Vec::new();
    for (index, role) in REQUIRED_REVIEW_ROLES.iter().enumerate() {
        let relative = PathBuf::from(format!("reviews/{role}/{reviewed_commit}.md"));
        let body = format!(
            "---\nrole: {role}\nreviewed_commit: {reviewed_commit}\n\
             session_id: independent-review-{role}\ndate: 2026-07-16\n\
             reviewer_identity: github:independent-reviewer-{index}\n\
             reviewer_provenance: https://github.com/scena-rs/scena/actions/runs/{index}\n\
             blocker_status: clear\nfindings_count: 0\n---\n\n\
             # Independent {role} release review\n\nNo findings.\n"
        )
        .into_bytes();
        let path = fixture_root.join("release-review-evidence").join(&relative);
        fs::create_dir_all(path.parent().expect("review report parent")).expect("review dir");
        fs::write(&path, &body).expect("review report fixture");
        expected.push((relative, body));
    }

    let findings_relative = PathBuf::from("reviews/findings.json");
    let findings = format!(
        "{{\n  \"schema\": \"scena.release.findings.v1\",\n  \
         \"reviewed_commit\": \"{reviewed_commit}\",\n  \
         \"generated_at\": \"2026-07-16T00:00:00Z\",\n  \"findings\": []\n}}\n"
    )
    .into_bytes();
    fs::create_dir_all(&reviews_root).expect("reviews root");
    fs::write(reviews_root.join("findings.json"), &findings).expect("findings fixture");
    expected.push((findings_relative, findings));

    let signoff_relative = PathBuf::from("reviews/maintainer-signoff.toml");
    let required_roles = REQUIRED_REVIEW_ROLES
        .iter()
        .map(|role| format!("\"{role}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let report_hashes = expected
        .iter()
        .filter_map(|(relative, _)| {
            if relative.extension() != Some(OsStr::new("md")) {
                return None;
            }
            let role = relative.parent()?.file_name()?.to_str()?;
            let key = format!("{}_sha256", role.replace('-', "_"));
            let hash = sha256_hex(&fixture_root.join("release-review-evidence").join(relative))
                .expect("review hash");
            Some(format!("{key} = \"{hash}\""))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let findings_sha256 = sha256_hex(&reviews_root.join("findings.json")).expect("findings hash");
    let signoff = format!(
        "[maintainer]\nname = \"Independent Maintainer\"\n\
         identity = \"github:independent-maintainer\"\n\
         signed_commit = \"{reviewed_commit}\"\n\n\
         [reviews]\nall_clear = true\nfindings_register = \"reviews/findings.json\"\n\
         findings_sha256 = \"{findings_sha256}\"\n\
         required_roles = [{required_roles}]\n{report_hashes}\n\n\
         [approval]\ndecision = \"approve\"\n\
         approved_at = \"2026-07-16T00:00:00Z\"\n"
    )
    .into_bytes();
    fs::write(reviews_root.join("maintainer-signoff.toml"), &signoff)
        .expect("maintainer signoff fixture");
    expected.push((signoff_relative, signoff));
    expected
}

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
        "software_adapter": false,
        "skip_marker_observed": false,
        "fallback_observed": false,
        "rust_test_output_observed": true,
        "png_path": "m8-real-asset/waterbottle_gpu.png",
        "png_sha256": png_sha256,
        "command_record_sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        "metrics": {
            "nonblack_passed": true,
            "region_checks_passed": true,
            "color_family_histograms_passed": true,
            "reference_diff": "not-claimed"
        },
        "source_checksums": [
            {"path":"m8-real-asset/waterbottle_gpu.png", "sha256":png_sha256},
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

#[test]
pub(crate) fn release_readiness_rejects_automation_review_identity_without_staging() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-release-readiness-test/direct-review-bundle");
    let _ = fs::remove_dir_all(&fixture_root);
    write_stage_review_fixture(&fixture_root, STAGE_TEST_COMMIT);
    let artifact_root = fixture_root.join("release-review-evidence");
    let role = REQUIRED_REVIEW_ROLES[0];
    let report_path = artifact_root.join(format!("reviews/{role}/{STAGE_TEST_COMMIT}.md"));
    let report = fs::read_to_string(&report_path).expect("direct review fixture reads");
    fs::write(
        &report_path,
        report.replace(
            "github:independent-reviewer-0",
            "github:scena-release-automation",
        ),
    )
    .expect("direct automation review writes");
    let mut findings = Vec::new();

    check_release_review_artifacts(&artifact_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "RELEASE-REVIEWS-INTEGRITY" && finding.message.contains("automation")
        }),
        "release readiness must enforce reviewer identity without relying on staging: \
         {findings:?}",
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
    let command_path =
        fixture_root.join("target/gate-artifacts/release-lanes/macos-metal.commands.jsonl");
    let log_path = fixture_root.join("target/gate-artifacts/release-lanes/macos-metal.log");
    fs::create_dir_all(result_path.parent().expect("result parent")).expect("result directory");
    fs::create_dir_all(command_path.parent().expect("command parent")).expect("command directory");
    fs::write(&png_path, b"measured PNG bytes").expect("PNG fixture writes");
    let png_sha256 = sha256_hex(&png_path).expect("PNG fixture hash");
    fs::write(
        &result_path,
        format!("{{\"png_sha256\":\"{png_sha256}\"}}\n"),
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
