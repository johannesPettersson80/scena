use crate::app::prelude::*;

const PRESETS: &[&str] = &[
    "matte",
    "plastic",
    "metal",
    "rough_metal",
    "chrome",
    "brushed_steel",
    "clearcoat_plastic",
    "satin",
    "leather",
    "clear_glass",
    "frosted_glass",
    "rubber",
];

const NEIGHBOR_PAIRS: &[(&str, &str)] = &[
    ("metal", "rough_metal"),
    ("metal", "chrome"),
    ("chrome", "plastic"),
    ("clearcoat_plastic", "plastic"),
    ("clear_glass", "frosted_glass"),
    ("rubber", "plastic"),
];

#[test]
pub(crate) fn q02_release_content_accepts_bound_results_and_rejects_surface_tampering() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/q02-bound-results");
    let artifact_root = fixture_root.join("target/gate-artifacts");
    let _ = fs::remove_dir_all(&fixture_root);
    let headless = artifact_root.join("m9-platform/headless-cpu");
    fs::create_dir_all(&headless).expect("Q02 headless fixture dir");
    fs::write(
        headless.join("rendered-output.json"),
        r#"{
          "schema":"scena.m9.platform_render.v1",
          "lane":"headless-cpu",
          "backend":"Headless",
          "headless_cpu_proof":true,
          "static_gltf":{
            "proof_class":"cpu-camera-framed-non-ndc",
            "production_claim":true,
            "nonblack_pixels":42
          }
        }"#,
    )
    .expect("Q02 headless rendered output");
    crate::app::tests_24::write_q01_cpu_proof_fixture(
        &artifact_root,
        "0123456789abcdef0123456789abcdef01234567",
    );
    crate::app::tests_19::write_q11_reference_stability_fixture(
        &artifact_root,
        "0123456789abcdef0123456789abcdef01234567",
        "linux",
        "x86_64",
    );
    let mut browser_probe =
        crate::app::tests_30::required_webgpu_fixture("DiscreteGpu", "fixture discrete gpu", 1, 42);
    browser_probe["results"]
        .as_array_mut()
        .expect("Q02 M6 results array")
        .push(json!({"backend":"webgl2","status":"passed","pixels":{"nonblack":42}}));
    fs::write(
        artifact_root.join("m6-rust-wasm-renderer-probe.json"),
        serde_json::to_string_pretty(&browser_probe).expect("Q02 M6 fixture serializes"),
    )
    .expect("Q02 browser M6 fixture");
    write_q02_release_proof_fixtures(&fixture_root, "0123456789abcdef0123456789abcdef01234567");

    for lane in [
        "headless-cpu",
        "linux-webgl2-chromium",
        "linux-webgpu-chromium",
    ] {
        assert!(
            release_lane_content_ok(&fixture_root, lane)
                .unwrap_or_else(|error| panic!("Q02 {lane} content validates: {error}")),
            "Q02 {lane} content should accept complete bound evidence"
        );
    }

    let mutations = [
        (
            "target/gate-artifacts/round-e-cpu-material-proof.json",
            "\"sha256\": \"",
            "\"sha256\": \"f",
            "headless-cpu",
        ),
        (
            "target/gate-artifacts/round-e-cloudflare-material-proof.json",
            "\"passed_reference_delta\": true",
            "\"passed_reference_delta\": false",
            "linux-webgl2-chromium",
        ),
        (
            "target/gate-artifacts/round-e-webgpu-material-proof/result.json",
            "\"source\": \"renderer-owned-gpu-copy\"",
            "\"source\": \"canvas-readback\"",
            "linux-webgpu-chromium",
        ),
    ];
    for (relative, needle, replacement, lane) in mutations {
        let path = fixture_root.join(relative);
        let original = fs::read_to_string(&path).expect("Q02 mutation artifact reads");
        assert!(
            original.contains(needle),
            "Q02 mutation needle exists: {needle}"
        );
        fs::write(&path, original.replacen(needle, replacement, 1)).expect("Q02 mutation writes");
        assert!(
            !release_lane_content_ok(&fixture_root, lane).expect("Q02 tampered content check runs"),
            "Q02 {lane} content must reject tampering"
        );
        fs::write(&path, original).expect("Q02 mutation restores");
    }
}

pub(crate) fn write_q02_release_proof_fixtures(root: &Path, commit: &str) {
    let artifact_root = root.join("target/gate-artifacts");
    fs::create_dir_all(&artifact_root).expect("Q02 fixture artifact root");
    let source_path = root.join("q02-fixture-source.txt");
    fs::write(&source_path, b"Q02 fixture source\n").expect("Q02 fixture source writes");
    let source_sha = sha256_hex(&source_path).expect("Q02 fixture source hash");

    let cpu_frame = "target/gate-artifacts/round-e-cpu-material-proof/live-frame.png";
    write_fixture_png(&root.join(cpu_frame));
    write_material_result(
        root,
        "target/gate-artifacts/round-e-cpu-material-proof.json",
        "scena.q02.round_e_cpu_material_proof.v1",
        "live-cpu-round-e-shared-threshold-evaluation",
        "live-cpu-headless",
        cpu_frame,
        commit,
        &source_sha,
        json!({"live_renderer":"Renderer::headless fixture"}),
    );
    let cpu_manifest = artifact_root.join("round-e-cpu-material-proof/live-cpu-frame.json");
    fs::write(
        cpu_manifest,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "schema":"scena.q02.live_cpu_material_frame.v1",
                "producer":"cargo test --test examples_visual_proof q02_live_cpu_round_e_showcase_emits_shared_evaluator_frame -- --exact",
                "commit_sha":commit,
                "timestamp_unix_seconds":current_unix_seconds(),
                "source_checksums":[{"path":"q02-fixture-source.txt","sha256":source_sha}],
            }))
            .expect("Q02 CPU manifest serializes")
        ),
    )
    .expect("Q02 CPU manifest writes");

    let webgl_frame = "target/gate-artifacts/round-e-cloudflare-material-proof/canvas.png";
    write_fixture_png(&root.join(webgl_frame));
    for preset in PRESETS {
        write_fixture_png(
            &artifact_root.join(format!("round-e-cloudflare-material-proof/{preset}.png")),
        );
    }
    write_material_result(
        root,
        "target/gate-artifacts/round-e-cloudflare-material-proof.json",
        "scena.q02.round_e_webgl2_material_proof.v1",
        "round-e-cloudflare-material-proof",
        "live-webgl2-chromium",
        webgl_frame,
        commit,
        &source_sha,
        json!({"webgl_fixture":true}),
    );

    let webgpu_frame = "target/gate-artifacts/round-e-webgpu-material-proof/live-frame.png";
    write_fixture_png(&root.join(webgpu_frame));
    write_material_result(
        root,
        "target/gate-artifacts/round-e-webgpu-material-proof/result.json",
        "scena.q02.round_e_webgpu_material_proof.v1",
        "required-live-webgpu-round-e-shared-threshold-evaluation",
        "live-webgpu-chromium",
        webgpu_frame,
        commit,
        &source_sha,
        json!({
            "backend":"WebGpu",
            "renderer_readback":{"source":"renderer-owned-gpu-copy"}
        }),
    );
}

#[allow(clippy::too_many_arguments)]
fn write_material_result(
    root: &Path,
    relative: &str,
    schema: &str,
    proof_class: &str,
    surface: &str,
    live_frame: &str,
    commit: &str,
    source_sha: &str,
    extra: Value,
) {
    let mut materials = serde_json::Map::new();
    for preset in PRESETS {
        let mut material = json!({});
        if surface == "live-webgl2-chromium" {
            material = json!({
                "crop_path":format!(
                    "target/gate-artifacts/round-e-cloudflare-material-proof/{preset}.png"
                ),
                "reference_path":format!("tests/visual/references/round_e/{preset}.png"),
                "reference_delta_gate":"hard",
                "passed_reference_delta":true,
            });
        }
        materials.insert((*preset).to_string(), material);
    }
    let frame_sha = sha256_hex(&root.join(live_frame)).expect("Q02 fixture frame hash");
    let mut value = json!({
        "schema":schema,
        "status":"passed",
        "proof_class":proof_class,
        "commit_sha":commit,
        "timestamp_unix_seconds":current_unix_seconds(),
        "source_checksums":[{"path":"q02-fixture-source.txt","sha256":source_sha}],
        "threshold_evaluator":{
            "proof_class":"round-e-shared-material-threshold-evaluator",
            "surface":surface,
        },
        "live_frame":{"path":live_frame,"sha256":frame_sha},
        "per_material":materials,
        "neighbor_pairs":NEIGHBOR_PAIRS
            .iter()
            .map(|(left, right)| json!({"pair":[left,right],"passed":true}))
            .collect::<Vec<_>>(),
        "errors":[],
    });
    for (key, entry) in extra.as_object().expect("Q02 fixture extra object") {
        value[key] = entry.clone();
    }
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("Q02 result parent")).expect("Q02 result dir");
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).expect("Q02 result serializes")
        ),
    )
    .expect("Q02 result writes");
}

fn write_fixture_png(path: &Path) {
    fs::create_dir_all(path.parent().expect("Q02 PNG parent")).expect("Q02 PNG dir");
    fs::write(path, b"Q02 PNG fixture bytes\n").expect("Q02 PNG writes");
}
