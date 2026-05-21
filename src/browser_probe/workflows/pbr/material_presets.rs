use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::super::{WorkflowScene, add_default_camera};
use super::radiance_hdr_data_uri;
use crate::{Assets, Color, DirectionalLight, GeometryDesc, MaterialDesc, Scene, Transform, Vec3};

pub(in crate::browser_probe::workflows) fn material_presets_scene() -> Result<WorkflowScene, JsValue>
{
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.12, 0.36, 0.04));
    let materials = [
        assets.create_material(MaterialDesc::matte(Color::BLUE).with_double_sided(true)),
        assets.create_material(MaterialDesc::plastic(Color::BLUE).with_double_sided(true)),
        assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY).with_double_sided(true)),
        assets
            .create_material(MaterialDesc::rough_metal(Color::LIGHT_GRAY).with_double_sided(true)),
        assets.create_material(MaterialDesc::chrome().with_double_sided(true)),
        assets.create_material(MaterialDesc::brushed_steel().with_double_sided(true)),
        assets
            .create_material(MaterialDesc::clearcoat_plastic(Color::BLUE).with_double_sided(true)),
        assets.create_material(MaterialDesc::satin(Color::MAGENTA).with_double_sided(true)),
        assets.create_material(MaterialDesc::leather(Color::ORANGE).with_double_sided(true)),
        assets.create_material(MaterialDesc::clear_glass(Color::CYAN)),
        assets.create_material(MaterialDesc::frosted_glass(Color::COOL_WHITE)),
        assets.create_material(MaterialDesc::rubber().with_double_sided(true)),
    ];
    let mut scene = Scene::new();
    for (index, material) in materials.into_iter().enumerate() {
        let x = -0.88 + index as f32 * 0.16;
        scene
            .mesh(geometry, material)
            .transform(Transform::at(Vec3::new(x, 0.0, 0.0)))
            .add()
            .map_err(|error| {
                JsValue::from_str(&format!("material-preset mesh {index} failed: {error:?}"))
            })?;
    }
    scene
        .directional_light(DirectionalLight::key_light().with_shadows(false))
        .add()
        .map_err(|error| JsValue::from_str(&format!("material-preset light failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "browser-pbr-material-preset-expanded-set",
            "material_kind": "pbr-metallic-roughness",
            "preset_names": [
                "matte",
                "plastic",
                "metal",
                "rough_metal",
                "chrome",
                "brushed_steel",
                "clearcoat_plastic",
                "satin",
                "leather",
                "clear_glass",
                "frosted_glass",
                "rubber"
            ],
            "webgl2_smooth_metal_sample_floor": 96,
            "glass_contract": "blend-plus-transmission-preview-no-refraction-claim",
            "environment_path": radiance_hdr_data_uri(
                4,
                1,
                &[
                    [0, 0, 0, 128],
                    [255, 255, 255, 136],
                    [32, 64, 255, 132],
                    [0, 0, 0, 128]
                ],
                "material-preset-contrast_4x1.hdr",
            ),
        }),
    })
}
