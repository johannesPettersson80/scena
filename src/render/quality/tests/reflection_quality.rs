use super::*;

#[test]
fn reflection_quality_known_bad_fireflies_fail_exact_reason_and_good_passes() {
    let width = 64;
    let height = 32;
    let region = RenderQualityRegion::full_frame(width, height);
    let expectation = crate::SceneRecipeQualityReflectionV1 {
        target: None,
        min_luminance_range: Some(0.12),
        min_sobel_energy: Some(0.02),
        min_chroma_range: Some(0.06),
        max_firefly_fraction: Some(0.005),
    };

    let bad = firefly_reflection_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/reflection-known-bad-fireflies.ppm",
        &bad,
        width,
        height,
    );
    let bad_checks = evaluate_reflection_region_quality(
        "bad-reflection",
        &bad,
        width,
        height,
        region,
        expectation.clone(),
    );
    assert!(
        bad_checks
            .iter()
            .any(|check| check.code == "reflection_firefly_outliers"
                && check.status == RenderQualityStatusV1::Failed),
        "isolated bright reflection specks must fail exact reflection_firefly_outliers: {bad_checks:#?}"
    );

    let good = structured_reflection_fixture(width, height);
    write_ppm_artifact(
        "target/gate-artifacts/render-quality/reflection-known-good-structured.ppm",
        &good,
        width,
        height,
    );
    let good_checks = evaluate_reflection_region_quality(
        "good-reflection",
        &good,
        width,
        height,
        region,
        expectation,
    );
    assert_no_failed_checks(
        &good_checks,
        "structured reflection without isolated bright specks should pass quality checks",
    );
}
