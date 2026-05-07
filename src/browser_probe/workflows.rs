use serde_json::json;
use wasm_bindgen::prelude::JsValue;

mod ergonomics;

use crate::{
    AnimationPlaybackState, Assets, Color, CursorPosition, GeometryDesc, HitTarget, LabelDesc,
    MaterialDesc, PerspectiveCamera, Primitive, Scene, Transform, Vec3, Vertex, Viewport,
};

pub(super) struct WorkflowScene {
    pub(super) assets: Assets,
    pub(super) scene: Scene,
    pub(super) camera: crate::CameraKey,
    pub(super) metadata: serde_json::Value,
}

pub(super) async fn build_workflow_scene(workflow: &str) -> Result<WorkflowScene, JsValue> {
    match workflow {
        "model-viewer" => model_viewer_scene().await,
        "instancing" => Ok(instancing_scene()),
        "picking-selection" => Ok(picking_selection_scene()?),
        "animation" => animation_scene().await,
        "labels-helpers" => Ok(labels_helpers_scene()?),
        "industrial-static-scene" => Ok(industrial_static_scene()?),
        other => ergonomics::build_ergonomics_scene(other).await,
    }
}

async fn model_viewer_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let scene_asset = assets
        .load_scene("/fixtures/gltf/mesh_material_vertex_color_scene.gltf")
        .await
        .map_err(|error| JsValue::from_str(&format!("model-viewer load failed: {error:?}")))?;
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).map_err(|error| {
        JsValue::from_str(&format!("model-viewer instantiate failed: {error:?}"))
    })?;
    let camera = add_default_camera(&mut scene)?;
    if let Some(bounds) = import.bounds_world(&scene) {
        scene
            .frame(camera, bounds)
            .map_err(|error| JsValue::from_str(&format!("model-viewer frame failed: {error:?}")))?;
    }
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "source": "/fixtures/gltf/mesh_material_vertex_color_scene.gltf",
            "roots": import.roots().len(),
            "framed": import.bounds_local().is_some(),
        }),
    })
}

fn instancing_scene() -> WorkflowScene {
    instancing_scene_with_count(10)
}

pub(super) fn instancing_scene_with_count(instance_count: usize) -> WorkflowScene {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.2, 0.2, 0.2));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 220, 160)));
    let mut scene = Scene::new();
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("probe instance set inserts");
    scene
        .reserve_instances(set, instance_count)
        .expect("probe reserves instances");
    for index in 0..instance_count {
        let x = if instance_count <= 1 {
            0.0
        } else {
            -0.9 + 1.8 * (index as f32 / (instance_count - 1) as f32)
        };
        scene
            .push_instance(
                set,
                Transform {
                    translation: Vec3::new(x, 0.0, 0.0),
                    scale: Vec3::new(1.0, 1.0, 1.0),
                    ..Transform::default()
                },
            )
            .expect("probe instance inserts");
    }
    let camera = add_default_camera(&mut scene).expect("probe camera inserts");
    WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({ "instances": instance_count }),
    }
}

pub(super) fn picking_selection_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let node = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .map_err(|error| JsValue::from_str(&format!("picking node insert failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    let viewport =
        Viewport::new(64, 64, 1.0).ok_or_else(|| JsValue::from_str("invalid picking viewport"))?;
    let hit = scene
        .pick(camera, CursorPosition::physical(32.0, 32.0), viewport)
        .map_err(|error| JsValue::from_str(&format!("picking query failed: {error:?}")))?;
    if let Some(hit) = hit {
        scene.interaction_mut().set_hover(Some(hit.target()));
        scene
            .interaction_mut()
            .set_primary_selection(Some(HitTarget::Node(node)));
    }
    let hover = scene.interaction().hover().is_some();
    let selection = scene.interaction().primary_selection().is_some();
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "picked": hit.is_some(),
            "hover": hover,
            "selection": selection,
        }),
    })
}

pub(super) async fn animation_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let scene_asset = assets
        .load_scene("/fixtures/gltf/khronos/MorphCube/AnimatedMorphCube.gltf")
        .await
        .map_err(|error| JsValue::from_str(&format!("animation load failed: {error:?}")))?;
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .map_err(|error| JsValue::from_str(&format!("animation instantiate failed: {error:?}")))?;
    let mixer = scene
        .create_animation_mixer(&import, "Square")
        .map_err(|error| JsValue::from_str(&format!("animation mixer failed: {error:?}")))?;
    scene
        .play_animation(mixer)
        .map_err(|error| JsValue::from_str(&format!("animation play failed: {error:?}")))?;
    scene
        .update_animation(mixer, 1.0 / 30.0)
        .map_err(|error| JsValue::from_str(&format!("animation update failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    if let Some(bounds) = import.bounds_world(&scene) {
        scene
            .frame(camera, bounds)
            .map_err(|error| JsValue::from_str(&format!("animation frame failed: {error:?}")))?;
    }
    let state = scene
        .animation_mixer(mixer)
        .map_err(|error| JsValue::from_str(&format!("animation state failed: {error:?}")))?
        .state();
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "clip": "Square",
            "state": format!("{state:?}"),
            "playing": state == AnimationPlaybackState::Playing,
        }),
    })
}

fn labels_helpers_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let axes = assets.create_geometry(GeometryDesc::axes(1.0));
    let material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(200, 220, 255), 1.0));
    let mut scene = Scene::new();
    scene
        .mesh(axes, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("axes helper insert failed: {error:?}")))?;
    scene
        .add_label(
            scene.root(),
            LabelDesc::msdf("origin")
                .with_color(Color::WHITE)
                .with_size(14.0),
            Transform {
                translation: Vec3::new(0.0, 0.15, 0.0),
                ..Transform::default()
            },
        )
        .map_err(|error| JsValue::from_str(&format!("label insert failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({ "helpers": ["axes"], "labels": 1 }),
    })
}

fn industrial_static_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let floor = assets.create_geometry(GeometryDesc::grid(10.0, 20));
    let material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(110, 130, 150), 1.0));
    let mut scene = Scene::new();
    scene
        .mesh(floor, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("industrial grid insert failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({ "profile": "industrial-static-scene", "grid": true }),
    })
}

pub(super) fn scene_with_triangle() -> (Scene, crate::CameraKey) {
    let mut scene = Scene::new();
    let camera = add_default_camera(&mut scene).expect("probe camera inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::triangle([
                Vertex {
                    position: Vec3::new(-0.8, -0.6, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.8, -0.6, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
                Vertex {
                    position: Vec3::new(0.0, 0.8, 0.0),
                    color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                },
            ])],
            Transform::default(),
        )
        .expect("probe triangle inserts");
    (scene, camera)
}

fn add_default_camera(scene: &mut Scene) -> Result<crate::CameraKey, JsValue> {
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform {
                translation: Vec3::new(0.0, 0.0, 2.0),
                ..Transform::default()
            },
        )
        .map_err(|error| JsValue::from_str(&format!("camera insert failed: {error:?}")))?;
    scene
        .set_active_camera(camera)
        .map_err(|error| JsValue::from_str(&format!("set active camera failed: {error:?}")))?;
    Ok(camera)
}
