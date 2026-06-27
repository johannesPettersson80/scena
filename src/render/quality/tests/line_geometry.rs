use super::*;
#[test]
fn line_quality_known_bad_fails_exact_reason_and_good_passes() {
    let width = 64;
    let height = 32;
    let region = RenderQualityRegion::full_frame(width, height);
    let expectation = SceneRecipeQualityLineV1 {
        min_intermediate_edge_fraction: Some(0.05),
        max_straightness_error: Some(0.08),
    };
    let bad = aliased_line_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/line-known-bad-aliased.ppm",
        &bad,
        width,
        height,
    );
    let bad_checks =
        evaluate_line_region_quality("bad-line", &bad, width, height, region, expectation);
    assert!(
        bad_checks
            .iter()
            .any(|check| check.code == "line_missing_antialiasing"
                && check.status == RenderQualityStatusV1::Failed),
        "hard 1px line fixture must fail exact line_missing_antialiasing: {bad_checks:#?}"
    );

    let good = antialiased_line_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/line-known-good-antialiased.ppm",
        &good,
        width,
        height,
    );
    let good_checks =
        evaluate_line_region_quality("good-line", &good, width, height, region, expectation);
    assert_no_failed_checks(
        &good_checks,
        "known-good antialiased straight line should pass quality checks",
    );
}

#[test]
fn geometry_edge_quality_known_bad_fails_exact_reason_and_good_passes() {
    let width = 32;
    let height = 24;
    let region = RenderQualityRegion::full_frame(width, height);
    let expectation = crate::SceneRecipeQualityGeometryV1 {
        min_intermediate_edge_fraction: Some(0.05),
    };

    let bad = hard_geometry_edge_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/geometry-edge-known-bad-hard.ppm",
        &bad,
        width,
        height,
    );
    let bad_checks = evaluate_geometry_region_quality(
        "bad-geometry-edge",
        &bad,
        width,
        height,
        region,
        expectation,
    );
    assert!(
        bad_checks
            .iter()
            .any(|check| check.code == "geometry_missing_antialiasing"
                && check.status == RenderQualityStatusV1::Failed),
        "hard 1-bit geometry edge fixture must fail exact geometry_missing_antialiasing: {bad_checks:#?}"
    );

    let good = antialiased_geometry_edge_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/geometry-edge-known-good-sampled.ppm",
        &good,
        width,
        height,
    );
    let good_checks = evaluate_geometry_region_quality(
        "good-geometry-edge",
        &good,
        width,
        height,
        region,
        expectation,
    );
    assert_no_failed_checks(
        &good_checks,
        "known-good sampled geometry edge should pass quality checks",
    );
}
