use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::super::{WorkflowScene, add_default_camera};
use crate::{
    Aabb, AssetLoadOptions, Assets, Color, DirectionalLight, MaterialDesc, Scene, Transform, Vec3,
};

pub(super) async fn source_gltf_materials_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let report = assets
        .load_scene_with_report_options(
            "/fixtures/gltf/khronos/WaterBottle/WaterBottle.gltf",
            AssetLoadOptions::default().with_strict_textures(true),
        )
        .await
        .map_err(|error| {
            JsValue::from_str(&format!(
                "source glTF material fixture load failed: {error:?}"
            ))
        })?;
    let scene_asset = report.asset().clone();
    let (geometry, source_material) = scene_asset
        .nodes()
        .iter()
        .find_map(|node| {
            node.meshes()
                .first()
                .map(|mesh| (mesh.geometry(), mesh.material()))
        })
        .ok_or_else(|| JsValue::from_str("source glTF material fixture has no mesh"))?;
    let material = assets.material(source_material).ok_or_else(|| {
        JsValue::from_str("source glTF material fixture produced no source material descriptor")
    })?;
    let source_base_color_decoded = material
        .base_color_texture()
        .and_then(|texture| assets.texture(texture))
        .is_some_and(|texture| texture.has_decoded_pixels());
    let source_texture_bindings = [
        material.base_color_texture(),
        material.normal_texture(),
        material.metallic_roughness_texture(),
        material.occlusion_texture(),
        material.emissive_texture(),
    ]
    .into_iter()
    .flatten()
    .count();

    let unlit_material = assets.create_material(
        MaterialDesc::unlit(Color::from_srgb_u8(80, 185, 255)).with_double_sided(true),
    );
    let pbr_material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(240, 190, 85), 0.0, 0.65)
            .with_double_sided(true),
    );

    let mut scene = Scene::new();
    scene
        .mesh(geometry, unlit_material)
        .transform(Transform::at(Vec3::new(-0.56, 0.0, 0.0)).scale_by(4.0))
        .add()
        .map_err(|error| JsValue::from_str(&format!("unlit comparison mesh failed: {error:?}")))?;
    scene
        .mesh(geometry, source_material)
        .transform(Transform::at(Vec3::ZERO).scale_by(4.0))
        .add()
        .map_err(|error| JsValue::from_str(&format!("source material mesh failed: {error:?}")))?;
    scene
        .mesh(geometry, pbr_material)
        .transform(Transform::at(Vec3::new(0.56, 0.0, 0.0)).scale_by(4.0))
        .add()
        .map_err(|error| JsValue::from_str(&format!("PBR comparison mesh failed: {error:?}")))?;
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(18_000.0))
        .add()
        .map_err(|error| JsValue::from_str(&format!("source material light failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    scene
        .frame(
            camera,
            Aabb::new(Vec3::new(-1.0, -0.65, -0.3), Vec3::new(1.0, 0.65, 0.3)),
        )
        .map_err(|error| JsValue::from_str(&format!("source material frame failed: {error:?}")))?;

    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-source-gltf-material-comparison",
            "source": "/fixtures/gltf/khronos/WaterBottle/WaterBottle.gltf",
            "construction": "SceneAsset::nodes mesh.geometry mesh.material",
            "source_material_kind": format!("{:?}", material.kind()),
            "source_base_color_decoded": source_base_color_decoded,
            "source_texture_bindings": source_texture_bindings,
            "load_warnings": report.warnings().len(),
            "comparison_lanes": ["generated-unlit", "source-gltf-material", "generated-pbr"],
        }),
    })
}
