use scena::{AutoExposureConfig, Color, estimate_auto_exposure_from_linear_colors};

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
    assert_close(product.max_ev(), 0.65);
    assert_close(product.highlight_percentile(), 0.88);
    assert_close(product.highlight_target_luminance(), 0.70);

    assert!(indoor.max_ev() > product.max_ev());
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
