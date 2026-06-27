use serde_json::{Value, json};

use super::render_recipe_material_gpu;

#[test]
fn authored_advanced_pbr_recipe_scalars_each_change_headless_gpu_pixels() {
    struct ScalarGpuCase {
        name: &'static str,
        baseline: Vec<(&'static str, Value)>,
        changed: Vec<(&'static str, Value)>,
        min_delta: u64,
    }

    let linear_texture = json!({
        "uri": "gltf/khronos/WaterBottle/WaterBottle_baseColor.png",
        "color_space": "linear"
    });
    let cases = vec![
        ScalarGpuCase {
            name: "clearcoat_factor",
            baseline: vec![
                ("roughness", json!(0.24)),
                ("clearcoat_factor", json!(0.0)),
                ("clearcoat_roughness_factor", json!(0.12)),
            ],
            changed: vec![
                ("roughness", json!(0.24)),
                ("clearcoat_factor", json!(0.9)),
                ("clearcoat_roughness_factor", json!(0.12)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "clearcoat_roughness_factor",
            baseline: vec![
                ("roughness", json!(0.18)),
                ("clearcoat_factor", json!(0.9)),
                ("clearcoat_roughness_factor", json!(0.02)),
            ],
            changed: vec![
                ("roughness", json!(0.18)),
                ("clearcoat_factor", json!(0.9)),
                ("clearcoat_roughness_factor", json!(0.72)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "clearcoat_normal_scale",
            baseline: vec![
                ("roughness", json!(0.28)),
                ("clearcoat_factor", json!(0.85)),
                ("clearcoat_roughness_factor", json!(0.08)),
                ("clearcoat_normal_scale", json!(0.0)),
                ("clearcoat_normal_texture", linear_texture.clone()),
            ],
            changed: vec![
                ("roughness", json!(0.28)),
                ("clearcoat_factor", json!(0.85)),
                ("clearcoat_roughness_factor", json!(0.08)),
                ("clearcoat_normal_scale", json!(2.0)),
                ("clearcoat_normal_texture", linear_texture.clone()),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "sheen_color_factor",
            baseline: vec![
                ("roughness", json!(0.5)),
                ("sheen_color_factor", json!("base")),
                ("sheen_roughness_factor", json!(0.35)),
            ],
            changed: vec![
                ("roughness", json!(0.5)),
                ("sheen_color_factor", json!("blue")),
                ("sheen_roughness_factor", json!(0.35)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "sheen_roughness_factor",
            baseline: vec![
                ("roughness", json!(0.46)),
                ("sheen_color_factor", json!("white")),
                ("sheen_roughness_factor", json!(0.02)),
            ],
            changed: vec![
                ("roughness", json!(0.46)),
                ("sheen_color_factor", json!("white")),
                ("sheen_roughness_factor", json!(0.9)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "anisotropy_strength_factor",
            baseline: vec![
                ("roughness", json!(0.28)),
                ("anisotropy_strength_factor", json!(0.0)),
                ("anisotropy_rotation_radians", json!(0.65)),
            ],
            changed: vec![
                ("roughness", json!(0.28)),
                ("anisotropy_strength_factor", json!(0.9)),
                ("anisotropy_rotation_radians", json!(0.65)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "anisotropy_rotation_radians",
            baseline: vec![
                ("roughness", json!(0.28)),
                ("anisotropy_strength_factor", json!(0.85)),
                ("anisotropy_rotation_radians", json!(0.0)),
            ],
            changed: vec![
                ("roughness", json!(0.28)),
                ("anisotropy_strength_factor", json!(0.85)),
                ("anisotropy_rotation_radians", json!(1.25)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "iridescence_factor",
            baseline: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.0)),
                ("iridescence_ior", json!(1.45)),
                ("iridescence_thickness_minimum_nm", json!(140.0)),
                ("iridescence_thickness_maximum_nm", json!(560.0)),
            ],
            changed: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.85)),
                ("iridescence_ior", json!(1.45)),
                ("iridescence_thickness_minimum_nm", json!(140.0)),
                ("iridescence_thickness_maximum_nm", json!(560.0)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "iridescence_ior",
            baseline: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.8)),
                ("iridescence_ior", json!(1.1)),
                ("iridescence_thickness_minimum_nm", json!(140.0)),
                ("iridescence_thickness_maximum_nm", json!(560.0)),
            ],
            changed: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.8)),
                ("iridescence_ior", json!(2.0)),
                ("iridescence_thickness_minimum_nm", json!(140.0)),
                ("iridescence_thickness_maximum_nm", json!(560.0)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "iridescence_thickness_minimum_nm",
            baseline: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.8)),
                ("iridescence_ior", json!(1.45)),
                ("iridescence_thickness_minimum_nm", json!(100.0)),
                ("iridescence_thickness_maximum_nm", json!(650.0)),
                ("iridescence_thickness_texture", linear_texture.clone()),
            ],
            changed: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.8)),
                ("iridescence_ior", json!(1.45)),
                ("iridescence_thickness_minimum_nm", json!(500.0)),
                ("iridescence_thickness_maximum_nm", json!(650.0)),
                ("iridescence_thickness_texture", linear_texture.clone()),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "iridescence_thickness_maximum_nm",
            baseline: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.8)),
                ("iridescence_ior", json!(1.45)),
                ("iridescence_thickness_minimum_nm", json!(120.0)),
                ("iridescence_thickness_maximum_nm", json!(180.0)),
            ],
            changed: vec![
                ("roughness", json!(0.34)),
                ("iridescence_factor", json!(0.8)),
                ("iridescence_ior", json!(1.45)),
                ("iridescence_thickness_minimum_nm", json!(120.0)),
                ("iridescence_thickness_maximum_nm", json!(700.0)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "dispersion_factor",
            baseline: vec![
                ("roughness", json!(0.22)),
                ("transmission_factor", json!(0.65)),
                ("ior", json!(1.55)),
                ("dispersion_factor", json!(0.0)),
            ],
            changed: vec![
                ("roughness", json!(0.22)),
                ("transmission_factor", json!(0.65)),
                ("ior", json!(1.55)),
                ("dispersion_factor", json!(0.08)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "transmission_factor",
            baseline: vec![
                ("roughness", json!(0.18)),
                ("alpha_mode", json!({ "kind": "blend" })),
                ("transmission_factor", json!(0.0)),
                ("ior", json!(1.55)),
            ],
            changed: vec![
                ("roughness", json!(0.18)),
                ("alpha_mode", json!({ "kind": "blend" })),
                ("transmission_factor", json!(0.65)),
                ("ior", json!(1.55)),
            ],
            min_delta: 16,
        },
        ScalarGpuCase {
            name: "ior",
            baseline: vec![
                ("roughness", json!(0.22)),
                ("dispersion_factor", json!(0.08)),
                ("ior", json!(1.01)),
            ],
            changed: vec![
                ("roughness", json!(0.22)),
                ("dispersion_factor", json!(0.08)),
                ("ior", json!(4.0)),
            ],
            min_delta: 16,
        },
    ];

    let mut failures = Vec::new();
    for case in cases {
        let baseline_frame =
            render_recipe_material_gpu(advanced_material_recipe("baseline", case.baseline));
        let changed_frame =
            render_recipe_material_gpu(advanced_material_recipe("changed", case.changed));
        let delta = rgba8_absolute_delta(&baseline_frame, &changed_frame);
        if delta < case.min_delta {
            failures.push(format!(
                "{} delta {delta}, expected >= {}",
                case.name, case.min_delta
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "advanced PBR recipe scalar GPU attribution failures: {}",
        failures.join("; ")
    );
}

#[test]
fn authored_advanced_pbr_recipe_volume_scalars_change_headless_gpu_pixels_in_coupled_scene() {
    let red_frame = render_recipe_material_gpu(advanced_material_recipe(
        "red_volume",
        coupled_volume_fields("red", 0.22),
    ));
    let blue_frame = render_recipe_material_gpu(advanced_material_recipe(
        "blue_volume",
        coupled_volume_fields("blue", 0.22),
    ));
    let (red_absorption, blue_absorption, color_changed_pixels, color_delta) =
        changed_region_rgb_averages(&red_frame, &blue_frame);
    assert!(
        color_delta >= 24 && color_changed_pixels >= 16,
        "attenuation_color must visibly change the transmitted region: delta={color_delta}, changed_pixels={color_changed_pixels}, red={red_absorption:?}, blue={blue_absorption:?}"
    );
    assert!(
        red_absorption[0] > blue_absorption[0] && blue_absorption[2] > red_absorption[2],
        "red vs blue KHR_materials_volume attenuation should bias different channels: red={red_absorption:?}, blue={blue_absorption:?}"
    );

    let strong_frame = render_recipe_material_gpu(advanced_material_recipe(
        "short_distance",
        coupled_volume_fields("red", 0.18),
    ));
    let weak_frame = render_recipe_material_gpu(advanced_material_recipe(
        "long_distance",
        coupled_volume_fields("red", 3.0),
    ));
    let (strong_absorption, weak_absorption, distance_changed_pixels, distance_delta) =
        changed_region_rgb_averages(&strong_frame, &weak_frame);
    assert!(
        distance_delta >= 18 && distance_changed_pixels >= 16,
        "attenuation_distance must change volume absorption when transmission and thickness are active: delta={distance_delta}, changed_pixels={distance_changed_pixels}, strong={strong_absorption:?}, weak={weak_absorption:?}"
    );
}

fn coupled_volume_fields(color: &str, attenuation_distance: f64) -> Vec<(&str, Value)> {
    vec![
        ("base_color", json!("white")),
        ("alpha_mode", json!({ "kind": "blend" })),
        ("roughness", json!(0.08)),
        ("transmission_factor", json!(1.0)),
        ("ior", json!(1.5)),
        ("thickness_factor", json!(1.0)),
        ("attenuation_distance", json!(attenuation_distance)),
        ("attenuation_color", json!(color)),
    ]
}

fn advanced_material_recipe(id: &str, fields: Vec<(&str, Value)>) -> Value {
    let mut material = json!({
        "id": id,
        "kind": "pbr_metallic_roughness",
        "base_color": "base",
        "metallic": 0.0,
        "roughness": 0.42,
        "double_sided": true
    });
    let object = material
        .as_object_mut()
        .expect("material recipe is an object");
    for (field, value) in fields {
        object.insert(field.to_owned(), value);
    }
    material
}

fn rgba8_absolute_delta(a: &[u8], b: &[u8]) -> u64 {
    assert_eq!(a.len(), b.len(), "frames must have matching dimensions");
    a.iter()
        .zip(b)
        .map(|(left, right)| u64::from(left.abs_diff(*right)))
        .sum()
}

fn changed_region_rgb_averages(left: &[u8], right: &[u8]) -> ([u8; 3], [u8; 3], usize, u64) {
    const WIDTH: usize = 64;
    const HEIGHT: usize = 64;
    assert_eq!(left.len(), WIDTH * HEIGHT * 4);
    assert_eq!(right.len(), left.len());
    let mut left_sum = [0u64; 3];
    let mut right_sum = [0u64; 3];
    let mut count = 0u64;
    let mut delta = 0u64;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let offset = (y * WIDTH + x) * 4;
            let pixel_delta = (0..3)
                .map(|channel| u64::from(left[offset + channel].abs_diff(right[offset + channel])))
                .sum::<u64>();
            delta += pixel_delta;
            if pixel_delta >= 8 {
                for channel in 0..3 {
                    left_sum[channel] += u64::from(left[offset + channel]);
                    right_sum[channel] += u64::from(right[offset + channel]);
                }
                count += 1;
            }
        }
    }
    if count == 0 {
        return ([0; 3], [0; 3], 0, delta);
    }
    (
        [
            (left_sum[0] / count) as u8,
            (left_sum[1] / count) as u8,
            (left_sum[2] / count) as u8,
        ],
        [
            (right_sum[0] / count) as u8,
            (right_sum[1] / count) as u8,
            (right_sum[2] / count) as u8,
        ],
        count as usize,
        delta,
    )
}
