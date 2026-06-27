use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::*;
use crate::{Assets, Color, DirectionalLight, GeometryDesc, Renderer, Scene, Transform, Vec3};

mod scalars;

#[test]
fn authored_advanced_pbr_recipe_fields_map_to_material_descriptor() {
    let texture = "gltf/khronos/WaterBottle/WaterBottle_baseColor.png";
    let material = recipe_material(json!({
        "id": "advanced",
        "kind": "pbr_metallic_roughness",
        "base_color": "base",
        "metallic": 0.0,
        "roughness": 0.42,
        "clearcoat_factor": 0.8,
        "clearcoat_roughness_factor": 0.16,
        "clearcoat_normal_scale": 1.25,
        "sheen_color_factor": "white",
        "sheen_roughness_factor": 0.35,
        "anisotropy_strength_factor": 0.65,
        "anisotropy_rotation_radians": 0.3,
        "iridescence_factor": 0.45,
        "iridescence_ior": 1.45,
        "iridescence_thickness_minimum_nm": 120.0,
        "iridescence_thickness_maximum_nm": 480.0,
        "dispersion_factor": 0.02,
        "transmission_factor": 0.18,
        "ior": 1.52,
        "thickness_factor": 0.35,
        "attenuation_distance": 2.0,
        "attenuation_color": "blue",
        "clearcoat_texture": { "uri": texture, "color_space": "linear" },
        "clearcoat_roughness_texture": { "uri": texture, "color_space": "linear" },
        "clearcoat_normal_texture": { "uri": texture, "color_space": "linear" },
        "sheen_color_texture": { "uri": texture, "color_space": "srgb" },
        "sheen_roughness_texture": { "uri": texture, "color_space": "linear" },
        "anisotropy_texture": { "uri": texture, "color_space": "linear" },
        "iridescence_texture": { "uri": texture, "color_space": "linear" },
        "iridescence_thickness_texture": { "uri": texture, "color_space": "linear" }
    }));

    assert_close(material.clearcoat_factor(), 0.8);
    assert_close(material.clearcoat_roughness_factor(), 0.16);
    assert_close(material.clearcoat_normal_scale(), 1.25);
    assert_close(material.sheen_color_factor().r, 1.0);
    assert_close(material.sheen_roughness_factor(), 0.35);
    assert_close(material.anisotropy_strength_factor(), 0.65);
    assert_close(material.anisotropy_rotation_radians(), 0.3);
    assert_close(material.iridescence_factor(), 0.45);
    assert_close(material.iridescence_ior(), 1.45);
    assert_close(material.iridescence_thickness_minimum_nm(), 120.0);
    assert_close(material.iridescence_thickness_maximum_nm(), 480.0);
    assert_close(material.dispersion_factor(), 0.02);
    assert_close(material.transmission_factor(), 0.18);
    assert_close(material.ior(), 1.52);
    assert_close(material.thickness_factor(), 0.35);
    assert_close(material.attenuation_distance(), 2.0);
    assert!(material.attenuation_color().b > material.attenuation_color().r);
    assert!(material.clearcoat_texture().is_some());
    assert!(material.clearcoat_roughness_texture().is_some());
    assert!(material.clearcoat_normal_texture().is_some());
    assert!(material.sheen_color_texture().is_some());
    assert!(material.sheen_roughness_texture().is_some());
    assert!(material.anisotropy_texture().is_some());
    assert!(material.iridescence_texture().is_some());
    assert!(material.iridescence_thickness_texture().is_some());
    assert!(material.transmission_texture().is_none());
    assert!(material.thickness_texture().is_none());
}

#[test]
fn authored_advanced_pbr_recipe_ior_matches_public_setter_domain() {
    let zero = recipe_material(json!({
        "id": "advanced",
        "kind": "pbr_metallic_roughness",
        "base_color": "base",
        "ior": 0.0
    }));
    assert_close(zero.ior(), 0.0);

    let boundary = recipe_material(json!({
        "id": "advanced",
        "kind": "pbr_metallic_roughness",
        "base_color": "base",
        "ior": 1.0
    }));
    assert_close(boundary.ior(), 1.0);
}

#[test]
fn authored_advanced_pbr_recipe_rejects_gpu_unsupported_volume_texture_fields() {
    for (field, value) in [
        (
            "transmission_texture",
            json!({
                "uri": "gltf/khronos/WaterBottle/WaterBottle_baseColor.png",
                "color_space": "linear"
            }),
        ),
        (
            "thickness_texture",
            json!({
                "uri": "gltf/khronos/WaterBottle/WaterBottle_baseColor.png",
                "color_space": "linear"
            }),
        ),
    ] {
        let mut material = json!({
            "id": "advanced",
            "kind": "pbr_metallic_roughness",
            "base_color": "base"
        });
        material
            .as_object_mut()
            .expect("material recipe is an object")
            .insert(field.to_owned(), value);
        let result = try_recipe_material(material);
        let error = result.expect_err("GPU-unsupported recipe volume field should fail closed");
        assert_eq!(error.code, "unsupported_feature");
        assert_eq!(error.path, format!("$.materials[0].{field}"));
    }
}

fn recipe_material(value: serde_json::Value) -> MaterialDesc {
    try_recipe_material(value).expect("recipe material builds")
}

fn render_recipe_material_gpu(value: Value) -> Vec<u8> {
    let recipe: SceneRecipeMaterialV1 =
        serde_json::from_value(value).expect("recipe material decodes");
    let colors = test_colors();
    let base_color = recipe
        .base_color
        .as_deref()
        .map(|base_color| authored_color(&colors, base_color).expect("base color resolves"));
    let host = SceneHostCore::headless(64, 64).expect("host builds");
    let mut texture_budget = RecipeTextureBudget::default();
    let mut resources = MaterialRecipeBuildRefs {
        colors: &colors,
        texture_budget: &mut texture_budget,
    };
    let (_, material) = pollster::block_on(authored_material(
        &RecipeBuildPolicy::testing(),
        &host,
        "tests/assets/slice9.recipe.json",
        &recipe,
        base_color,
        "$.materials[0]",
        &mut resources,
    ))
    .expect("recipe material builds");
    render_material_with_assets_gpu(&host.assets, material)
}

fn try_recipe_material(
    value: serde_json::Value,
) -> Result<MaterialDesc, Box<SceneRecipeDiagnosticV1>> {
    let recipe: SceneRecipeMaterialV1 =
        serde_json::from_value(value).expect("recipe material decodes");
    let colors = test_colors();
    let base_color = recipe
        .base_color
        .as_deref()
        .map(|base_color| authored_color(&colors, base_color).expect("base color resolves"));
    let host = SceneHostCore::headless(64, 64).expect("host builds");
    let mut texture_budget = RecipeTextureBudget::default();
    let mut resources = MaterialRecipeBuildRefs {
        colors: &colors,
        texture_budget: &mut texture_budget,
    };
    pollster::block_on(authored_material(
        &RecipeBuildPolicy::testing(),
        &host,
        "tests/assets/slice9.recipe.json",
        &recipe,
        base_color,
        "$.materials[0]",
        &mut resources,
    ))
    .map(|(_, material)| material)
}

fn render_material_with_assets_gpu(assets: &Assets, material: MaterialDesc) -> Vec<u8> {
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.65, 0.65, 0.05));
    let material = assets.create_material(material);
    let backdrop_geometry = assets.create_geometry(GeometryDesc::box_xyz(3.0, 3.0, 0.02));
    let backdrop = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    scene
        .mesh(backdrop_geometry, backdrop)
        .transform(Transform::at(Vec3::new(0.0, 0.0, -0.28)))
        .add()
        .expect("backdrop mesh inserts");
    scene
        .mesh(geometry, material)
        .transform(Transform::at(Vec3::ZERO))
        .add()
        .expect("mesh inserts");
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(18_000.0))
        .add()
        .expect("light inserts");
    let camera = scene.add_default_camera().expect("camera inserts");
    let mut renderer = Renderer::headless_gpu(64, 64).expect("HeadlessGpu renderer builds");
    renderer.set_background_color(Color::from_srgb_u8(18, 24, 32));
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("scene prepares on HeadlessGpu");
    renderer
        .render(&scene, camera)
        .expect("scene renders on HeadlessGpu");
    renderer.frame_rgba8().to_vec()
}

fn test_colors() -> BTreeMap<String, SceneRecipeColorV1> {
    [
        (
            "base".to_owned(),
            SceneRecipeColorV1::Hex("#7C8798".to_owned()),
        ),
        (
            "white".to_owned(),
            SceneRecipeColorV1::Hex("#FFFFFF".to_owned()),
        ),
        (
            "blue".to_owned(),
            SceneRecipeColorV1::Hex("#BFD7FF".to_owned()),
        ),
        (
            "red".to_owned(),
            SceneRecipeColorV1::Hex("#FF4040".to_owned()),
        ),
        (
            "black".to_owned(),
            SceneRecipeColorV1::Hex("#000000".to_owned()),
        ),
    ]
    .into_iter()
    .collect()
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-5,
        "expected {actual} to equal {expected}"
    );
}
