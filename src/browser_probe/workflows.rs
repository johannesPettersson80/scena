use serde_json::json;
use wasm_bindgen::prelude::JsValue;

mod compressed;
mod ergonomics;
mod pbr;

use crate::{
    AnimationPlaybackState, AssetCatalogAssetV1, AssetCatalogV1, Assets, Color, CursorPosition,
    GeometryDesc, HitTarget, LabelDesc, MaterialDesc, PerspectiveCamera, Primitive, Scene,
    Transform, Vec3, Vertex, Viewport,
};

const ASSET_CATALOG_FIXTURE: &str =
    include_str!("../../tests/assets/catalog/readiness_catalog.v1.json");
const ASSET_CATALOG_PREVIEW_ASSET_ID: &str = "variant-triangle";

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
        "depth-overlap" => Ok(depth_overlap_scene()?),
        "pbr-point-light" => Ok(pbr::point_light_scene()?),
        "pbr-spot-light" => Ok(pbr::spot_light_scene()?),
        "pbr-normal-map" => pbr::normal_map_scene().await,
        "pbr-environment" => Ok(pbr::environment_scene()?),
        "pbr-shadow-visibility" => Ok(pbr::shadow_visibility_scene()?),
        "pbr-material-extensions" => pbr::material_extensions_scene().await,
        "pbr-material-presets" => pbr::material_presets_scene().await,
        "compressed-assets" => compressed::compressed_assets_scene().await,
        "source-gltf-materials" => ergonomics::build_ergonomics_scene(workflow).await,
        "asset-catalog-preview" => asset_catalog_preview_scene().await,
        other => ergonomics::build_ergonomics_scene(other).await,
    }
}

async fn model_viewer_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let scene_asset = assets
        .load_scene("/fixtures/gltf/non_ndc_camera_scene.gltf")
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
            "source": "/fixtures/gltf/non_ndc_camera_scene.gltf",
            "roots": import.roots().len(),
            "framed": import.bounds_local().is_some(),
            "proof_class": "camera-framed-non-ndc",
        }),
    })
}

async fn asset_catalog_preview_scene() -> Result<WorkflowScene, JsValue> {
    let catalog: AssetCatalogV1 = serde_json::from_str(ASSET_CATALOG_FIXTURE)
        .map_err(|error| JsValue::from_str(&format!("asset catalog parse failed: {error}")))?;
    let asset = catalog
        .assets
        .iter()
        .find(|asset| asset.id == ASSET_CATALOG_PREVIEW_ASSET_ID)
        .ok_or_else(|| {
            JsValue::from_str(&format!(
                "asset catalog fixture missing {ASSET_CATALOG_PREVIEW_ASSET_ID}"
            ))
        })?;
    let preview = asset.preview.as_ref().ok_or_else(|| {
        JsValue::from_str(&format!(
            "asset catalog entry {} has no preview metadata",
            asset.id
        ))
    })?;
    if preview.kind != "generated" {
        return Err(JsValue::from_str(&format!(
            "asset catalog preview proof requires generated preview metadata, got {}",
            preview.kind
        )));
    }

    let source = browser_fixture_source(asset)?;
    let assets = Assets::new();
    let scene_asset = assets.load_scene(source.as_str()).await.map_err(|error| {
        JsValue::from_str(&format!(
            "asset catalog preview load failed for {source}: {error:?}"
        ))
    })?;
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).map_err(|error| {
        JsValue::from_str(&format!(
            "asset catalog preview instantiate failed for {source}: {error:?}"
        ))
    })?;
    let selected_variant = asset
        .material_requirements
        .required_variants
        .first()
        .cloned();
    if let Some(name) = selected_variant.as_deref() {
        scene
            .set_active_variant(&import, Some(name))
            .map_err(|error| {
                JsValue::from_str(&format!(
                    "asset catalog preview variant {name} failed: {error:?}"
                ))
            })?;
    }
    let camera = add_default_camera(&mut scene)?;
    let framed = if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).map_err(|error| {
            JsValue::from_str(&format!(
                "asset catalog preview frame failed for {source}: {error:?}"
            ))
        })?;
        true
    } else {
        false
    };

    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "asset-catalog-preview",
            "catalog_schema": catalog.schema,
            "asset_id": asset.id,
            "display_name": asset.display_name,
            "catalog_source": asset.source,
            "source": source,
            "preview_kind": preview.kind,
            "preview_width": preview.width,
            "preview_height": preview.height,
            "preview_background": preview.background,
            "required_variants": asset.material_requirements.required_variants,
            "selected_variant": selected_variant,
            "active_variant": import.active_variant(),
            "roots": import.roots().len(),
            "framed": framed,
        }),
    })
}

fn browser_fixture_source(asset: &AssetCatalogAssetV1) -> Result<String, JsValue> {
    asset
        .source
        .strip_prefix("tests/assets/")
        .map(|path| format!("/fixtures/{path}"))
        .ok_or_else(|| {
            JsValue::from_str(&format!(
                "asset catalog preview source must be under tests/assets for browser proof: {}",
                asset.source
            ))
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
    for index in 0..12 {
        let col = index % 4;
        let row = index / 4;
        let label = if index % 2 == 0 {
            LabelDesc::new(format!("S{index:02}"))
        } else {
            LabelDesc::new(format!("M{index:02}"))
        }
        .with_color(Color::from_srgb_u8(245, 250, 255))
        .with_size(13.0)
        .with_background(Color::from_srgb_u8(5, 12, 22));
        scene
            .add_label(
                scene.root(),
                label,
                Transform {
                    translation: Vec3::new(-0.45 + col as f32 * 0.3, 0.35 - row as f32 * 0.25, 0.0),
                    ..Transform::default()
                },
            )
            .map_err(|error| JsValue::from_str(&format!("label insert failed: {error:?}")))?;
    }
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "helpers": ["axes"],
            "labels": 12,
            "proof_class": "browser-truetype-labels",
            "rasterization": "truetype-atlas-aa",
        }),
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

fn depth_overlap_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let camera = add_default_camera(&mut scene)?;
    scene
        .add_renderable(
            scene.root(),
            vec![
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.6, -0.6, 0.35),
                        color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                    },
                    Vertex {
                        position: Vec3::new(0.6, -0.6, 0.35),
                        color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                    },
                    Vertex {
                        position: Vec3::new(0.0, 0.6, 0.35),
                        color: Color::from_linear_rgb(0.0, 1.0, 0.0),
                    },
                ]),
                Primitive::triangle([
                    Vertex {
                        position: Vec3::new(-0.6, -0.6, -0.35),
                        color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                    },
                    Vertex {
                        position: Vec3::new(0.6, -0.6, -0.35),
                        color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                    },
                    Vertex {
                        position: Vec3::new(0.0, 0.6, -0.35),
                        color: Color::from_linear_rgb(1.0, 0.0, 0.0),
                    },
                ]),
            ],
            Transform::default(),
        )
        .map_err(|error| JsValue::from_str(&format!("depth overlap insert failed: {error:?}")))?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "proof_class": "depth-overlap-near-wins",
            "near_color": "green",
            "far_color": "red",
            "submission_order": "near-then-far",
        }),
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
