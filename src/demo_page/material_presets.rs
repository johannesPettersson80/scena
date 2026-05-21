use wasm_bindgen::prelude::*;

use crate::{
    Aabb, Assets, Color, FramingOptions, GeometryDesc, MaterialDesc, OrbitControls,
    PerspectiveCamera, Scene, Transform, Vec3,
};

use super::{DemoApp, log_timing, now_ms};

#[wasm_bindgen]
pub fn load_material_presets_scene(
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    let total_start = now_ms();
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::sphere(0.16, 32, 16));
    let materials = [
        MaterialDesc::matte(Color::BLUE),
        MaterialDesc::plastic(Color::BLUE),
        MaterialDesc::metal(Color::LIGHT_GRAY),
        MaterialDesc::rough_metal(Color::LIGHT_GRAY),
        MaterialDesc::chrome(),
        MaterialDesc::brushed_steel(),
        MaterialDesc::clearcoat_plastic(Color::BLUE),
        MaterialDesc::satin(Color::MAGENTA),
        MaterialDesc::leather(Color::ORANGE),
        MaterialDesc::clear_glass(Color::CYAN),
        MaterialDesc::frosted_glass(Color::COOL_WHITE),
        MaterialDesc::rubber(),
    ];

    let mut scene = Scene::new();
    for (index, material) in materials.into_iter().enumerate() {
        let column = index % 6;
        let row = index / 6;
        let x = -1.0 + column as f32 * 0.4;
        let y = 0.22 - row as f32 * 0.44;
        scene
            .mesh(geometry, assets.create_material(material))
            .transform(Transform::at(Vec3::new(x, y, 0.0)))
            .add()
            .map_err(|err| {
                JsValue::from_str(&format!("add material preset mesh failed: {err:?}"))
            })?;
    }

    let bounds = Aabb::new(Vec3::new(-1.18, -0.42, -0.18), Vec3::new(1.18, 0.42, 0.18));
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::standard(),
            Transform::at(Vec3::new(0.0, 0.0, 2.0)),
        )
        .map_err(|err| JsValue::from_str(&format!("add_perspective_camera failed: {err:?}")))?;
    scene
        .set_active_camera(camera)
        .map_err(|err| JsValue::from_str(&format!("set_active_camera failed: {err:?}")))?;
    let framing = scene
        .frame_bounds(
            camera,
            bounds,
            FramingOptions::new()
                .azimuth_elevation(-8.0, 6.0)
                .fill(0.82)
                .margin_px(18.0)
                .viewport(viewport_width.max(1), viewport_height.max(1)),
        )
        .map_err(|err| {
            JsValue::from_str(&format!("material preset frame_bounds failed: {err:?}"))
        })?;
    scene
        .add_studio_lighting()
        .map_err(|err| JsValue::from_str(&format!("add_studio_lighting failed: {err:?}")))?;
    let controls = OrbitControls::from_framing(framing).cinematic();
    log_timing("load_material_presets_scene total", total_start);

    Ok(DemoApp {
        assets,
        scene,
        camera,
        controls,
        renderer: None,
        connector_replay: None,
    })
}
