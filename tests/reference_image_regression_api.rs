use scena::{
    Color, PerspectiveCamera, Primitive, ReferenceImage, ReferenceImageError,
    ReferenceImageTolerance, Renderer, Scene, Transform, Vec3, Vertex, regress,
    regress_with_tolerance,
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
fn reference_image_regression_compares_real_renderer_output_to_committed_golden() {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera becomes active");
    let color = Color::from_srgb_u8(80, 140, 220);
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-2.0, -2.0, 0.0),
                    color,
                },
                Vertex {
                    position: Vec3::new(4.0, -2.0, 0.0),
                    color,
                },
                Vertex {
                    position: Vec3::new(-2.0, 4.0, 0.0),
                    color,
                },
            ])],
            Transform::default(),
        )
        .expect("fullscreen triangle inserts");

    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let actual = ReferenceImage::from_rgba8(4, 4, renderer.frame_rgba8().to_vec())
        .expect("renderer output is rgba8");
    let expected = ReferenceImage::from_rgba8(
        4,
        4,
        vec![
            56, 130, 214, 255, 56, 130, 214, 255, 56, 130, 214, 255, 0, 0, 0, 255, 56, 130, 214,
            255, 56, 130, 214, 255, 56, 130, 214, 255, 56, 130, 214, 255, 56, 130, 214, 255, 56,
            130, 214, 255, 56, 130, 214, 255, 56, 130, 214, 255, 56, 130, 214, 255, 56, 130, 214,
            255, 56, 130, 214, 255, 56, 130, 214, 255,
        ],
    )
    .expect("committed renderer golden is rgba8");

    let report = regress(&actual, &expected).expect("renderer output matches committed golden");
    assert_eq!(report.total_pixels(), 16);
    assert_eq!(report.mismatched_pixels(), 0);
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
