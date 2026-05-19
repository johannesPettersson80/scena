use scena::{
    ReferenceImage, ReferenceImageError, ReferenceImageTolerance, regress, regress_with_tolerance,
};

#[test]
fn reference_image_regression_accepts_exact_rgba8_match() {
    let expected = ReferenceImage::from_rgba8(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255])
        .expect("expected reference image is valid");
    let actual = ReferenceImage::from_rgba8(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255])
        .expect("actual reference image is valid");

    let report = regress(&actual, &expected).expect("exact image match passes");

    assert!(report.passed());
    assert_eq!(report.total_pixels(), 2);
    assert_eq!(report.mismatched_pixels(), 0);
    assert_eq!(report.max_abs_diff(), 0);
}

#[test]
fn reference_image_regression_reports_tolerance_failure() {
    let expected = ReferenceImage::from_rgba8(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255])
        .expect("expected reference image is valid");
    let actual = ReferenceImage::from_rgba8(2, 1, vec![10, 20, 31, 255, 44, 50, 60, 255])
        .expect("actual reference image is valid");

    let error = regress_with_tolerance(
        &actual,
        &expected,
        ReferenceImageTolerance::new()
            .with_max_abs_diff(2)
            .with_max_mismatched_pixels(0),
    )
    .expect_err("second pixel exceeds the max channel tolerance");

    let ReferenceImageError::DiffExceeded(report) = error else {
        panic!("expected a diff-exceeded report");
    };
    assert!(!report.passed());
    assert_eq!(report.mismatched_pixels(), 1);
    assert_eq!(report.max_abs_diff(), 4);
    assert_eq!(report.tolerance().max_abs_diff(), 2);
}

#[test]
fn reference_image_regression_rejects_invalid_rgba_length() {
    let error =
        ReferenceImage::from_rgba8(2, 2, vec![0, 0, 0, 255]).expect_err("length is invalid");

    assert!(matches!(
        error,
        ReferenceImageError::InvalidRgbaLength {
            width: 2,
            height: 2,
            expected_len: 16,
            actual_len: 4,
        }
    ));
}

#[test]
fn reference_image_regression_rejects_dimension_mismatch() {
    let expected =
        ReferenceImage::from_rgba8(1, 2, vec![10, 20, 30, 255, 40, 50, 60, 255]).unwrap();
    let actual = ReferenceImage::from_rgba8(2, 1, vec![10, 20, 30, 255, 40, 50, 60, 255]).unwrap();

    let error = regress(&actual, &expected).expect_err("dimensions differ");

    assert!(matches!(
        error,
        ReferenceImageError::DimensionMismatch {
            actual_width: 2,
            actual_height: 1,
            expected_width: 1,
            expected_height: 2,
        }
    ));
}
