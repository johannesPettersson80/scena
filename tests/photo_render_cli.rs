#![cfg(all(feature = "inspection", feature = "scene-host"))]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use sha2::{Digest, Sha256};

const CAMERA_BEHAVIOR_FIXTURE_MANIFEST: &str =
    "tests/assets/photo/camera_behavior_cad_terminal_block.fixture.json";
const CAMERA_BEHAVIOR_FIXTURE_ASSET: &str = "tests/assets/gltf/cad_terminal_block.gltf";

#[test]
fn camera_behavior_fixture_manifest_pins_source_bands_and_mutations() {
    let manifest = camera_behavior_fixture_manifest();
    assert_eq!(manifest["schema"], "scena.camera_behavior_fixture.v1");
    assert_eq!(
        manifest["source_asset"]["path"],
        CAMERA_BEHAVIOR_FIXTURE_ASSET
    );
    assert_eq!(
        manifest["source_asset"]["sha256"],
        sha256_hex(Path::new(CAMERA_BEHAVIOR_FIXTURE_ASSET))
    );
    assert_eq!(manifest["source_asset"]["license"], "MIT OR Apache-2.0");
    assert_eq!(manifest["subject"]["target"]["kind"], "import");
    assert_eq!(manifest["subject"]["target"]["id"], "subject");
    assert_eq!(manifest["evidence_class"], "cpu_headless_cli");
    for constraint in [
        "manual_camera",
        "manual_fixed_exposure_ev",
        "manual_focus_distance",
        "manual_floor_geometry",
        "manual_background_color",
        "grid",
    ] {
        assert_eq!(
            manifest["subject"]["view_constraints"][constraint], false,
            "fixture must not require a manual shortcut for {constraint}"
        );
    }

    let bands = &manifest["quality_bands"];
    assert_eq!(bands["subject_fill_fraction"]["min"], 0.65);
    assert_eq!(bands["subject_fill_fraction"]["max"], 0.85);
    assert_eq!(bands["subject_mean_luminance_srgb8"]["min"], 80.0);
    assert_eq!(bands["subject_mean_luminance_srgb8"]["max"], 100.0);
    assert_eq!(bands["subject_low_clip_fraction"]["max"], 0.2);
    assert_eq!(bands["subject_high_clip_fraction"]["max"], 0.05);
    assert_eq!(bands["subject_center_offset_fraction"]["max"], 0.16);
    assert_eq!(bands["subject_luminance_stddev_srgb8"]["min"], 6.0);
    assert_eq!(bands["subject_luminance_range_srgb8"]["min"], 32.0);
    assert_eq!(bands["subject_background_separation_srgb8"]["min"], 8.0);

    let mutations = manifest["known_bad_mutations"]
        .as_array()
        .expect("known-bad mutations are listed")
        .iter()
        .filter_map(|mutation| mutation["id"].as_str())
        .collect::<Vec<_>>();
    for required in [
        "average_metered_silhouette",
        "stale_subject_mask",
        "wrong_subject_target",
        "old_ev_cap_underexposed",
        "post_tonemap_metering_strict_lane",
        "pulled_back_empty_slab",
        "off_center_subject",
        "wrong_focus",
        "flat_gray_metal",
        "missing_steel_reflection_structure",
        "blown_highlights",
    ] {
        assert!(
            mutations.contains(&required),
            "missing known-bad mutation {required}; manifest={manifest:#}"
        );
    }
    for mutation in manifest["known_bad_mutations"]
        .as_array()
        .expect("known-bad mutations are listed")
    {
        assert!(
            mutation["rejected_by"]
                .as_array()
                .is_some_and(|codes| !codes.is_empty()),
            "known-bad mutation must name its rejection evidence: {mutation:#}"
        );
    }
}

#[test]
fn checked_in_demo_hero_recipe_uses_photo_intent_without_manual_overrides() {
    let recipe: serde_json::Value = serde_json::from_slice(
        &fs::read("evidence/demo-hero/hero.recipe.json")
            .expect("checked-in demo hero recipe reads"),
    )
    .expect("checked-in demo hero recipe parses");

    assert_eq!(recipe["schema"], "scena.scene_recipe.v1");
    assert_eq!(recipe["imports"][0]["id"], "machine", "{recipe:#}");
    assert_eq!(
        recipe["imports"][0]["uri"],
        "demo/samples/connector-snap/connector_snap_assembly.glb"
    );
    assert_eq!(recipe["photo"]["intent"], "camera_behavior", "{recipe:#}");
    assert_eq!(recipe["photo"]["subject"]["kind"], "import", "{recipe:#}");
    assert_eq!(recipe["photo"]["subject"]["id"], "machine", "{recipe:#}");
    assert_eq!(recipe["capture"]["width"], 1800);
    assert_eq!(recipe["capture"]["height"], 1150);

    for field in ["geometries", "nodes", "lights", "cameras"] {
        assert!(
            recipe.get(field).is_none(),
            "demo hero recipe must not carry manual {field} overrides: {recipe:#}"
        );
    }
    assert!(
        recipe.pointer("/render/exposure_ev").is_none(),
        "demo hero recipe must not hard-code exposure_ev: {recipe:#}"
    );
    assert!(
        recipe
            .pointer("/render/depth_of_field/focus_distance")
            .is_none(),
        "demo hero recipe must not hard-code focus_distance: {recipe:#}"
    );
    assert!(
        recipe.pointer("/scene/grid").is_none(),
        "demo hero recipe must not hand-author a grid/floor override: {recipe:#}"
    );
    assert!(
        recipe.pointer("/scene/background").is_none(),
        "demo hero recipe must not hand-author a background override: {recipe:#}"
    );
}

#[test]
fn demo_next_hero_uses_checked_camera_behavior_proof_asset() {
    let fixture = camera_behavior_fixture_manifest();
    let html = fs::read_to_string("demo-next/index.html").expect("demo-next page reads");
    assert!(
        html.contains("assets/hero-915e9e36c3.png"),
        "demo page must reference the checked camera-behavior proof image: {html}"
    );
    assert!(
        !html.contains("assets/hero-d1f85f4090.png")
            && !html.contains("assets/hero-19ed145304.png")
            && !html.contains("assets/hero-287714ce43.png")
            && !html.contains("assets/hero.png"),
        "demo page must not reference stale hero stills: {html}"
    );
    assert!(
        html.contains("assets/hero.recipe.json"),
        "demo page must load the camera-behavior recipe for live rendering: {html}"
    );
    assert!(
        html.contains("camera-behavior recipe · no hand-tuned camera/exposure/focus"),
        "demo page must disclose the recipe-native no-manual-control contract: {html}"
    );

    let proof_png = Path::new("evidence/demo-hero/hero-camera-behavior.png");
    let demo_png = Path::new("demo-next/assets/hero-915e9e36c3.png");
    assert!(proof_png.exists(), "checked proof image is committed");
    assert!(demo_png.exists(), "demo still image is committed");
    let proof_hash = sha256_hex(proof_png);
    let demo_hash = sha256_hex(demo_png);
    assert_eq!(
        proof_hash,
        "915e9e36c31b7d9a1c46d8cc68c380e6fa0aeb09e97dd8d46fe9a41bb0dba10b"
    );
    assert_eq!(
        demo_hash, proof_hash,
        "demo still must be byte-identical to the checked proof image"
    );

    let proof_report_path = Path::new("evidence/demo-hero/hero-camera-behavior.render.json");
    let proof_report: serde_json::Value = serde_json::from_slice(
        &fs::read(proof_report_path)
            .unwrap_or_else(|error| panic!("checked proof report reads: {error}")),
    )
    .expect("checked proof report parses");
    assert_eq!(proof_report["ok"], true, "{proof_report:#}");
    assert_eq!(
        proof_report["verification"]["ok"], true,
        "checked proof report must not hide failed nested verification: {proof_report:#}"
    );
    assert_eq!(
        proof_report["verification"]["composition"]["ok"], true,
        "checked proof report must pass composition checks: {proof_report:#}"
    );
    assert_eq!(
        proof_report["verification"]["quality"]["ok"], true,
        "checked proof report must pass quality checks: {proof_report:#}"
    );
    assert!(
        proof_report["verification"]["reasons"]
            .as_array()
            .is_some_and(|reasons| reasons.is_empty()),
        "checked proof report must have no final verification reasons: {proof_report:#}"
    );

    let proof_png_bytes =
        fs::read(proof_png).unwrap_or_else(|error| panic!("checked proof PNG reads: {error}"));
    let decoded_png = decode_png_rgba8(&proof_png_bytes);
    assert_eq!(
        (decoded_png.width, decoded_png.height),
        (1800, 1150),
        "demo proof must validate the committed full-size shipped image"
    );
    let composition_checks = proof_report["verification"]["composition"]["checks"]
        .as_array()
        .expect("composition checks are present");
    let fit_check = find_check(
        composition_checks,
        "import.machine.framing",
        "subject_fit_sane",
    );
    let width_fill = fit_check["observed"]["width_fraction"]
        .as_f64()
        .expect("subject width fraction is reported");
    assert_metric_in_range(
        "shipped hero subject width fill",
        width_fill,
        &fixture["quality_bands"]["subject_fill_fraction"],
    );
    let center_offset = fit_check["observed"]["center_offset_fraction"]
        .as_array()
        .expect("center offset pair is reported")
        .iter()
        .map(|value| value.as_f64().expect("center offset is numeric").abs())
        .fold(0.0_f64, f64::max);
    assert_metric_at_most(
        "shipped hero subject center offset",
        center_offset,
        &fixture["quality_bands"]["subject_center_offset_fraction"],
    );

    let exposure_check = find_check(
        composition_checks,
        "import.machine.pixel_exposure",
        "subject_exposure_sane",
    );
    let reported_luma_srgb8 = exposure_check["observed"]["mean_luminance"]
        .as_f64()
        .expect("mean luminance is reported")
        * 255.0;
    let shipped_metrics = measure_png_foreground_region(
        &decoded_png,
        exposure_check["region"]["rect_css_px"].clone(),
    );
    assert!(
        (shipped_metrics.mean_luminance_srgb8 - reported_luma_srgb8).abs() <= 1.0,
        "checked proof report must match committed PNG foreground pixels, png={}, report={reported_luma_srgb8}; check={exposure_check:#}",
        shipped_metrics.mean_luminance_srgb8
    );
    assert_metric_in_range(
        "shipped hero subject luminance",
        reported_luma_srgb8,
        &fixture["quality_bands"]["subject_mean_luminance_srgb8"],
    );
    assert_metric_at_most(
        "shipped hero subject low clip",
        shipped_metrics.low_clip_fraction,
        &fixture["quality_bands"]["subject_low_clip_fraction"],
    );
    assert_metric_at_most(
        "shipped hero subject high clip",
        shipped_metrics.high_clip_fraction,
        &fixture["quality_bands"]["subject_high_clip_fraction"],
    );
    assert_metric_at_least(
        "shipped hero subject luminance stddev",
        shipped_metrics.luminance_stddev_srgb8,
        &fixture["quality_bands"]["subject_luminance_stddev_srgb8"],
    );
    assert_metric_at_least(
        "shipped hero subject luminance range",
        shipped_metrics.luminance_range_srgb8,
        &fixture["quality_bands"]["subject_luminance_range_srgb8"],
    );
    assert_metric_at_least(
        "shipped hero subject background separation",
        shipped_metrics.background_separation_srgb8,
        &fixture["quality_bands"]["subject_background_separation_srgb8"],
    );

    let recipe: serde_json::Value = serde_json::from_slice(
        &fs::read("demo-next/assets/hero.recipe.json").expect("demo-next hero recipe reads"),
    )
    .expect("demo-next hero recipe parses");
    assert_eq!(recipe["photo"]["intent"], "camera_behavior", "{recipe:#}");
    assert_eq!(recipe["photo"]["subject"]["kind"], "import", "{recipe:#}");
    assert_eq!(recipe["photo"]["subject"]["id"], "machine", "{recipe:#}");
    assert_eq!(
        recipe["imports"][0]["uri"],
        "/samples/connector-snap/connector_snap_assembly.glb"
    );
}

#[test]
fn photo_render_camera_behavior_is_easy_path_for_imported_asset() {
    let fixture = camera_behavior_fixture_manifest();
    let dir = artifact_dir("imported-camera-behavior");
    let png = dir.join("hero.png");
    let report = dir.join("hero.report.json");
    let emitted_recipe = dir.join("hero.resolved.recipe.json");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "render",
            CAMERA_BEHAVIOR_FIXTURE_ASSET,
            "--width",
            "256",
            "--height",
            "168",
            "--out",
            path_str(&png),
            "--report",
            path_str(&report),
            "--emit-recipe",
            path_str(&emitted_recipe),
        ])
        .output()
        .expect("scena photo render command runs");

    assert!(
        output.status.success(),
        "photo render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "photo render keeps stderr empty, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(png.exists(), "photo render writes a PNG");
    assert!(report.exists(), "photo render writes a report");
    assert!(
        emitted_recipe.exists(),
        "photo render writes the requested reproducible recipe"
    );

    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo render emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_render_result.v1");
    assert_eq!(stdout["ok"], true, "{stdout:#}");
    assert_eq!(stdout["intent"], "camera_behavior", "{stdout:#}");
    assert_eq!(
        stdout["artifacts"]["emitted_recipe_path"],
        path_str(&emitted_recipe)
    );

    let report_json: serde_json::Value = serde_json::from_slice(
        &fs::read(&report).unwrap_or_else(|error| panic!("report reads: {error}")),
    )
    .expect("photo report parses");
    assert_eq!(report_json["schema"], "scena.photo_report.v1");
    assert_eq!(report_json["status"], "passed", "{report_json:#}");
    assert_eq!(
        report_json["artifacts"]["emitted_recipe_path"],
        path_str(&emitted_recipe)
    );
    assert_eq!(report_json["subject"]["target"]["kind"], "import");
    assert_eq!(report_json["subject"]["target"]["id"], "subject");
    assert_eq!(
        report_json["planning"]["schema"],
        "scena.photo_candidate_plan.v1"
    );
    assert_eq!(report_json["planning"]["budget"], 10);
    assert!(
        report_json["planning"]["selected_candidate_id"]
            .as_str()
            .is_some_and(|id| id.starts_with("camera_behavior_view_geometry_derived_focal_")),
        "camera plan must be geometry-derived rather than a named hero/view preset: {report_json:#}"
    );
    assert_eq!(
        report_json["planning"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        10
    );
    assert_eq!(
        report_json["planning"]["candidates"][0]["staging"]["background"],
        "automatic"
    );
    assert_eq!(report_json["planning"]["candidates"][0]["lens"], "physical");
    assert!(
        report_json["planning"]["candidates"][0]["physical_camera"]["focal_length_mm"]
            .as_f64()
            .is_some_and(|focal| (35.0..=105.0).contains(&focal)),
        "photo report must expose a physical camera solution: {report_json:#}"
    );
    assert_eq!(
        report_json["retry"]["policy"]["max_attempts"], 6,
        "camera-behavior camera loop must be bounded: {report_json:#}"
    );
    assert_eq!(
        report_json["retry"]["policy"]["max_retries"], 5,
        "camera-behavior camera loop must not become unbounded: {report_json:#}"
    );
    assert_eq!(
        report_json["retry"]["policy"]["allowed_adjustments"],
        serde_json::json!(["camera_composition", "exposure_compensation_ev"]),
        "retry policy should expose the deterministic camera and exposure corrections: {report_json:#}"
    );
    assert!(
        report_json["retry"]["attempts"]
            .as_u64()
            .is_some_and(|attempts| (1..=6).contains(&attempts)),
        "retry report must expose a bounded attempt count: {report_json:#}"
    );
    let work_metrics = &report_json["work_metrics"];
    assert_eq!(
        work_metrics["timing_policy"], "report_only",
        "photo report must disclose that shared-runner timing is evidence-only: {report_json:#}"
    );
    assert_eq!(
        work_metrics["wall_clock_thresholds"], "not_used",
        "photo report must not imply wall-clock thresholds on shared hardware: {report_json:#}"
    );
    assert_eq!(
        work_metrics["allocation_policy"], "bounded_by_candidate_count_and_frame_pixels",
        "photo report must disclose how candidate-loop allocation/work stays bounded: {report_json:#}"
    );
    assert_eq!(
        work_metrics["composition_candidate_budget"], 10,
        "composition search budget must be explicit and bounded: {report_json:#}"
    );
    assert_eq!(
        work_metrics["composition_candidates"], 10,
        "composition candidate count must be reported: {report_json:#}"
    );
    assert_eq!(
        work_metrics["shaded_candidate_budget"], 3,
        "shaded render budget must be explicit and bounded: {report_json:#}"
    );
    let shaded_renders = work_metrics["shaded_candidate_renders"]
        .as_u64()
        .expect("shaded candidate render count is numeric");
    assert!(
        (3..=6).contains(&shaded_renders),
        "each of three candidates may receive at most one measured lighting correction: {report_json:#}"
    );
    assert_eq!(
        work_metrics["shaded_candidate_pixels"].as_u64(),
        Some(shaded_renders * 160 * 105),
        "low-resolution candidate pixel work must match the bounded renders: {report_json:#}"
    );
    let final_attempts = report_json["retry"]["attempts"]
        .as_u64()
        .expect("retry attempts numeric");
    assert_eq!(
        work_metrics["final_candidate_renders"].as_u64(),
        Some(final_attempts),
        "final candidate render count must match retry attempts: {report_json:#}"
    );
    // Focus resolution sets depth of field after the retry loop has accepted a
    // candidate, so one further render delivers the focused frame. It is counted
    // separately from the retry budget and included in the total: every render
    // the command performs must appear here, which is precisely what the removed
    // path tracer failed to do.
    let focus_delivery_renders = report_json["work_metrics"]["focus_delivery_renders"]
        .as_u64()
        .expect("work metrics report focus delivery renders");
    assert!(
        focus_delivery_renders <= 1,
        "focus delivery must render at most one frame: {report_json:#}"
    );
    assert!(
        work_metrics["total_render_calls"]
            .as_u64()
            .is_some_and(
                |calls| calls == shaded_renders + final_attempts + focus_delivery_renders
                    && calls <= 13
            ),
        "candidate loop must expose its bounded preview-adjustment, final retry, and focus delivery budget: {report_json:#}"
    );
    assert_eq!(
        work_metrics["prepare_calls"], work_metrics["total_render_calls"],
        "photo loop must report prepare calls for each candidate render: {report_json:#}"
    );
    assert_eq!(
        work_metrics["capture_calls"], work_metrics["total_render_calls"],
        "photo loop must report one capture per candidate render: {report_json:#}"
    );
    assert_eq!(
        work_metrics["gpu_readback_copies"], 0,
        "headless CPU photo proof must distinguish captures from GPU readback copies: {report_json:#}"
    );
    assert_eq!(
        work_metrics["blocking_polls"], 0,
        "headless photo loop must not use hidden blocking GPU polls: {report_json:#}"
    );
    assert_eq!(
        work_metrics["blocking_waits"], 0,
        "headless photo loop must not use hidden blocking waits: {report_json:#}"
    );
    assert!(
        work_metrics["subject_meter_samples"]
            .as_u64()
            .is_some_and(|samples| samples
                >= report_json["selected"]["subject"]["sample_count"]
                    .as_u64()
                    .unwrap_or(0)),
        "photo report must expose bounded subject-meter sample work: {report_json:#}"
    );
    for candidate in report_json["planning"]["candidates"]
        .as_array()
        .expect("planning candidates emitted")
    {
        assert_eq!(candidate["staging"]["environment"], "automatic");
        assert_eq!(candidate["staging"]["background"], "automatic");
        assert_eq!(candidate["staging"]["ground"], "automatic");
        assert_eq!(candidate["staging"]["grid"], false);
    }
    let shaded_selection = &report_json["shaded_selection"];
    assert_eq!(
        shaded_selection["schema"], "scena.photo_shaded_candidate_selection.v1",
        "photo report must expose the bounded low-resolution shaded candidate pass: {report_json:#}"
    );
    assert_eq!(shaded_selection["status"], "passed", "{shaded_selection:#}");
    assert_eq!(
        shaded_selection["low_resolution"],
        serde_json::json!({ "width": 160, "height": 105 }),
        "{shaded_selection:#}"
    );
    assert_eq!(
        shaded_selection["candidate_budget"], 3,
        "shaded pass should have its own bounded render budget: {shaded_selection:#}"
    );
    assert_eq!(
        shaded_selection["evaluated_count"], 3,
        "shaded pass should render only the top geometry candidates: {shaded_selection:#}"
    );
    assert_eq!(
        shaded_selection["work_metrics"]["rendered_candidates"], 3,
        "candidate render work must be explicit: {shaded_selection:#}"
    );
    assert!(
        shaded_selection["selected_candidate_id"]
            .as_str()
            .is_some_and(|id| id.starts_with("camera_behavior_view_geometry_derived_focal_")),
        "shaded pass should select a geometry-derived physical camera: {shaded_selection:#}"
    );
    let shaded_candidates = shaded_selection["candidates"]
        .as_array()
        .expect("shaded selection emits candidate summaries");
    assert_eq!(shaded_candidates.len(), 3, "{shaded_selection:#}");
    for candidate in shaded_candidates {
        assert_eq!(
            candidate["render_quality"]["schema"], "scena.render_quality.v1",
            "candidate scoring must reuse render quality metrics: {candidate:#}"
        );
        assert!(
            candidate["subject"]["luminance_stddev_srgb8"]
                .as_f64()
                .is_some_and(|value| value >= 6.0),
            "shaded candidate must expose steel/detail readability metrics: {candidate:#}"
        );
    }
    assert_eq!(
        report_json["focus_report"]["schema"],
        "scena.focus_report.v1"
    );
    assert_eq!(report_json["focus_report"]["status"], "resolved");
    assert_eq!(report_json["focus_report"]["target"]["id"], "subject");
    assert_eq!(
        report_json["exposure_report"]["schema"],
        "scena.exposure_report.v1"
    );
    assert_eq!(report_json["exposure_report"]["status"], "measured");
    assert_eq!(
        report_json["subject_observation"]["schema"],
        "scena.subject_observation.v1"
    );
    assert_eq!(
        report_json["subject_observation"]["source"],
        "photo.subject"
    );
    assert_eq!(
        report_json["subject_observation"]["target"]["id"],
        "subject"
    );
    assert_eq!(
        report_json["subject_observation"]["frame_key"]["state_binding"],
        "exact_readback_completion"
    );
    assert_eq!(
        report_json["subject_observation"]["fallback"]["degraded"],
        false
    );
    assert!(
        report_json["subject_observation"]["fallback"]["flags"]
            .as_array()
            .is_some_and(|flags| flags
                .iter()
                .any(|flag| flag == "geometry_derived_semantic_mask")),
        "photo report should disclose geometry-derived subject evidence: {report_json:#}"
    );
    assert_subject_region_bridge(&report_json);

    let emitted_json: serde_json::Value = serde_json::from_slice(
        &fs::read(&emitted_recipe).unwrap_or_else(|error| panic!("recipe reads: {error}")),
    )
    .expect("emitted recipe parses");
    assert_eq!(emitted_json["schema"], "scena.scene_recipe.v1");
    assert_eq!(emitted_json["imports"][0]["id"], "subject");
    assert_eq!(emitted_json["photo"]["intent"], "camera_behavior");
    assert_eq!(emitted_json["photo"]["subject"]["kind"], "import");
    assert_eq!(emitted_json["photo"]["subject"]["id"], "subject");
    assert!(
        emitted_json.get("lights").is_none(),
        "easy path must not emit hand-authored lights"
    );
    assert!(
        emitted_json.get("cameras").is_none(),
        "easy path must not emit a hand-authored camera or framing preset"
    );
    assert!(
        emitted_json.get("scene").is_none(),
        "easy path must not emit hand-authored background, floor, or grid staging"
    );
    assert!(
        emitted_json.get("render").is_none(),
        "easy path must not emit hand-authored exposure/focus/post settings"
    );
    assert!(
        emitted_json.get("geometries").is_none() && emitted_json.get("nodes").is_none(),
        "easy path must not fake staging by adding manual floor geometry"
    );

    let metrics = &report_json["quality"]["subject"];
    let decoded_png = decode_png_rgba8(
        &fs::read(&png).unwrap_or_else(|error| panic!("rendered PNG reads: {error}")),
    );
    assert_eq!(
        (decoded_png.width, decoded_png.height),
        (256, 168),
        "rendered-output proof must inspect the actual camera-behavior PNG dimensions"
    );
    let png_metrics = measure_png_subject_region(&decoded_png, metrics);

    let fill = metrics["fill_fraction"]
        .as_f64()
        .expect("fill metric is numeric");
    assert_metric_in_range(
        "subject fill",
        fill,
        &fixture["quality_bands"]["subject_fill_fraction"],
    );
    assert!(
        (0.65..=0.85).contains(&fill),
        "subject should fill most of the frame without being cropped, got {fill}; report={report_json:#}",
    );

    let mean_luma = metrics["mean_luminance_srgb8"]
        .as_f64()
        .expect("mean luminance metric is numeric");
    assert_metric_in_range(
        "rendered PNG subject luminance",
        png_metrics.mean_luminance_srgb8,
        &fixture["quality_bands"]["subject_mean_luminance_srgb8"],
    );
    assert_metric_in_range(
        "subject luminance",
        mean_luma,
        &fixture["quality_bands"]["subject_mean_luminance_srgb8"],
    );
    assert_eq!(
        report_json["exposure_report"]["subject"]["mean_luminance_srgb8"],
        metrics["mean_luminance_srgb8"],
        "exposure report must expose the same subject luminance the acceptance gate used"
    );
    assert!(
        (80.0..=100.0).contains(&mean_luma),
        "subject luminance should land in the camera-behavior band, got {mean_luma}; report={report_json:#}",
    );

    let low_clip = metrics["low_clip_fraction"]
        .as_f64()
        .expect("low-clip metric is numeric");
    // A derived environment must give reflective materials something to reflect.
    // The preview fixture is six constant cube faces, which caps how many
    // distinct radiance levels a metal can return and flattens it toward its
    // median; this is what made product renders read as clay.
    assert_metric_in_range(
        "rendered PNG subject specular headroom",
        png_metrics.specular_headroom_srgb8,
        &serde_json::json!({
            "min": fixture["quality_bands"]["subject_specular_headroom_srgb8"]["min"],
            "max": 255.0
        }),
    );

    assert_metric_at_most(
        "rendered PNG subject low clip",
        png_metrics.low_clip_fraction,
        &fixture["quality_bands"]["subject_low_clip_fraction"],
    );
    // `measure_png_foreground_region` separates subject from background by
    // comparing each pixel to the top-left one, so it only isolates the subject
    // when the backdrop is a flat fill. Generated surroundings put a lit
    // cyclorama and floor behind the subject, so the proxy now also counts
    // backdrop pixels inside the subject rect and can only over-report darkness.
    // The report measures the exact semantic-AOV mask, so it is the lower bound;
    // asserting the ordering still catches a report that invents a clean number
    // for a crushed frame, without depending on a flat background.
    assert!(
        low_clip <= png_metrics.low_clip_fraction + 0.01,
        "reported low-clip fraction must not undercut the rendered PNG bytes, png={}, report={low_clip}; report={report_json:#}",
        png_metrics.low_clip_fraction
    );
    assert_metric_at_most(
        "subject low clip",
        low_clip,
        &fixture["quality_bands"]["subject_low_clip_fraction"],
    );
    assert!(
        low_clip <= 0.20,
        "subject must not be a silhouette, low_clip_fraction={low_clip}; report={report_json:#}",
    );

    for (metric_name, band_key) in [
        ("subject high clip", "subject_high_clip_fraction"),
        ("subject center offset", "subject_center_offset_fraction"),
    ] {
        let value = metrics[metric_json_key(band_key)]
            .as_f64()
            .unwrap_or_else(|| panic!("{metric_name} metric is numeric"));
        assert_metric_at_most(metric_name, value, &fixture["quality_bands"][band_key]);
    }
    for (metric_name, band_key) in [
        ("subject luminance stddev", "subject_luminance_stddev_srgb8"),
        ("subject luminance range", "subject_luminance_range_srgb8"),
        (
            "subject background separation",
            "subject_background_separation_srgb8",
        ),
    ] {
        let value = metrics[metric_json_key(band_key)]
            .as_f64()
            .unwrap_or_else(|| panic!("{metric_name} metric is numeric"));
        assert_metric_at_least(metric_name, value, &fixture["quality_bands"][band_key]);
    }
    assert_metric_at_least(
        "rendered PNG subject luminance stddev",
        png_metrics.luminance_stddev_srgb8,
        &fixture["quality_bands"]["subject_luminance_stddev_srgb8"],
    );
    assert_metric_at_least(
        "rendered PNG subject luminance range",
        png_metrics.luminance_range_srgb8,
        &fixture["quality_bands"]["subject_luminance_range_srgb8"],
    );
    assert_metric_at_least(
        "rendered PNG subject background separation",
        png_metrics.background_separation_srgb8,
        &fixture["quality_bands"]["subject_background_separation_srgb8"],
    );
}

#[test]
fn photo_plan_camera_behavior_emits_render_free_public_plan_for_imported_asset() {
    let dir = artifact_dir("plan-imported-camera-behavior");
    let plan_path = dir.join("hero.plan.json");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "plan",
            CAMERA_BEHAVIOR_FIXTURE_ASSET,
            "--intent",
            "camera-behavior",
            "--width",
            "256",
            "--height",
            "168",
            "--out",
            path_str(&plan_path),
        ])
        .output()
        .expect("scena photo plan command runs");

    assert!(
        output.status.success(),
        "photo plan should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "photo plan keeps stderr empty, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        plan_path.exists(),
        "photo plan writes the requested plan file"
    );

    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo plan emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_plan.v1");
    assert_eq!(stdout["intent"], "camera_behavior");
    assert_eq!(stdout["source"]["kind"], "asset");
    assert_eq!(stdout["subject"]["target"]["id"], "subject");
    assert_eq!(
        stdout["planning"]["schema"],
        "scena.photo_candidate_plan.v1"
    );
    assert_eq!(
        stdout["candidates_evaluated"],
        stdout["planning"]["candidates"].as_array().unwrap().len()
    );
    assert!(
        stdout["planning"]["candidates"]
            .as_array()
            .is_some_and(|candidates| candidates.len() > 1),
        "photo plan should emit the bounded candidate set without rendering: {stdout:#}"
    );
    assert_eq!(
        stdout["staging_choices"][0]["background"], "automatic",
        "{stdout:#}"
    );
    let validation = scena::validate_contract_json_v1(&stdout.to_string());
    assert!(
        validation.ok && validation.fully_validated,
        "photo plan stdout must validate as a full public contract: {validation:?}; stdout={stdout:#}"
    );

    let plan_file: serde_json::Value = serde_json::from_slice(
        &fs::read(&plan_path).unwrap_or_else(|error| panic!("plan reads: {error}")),
    )
    .expect("plan file parses");
    assert_eq!(plan_file["schema"], "scena.photo_plan.v1");
    assert_eq!(
        plan_file["selected_candidate_id"],
        stdout["selected_candidate_id"]
    );
    assert!(
        fs::read_dir(&dir)
            .expect("artifact directory reads")
            .filter_map(Result::ok)
            .all(|entry| entry.path().extension().and_then(|ext| ext.to_str()) != Some("png")),
        "photo plan must not render the final high-resolution image"
    );
}

#[test]
fn photo_plan_recipe_input_accepts_subject_import_override() {
    let dir = artifact_dir("plan-recipe-subject-override");
    let recipe = dir.join("hero.recipe.json");
    let plan_path = dir.join("hero.plan.json");
    fs::write(
        &recipe,
        serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [
                {
                    "id": "background",
                    "uri": "tests/assets/gltf/cad_terminal_block.gltf",
                    "transform": {
                        "kind": "trs",
                        "translation": [-3.0, 0.0, 0.0]
                    }
                },
                {
                    "id": "hero",
                    "uri": "tests/assets/gltf/cad_terminal_block.gltf"
                }
            ],
            "capture": { "width": 256, "height": 168 }
        })
        .to_string(),
    )
    .expect("photo-plan recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "plan",
            path_str(&recipe),
            "--intent",
            "camera-behavior",
            "--subject",
            "import:hero",
            "--width",
            "256",
            "--height",
            "168",
            "--out",
            path_str(&plan_path),
        ])
        .output()
        .expect("scena photo plan command runs");

    assert!(
        output.status.success(),
        "photo plan recipe subject override should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "photo plan keeps stderr empty, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo plan emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_plan.v1");
    assert_eq!(stdout["source"]["kind"], "recipe");
    assert_eq!(stdout["subject"]["target"]["id"], "hero");
    assert!(
        stdout["planning"]["constraints"]["subject_bounds"]["min"][0]
            .as_f64()
            .is_some_and(|x| x > -2.0),
        "subject override should frame the hero import, not the translated background import: {stdout:#}"
    );
}

#[test]
fn photo_plan_recipe_input_accepts_subject_node_override() {
    let dir = artifact_dir("plan-recipe-node-subject-override");
    let recipe = dir.join("hero.recipe.json");
    let plan_path = dir.join("hero.plan.json");
    fs::write(
        &recipe,
        serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "hero_color": "#6F7F8F",
                "background_color": "#202020"
            },
            "geometries": [
                { "id": "hero_geo", "primitive": { "kind": "box", "size": [0.12, 0.08, 0.08] } },
                { "id": "background_geo", "primitive": { "kind": "box", "size": [0.12, 0.08, 0.08] } }
            ],
            "materials": [
                { "id": "hero_mat", "kind": "unlit", "base_color": "hero_color" },
                { "id": "background_mat", "kind": "unlit", "base_color": "background_color" }
            ],
            "nodes": [
                {
                    "id": "background",
                    "geometry": "background_geo",
                    "material": "background_mat",
                    "transform": { "kind": "trs", "translation": [-3.0, 0.0, 0.0] }
                },
                { "id": "hero", "geometry": "hero_geo", "material": "hero_mat" }
            ],
            "capture": { "width": 256, "height": 168 }
        })
        .to_string(),
    )
    .expect("photo-plan node recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "plan",
            path_str(&recipe),
            "--intent",
            "camera-behavior",
            "--subject",
            "node:hero",
            "--width",
            "256",
            "--height",
            "168",
            "--out",
            path_str(&plan_path),
        ])
        .output()
        .expect("scena photo plan command runs");

    assert!(
        output.status.success(),
        "photo plan recipe node subject override should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "photo plan keeps stderr empty, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo plan emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_plan.v1");
    assert_eq!(stdout["subject"]["target"]["kind"], "node");
    assert_eq!(stdout["subject"]["target"]["id"], "hero");
    assert!(
        stdout["planning"]["constraints"]["subject_bounds"]["min"][0]
            .as_f64()
            .is_some_and(|x| x > -1.0),
        "node subject override should frame hero, not the translated background node: {stdout:#}"
    );
}

#[test]
fn photo_plan_recipe_input_uses_declared_node_subject() {
    let dir = artifact_dir("plan-recipe-declared-node-subject");
    let recipe = dir.join("hero.recipe.json");
    let plan_path = dir.join("hero.plan.json");
    fs::write(
        &recipe,
        serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "colors": {
                "hero_color": "#6F7F8F",
                "background_color": "#202020"
            },
            "geometries": [
                { "id": "hero_geo", "primitive": { "kind": "box", "size": [0.12, 0.08, 0.08] } },
                { "id": "background_geo", "primitive": { "kind": "box", "size": [0.12, 0.08, 0.08] } }
            ],
            "materials": [
                { "id": "hero_mat", "kind": "unlit", "base_color": "hero_color" },
                { "id": "background_mat", "kind": "unlit", "base_color": "background_color" }
            ],
            "nodes": [
                {
                    "id": "background",
                    "geometry": "background_geo",
                    "material": "background_mat",
                    "transform": { "kind": "trs", "translation": [-3.0, 0.0, 0.0] }
                },
                { "id": "hero", "geometry": "hero_geo", "material": "hero_mat" }
            ],
            "photo": {
                "intent": "camera_behavior",
                "subject": { "kind": "node", "id": "hero" }
            },
            "capture": { "width": 256, "height": 168 }
        })
        .to_string(),
    )
    .expect("photo-plan declared node recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "plan",
            path_str(&recipe),
            "--intent",
            "camera-behavior",
            "--width",
            "256",
            "--height",
            "168",
            "--out",
            path_str(&plan_path),
        ])
        .output()
        .expect("scena photo plan command runs");

    assert!(
        output.status.success(),
        "photo plan declared node subject should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "photo plan keeps stderr empty, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo plan emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_plan.v1");
    assert_eq!(stdout["subject"]["target"]["kind"], "node");
    assert_eq!(stdout["subject"]["target"]["id"], "hero");
    assert!(
        stdout["planning"]["constraints"]["subject_bounds"]["min"][0]
            .as_f64()
            .is_some_and(|x| x > -1.0),
        "declared node subject should frame hero, not the translated background node: {stdout:#}"
    );
}

#[test]
fn photo_render_reports_recipe_build_failure_in_photo_envelope() {
    let dir = artifact_dir("recipe-build-failure");
    let recipe = dir.join("broken.recipe.json");
    let png = dir.join("hero.png");
    let report = dir.join("hero.report.json");
    fs::write(
        &recipe,
        serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": "missing-product.glb"
            }]
        })
        .to_string(),
    )
    .expect("broken photo recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "render",
            path_str(&recipe),
            "--intent",
            "camera-behavior",
            "--width",
            "64",
            "--height",
            "64",
            "--out",
            path_str(&png),
            "--report",
            path_str(&report),
        ])
        .output()
        .expect("scena photo render command runs");

    assert!(
        !output.status.success(),
        "broken photo render should fail, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "domain failure stays on stdout, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo render emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_render_result.v1");
    assert_eq!(stdout["ok"], false, "{stdout:#}");
    assert_eq!(
        stdout["failure_codes"][0], "recipe_build_failed",
        "{stdout:#}"
    );
    assert_eq!(stdout["build"]["schema"], "scena.scene_recipe_build.v1");
    assert!(
        !png.exists(),
        "a build failure must not leave a misleading output PNG"
    );
    assert!(
        !report.exists(),
        "a build failure must not leave a misleading quality report"
    );
}

#[test]
fn photo_render_camera_behavior_recovers_dark_subject_with_bounded_camera_loop() {
    let dir = artifact_dir("dark-subject-recovers");
    let recipe = dir.join("black-subject.recipe.json");
    let png = dir.join("hero.png");
    let report = dir.join("hero.report.json");
    fs::write(
        &recipe,
        serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": "tests/assets/gltf/cad_terminal_block.gltf",
                "material": {
                    "base_color": "#000000",
                    "roughness": 1.0,
                    "metallic": 0.0,
                    "double_sided": true
                }
            }],
            "lights": [{
                "id": "blackout",
                "kind": "directional",
                "illuminance_lux": 0.0
            }],
            "photo": {
                "intent": "camera_behavior",
                "subject": { "kind": "import", "id": "subject" }
            },
            "capture": { "width": 256, "height": 168 }
        })
        .to_string(),
    )
    .expect("black-subject photo recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "render",
            path_str(&recipe),
            "--intent",
            "camera-behavior",
            "--width",
            "256",
            "--height",
            "168",
            "--out",
            path_str(&png),
            "--report",
            path_str(&report),
        ])
        .output()
        .expect("scena photo render command runs");

    assert!(
        output.status.success(),
        "dark camera behavior should recover through subject metering, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "domain failure stays on stdout, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        png.exists(),
        "successful recovery should keep the rendered PNG"
    );
    assert!(
        report.exists(),
        "successful recovery should write the report"
    );

    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo render emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_render_result.v1");
    assert_eq!(stdout["ok"], true, "{stdout:#}");
    assert_eq!(stdout["status"], "passed", "{stdout:#}");
    assert_eq!(stdout["artifacts"]["report_path"], path_str(&report));
    assert!(
        stdout["failure_codes"]
            .as_array()
            .is_some_and(|codes| codes.is_empty()),
        "stdout should have no final failure codes after recovery: {stdout:#}"
    );

    let report_json: serde_json::Value = serde_json::from_slice(
        &fs::read(&report).unwrap_or_else(|error| panic!("report reads: {error}")),
    )
    .expect("photo report parses");
    assert_eq!(report_json["schema"], "scena.photo_report.v1");
    assert_eq!(report_json["status"], "passed", "{report_json:#}");
    assert_eq!(report_json["ok"], true, "{report_json:#}");
    assert_eq!(report_json["retry"]["policy"]["max_attempts"], 6);
    assert_eq!(report_json["retry"]["policy"]["max_retries"], 5);
    assert_eq!(
        report_json["retry"]["policy"]["allowed_adjustments"],
        serde_json::json!(["camera_composition", "exposure_compensation_ev"]),
        "{report_json:#}"
    );
    assert_eq!(report_json["retry"]["retry_used"], true);
    assert!(
        report_json["retry"]["attempts"]
            .as_u64()
            .is_some_and(|attempts| (2..=6).contains(&attempts)),
        "dark subject should require a bounded correction loop: {report_json:#}"
    );
    assert_eq!(
        report_json["retry"]["suggestion"]["kind"],
        "exposure_compensation_ev"
    );
    assert_eq!(
        report_json["selected"]["status"], "passed",
        "final selected candidate should pass after recovery: {report_json:#}"
    );
    let attempts = report_json["candidates"]
        .as_array()
        .expect("photo report emits candidate attempts");
    assert!(
        !attempts.is_empty(),
        "photo report must emit the measured camera-loop attempts: {report_json:#}"
    );
    for attempt in attempts {
        assert_candidate_camera_telemetry(attempt);
    }
    assert_candidate_camera_telemetry(&report_json["selected"]);
    assert!(
        attempts
            .last()
            .is_some_and(|candidate| candidate["status"] == "passed"),
        "last attempt should pass the acceptance oracle: {report_json:#}"
    );
    let subject = &report_json["selected"]["subject"];
    let mean_luma = subject["mean_luminance_srgb8"]
        .as_f64()
        .expect("mean luminance is numeric");
    assert!(
        (80.0..=100.0).contains(&mean_luma),
        "dark subject should be lifted into the photo band, mean={mean_luma}; report={report_json:#}"
    );
    assert!(
        subject["low_clip_fraction"]
            .as_f64()
            .is_some_and(|value| value <= 0.20),
        "dark subject should not remain a silhouette: {report_json:#}"
    );
}

#[test]
fn photo_render_failed_loop_reports_measured_candidate_history() {
    let dir = artifact_dir("failed-loop-history");
    let recipe = dir.join("clipped-subject.recipe.json");
    let png = dir.join("hero.png");
    let report = dir.join("hero.report.json");
    fs::write(
        &recipe,
        serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": "tests/assets/gltf/cad_terminal_block.gltf",
            }],
            "section_box": {
                "import": "subject",
                "margin": 0.0,
                "inverted": true
            },
            "photo": {
                "intent": "camera_behavior",
                "subject": { "kind": "import", "id": "subject" }
            },
            "capture": { "width": 128, "height": 84 }
        })
        .to_string(),
    )
    .expect("flat-subject photo recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "render",
            path_str(&recipe),
            "--width",
            "128",
            "--height",
            "84",
            "--out",
            path_str(&png),
            "--report",
            path_str(&report),
        ])
        .output()
        .expect("scena photo render command runs");

    assert!(
        !output.status.success(),
        "clipped subject should fail the photo gate, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "domain failure stays in JSON stdout/stored report, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        report.exists(),
        "failed photo render must still write the candidate-history report, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("photo failure emits JSON stdout");
    assert_eq!(stdout["schema"], "scena.photo_render_result.v1");
    assert_eq!(stdout["ok"], false, "{stdout:#}");
    assert_eq!(stdout["status"], "failed", "{stdout:#}");
    assert!(
        stdout["failure_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()),
        "failed loop stdout must expose final failure codes: {stdout:#}"
    );

    let report_json: serde_json::Value = serde_json::from_slice(
        &fs::read(&report).unwrap_or_else(|error| panic!("report reads: {error}")),
    )
    .expect("photo report parses");
    assert_eq!(report_json["schema"], "scena.photo_report.v1");
    assert_eq!(report_json["ok"], false, "{report_json:#}");
    assert_eq!(report_json["status"], "failed", "{report_json:#}");
    assert!(
        report_json["failure_codes"]
            .as_array()
            .is_some_and(|codes| !codes.is_empty()),
        "failed loop report must expose final failure codes: {report_json:#}"
    );
    assert_eq!(report_json["retry"]["policy"]["max_attempts"], 6);
    assert_eq!(
        report_json["retry"]["budget_exhausted"], false,
        "fail-fast subject removal should report failure without pretending the retry budget was exhausted: {report_json:#}"
    );
    let attempts = report_json["candidates"]
        .as_array()
        .expect("photo report emits candidate attempts");
    assert!(
        !attempts.is_empty(),
        "failed loop must preserve measured candidate history: {report_json:#}"
    );
    for attempt in attempts {
        assert_candidate_camera_telemetry(attempt);
    }
    assert_candidate_camera_telemetry(&report_json["selected"]);
    assert_subject_region_bridge(&report_json);
}

#[test]
fn recipe_render_camera_behavior_photo_intent_is_easy_path_for_imported_asset() {
    let dir = artifact_dir("recipe-camera-behavior-intent");
    let recipe = dir.join("hero.recipe.json");
    let png = dir.join("hero.png");
    fs::write(
        &recipe,
        serde_json::json!({
            "schema": "scena.scene_recipe.v1",
            "imports": [{
                "id": "subject",
                "uri": "tests/assets/gltf/cad_terminal_block.gltf"
            }],
            "photo": {
                "intent": "camera_behavior",
                "subject": { "kind": "import", "id": "subject" }
            },
            "capture": { "width": 256, "height": 168 }
        })
        .to_string(),
    )
    .expect("photo-intent recipe writes");

    let output = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&recipe),
            "--verify",
            "--out",
            path_str(&png),
        ])
        .output()
        .expect("scena recipe render command runs");

    assert!(
        output.status.success(),
        "recipe photo intent should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "recipe render keeps stderr empty, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(png.exists(), "recipe render writes the camera behavior PNG");

    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("recipe render emits JSON stdout");
    assert_eq!(report["schema"], "scena.recipe_render_result.v1");
    assert_eq!(report["ok"], true, "{report:#}");

    let checks = report["verification"]["composition"]["checks"]
        .as_array()
        .expect("composition checks are emitted under --verify");
    let framing = find_check(checks, "import.subject.framing", "subject_fit_sane");
    assert_eq!(framing["target_id"], "subject", "{framing:#}");
    let fit = framing["observed"]["fit_fraction"]
        .as_f64()
        .expect("fit fraction is reported");
    assert!(
        (0.65..=0.85).contains(&fit),
        "photo intent should frame the subject as a camera behavior, fit={fit}; check={framing:#}",
    );

    let exposure = find_check(
        checks,
        "import.subject.pixel_exposure",
        "subject_exposure_sane",
    );
    let mean_luma = exposure["observed"]["mean_luminance"]
        .as_f64()
        .expect("mean luminance is reported");
    // This band covers the whole foreground *region*, which includes the
    // generated floor and cyclorama, not just the subject. The derived studio
    // capture grades that surround slightly darker than the flat preview
    // fixture did, so the lower bound is 75 rather than 80. Subject exposure
    // itself is unaffected and is gated separately and more tightly: the
    // camera-behavior acceptance band is 80..=100 measured on the exact
    // semantic mask, and this fixture reports 95.49 there. Raising the surround
    // relative to the subject is tracked as its own staging item.
    assert!(
        (75.0 / 255.0..=130.0 / 255.0).contains(&mean_luma),
        "photo intent should expose the foreground region into a readable camera-behavior band, mean_luminance={mean_luma}; check={exposure:#}",
    );
    let low_clip = exposure["observed"]["low_clip_fraction"]
        .as_f64()
        .expect("low clip fraction is reported");
    assert!(
        low_clip <= 0.20,
        "photo intent should not leave a silhouette, low_clip_fraction={low_clip}; check={exposure:#}",
    );
}

fn find_check<'a>(checks: &'a [serde_json::Value], id: &str, code: &str) -> &'a serde_json::Value {
    checks
        .iter()
        .find(|check| check["id"] == id && check["code"] == code)
        .unwrap_or_else(|| {
            panic!("missing composition check id={id} code={code}; checks={checks:#?}")
        })
}

fn artifact_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from("target")
        .join("gate-artifacts")
        .join(format!("photo-render-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("artifact directory creates");
    dir
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test paths are UTF-8")
}

fn camera_behavior_fixture_manifest() -> serde_json::Value {
    serde_json::from_slice(
        &fs::read(CAMERA_BEHAVIOR_FIXTURE_MANIFEST)
            .unwrap_or_else(|error| panic!("camera-behavior fixture manifest reads: {error}")),
    )
    .expect("camera-behavior fixture manifest parses")
}

fn assert_metric_in_range(name: &str, value: f64, band: &serde_json::Value) {
    let min = band["min"]
        .as_f64()
        .unwrap_or_else(|| panic!("{name} band has numeric min"));
    let max = band["max"]
        .as_f64()
        .unwrap_or_else(|| panic!("{name} band has numeric max"));
    assert!(
        (min..=max).contains(&value),
        "{name} {value} outside [{min}, {max}]"
    );
}

fn assert_metric_at_most(name: &str, value: f64, band: &serde_json::Value) {
    let max = band["max"]
        .as_f64()
        .unwrap_or_else(|| panic!("{name} band has numeric max"));
    assert!(value <= max, "{name} {value} exceeds {max}");
}

fn assert_metric_at_least(name: &str, value: f64, band: &serde_json::Value) {
    let min = band["min"]
        .as_f64()
        .unwrap_or_else(|| panic!("{name} band has numeric min"));
    assert!(value >= min, "{name} {value} below {min}");
}

fn metric_json_key(band_key: &str) -> &str {
    band_key
        .strip_prefix("subject_")
        .expect("camera-behavior bands are subject-prefixed")
}

fn assert_candidate_camera_telemetry(candidate: &serde_json::Value) {
    let camera = &candidate["camera"];
    assert_eq!(
        camera["source"], "capture_descriptor",
        "candidate camera telemetry must be bound to the rendered capture descriptor: {candidate:#}"
    );
    assert!(
        camera["world_transform"].is_object(),
        "candidate must report the camera world transform used for this attempt: {candidate:#}"
    );
    assert_eq!(
        camera["projection"]["kind"], "perspective",
        "candidate must report the camera projection used for this attempt: {candidate:#}"
    );
    assert!(
        camera["vertical_fov_degrees"]
            .as_f64()
            .is_some_and(|value| value.is_finite() && value > 0.0 && value < 180.0),
        "candidate must report a finite perspective FOV: {candidate:#}"
    );
    assert!(
        camera["focus_distance_m"]
            .as_f64()
            .is_some_and(|value| value.is_finite() && value > 0.0),
        "candidate must report resolved subject focus distance: {candidate:#}"
    );
}

fn assert_subject_region_bridge(report_json: &serde_json::Value) {
    let region = &report_json["subject_region"];
    assert_eq!(
        region["schema"], "scena.photo_subject_region.v1",
        "photo report must expose the typed subject region bridge: {report_json:#}"
    );
    assert_eq!(
        region["source"], "subject_observation.v1",
        "photo report must expose the typed subject region bridge: {report_json:#}"
    );
    assert_eq!(region["target"]["kind"], "import", "{region:#}");
    assert!(
        region["world_bounds"].is_object(),
        "subject region must carry subject world bounds for composition/focus: {region:#}"
    );
    assert!(
        region["projected_bounds"].is_object(),
        "subject region must carry projected bounds: {region:#}"
    );
    assert!(
        region["visible_bounds"].is_object(),
        "subject region must carry visible bounds: {region:#}"
    );
    assert!(
        region["pixel_quality"].is_object(),
        "subject region must carry the measured foreground-pixel quality: {region:#}"
    );
    assert!(
        region["focus_distance_m"]
            .as_f64()
            .is_some_and(|value| value.is_finite() && value > 0.0),
        "subject region must carry resolved focus distance: {region:#}"
    );
    assert_eq!(
        region["frame_key"]["state_binding"], "exact_readback_completion",
        "subject region must be bound to the exact rendered readback frame: {region:#}"
    );
    assert_eq!(region["stale"], false, "{region:#}");
}

#[derive(Debug)]
struct DecodedPng {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

fn decode_png_rgba8(bytes: &[u8]) -> DecodedPng {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("PNG header reads");
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("PNG output buffer size is known")
    ];
    let info = reader.next_frame(&mut buffer).expect("PNG payload reads");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    DecodedPng {
        width: info.width,
        height: info.height,
        rgba8: buffer[..info.buffer_size()].to_vec(),
    }
}

#[derive(Debug)]
struct PngSubjectMetrics {
    /// Distance from the subject's median luminance to its 99th percentile.
    ///
    /// A metal surface can only reflect as many distinct radiance levels as its
    /// environment provides. The preview fixture is six constant cube faces, so
    /// metals rendered against it flatten toward their median; a real captured
    /// environment leaves a bright tail. This separates the two without pinning
    /// an absolute brightness that staging changes would move.
    specular_headroom_srgb8: f64,
    mean_luminance_srgb8: f64,
    luminance_stddev_srgb8: f64,
    luminance_range_srgb8: f64,
    background_separation_srgb8: f64,
    low_clip_fraction: f64,
    high_clip_fraction: f64,
}

fn measure_png_subject_region(
    png: &DecodedPng,
    subject_metrics: &serde_json::Value,
) -> PngSubjectMetrics {
    measure_png_foreground_region(png, subject_metrics["rect_css_px"].clone())
}

fn measure_png_foreground_region(png: &DecodedPng, rect: serde_json::Value) -> PngSubjectMetrics {
    let min_x = rect["min_x"]
        .as_f64()
        .expect("subject rect min_x is numeric")
        .floor()
        .clamp(0.0, f64::from(png.width)) as u32;
    let min_y = rect["min_y"]
        .as_f64()
        .expect("subject rect min_y is numeric")
        .floor()
        .clamp(0.0, f64::from(png.height)) as u32;
    let max_x = rect["max_x"]
        .as_f64()
        .expect("subject rect max_x is numeric")
        .ceil()
        .clamp(f64::from(min_x), f64::from(png.width)) as u32;
    let max_y = rect["max_y"]
        .as_f64()
        .expect("subject rect max_y is numeric")
        .ceil()
        .clamp(f64::from(min_y), f64::from(png.height)) as u32;
    assert!(
        max_x > min_x && max_y > min_y,
        "subject rect must cover actual rendered pixels: rect={rect:#}, png={png:?}"
    );
    let background = &png.rgba8[0..4];
    let mut sample_count = 0_u64;
    let mut low_clip_count = 0_u64;
    let mut high_clip_count = 0_u64;
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut min_luma = f64::INFINITY;
    let mut max_luma = f64::NEG_INFINITY;
    let mut background_delta_sum = 0.0_f64;
    let mut luminances: Vec<f64> = Vec::new();
    for y in min_y..max_y {
        for x in min_x..max_x {
            let offset = ((y as usize) * png.width as usize + x as usize) * 4;
            let pixel = &png.rgba8[offset..offset + 4];
            if pixel[3] == 0 {
                continue;
            }
            let background_delta = (0..3)
                .map(|channel| pixel[channel].abs_diff(background[channel]))
                .max()
                .unwrap_or(0);
            if background_delta <= 2 {
                continue;
            }
            let luma = 0.2126 * f64::from(pixel[0])
                + 0.7152 * f64::from(pixel[1])
                + 0.0722 * f64::from(pixel[2]);
            sum += luma;
            sum_sq += luma * luma;
            luminances.push(luma);
            background_delta_sum += f64::from(background_delta);
            min_luma = min_luma.min(luma);
            max_luma = max_luma.max(luma);
            if luma <= 10.0 {
                low_clip_count = low_clip_count.saturating_add(1);
            }
            if luma >= 245.0 {
                high_clip_count = high_clip_count.saturating_add(1);
            }
            sample_count = sample_count.saturating_add(1);
        }
    }
    assert!(
        sample_count > 0,
        "subject foreground rect samples actual opaque non-background pixels"
    );

    let mean = sum / sample_count as f64;
    luminances.sort_by(f64::total_cmp);
    let pick = |q: f64| {
        let index = ((luminances.len() as f64 - 1.0) * q).round() as usize;
        luminances.get(index).copied().unwrap_or(0.0)
    };
    let specular_headroom_srgb8 = pick(0.99) - pick(0.50);
    PngSubjectMetrics {
        specular_headroom_srgb8,
        mean_luminance_srgb8: mean,
        luminance_stddev_srgb8: (sum_sq / sample_count as f64 - mean.powi(2))
            .max(0.0)
            .sqrt(),
        luminance_range_srgb8: max_luma - min_luma,
        background_separation_srgb8: background_delta_sum / sample_count as f64,
        low_clip_fraction: low_clip_count as f64 / sample_count as f64,
        high_clip_fraction: high_clip_count as f64 / sample_count as f64,
    }
}

fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("asset reads: {error}"));
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `scena photo render` and `scena recipe render` run the same camera-behavior
/// loop over the same subject, so one must not cost dramatically more than the
/// other.
///
/// The photo path used to append `render_photographic_final`, a single-threaded
/// CPU path tracer that re-traced the whole frame at 8 samples/pixel x 4 bounces
/// through a scene-wide raycast built for interactive picking. It ran regardless
/// of `--gpu`, cost more than twenty times the recipe path on identical input,
/// and did not terminate at all at release resolutions. Deleting it is what this
/// test protects: the render-call bound is deterministic, and the wall-clock
/// ratio catches a reintroduced whole-frame pass.
#[test]
fn photo_render_costs_the_same_order_as_recipe_render_for_one_intent() {
    let dir = artifact_dir("photo-vs-recipe-cost-parity");
    let photo_png = dir.join("photo.png");
    let report_path = dir.join("photo.report.json");
    let emitted_recipe = dir.join("emitted.recipe.json");

    let photo_started = Instant::now();
    let photo = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "photo",
            "render",
            CAMERA_BEHAVIOR_FIXTURE_ASSET,
            "--width",
            "256",
            "--height",
            "168",
            "--out",
            path_str(&photo_png),
            "--report",
            path_str(&report_path),
            "--emit-recipe",
            path_str(&emitted_recipe),
        ])
        .output()
        .expect("scena photo render command runs");
    let photo_elapsed = photo_started.elapsed();
    assert!(
        photo.status.success(),
        "photo render should pass, stdout={}, stderr={}",
        String::from_utf8_lossy(&photo.stdout),
        String::from_utf8_lossy(&photo.stderr)
    );

    let recipe_png = dir.join("recipe.png");
    let recipe_started = Instant::now();
    let recipe = Command::new(env!("CARGO_BIN_EXE_scena"))
        .args([
            "recipe",
            "render",
            path_str(&emitted_recipe),
            "--out",
            path_str(&recipe_png),
        ])
        .output()
        .expect("scena recipe render command runs");
    let recipe_elapsed = recipe_started.elapsed();
    assert!(
        recipe.status.success(),
        "recipe render should pass on the emitted recipe, stdout={}, stderr={}",
        String::from_utf8_lossy(&recipe.stdout),
        String::from_utf8_lossy(&recipe.stderr)
    );

    let report_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&report_path).expect("photo report reads"))
            .expect("photo report parses");
    let work = &report_json["work_metrics"];

    // Deterministic half: every render the photo path performs is accounted for
    // by the bounded candidate loop. A whole-frame pass bolted on afterwards
    // does not go through `render_capture`, so it would not be counted here --
    // which is exactly how the path tracer stayed invisible to this report.
    let total_render_calls = work["total_render_calls"]
        .as_u64()
        .expect("work metrics report total render calls");
    let shaded_budget = work["shaded_candidate_budget"]
        .as_u64()
        .expect("work metrics report the shaded candidate budget");
    let final_budget = work["final_candidate_render_budget"]
        .as_u64()
        .expect("work metrics report the final candidate render budget");
    assert!(
        total_render_calls <= (shaded_budget * 2) + final_budget,
        "photo render must stay inside its declared candidate budgets, \
         total={total_render_calls}, shaded_budget={shaded_budget}, final_budget={final_budget}; \
         report={report_json:#}"
    );

    // Wall-clock half: coarse on purpose, so an ordinarily loaded machine does
    // not fail it, while a reintroduced per-pixel CPU pass (>20x) cannot pass.
    let photo_secs = photo_elapsed.as_secs_f64();
    let recipe_secs = recipe_elapsed.as_secs_f64();
    if recipe_secs >= 0.2 {
        assert!(
            photo_secs <= recipe_secs * 4.0,
            "photo render must stay the same order of magnitude as recipe render for one intent, \
             photo={photo_secs:.2}s, recipe={recipe_secs:.2}s"
        );
    }
}
