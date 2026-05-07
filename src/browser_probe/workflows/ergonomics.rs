use serde_json::json;
use wasm_bindgen::prelude::JsValue;

use super::{WorkflowScene, add_default_camera};
use crate::{
    Aabb, AlphaMode, Assets, Color, DiagnosticSeverity, DirectionalLight, GeometryDesc,
    MaterialDesc, Primitive, Renderer, RetainPolicy, Scene, SourceCoordinateSystem, SourceUnits,
    TextureColorSpace, TextureTransform, Transform, Vec3,
};

pub(super) async fn build_ergonomics_scene(workflow: &str) -> Result<WorkflowScene, JsValue> {
    match workflow {
        "camera-framing" => camera_framing_scene(),
        "anchor-alignment" => anchor_alignment_scene().await,
        "coordinate-units" => coordinate_units_scene(),
        "static-batching" => static_batching_scene(),
        "layers-helper-on-top" => layers_helper_on_top_scene(),
        "beginner-diagnostics" => beginner_diagnostics_scene(),
        "material-textures" => material_textures_scene().await,
        "asset-cache-reload" => asset_cache_reload_scene().await,
        other => Err(JsValue::from_str(&format!(
            "unknown M6 browser workflow probe: {other}"
        ))),
    }
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

async fn material_textures_scene() -> Result<WorkflowScene, JsValue> {
    let assets = Assets::new();
    let base = assets
        .load_texture("/fixtures/textures/m8-base.png", TextureColorSpace::Srgb)
        .await
        .map_err(|error| JsValue::from_str(&format!("base texture failed: {error:?}")))?;
    let normal = assets
        .load_texture(
            "/fixtures/textures/m8-normal.png",
            TextureColorSpace::Linear,
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("normal texture failed: {error:?}")))?;
    let metallic_roughness = assets
        .load_texture(
            "/fixtures/textures/m8-metallic-roughness.png",
            TextureColorSpace::Linear,
        )
        .await
        .map_err(|error| {
            JsValue::from_str(&format!("metallic-roughness texture failed: {error:?}"))
        })?;
    let occlusion = assets
        .load_texture(
            "/fixtures/textures/m8-occlusion.png",
            TextureColorSpace::Linear,
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("occlusion texture failed: {error:?}")))?;
    let emissive = assets
        .load_texture(
            "/fixtures/textures/m8-emissive.png",
            TextureColorSpace::Srgb,
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("emissive texture failed: {error:?}")))?;

    let material = assets.create_material(
        MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(170, 210, 255), 0.2, 0.65)
            .with_base_color_texture(base)
            .with_base_color_texture_transform(TextureTransform::new(
                [0.25, 0.5],
                0.0,
                [1.0, 1.0],
                None,
            ))
            .with_normal_texture(normal)
            .with_normal_texture_transform(TextureTransform::new(
                [0.0, 0.0],
                0.0,
                [1.0, 1.0],
                Some(1),
            ))
            .with_metallic_roughness_texture(metallic_roughness)
            .with_occlusion_texture(occlusion)
            .with_emissive_texture(emissive)
            .with_emissive(Color::from_linear_rgb(0.02, 0.04, 0.08))
            .with_emissive_strength(1.5)
            .with_alpha_mode(AlphaMode::Blend)
            .with_double_sided(true),
    );
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.5, 0.5, 0.5));
    let mut scene = Scene::new();
    scene
        .mesh(geometry, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("material mesh failed: {error:?}")))?;
    scene
        .directional_light(DirectionalLight::default())
        .add()
        .map_err(|error| JsValue::from_str(&format!("material light failed: {error:?}")))?;
    let camera = add_default_camera(&mut scene)?;
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "base_color_texture": true,
            "normal_texture": true,
            "metallic_roughness_texture": true,
            "occlusion_texture": true,
            "emissive_texture": true,
            "alpha": "Blend",
            "double_sided": true,
            "texture_transform": true,
        }),
    })
}

async fn asset_cache_reload_scene() -> Result<WorkflowScene, JsValue> {
    let mut assets = Assets::new();
    assets.set_retain_policy(RetainPolicy::Always);
    let first = assets
        .load_scene_with_report("/fixtures/gltf/mesh_material_vertex_color_scene.gltf")
        .await
        .map_err(|error| JsValue::from_str(&format!("browser first load failed: {error:?}")))?;
    let cached = assets
        .load_scene_with_report("/fixtures/gltf/mesh_material_vertex_color_scene.gltf")
        .await
        .map_err(|error| JsValue::from_str(&format!("browser cached load failed: {error:?}")))?;
    let reloaded = assets
        .reload_scene(first.asset())
        .await
        .map_err(|error| JsValue::from_str(&format!("browser reload failed: {error:?}")))?;
    let texture_a = assets
        .load_texture(
            "/fixtures/textures/browser-cache.png",
            TextureColorSpace::Srgb,
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("browser texture A failed: {error:?}")))?;
    let texture_b = assets
        .load_texture(
            "/fixtures/textures/browser-cache.png",
            TextureColorSpace::Srgb,
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("browser texture B failed: {error:?}")))?;
    let texture_linear = assets
        .load_texture(
            "/fixtures/textures/browser-cache.png",
            TextureColorSpace::Linear,
        )
        .await
        .map_err(|error| JsValue::from_str(&format!("browser texture linear failed: {error:?}")))?;

    let mut scene = Scene::new();
    let import = scene.instantiate(&reloaded).map_err(|error| {
        JsValue::from_str(&format!("browser reload instantiate failed: {error:?}"))
    })?;
    let camera = add_default_camera(&mut scene)?;
    if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).map_err(|error| {
            JsValue::from_str(&format!("browser cache frame failed: {error:?}"))
        })?;
    }
    Ok(WorkflowScene {
        assets,
        scene,
        camera,
        metadata: json!({
            "first_cache_hit": first.cache_hit(),
            "cached_cache_hit": cached.cache_hit(),
            "first_fetched_bytes": first.fetched_bytes(),
            "cached_fetched_bytes": cached.fetched_bytes(),
            "reload_node_count": reloaded.node_count(),
            "texture_dedup": texture_a == texture_b,
            "texture_color_space_split": texture_a != texture_linear,
            "retain_policy": "Always",
        }),
    })
}
