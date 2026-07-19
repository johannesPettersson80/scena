use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::super::{WorkflowScene, add_default_camera};
use crate::material_showcase::{
    MaterialShowcaseLighting, glass_background_target_bars, material_preset_showcase,
};
use crate::{
    Aabb, Assets, Color, DirectionalLight, FramingOptions, GeometryDesc, MaterialDesc, Scene,
    Transform, Vec3,
};

const MATERIAL_PROOF_WIDTH: u32 = 512;
const MATERIAL_PROOF_HEIGHT: u32 = 384;

pub(in crate::browser_probe::workflows) async fn material_presets_scene()
-> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let glass_target_geometry = assets.create_geometry(GeometryDesc::box_xyz(1.0, 1.0, 1.0));
    for preset in material_preset_showcase() {
        if let Some(position) = preset.background_target_position() {
            for bar in glass_background_target_bars() {
                scene
                    .mesh(
                        glass_target_geometry,
                        assets.create_material(MaterialDesc::matte(bar.color)),
                    )
                    .transform(Transform {
                        translation: position + bar.offset,
                        rotation: crate::Quat::IDENTITY,
                        scale: bar.scale,
                    })
                    .add()
                    .map_err(|error| {
                        JsValue::from_str(&format!(
                            "material-preset {} background target failed: {error:?}",
                            preset.id
                        ))
                    })?;
            }
        }
        scene
            .mesh(
                assets.create_geometry(preset.geometry_desc()),
                match preset.id {
                    "satin" => assets.material_presets().satin().await.map_err(|error| {
                        JsValue::from_str(&format!("material-preset satin load failed: {error:?}"))
                    })?,
                    "leather" => assets.material_presets().leather().await.map_err(|error| {
                        JsValue::from_str(&format!(
                            "material-preset leather load failed: {error:?}"
                        ))
                    })?,
                    "rubber" => assets.material_presets().rubber().await.map_err(|error| {
                        JsValue::from_str(&format!("material-preset rubber load failed: {error:?}"))
                    })?,
                    _ => assets.create_material(preset.material_desc().with_double_sided(true)),
                },
            )
            .transform(preset.transform())
            .add()
            .map_err(|error| {
                JsValue::from_str(&format!(
                    "material-preset {} mesh failed: {error:?}",
                    preset.id
                ))
            })?;
    }
    let needs_direct_light = material_preset_showcase()
        .iter()
        .any(|preset| preset.lighting_mode == MaterialShowcaseLighting::Studio);
    if needs_direct_light {
        scene
            .directional_light(
                DirectionalLight::key_light()
                    .with_illuminance_lux(1_600.0)
                    .with_shadows(false),
            )
            .add()
            .map_err(|error| {
                JsValue::from_str(&format!("material-preset light failed: {error:?}"))
            })?;
    }
    let camera = add_default_camera(&mut scene)?;
    scene
        .frame_bounds(
            camera,
            Aabb::new(Vec3::new(-1.18, -0.86, -0.24), Vec3::new(1.18, 0.82, 0.24)),
            FramingOptions::new()
                .azimuth_elevation(-18.0, 18.0)
                .fill(0.82)
                .margin_px(18.0)
                .viewport(MATERIAL_PROOF_WIDTH, MATERIAL_PROOF_HEIGHT),
        )
        .map_err(|error| JsValue::from_str(&format!("material-preset frame failed: {error:?}")))?;
    let mut glass_pixel_probes = Vec::new();
    for preset in material_preset_showcase() {
        let Some(target_position) = preset.background_target_position() else {
            continue;
        };
        for (bar_index, bar) in glass_background_target_bars().iter().enumerate() {
            let Some(projected) = scene
                .project_world_point(
                    camera,
                    target_position + bar.offset,
                    MATERIAL_PROOF_WIDTH,
                    MATERIAL_PROOF_HEIGHT,
                )
                .map_err(|error| {
                    JsValue::from_str(&format!(
                        "material-preset {} glass probe projection failed: {error:?}",
                        preset.id
                    ))
                })?
            else {
                continue;
            };
            glass_pixel_probes.push(json!({
                "preset": preset.id,
                "bar_index": bar_index,
                "expected": if bar.color == Color::WHITE { "bright" } else { "dark" },
                "x_norm": round3(projected.x / MATERIAL_PROOF_WIDTH as f32),
                "y_norm": round3(projected.y / MATERIAL_PROOF_HEIGHT as f32),
            }));
        }
    }
    let preset_names = material_preset_showcase()
        .iter()
        .map(|preset| preset.id)
        .collect::<Vec<_>>();
    let showcase_geometry = material_preset_showcase()
        .iter()
        .map(|preset| preset.geometry.as_str())
        .collect::<Vec<_>>();
    let source_surfaces = material_preset_showcase()
        .iter()
        .map(|preset| json!({ "id": preset.id, "surface": preset.source_surface }))
        .collect::<Vec<_>>();
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-pbr-material-preset-expanded-set",
            "material_kind": "pbr-metallic-roughness",
            "preset_names": preset_names,
            "showcase_geometry": showcase_geometry,
            "source_surfaces": source_surfaces,
            "webgl2_smooth_metal_sample_floor": 96,
            "glass_contract": "scene-color-ior-thickness-rough-blur-sorted-transparency",
            "glass_pixel_probes": glass_pixel_probes,
            "glass_pixel_probe_viewport": [MATERIAL_PROOF_WIDTH, MATERIAL_PROOF_HEIGHT],
            "environment_path": "/demo/samples/environment/white_studio_03_1k.hdr",
            "background": "neutral_gray",
            "exposure_ev": 0.0,
        }),
    })
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}
