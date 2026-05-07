use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use crate::{
    Aabb, AnimationPlaybackState, Assets, Color, CursorPosition, DiagnosticSeverity, GeometryDesc,
    HitTarget, LabelDesc, MaterialDesc, PerspectiveCamera, Primitive, Renderer, Scene,
    SourceCoordinateSystem, SourceUnits, Transform, Vec3, Vertex, Viewport,
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
        "camera-framing" => Ok(camera_framing_scene()?),
        "anchor-alignment" => anchor_alignment_scene().await,
        "coordinate-units" => Ok(coordinate_units_scene()?),
        "static-batching" => Ok(static_batching_scene()?),
        "layers-helper-on-top" => Ok(layers_helper_on_top_scene()?),
        "beginner-diagnostics" => Ok(beginner_diagnostics_scene()?),
        other => Err(JsValue::from_str(&format!(
            "unknown M6 browser workflow probe: {other}"
        ))),
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

fn camera_framing_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(1.2, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 160, 240)));
    let mut scene = Scene::new();
    let inspected_part = scene
        .mesh(geometry, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("camera framing mesh failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    let bounds = Aabb::new(Vec3::new(-0.6, -0.2, -0.2), Vec3::new(0.6, 0.2, 0.2));
    scene
        .frame(camera, bounds)
        .map_err(|error| JsValue::from_str(&format!("camera frame failed: {error:?}")))?;
    scene
        .look_at(camera, inspected_part)
        .map_err(|error| JsValue::from_str(&format!("camera look_at failed: {error:?}")))?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({ "bounds": "box", "framed": true, "look_at": true }),
    })
}

async fn anchor_alignment_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let scene_asset = assets
        .load_scene("/fixtures/gltf/anchor_debug_scene.gltf")
        .await
        .map_err(|error| JsValue::from_str(&format!("anchor fixture load failed: {error:?}")))?;
    let marker_geometry = assets.create_geometry(GeometryDesc::anchor_marker(0.2));
    let marker_material =
        assets.create_material(MaterialDesc::line(Color::from_srgb_u8(255, 220, 70), 1.0));
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .map_err(|error| JsValue::from_str(&format!("anchor instantiate failed: {error:?}")))?;
    let marker = scene
        .mesh(marker_geometry, marker_material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("anchor marker failed: {error:?}")))?;
    scene
        .snap_anchor(
            marker,
            import
                .anchor("inspection")
                .map_err(|error| JsValue::from_str(&format!("anchor lookup failed: {error:?}")))?,
        )
        .map_err(|error| JsValue::from_str(&format!("anchor snap failed: {error:?}")))?;
    let anchor_debug = import
        .anchor_debug_metadata()
        .map_err(|error| JsValue::from_str(&format!("anchor debug failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    if let Some(bounds) = import.bounds_world(&scene) {
        scene
            .frame(camera, bounds)
            .map_err(|error| JsValue::from_str(&format!("anchor frame failed: {error:?}")))?;
    }
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({ "anchor": "inspection", "anchor_debug_count": anchor_debug.len() }),
    })
}

fn coordinate_units_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.12, 0.12, 0.12));
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 230, 90)));
    let cad_position_mm = Vec3::new(250.0, 0.0, 100.0);
    let meters_per_unit = SourceUnits::Millimeters.meters_per_unit();
    let y_up_position = SourceCoordinateSystem::ZUpRightHanded.convert_position(cad_position_mm);
    let render_position = Vec3::new(
        y_up_position.x * meters_per_unit,
        y_up_position.y * meters_per_unit,
        y_up_position.z * meters_per_unit,
    );
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .transform(Transform::at(render_position))
        .add()
        .map_err(|error| JsValue::from_str(&format!("converted mesh failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    scene
        .look_at_point(camera, render_position)
        .map_err(|error| JsValue::from_str(&format!("converted look_at failed: {error:?}")))?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "source_units": "millimeters",
            "coordinate_system": "ZUpRightHanded",
            "meters_per_unit": meters_per_unit,
        }),
    })
}

fn static_batching_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let source = GeometryDesc::box_xyz(0.12, 0.12, 0.12);
    let transforms = (0..12).map(|index| {
        Transform::at(Vec3::new(
            (index % 6) as f32 * 0.18 - 0.45,
            (index / 6) as f32 * 0.18 - 0.09,
            0.0,
        ))
    });
    let (batch, report) = assets.create_static_batch_with_report(&source, transforms);
    let material = assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 200, 60)));
    let mut scene = Scene::new();
    scene
        .mesh(batch, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("static batch mesh failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "instances": report.instance_count(),
            "vertices": report.output_vertices(),
            "requires_prepare_after_rebuild": report.requires_prepare_after_rebuild(),
            "picking_debug_instances": report.picking_debug_instances(),
        }),
    })
}

fn layers_helper_on_top_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.3, 0.3, 0.3));
    let visible_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(80, 170, 255)));
    let helper_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(255, 230, 80)));
    let mut scene = Scene::new();
    let machine = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(-0.25, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("machine mesh failed: {error:?}")))?;
    let helper = scene
        .mesh(geometry, helper_material)
        .transform(Transform::at(Vec3::new(0.25, 0.0, 0.0)).scale_by(0.5))
        .add()
        .map_err(|error| JsValue::from_str(&format!("helper mesh failed: {error:?}")))?;
    let hidden = scene
        .mesh(geometry, visible_material)
        .transform(Transform::at(Vec3::new(0.0, 0.4, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("hidden mesh failed: {error:?}")))?;
    scene
        .add_tag(machine, "operational")
        .map_err(|error| JsValue::from_str(&format!("tag failed: {error:?}")))?;
    scene
        .set_layer_mask(machine, 0b0001)
        .map_err(|error| JsValue::from_str(&format!("machine layer failed: {error:?}")))?;
    scene
        .set_layer_mask(helper, 0b0001)
        .map_err(|error| JsValue::from_str(&format!("helper layer failed: {error:?}")))?;
    scene
        .set_layer_mask(hidden, 0b0010)
        .map_err(|error| JsValue::from_str(&format!("hidden layer failed: {error:?}")))?;
    scene
        .set_visible(hidden, false)
        .map_err(|error| JsValue::from_str(&format!("hidden visibility failed: {error:?}")))?;
    scene
        .set_render_group(helper, 10)
        .map_err(|error| JsValue::from_str(&format!("helper group failed: {error:?}")))?;
    scene
        .set_helper_on_top(helper, true)
        .map_err(|error| JsValue::from_str(&format!("helper on top failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    scene
        .set_camera_layer_mask(camera, 0b0001)
        .map_err(|error| JsValue::from_str(&format!("camera layer failed: {error:?}")))?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "tagged_operational": 1,
            "helper_on_top": true,
            "camera_layer_mask": 1,
        }),
    })
}

fn beginner_diagnostics_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let diagnostic_renderer = Renderer::headless(16, 16)
        .map_err(|error| JsValue::from_str(&format!("diagnostic renderer failed: {error:?}")))?;
    let errors = diagnostic_renderer
        .diagnose_scene(&scene)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .count();
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .map_err(|error| JsValue::from_str(&format!("diagnostic triangle failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({ "initial_error_diagnostics": errors, "recovered": true }),
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
