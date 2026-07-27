use scena::{
    AutoExposureConfig, AutoExposureMeteringDomain, Color,
    estimate_auto_exposure_from_linear_colors, estimate_auto_exposure_from_srgb8,
};

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 1.0e-4,
        "expected {left} to be close to {right}"
    );
}

#[test]
fn named_auto_exposure_scenarios_are_public_and_ordered() {
    let product = AutoExposureConfig::product_studio();
    let indoor = AutoExposureConfig::indoor();
    let outdoor = AutoExposureConfig::outdoor();
    let mixed = AutoExposureConfig::mixed();

    assert_close(product.target_luminance(), 0.22);
    assert_close(product.min_ev(), -1.5);
    assert_close(product.max_ev(), 4.5);
    assert_close(product.highlight_percentile(), 0.88);
    assert_close(product.highlight_target_luminance(), 0.70);

    // A product still is a small subject in a large studio field, so its
    // foreground meter asks for several stops of lift. The previous ordering
    // assumed a product studio needs less headroom than an indoor scene; that
    // assumption clamped correctly-metered subjects to black silhouettes.
    assert!(product.max_ev() > indoor.max_ev());
    assert!(outdoor.min_ev() < indoor.min_ev());
    assert_eq!(mixed, AutoExposureConfig::default());
}

#[test]
fn scenario_presets_drive_different_ev_solutions() {
    let mut dark_product_scene = vec![Color::from_linear_rgb(0.018, 0.018, 0.018); 64];
    for pixel in dark_product_scene.iter_mut().skip(56) {
        *pixel = Color::from_linear_rgb(0.82, 0.82, 0.82);
    }

    let product = estimate_auto_exposure_from_linear_colors(
        &dark_product_scene,
        AutoExposureConfig::product_studio(),
    )
    .expect("product scene has valid luminance");
    assert!(
        product.exposure_ev() <= AutoExposureConfig::product_studio().max_ev(),
        "product preset must respect tight highlight-safe EV range"
    );

    let dim_indoor = [Color::from_linear_rgb(0.06, 0.06, 0.06); 16];
    let indoor =
        estimate_auto_exposure_from_linear_colors(&dim_indoor, AutoExposureConfig::indoor())
            .expect("indoor scene has valid luminance");
    let outdoor =
        estimate_auto_exposure_from_linear_colors(&dim_indoor, AutoExposureConfig::outdoor())
            .expect("outdoor comparison scene has valid luminance");
    assert!(
        outdoor.exposure_ev() <= indoor.exposure_ev(),
        "outdoor preset should not brighten dim samples more aggressively than indoor"
    );
}

#[test]
fn auto_exposure_compensation_offsets_metered_ev_without_replacing_metering() {
    let colors = [Color::from_linear_rgb(0.09, 0.09, 0.09); 32];
    let base_config = AutoExposureConfig::mixed();
    let compensated_config = base_config.with_compensation_ev(0.5);

    let base = estimate_auto_exposure_from_linear_colors(&colors, base_config)
        .expect("base auto exposure meters");
    let compensated = estimate_auto_exposure_from_linear_colors(&colors, compensated_config)
        .expect("compensated auto exposure meters");

    assert_close(compensated_config.compensation_ev(), 0.5);
    assert_close(compensated.base_exposure_ev(), base.exposure_ev());
    assert_close(compensated.compensation_ev(), 0.5);
    assert_close(compensated.exposure_ev(), base.exposure_ev() + 0.5);
}

#[test]
fn auto_exposure_reports_metering_domain_for_strict_camera_behavior_evidence() {
    let colors = [Color::from_linear_rgb(0.09, 0.09, 0.09); 32];
    let linear = estimate_auto_exposure_from_linear_colors(&colors, AutoExposureConfig::mixed())
        .expect("linear scene frame meters");
    assert_eq!(
        linear.metering_domain(),
        AutoExposureMeteringDomain::SceneLinearPreTonemap
    );
    assert_eq!(
        linear
            .metering_domain()
            .strict_camera_behavior_rejection_code(),
        None
    );

    let mut rgba8 = Vec::with_capacity(colors.len() * 4);
    for _ in 0..colors.len() {
        rgba8.extend_from_slice(&[84, 84, 84, 255]);
    }
    let encoded = estimate_auto_exposure_from_srgb8(&rgba8, AutoExposureConfig::mixed())
        .expect("encoded frame meters");
    assert_eq!(
        encoded.metering_domain(),
        AutoExposureMeteringDomain::EncodedOutputFeedback
    );
    assert_eq!(
        encoded
            .metering_domain()
            .strict_camera_behavior_rejection_code(),
        Some("metering_domain_encoded_output_feedback")
    );
}
