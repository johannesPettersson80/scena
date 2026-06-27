use super::*;
#[test]
fn exposure_quality_known_bad_fails_exact_black_crush_reason() {
    let width = 20;
    let height = 20;
    let mut bad = vec![0; (width * height * 4) as usize];
    for pixel in bad.chunks_exact_mut(4) {
        pixel[3] = 255;
    }
    let report =
        evaluate_render_quality_rgba8(quality_input(&bad, width, height, 0.2, false, 0.5), None);
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.code == "severe_black_crush"),
        "pitch-black fixture must fail with severe_black_crush: {report:#?}"
    );
}

#[test]
fn baseline_quality_fixtures_cover_blank_blown_out_and_tiny_codes() {
    let width = 16;
    let height = 16;
    let white = solid_frame(width, height, [255, 255, 255, 255]);
    let blown_out =
        evaluate_render_quality_rgba8(quality_input(&white, width, height, 0.5, false, 0.5), None);
    assert_has_code(&blown_out, "severe_blown_out");

    let gray = solid_frame(width, height, [128, 128, 128, 255]);
    let blank =
        evaluate_render_quality_rgba8(quality_input(&gray, width, height, 0.0, false, 0.5), None);
    assert_has_code(&blank, "blank_frame");

    let tiny =
        evaluate_render_quality_rgba8(quality_input(&gray, width, height, 0.5, true, 0.02), None);
    assert_has_code(&tiny, "subject_tiny_in_frame");

    let good =
        evaluate_render_quality_rgba8(quality_input(&gray, width, height, 0.5, false, 0.5), None);
    assert_lacks_code(&good, "severe_blown_out");
    assert_lacks_code(&good, "blank_frame");
    assert_lacks_code(&good, "subject_tiny_in_frame");
}

#[test]
fn opt_in_quality_fixtures_cover_exposure_contrast_and_noise_codes() {
    let width = 32;
    let height = 32;
    let exposure_expectation = SceneRecipeQualityExpectationV1 {
        profile: "product".to_owned(),
        exposure: Some(crate::SceneRecipeQualityExposureV1 {
            max_low_clip_fraction: Some(0.40),
            max_high_clip_fraction: Some(0.40),
        }),
        contrast: None,
        noise: None,
        text: None,
        line: None,
        geometry: None,
        reflection: None,
        area_light: None,
        grounding: None,
        depth_of_field: None,
    };
    let mut low_clip = solid_frame(width, height, [96, 96, 96, 255]);
    for pixel in low_clip
        .chunks_exact_mut(4)
        .take((width * height / 2) as usize)
    {
        pixel[0] = 0;
        pixel[1] = 0;
        pixel[2] = 0;
    }
    let low_report = evaluate_render_quality_rgba8(
        quality_input(&low_clip, width, height, 0.5, false, 0.5),
        Some(&exposure_expectation),
    );
    assert_has_code(&low_report, "low_clip_fraction_too_high");
    let mut high_clip = solid_frame(width, height, [96, 96, 96, 255]);
    for pixel in high_clip
        .chunks_exact_mut(4)
        .take((width * height / 2) as usize)
    {
        pixel[0] = 255;
        pixel[1] = 255;
        pixel[2] = 255;
    }
    let high_report = evaluate_render_quality_rgba8(
        quality_input(&high_clip, width, height, 0.5, false, 0.5),
        Some(&exposure_expectation),
    );
    assert_has_code(&high_report, "high_clip_fraction_too_high");
    let exposure_good_frame = solid_frame(width, height, [96, 96, 96, 255]);
    let exposure_good = evaluate_render_quality_rgba8(
        quality_input(&exposure_good_frame, width, height, 0.5, false, 0.5),
        Some(&exposure_expectation),
    );
    assert_lacks_code(&exposure_good, "low_clip_fraction_too_high");
    assert_lacks_code(&exposure_good, "high_clip_fraction_too_high");

    let contrast_expectation = SceneRecipeQualityExpectationV1 {
        profile: "product".to_owned(),
        exposure: None,
        contrast: Some(crate::SceneRecipeQualityContrastV1 {
            min_luminance_range: Some(0.25),
            min_sobel_energy: Some(0.05),
        }),
        noise: None,
        text: None,
        line: None,
        geometry: None,
        reflection: None,
        area_light: None,
        grounding: None,
        depth_of_field: None,
    };
    let flat_frame = solid_frame(width, height, [128, 128, 128, 255]);
    let flat = evaluate_render_quality_rgba8(
        quality_input(&flat_frame, width, height, 0.5, false, 0.5),
        Some(&contrast_expectation),
    );
    assert_has_code(&flat, "contrast_too_flat");
    assert_has_code(&flat, "edge_energy_too_low");
    let checker = checker_frame(width, height);
    let contrast_good = evaluate_render_quality_rgba8(
        quality_input(&checker, width, height, 0.5, false, 0.5),
        Some(&contrast_expectation),
    );
    assert_lacks_code(&contrast_good, "contrast_too_flat");
    assert_lacks_code(&contrast_good, "edge_energy_too_low");

    let noise_expectation = SceneRecipeQualityExpectationV1 {
        profile: "product".to_owned(),
        exposure: None,
        contrast: None,
        noise: Some(crate::SceneRecipeQualityNoiseV1 {
            max_outlier_fraction: Some(0.01),
        }),
        text: None,
        line: None,
        geometry: None,
        reflection: None,
        area_light: None,
        grounding: None,
        depth_of_field: None,
    };
    let mut noisy = solid_frame(width, height, [128, 128, 128, 255]);
    for y in (2..height - 2).step_by(4) {
        for x in (2..width - 2).step_by(4) {
            set_gray(&mut noisy, width, x, y, 255);
        }
    }
    let noisy_report = evaluate_render_quality_rgba8(
        quality_input(&noisy, width, height, 0.5, false, 0.5),
        Some(&noise_expectation),
    );
    assert_has_code(&noisy_report, "noise_outlier_fraction_too_high");
    let noise_good_frame = solid_frame(width, height, [128, 128, 128, 255]);
    let noise_good = evaluate_render_quality_rgba8(
        quality_input(&noise_good_frame, width, height, 0.5, false, 0.5),
        Some(&noise_expectation),
    );
    assert_lacks_code(&noise_good, "noise_outlier_fraction_too_high");
}

#[test]
fn reference_quality_fixtures_cover_abs_delta_e_and_ssim_metrics() {
    let width = 8;
    let height = 8;
    let expected = solid_frame(width, height, [80, 120, 160, 255]);
    let actual = solid_frame(width, height, [160, 80, 80, 255]);
    let changed = reference_quality_metrics(&actual, &expected, width, height)
        .expect("reference metrics compute for matching dimensions");
    assert!(changed.mean_abs_diff > 20.0, "{changed:#?}");
    assert!(changed.mean_delta_e2000 > 10.0, "{changed:#?}");
    assert!(changed.ssim < 0.99, "{changed:#?}");

    let identical = reference_quality_metrics(&expected, &expected, width, height)
        .expect("reference metrics compute for identical fixtures");
    assert_eq!(identical.mean_abs_diff, 0.0);
    assert_eq!(identical.mean_delta_e2000, 0.0);
    assert_eq!(identical.ssim, 1.0);
}

#[test]
fn grayscale_ssim_matches_reference_anchors() {
    let width = 8;
    let height = 8;
    let mut a = vec![0; (width * height * 4) as usize];
    for pixel in a.chunks_exact_mut(4) {
        pixel[0] = 128;
        pixel[1] = 128;
        pixel[2] = 128;
        pixel[3] = 255;
    }
    let mut b = a.clone();
    for pixel in b.chunks_exact_mut(4).take(8) {
        pixel[0] = 255;
        pixel[1] = 255;
        pixel[2] = 255;
    }
    let identical = ssim_grayscale(&a, &a, width, height).expect("SSIM computes");
    let changed = ssim_grayscale(&a, &b, width, height).expect("SSIM computes");
    assert_eq!(types::round3(identical), 1.0);
    assert!(
        changed < 0.9,
        "changed reference pair must produce lower SSIM before SSIM gates anything: {changed}"
    );
}
