use wasm_bindgen::prelude::*;

use crate::{
    Aabb, Assets, Color, FramingOptions, GeometryDesc, GeometryTopology, GeometryVertex, LabelDesc,
    MaterialDesc, OrbitControls, PerspectiveCamera, Scene, Transform, Vec3,
};

use super::{DemoApp, log_timing, now_ms};
use crate::material_showcase::{glass_background_target_bars, material_preset_showcase};

#[wasm_bindgen]
pub async fn load_material_presets_scene(
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    load_material_spheres_scene(viewport_width, viewport_height).await
}

#[allow(dead_code)]
async fn load_material_proof_scene(
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    let total_start = now_ms();
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
                    .map_err(|err| {
                        JsValue::from_str(&format!(
                            "add {} material background target failed: {err:?}",
                            preset.id
                        ))
                    })?;
            }
        }
        let material =
            match preset.id {
                "satin" => assets.material_presets().satin().await.map_err(|err| {
                    JsValue::from_str(&format!("load satin preset failed: {err:?}"))
                })?,
                "leather" => assets.material_presets().leather().await.map_err(|err| {
                    JsValue::from_str(&format!("load leather preset failed: {err:?}"))
                })?,
                "rubber" => assets.material_presets().rubber().await.map_err(|err| {
                    JsValue::from_str(&format!("load rubber preset failed: {err:?}"))
                })?,
                _ => assets.create_material(preset.material_desc().with_double_sided(true)),
            };
        scene
            .mesh(assets.create_geometry(preset.geometry_desc()), material)
            .transform(preset.transform())
            .add()
            .map_err(|err| {
                JsValue::from_str(&format!(
                    "add {} material preset failed: {err:?}",
                    preset.id
                ))
            })?;
        scene
            .add_label(
                scene.root(),
                LabelDesc::bitmap(preset.label)
                    .with_color(Color::from_srgb_u8(225, 230, 238))
                    .with_size(12.0),
                Transform::at(preset.label_position()),
            )
            .map_err(|err| {
                JsValue::from_str(&format!("add {} material label failed: {err:?}", preset.id))
            })?;
    }

    let bounds = Aabb::new(Vec3::new(-1.18, -0.86, -0.24), Vec3::new(1.18, 0.82, 0.24));
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
                .azimuth_elevation(-18.0, 18.0)
                .fill(0.82)
                .margin_px(18.0)
                .viewport(viewport_width.max(1), viewport_height.max(1)),
        )
        .map_err(|err| {
            JsValue::from_str(&format!("material preset frame_bounds failed: {err:?}"))
        })?;
    let controls = OrbitControls::from_framing(framing);
    log_timing("load_material_presets_scene total", total_start);

    Ok(DemoApp {
        assets,
        scene,
        camera,
        controls,
        controls_dirty: false,
        needs_prepare: true,
        renderer: None,
        connector_replay: None,
    })
}

fn material_studio_backdrop_geometry() -> GeometryDesc {
    let vertices = vec![
        GeometryVertex {
            position: Vec3::new(-1.70, 0.82, -0.30),
            normal: Vec3::new(0.0, 0.0, 1.0),
        },
        GeometryVertex {
            position: Vec3::new(1.70, 0.82, -0.30),
            normal: Vec3::new(0.0, 0.0, 1.0),
        },
        GeometryVertex {
            position: Vec3::new(-1.70, 0.08, -0.30),
            normal: Vec3::new(0.0, 0.0, 1.0),
        },
        GeometryVertex {
            position: Vec3::new(1.70, 0.08, -0.30),
            normal: Vec3::new(0.0, 0.0, 1.0),
        },
        GeometryVertex {
            position: Vec3::new(-1.70, -0.82, -0.30),
            normal: Vec3::new(0.0, 0.0, 1.0),
        },
        GeometryVertex {
            position: Vec3::new(1.70, -0.82, -0.30),
            normal: Vec3::new(0.0, 0.0, 1.0),
        },
    ];
    let colors = vec![
        Color::from_srgb_u8(5, 12, 20),
        Color::from_srgb_u8(5, 12, 20),
        Color::from_srgb_u8(15, 42, 59),
        Color::from_srgb_u8(15, 42, 59),
        Color::from_srgb_u8(10, 27, 38),
        Color::from_srgb_u8(10, 27, 38),
    ];
    GeometryDesc::try_new_with_vertex_colors(
        GeometryTopology::Triangles,
        vertices,
        vec![0, 2, 1, 1, 2, 3, 2, 4, 3, 3, 4, 5],
        colors,
    )
    .expect("material studio backdrop geometry is valid")
}

#[wasm_bindgen]
pub async fn load_material_spheres_scene(
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    let total_start = now_ms();
    let assets = Assets::new();
    let mut scene = Scene::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(1.0, 72, 36));
    let backdrop = assets.create_geometry(material_studio_backdrop_geometry());
    scene
        .mesh(
            backdrop,
            assets.create_material(MaterialDesc::unlit(Color::WHITE)),
        )
        .add()
        .map_err(|err| {
            JsValue::from_str(&format!("add material studio backdrop failed: {err:?}"))
        })?;

    for (index, preset) in material_preset_showcase().iter().enumerate() {
        let column = index % 6;
        let row = index / 6;
        let position = Vec3::new(-1.0 + column as f32 * 0.4, 0.32 - row as f32 * 0.64, 0.0);
        let material =
            match preset.id {
                "satin" => assets.material_presets().satin().await.map_err(|err| {
                    JsValue::from_str(&format!("load satin preset failed: {err:?}"))
                })?,
                "leather" => assets.material_presets().leather().await.map_err(|err| {
                    JsValue::from_str(&format!("load leather preset failed: {err:?}"))
                })?,
                "rubber" => assets.material_presets().rubber().await.map_err(|err| {
                    JsValue::from_str(&format!("load rubber preset failed: {err:?}"))
                })?,
                _ => assets.create_material(preset.material_desc()),
            };
        scene
            .mesh(sphere, material)
            .transform(Transform {
                translation: position,
                rotation: crate::Quat::IDENTITY,
                scale: Vec3::splat(0.18),
            })
            .add()
            .map_err(|err| {
                JsValue::from_str(&format!(
                    "add {} material sphere failed: {err:?}",
                    preset.id
                ))
            })?;
    }

    let bounds = Aabb::new(Vec3::new(-1.2, -0.52, -0.20), Vec3::new(1.2, 0.52, 0.20));
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
                .azimuth_elevation(0.0, 0.0)
                .fill(0.66)
                .margin_px(24.0)
                .viewport(viewport_width.max(1), viewport_height.max(1)),
        )
        .map_err(|err| {
            JsValue::from_str(&format!("material sphere frame_bounds failed: {err:?}"))
        })?;
    let controls = OrbitControls::from_framing(framing);
    scene
        .add_studio_lighting()
        .map_err(|err| JsValue::from_str(&format!("add_studio_lighting failed: {err:?}")))?;
    log_timing("load_material_spheres_scene total", total_start);

    Ok(DemoApp {
        assets,
        scene,
        camera,
        controls,
        controls_dirty: false,
        needs_prepare: true,
        renderer: None,
        connector_replay: None,
    })
}

#[wasm_bindgen]
pub async fn load_single_material_sphere_scene(
    preset_id: &str,
    viewport_width: u32,
    viewport_height: u32,
) -> Result<DemoApp, JsValue> {
    let total_start = now_ms();
    let assets = Assets::new();
    let mut scene = Scene::new();
    let sphere = assets.create_geometry(GeometryDesc::sphere(1.0, 72, 36));
    let backdrop = assets.create_geometry(material_studio_backdrop_geometry());
    scene
        .mesh(
            backdrop,
            assets.create_material(MaterialDesc::unlit(Color::WHITE)),
        )
        .add()
        .map_err(|err| {
            JsValue::from_str(&format!("add material studio backdrop failed: {err:?}"))
        })?;

    let preset = material_preset_showcase()
        .iter()
        .find(|preset| preset.id == preset_id)
        .copied()
        .ok_or_else(|| JsValue::from_str(&format!("unknown material preset: {preset_id}")))?;
    let material =
        match preset.id {
            "satin" => {
                assets.material_presets().satin().await.map_err(|err| {
                    JsValue::from_str(&format!("load satin preset failed: {err:?}"))
                })?
            }
            "leather" => assets.material_presets().leather().await.map_err(|err| {
                JsValue::from_str(&format!("load leather preset failed: {err:?}"))
            })?,
            "rubber" => {
                assets.material_presets().rubber().await.map_err(|err| {
                    JsValue::from_str(&format!("load rubber preset failed: {err:?}"))
                })?
            }
            _ => assets.create_material(preset.material_desc()),
        };
    scene
        .mesh(sphere, material)
        .transform(Transform {
            translation: Vec3::ZERO,
            rotation: crate::Quat::IDENTITY,
            scale: Vec3::splat(0.42),
        })
        .add()
        .map_err(|err| {
            JsValue::from_str(&format!(
                "add {} material sphere failed: {err:?}",
                preset.id
            ))
        })?;

    let bounds = Aabb::new(Vec3::new(-0.52, -0.52, -0.52), Vec3::new(0.52, 0.52, 0.52));
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
                .azimuth_elevation(0.0, 0.0)
                .fill(0.74)
                .margin_px(22.0)
                .viewport(viewport_width.max(1), viewport_height.max(1)),
        )
        .map_err(|err| {
            JsValue::from_str(&format!("material sphere frame_bounds failed: {err:?}"))
        })?;
    let controls = OrbitControls::from_framing(framing);
    scene
        .add_studio_lighting()
        .map_err(|err| JsValue::from_str(&format!("add_studio_lighting failed: {err:?}")))?;
    log_timing("load_single_material_sphere_scene total", total_start);

    Ok(DemoApp {
        assets,
        scene,
        camera,
        controls,
        controls_dirty: false,
        needs_prepare: true,
        renderer: None,
        connector_replay: None,
    })
}
