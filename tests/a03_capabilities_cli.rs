use std::collections::BTreeSet;
use std::process::{Command, Output};

const COMPILED_FEATURES: &[&str] = &[
    "agent",
    "browser_probe",
    "controls",
    "controls_web",
    "controls_winit",
    "demo_page",
    "hot_reload",
    "inspection",
    "khronos_samples",
    "ktx2",
    "material_library",
    "meshopt",
    "obj",
    "production_assets",
    "proof_harness",
    "scene_host",
    "viewer_element",
];

#[test]
fn static_capabilities_are_explicitly_no_device_and_json_alias_matches() {
    let report = run_json(&["capabilities"]);
    assert_eq!(report.status.code(), Some(0), "stderr={}", stderr(&report));
    let value = stdout_json(&report);
    assert_eq!(value["schema"], "scena.capability_report.v1");
    assert_eq!(value["probe"]["mode"], "static");
    assert_eq!(value["probe"]["status"], "static_no_device");
    assert_eq!(value["probe"]["source"], "compiled_backend_table");
    assert_eq!(value["probe"]["requested_backend"], "headless");
    assert!(value["probe"]["selected_backend"].is_null());
    assert!(value["adapter"].is_null());
    assert!(value["probe"]["device"].is_null());
    assert_eq!(
        value["probe"]["color_target"]["source"],
        "renderer_contract"
    );
    assert_eq!(value["probe"]["color_target"]["measured"], false);
    assert_eq!(
        value["probe"]["color_target"]["format"],
        value["capabilities"]["color_target_format"]
    );
    assert_eq!(value["probe"]["readback"]["status"], "not_probed");
    assert_eq!(value["probe"]["presentation"]["status"], "not_applicable");
    assert!(value["probe"]["probed_at_unix_ms"].is_null());
    assert!(value["probe"]["unavailable"].is_null());
    assert_eq!(
        value["capabilities"]["final_photo"], "error_if_required",
        "the static CPU table must not claim the GPU-only final-photo contract"
    );

    let explicit_json = run_json(&["capabilities", "--json"]);
    assert_eq!(
        explicit_json.status.code(),
        Some(0),
        "stderr={}",
        stderr(&explicit_json)
    );
    assert_eq!(stdout_json(&explicit_json), value);
}

#[test]
fn live_capabilities_are_measured_or_fail_closed_with_a_structured_reason() {
    let report = run_json(&["capabilities", "--live"]);
    let value = stdout_json(&report);
    assert_eq!(value["schema"], "scena.capability_report.v1");
    assert_eq!(value["probe"]["mode"], "live_adapter");
    assert_eq!(value["probe"]["requested_backend"], "headless_gpu");
    assert!(value["probe"]["probed_at_unix_ms"].as_u64().is_some());

    if report.status.success() {
        assert_eq!(value["probe"]["status"], "measured");
        assert_eq!(value["probe"]["source"], "live_wgpu_adapter");
        assert_eq!(value["probe"]["selected_backend"], "headless_gpu");
        assert!(value["adapter"].is_object());
        assert!(value["probe"]["device"].is_object());
        assert!(value["probe"]["device"]["features"].as_str().is_some());
        assert!(value["probe"]["device"]["limits"].is_object());
        assert_eq!(value["probe"]["color_target"]["measured"], true);
        assert_eq!(
            value["probe"]["color_target"]["source"],
            "adapter_format_features"
        );
        assert_eq!(
            value["probe"]["color_target"]["format"],
            value["capabilities"]["color_target_format"]
        );
        assert_nonempty_sample_counts(&value["probe"]["color_target"]);
        assert_nonempty_sample_counts(&value["probe"]["depth_target"]);
        assert_eq!(value["probe"]["readback"]["status"], "supported");
        assert_eq!(value["probe"]["presentation"]["status"], "not_probed");
        assert!(value["probe"]["unavailable"].is_null());
        assert_eq!(
            value["capabilities"]["final_photo"], "supported",
            "a live HeadlessGpu device is the native final-photo execution backend"
        );
    } else {
        assert_eq!(report.status.code(), Some(1), "stderr={}", stderr(&report));
        assert_eq!(value["probe"]["status"], "unavailable");
        assert_eq!(value["probe"]["source"], "live_wgpu_adapter_request");
        assert!(value["probe"]["selected_backend"].is_null());
        assert!(value["adapter"].is_null());
        assert!(value["probe"]["device"].is_null());
        assert!(value["probe"]["unavailable"]["code"].as_str().is_some());
        assert!(
            value["probe"]["unavailable"]["message"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
    }
}

#[test]
fn cli_version_reports_every_compiled_feature_that_changes_availability() {
    let output = run_json(&["--version"]);
    assert!(output.status.success(), "stderr={}", stderr(&output));
    let value = stdout_json(&output);
    let actual: BTreeSet<_> = value["features"]
        .as_object()
        .expect("features is an object")
        .keys()
        .map(String::as_str)
        .collect();
    let expected: BTreeSet<_> = COMPILED_FEATURES.iter().copied().collect();
    assert_eq!(actual, expected);
    assert_eq!(
        value["features"]["scene_host"],
        cfg!(feature = "scene-host")
    );
    assert_eq!(
        value["features"]["inspection"],
        cfg!(feature = "inspection")
    );
    assert_eq!(
        value["features"]["hot_reload"],
        cfg!(feature = "hot-reload")
    );
}

fn assert_nonempty_sample_counts(target: &serde_json::Value) {
    let counts = target["sample_counts"]
        .as_array()
        .expect("sample_counts is an array");
    assert!(!counts.is_empty());
    assert_eq!(counts[0], 1);
}

fn run_json(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scena"))
        .args(args)
        .output()
        .expect("scena command runs")
}

fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout must be JSON: {error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            stderr(output)
        )
    })
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
