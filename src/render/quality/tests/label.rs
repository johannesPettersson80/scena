use super::*;
#[test]
fn label_quality_known_bad_fails_exact_reason_and_good_passes() {
    let width = 32;
    let height = 16;
    let region = RenderQualityRegion::full_frame(width, height);
    let expectation = SceneRecipeQualityTextV1 {
        min_ink_coverage: Some(0.10),
        max_ink_isolation: Some(0.01),
        min_intermediate_edge_fraction: Some(0.01),
        max_background_luminance_range: None,
        max_background_mean_delta: None,
    };
    let bad = eroded_label_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/label-known-bad-eroded.ppm",
        &bad,
        width,
        height,
    );
    let bad_checks =
        evaluate_label_region_quality("bad-label", &bad, width, height, region, expectation);
    assert!(
        bad_checks
            .iter()
            .any(|check| check.code == "label_ink_isolation"),
        "eroded label fixture must fail with label_ink_isolation: {bad_checks:#?}"
    );

    let good = antialiased_label_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/label-known-good-antialiased.ppm",
        &good,
        width,
        height,
    );
    let good_checks =
        evaluate_label_region_quality("good-label", &good, width, height, region, expectation);
    assert!(
        good_checks.is_empty(),
        "known-good antialiased label should pass quality checks: {good_checks:#?}"
    );
}

#[test]
fn label_quality_known_bad_gpu_eroded_fixture_fails_exact_reason() {
    let (rgba8, width, height) = read_ppm_fixture(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/assets/render-quality/label-known-bad-gpu-eroded.ppm"),
    );
    let region = RenderQualityRegion {
        kind: "label",
        handle: None,
        x: 95,
        y: 55,
        width: 230,
        height: 30,
    };
    let expectation = SceneRecipeQualityTextV1 {
        min_ink_coverage: Some(0.30),
        max_ink_isolation: Some(0.01),
        min_intermediate_edge_fraction: Some(0.01),
        max_background_luminance_range: None,
        max_background_mean_delta: None,
    };
    let checks =
        evaluate_label_region_quality("gpu-eroded", &rgba8, width, height, region, expectation);
    write_ppm_crop_artifact(
        "target/gate-artifacts/render-quality/label-known-bad-gpu-eroded-crop.ppm",
        &rgba8,
        width,
        region,
    );
    assert!(
        checks
            .iter()
            .any(|check| check.code == "label_ink_coverage_too_low"),
        "old GPU-eroded label fixture must fail exact label_ink_coverage_too_low: {checks:#?}"
    );
}

#[test]
fn label_quality_known_bad_bitmap_fixture_fails_antialiasing_reason() {
    let width = 80;
    let height = 28;
    let region = RenderQualityRegion::full_frame(width, height);
    let expectation = SceneRecipeQualityTextV1 {
        min_ink_coverage: Some(0.08),
        max_ink_isolation: Some(0.02),
        min_intermediate_edge_fraction: Some(0.01),
        max_background_luminance_range: None,
        max_background_mean_delta: None,
    };
    let bad = blocky_text_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/label-known-bad-bitmap.ppm",
        &bad,
        width,
        height,
    );
    let checks =
        evaluate_label_region_quality("bad-bitmap-label", &bad, width, height, region, expectation);
    assert!(
        checks
            .iter()
            .any(|check| check.code == "label_missing_antialiasing"),
        "old blocky bitmap-style label fixture must fail exact label_missing_antialiasing: {checks:#?}"
    );
}

#[test]
fn label_background_quality_known_bad_fails_exact_reason_and_good_passes() {
    let width = 48;
    let height = 24;
    let region = RenderQualityRegion::full_frame(width, height);
    let expected = [29, 39, 51];
    let expectation = SceneRecipeQualityTextV1 {
        min_ink_coverage: Some(0.0),
        max_ink_isolation: Some(1.0),
        min_intermediate_edge_fraction: Some(0.0),
        max_background_luminance_range: Some(0.02),
        max_background_mean_delta: Some(0.02),
    };

    let bad = nonuniform_label_background_fixture(width, height, expected);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/label-background-known-bad.ppm",
        &bad,
        width,
        height,
    );
    let bad_checks = evaluate_label_region_quality_with_background(
        "bad-label-background",
        &bad,
        width,
        height,
        region,
        expectation,
        Some(expected),
    );
    assert!(
        bad_checks
            .iter()
            .any(|check| check.code == "label_background_not_uniform"),
        "non-uniform label background fixture must fail exact label_background_not_uniform: {bad_checks:#?}"
    );

    let good = solid_label_background_fixture(width, height, expected);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/label-background-known-good.ppm",
        &good,
        width,
        height,
    );
    let good_checks = evaluate_label_region_quality_with_background(
        "good-label-background",
        &good,
        width,
        height,
        region,
        expectation,
        Some(expected),
    );
    assert!(
        good_checks.is_empty(),
        "flat authored label background should pass quality checks: {good_checks:#?}"
    );
}
