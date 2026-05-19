use scena::{Color, DirectionalLight, PointLight};

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
fn named_directional_light_presets_are_public_and_ordered() {
    let sun = DirectionalLight::sun();
    let key = DirectionalLight::key_light();
    let fill = DirectionalLight::fill_light();
    let rim = DirectionalLight::rim_light();

    assert_color_close(sun.color(), Color::from_kelvin(5600.0));
    assert!(sun.casts_shadows(), "sun preset should opt into shadows");
    assert!(
        sun.illuminance_lux() > key.illuminance_lux(),
        "sun should be stronger than the product-viewer key light"
    );

    assert_color_close(key.color(), Color::WHITE);
    assert_close(key.illuminance_lux(), 13_500.0);
    assert!(
        key.casts_shadows(),
        "key light owns the single default shadow"
    );

    assert_color_close(fill.color(), Color::COOL_WHITE);
    assert_close(fill.illuminance_lux(), 4_500.0);
    assert!(!fill.casts_shadows(), "fill must not add a second shadow");

    assert_color_close(rim.color(), Color::WARM_WHITE);
    assert_close(rim.illuminance_lux(), 3_500.0);
    assert!(!rim.casts_shadows(), "rim must not add a second shadow");
}

#[test]
fn named_point_light_presets_are_kelvin_tinted_and_range_limited() {
    let softbox = PointLight::softbox();
    let warm = PointLight::bulb_warm();
    let cool = PointLight::bulb_cool();

    assert_color_close(softbox.color(), Color::from_kelvin(5600.0));
    assert_eq!(softbox.range(), Some(4.0));
    assert!(softbox.intensity_candela() > warm.intensity_candela());

    assert_color_close(warm.color(), Color::from_kelvin(2700.0));
    assert_color_close(cool.color(), Color::from_kelvin(5600.0));
    assert!(
        warm.color().r > warm.color().b,
        "warm bulb should be visibly warmer than blue"
    );
    assert!(
        cool.color().b > warm.color().b,
        "cool bulb should carry more blue than warm bulb"
    );
    assert_eq!(warm.range(), Some(6.0));
    assert_eq!(cool.range(), Some(6.0));
}
