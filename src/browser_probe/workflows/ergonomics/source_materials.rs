use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::super::{WorkflowScene, add_default_camera};
use crate::{
    Aabb, AssetLoadOptions, Assets, Color, DirectionalLight, Light, MaterialDesc, Scene, Transform,
    Vec3,
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
    let source_texture_roles = [
        ("base_color", material.base_color_texture()),
        ("normal", material.normal_texture()),
        ("metallic_roughness", material.metallic_roughness_texture()),
        ("occlusion", material.occlusion_texture()),
        ("emissive", material.emissive_texture()),
        ("clearcoat", material.clearcoat_texture()),
        (
            "clearcoat_roughness",
            material.clearcoat_roughness_texture(),
        ),
        ("clearcoat_normal", material.clearcoat_normal_texture()),
    ]
    .into_iter()
    .filter_map(|(role, texture)| texture.map(|_| role))
    .collect::<Vec<_>>();
    let source_texture_bindings = source_texture_roles.len();

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
    let frame_bounds = Aabb::new(Vec3::new(-1.0, -0.65, -0.3), Vec3::new(1.0, 0.65, 0.3));
    scene
        .frame(camera, frame_bounds)
        .map_err(|error| JsValue::from_str(&format!("source material frame failed: {error:?}")))?;
    let lights = scene
        .light_nodes()
        .map(|(_, _, light, _)| match light {
            Light::Directional(light) => json!({
                "kind": "directional",
                "illuminance_lux": light.illuminance_lux(),
            }),
            Light::Point(light) => json!({
                "kind": "point",
                "intensity_candela": light.intensity_candela(),
            }),
            Light::Spot(light) => json!({
                "kind": "spot",
                "intensity_candela": light.intensity_candela(),
            }),
            Light::Area(light) => json!({
                "kind": "area",
                "luminous_flux_lumens": light.luminous_flux_lumens(),
            }),
        })
        .collect::<Vec<_>>();

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
            "source_texture_roles": source_texture_roles,
            "frame_bounds": {
                "min": [frame_bounds.min.x, frame_bounds.min.y, frame_bounds.min.z],
                "max": [frame_bounds.max.x, frame_bounds.max.y, frame_bounds.max.z],
            },
            "lights": lights,
            "load_warnings": report.warnings().len(),
            "comparison_lanes": ["generated-unlit", "source-gltf-material", "generated-pbr"],
        }),
    })
}

pub(super) async fn oversized_browser_texture_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let report = assets
        .load_scene_with_report_options(
            "/fixtures/generated/oversized_texture_scene.gltf",
            AssetLoadOptions::default().with_strict_textures(true),
        )
        .await
        .map_err(|error| {
            JsValue::from_str(&format!("oversized texture fixture load failed: {error:?}"))
        })?;
    let scene_asset = report.asset().clone();
    let source_material = scene_asset
        .nodes()
        .iter()
        .find_map(|node| node.meshes().first().map(|mesh| mesh.material()))
        .ok_or_else(|| JsValue::from_str("oversized texture fixture has no material"))?;
    let material = assets.material(source_material).ok_or_else(|| {
        JsValue::from_str("oversized texture fixture produced no source material descriptor")
    })?;
    let browser_texture_size = material
        .base_color_texture()
        .and_then(|texture| assets.texture(texture))
        .and_then(|texture| {
            #[cfg(target_arch = "wasm32")]
            {
                texture
                    .browser_image()
                    .map(|image| vec![image.width(), image.height()])
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = texture;
                None
            }
        });

    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).map_err(|error| {
        JsValue::from_str(&format!("oversized texture instantiate failed: {error:?}"))
    })?;
    scene
        .directional_light(DirectionalLight::default().with_illuminance_lux(20_000.0))
        .add()
        .map_err(|error| {
            JsValue::from_str(&format!("oversized texture light failed: {error:?}"))
        })?;
    let camera = add_default_camera(&mut scene)?;
    if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).map_err(|error| {
            JsValue::from_str(&format!("oversized texture frame failed: {error:?}"))
        })?;
    }

    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-oversized-source-texture-clamp",
            "fixture": "/fixtures/generated/oversized_texture_scene.gltf",
            "source_texture_size": [2049, 2049],
            "max_browser_texture_dimension": crate::assets::BROWSER_TEXTURE_MAX_DIMENSION_2D,
            "browser_texture_size": browser_texture_size,
            "load_warnings": report.warnings().len(),
        }),
    })
}
