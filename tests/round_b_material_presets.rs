use scena::{AlphaMode, Color, MaterialDesc, MaterialKind};

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
    assert_close(metal.roughness_factor(), 0.42);

    let rubber = MaterialDesc::rubber();
    assert_eq!(rubber.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(rubber.base_color(), Color::CHARCOAL);
    assert_close(rubber.metallic_factor(), 0.0);
    assert_close(rubber.roughness_factor(), 0.86);
}

#[test]
fn expanded_material_presets_use_only_backed_material_lanes() {
    let rough_metal = MaterialDesc::rough_metal(Color::LIGHT_GRAY);
    assert_eq!(rough_metal.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(rough_metal.base_color(), Color::LIGHT_GRAY);
    assert_close(rough_metal.metallic_factor(), 1.0);
    assert_close(rough_metal.roughness_factor(), 0.82);

    let chrome = MaterialDesc::chrome();
    assert_eq!(chrome.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(chrome.base_color(), Color::LIGHT_GRAY);
    assert_close(chrome.metallic_factor(), 1.0);
    assert_close(chrome.roughness_factor(), 0.02);

    let brushed = MaterialDesc::brushed_steel();
    assert_eq!(brushed.kind(), MaterialKind::PbrMetallicRoughness);
    assert_color_close(brushed.base_color(), Color::LIGHT_GRAY);
    assert_close(brushed.metallic_factor(), 1.0);
    assert_close(brushed.roughness_factor(), 0.36);
    assert_close(brushed.anisotropy_strength_factor(), 0.72);

    let clearcoat = MaterialDesc::clearcoat_plastic(Color::BLUE);
    assert_color_close(clearcoat.base_color(), Color::BLUE);
    assert_close(clearcoat.metallic_factor(), 0.0);
    assert_close(clearcoat.roughness_factor(), 0.32);
    assert_close(clearcoat.clearcoat_factor(), 0.9);
    assert_close(clearcoat.clearcoat_roughness_factor(), 0.08);

    let satin = MaterialDesc::satin(Color::MAGENTA);
    assert_color_close(satin.base_color(), Color::MAGENTA);
    assert_close(satin.metallic_factor(), 0.0);
    assert_close(satin.roughness_factor(), 0.68);
    assert_color_close(satin.sheen_color_factor(), Color::WHITE);
    assert_close(satin.sheen_roughness_factor(), 0.48);

    let leather = MaterialDesc::leather(Color::ORANGE);
    assert_color_close(leather.base_color(), Color::ORANGE);
    assert_close(leather.metallic_factor(), 0.0);
    assert_close(leather.roughness_factor(), 0.78);
    assert_color_close(leather.sheen_color_factor(), Color::ORANGE);
    assert_close(leather.sheen_roughness_factor(), 0.72);

    let clear_glass = MaterialDesc::clear_glass(Color::CYAN);
    assert_eq!(clear_glass.alpha_mode(), AlphaMode::Blend);
    assert!(clear_glass.double_sided());
    assert_close(clear_glass.metallic_factor(), 0.0);
    assert_close(clear_glass.roughness_factor(), 0.02);
    assert_close(clear_glass.transmission_factor(), 1.0);
    assert_close(clear_glass.ior(), 1.45);
    assert_close(clear_glass.thickness_factor(), 0.08);
    assert_color_close(clear_glass.attenuation_color(), Color::CYAN);
    assert_close(clear_glass.base_color().a, 0.20);

    let frosted_glass = MaterialDesc::frosted_glass(Color::COOL_WHITE);
    assert_eq!(frosted_glass.alpha_mode(), AlphaMode::Blend);
    assert!(frosted_glass.double_sided());
    assert_close(frosted_glass.roughness_factor(), 1.0);
    assert_close(frosted_glass.transmission_factor(), 0.45);
    assert_close(frosted_glass.ior(), 1.45);
    assert_close(frosted_glass.thickness_factor(), 0.12);
    assert_color_close(frosted_glass.attenuation_color(), Color::COOL_WHITE);
    assert_close(frosted_glass.base_color().a, 0.88);
}

#[test]
fn real_world_material_shortcuts_keep_neighbor_presets_distinct() {
    let metal = MaterialDesc::metal(Color::LIGHT_GRAY);
    let rough_metal = MaterialDesc::rough_metal(Color::LIGHT_GRAY);
    let chrome = MaterialDesc::chrome();
    assert!(
        metal.roughness_factor() >= 0.36,
        "generic metal must be rougher than chrome so the public material demo does not show two mirror-like gray metals"
    );
    assert!(
        chrome.roughness_factor() <= 0.03,
        "chrome must stay the sharp mirror-like metal shortcut"
    );
    assert!(
        rough_metal.roughness_factor() >= 0.78,
        "rough_metal must stay visibly broader than generic metal in the material proof scene"
    );

    let clear = MaterialDesc::clear_glass(Color::CYAN);
    let frosted = MaterialDesc::frosted_glass(Color::COOL_WHITE);
    assert!(
        frosted.roughness_factor() >= 0.92 && frosted.transmission_factor() <= 0.55,
        "frosted glass must be materially rougher and less directly transmissive than clear glass"
    );
    assert!(
        frosted.base_color().a - clear.base_color().a >= 0.40,
        "frosted glass must be visibly more diffuse/opaque than clear glass in the WebGL2 fallback"
    );
}
