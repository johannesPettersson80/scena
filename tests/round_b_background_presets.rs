use scena::{Background, Color, Renderer};

fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= 1.0e-4,
        "expected {left} to be close to {right}"
    );
}

fn assert_color_close(left: Color, right: Color) {
    assert_close(left.r, right.r);
    assert_close(left.g, right.g);
    assert_close(left.b, right.b);
    assert_close(left.a, right.a);
}

#[test]
fn named_background_presets_map_to_public_colors() {
    assert_color_close(Background::Studio.color(), Color::STUDIO_BACKDROP);
    assert_color_close(Background::DarkStudio.color(), Color::CHARCOAL);
    assert_color_close(Background::NeutralGray.color(), Color::GRAY);
    assert_color_close(Background::White.color(), Color::WHITE);
    assert_color_close(Background::Black.color(), Color::BLACK);
    assert!(
        Background::Sky.color().b > Background::Sky.color().r,
        "sky background should read cooler than a neutral studio backdrop"
    );
    assert_close(Background::Transparent.color().a, 0.0);

    let custom = Color::from_linear_rgba(0.12, 0.34, 0.56, 0.78);
    assert_color_close(Background::Custom(custom).color(), custom);
}

#[test]
fn renderer_set_background_uses_named_scheme() {
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds");

    renderer.set_background(Background::DarkStudio);
    assert_color_close(renderer.background_color(), Color::CHARCOAL);

    renderer.set_background(Background::Custom(Color::ORANGE));
    assert_color_close(renderer.background_color(), Color::ORANGE);
}
