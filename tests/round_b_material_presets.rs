use scena::{Color, MaterialDesc, MaterialKind};

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
fn honest_material_presets_are_public_pbr_shortcuts() {
    let matte = MaterialDesc::matte(Color::BLUE);
    assert_eq!(matte.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(matte.base_color(), Color::BLUE);
    assert_close(matte.metallic_factor(), 0.0);
    assert_close(matte.roughness_factor(), 0.92);

    let plastic = MaterialDesc::plastic(Color::ORANGE);
    assert_eq!(plastic.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(plastic.base_color(), Color::ORANGE);
    assert_close(plastic.metallic_factor(), 0.0);
    assert_close(plastic.roughness_factor(), 0.42);

    let metal = MaterialDesc::metal(Color::LIGHT_GRAY);
    assert_eq!(metal.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(metal.base_color(), Color::LIGHT_GRAY);
    assert_close(metal.metallic_factor(), 1.0);
    assert_close(metal.roughness_factor(), 0.28);

    let rubber = MaterialDesc::rubber();
    assert_eq!(rubber.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(rubber.base_color(), Color::CHARCOAL);
    assert_close(rubber.metallic_factor(), 0.0);
    assert_close(rubber.roughness_factor(), 0.86);
}
