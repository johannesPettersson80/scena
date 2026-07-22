use serde_json::{Map, json};
use wasm_bindgen::prelude::JsValue;
use web_sys::HtmlCanvasElement;

use crate::{
    Assets, Backend, Color, GeometryDesc, HitTarget, MaterialDesc, NotPreparedReason, RenderError,
    RenderMode, Renderer, RendererOptions, Scene, SurfaceEvent, Transform, Vec3,
};

use super::super::renderer_readback_json;
use super::super::report::{capabilities_json, diagnostics_json, stats_json};

pub(in crate::browser_probe) async fn render_state_lifecycle_probe(
    canvas: HtmlCanvasElement,
    backend: Backend,
) -> Result<String, JsValue> {
    let mut renderer = Renderer::from_surface_async_with_options(
        super::browser_surface(&canvas, backend, "browser state lifecycle probe")?,
        RendererOptions::default().with_render_mode(RenderMode::OnChange),
    )
    .await
    .map_err(|error| JsValue::from_str(&format!("state lifecycle build failed: {error:?}")))?;

    let mut events = Vec::new();
    let lifetime = verify_resource_lifetime(&mut renderer, backend, &mut events)?;
    let assets = Assets::new();
    let base_geometry = assets.create_geometry(GeometryDesc::box_xyz(0.35, 0.35, 0.35));
    let base_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(70, 200, 180)));
    let accent_material =
        assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(240, 110, 90)));
    let mut scene = Scene::new();
    let node = scene
        .mesh(base_geometry, base_material)
        .transform(Transform::at(Vec3::new(-0.25, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("state mesh insert failed: {error:?}")))?;
    let camera = scene
        .add_default_camera()
        .map_err(|error| JsValue::from_str(&format!("state camera insert failed: {error:?}")))?;

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .map_err(|error| JsValue::from_str(&format!("state prepare failed: {error:?}")))?;
    let first_render = renderer
        .render(&scene, camera)
        .map_err(|error| JsValue::from_str(&format!("state render failed: {error:?}")))?;
    let idle_render = renderer
        .render(&scene, camera)
        .map_err(|error| JsValue::from_str(&format!("state idle render failed: {error:?}")))?;
    if !idle_render.skipped {
        return Err(JsValue::from_str(
            "idle-render-skipped: OnChange render should skip unchanged browser frame",
        ));
    }
    events.push("idle-render-skipped");

    let mut dirty_state = Map::new();

    scene
        .set_transform(node, Transform::at(Vec3::new(0.1, 0.05, 0.0)))
        .map_err(|error| JsValue::from_str(&format!("dirty transform failed: {error:?}")))?;
    dirty_state.insert(
        "transform".to_owned(),
        json!(expect_scene_changed(
            renderer.render(&scene, camera),
            "dirty-transform"
        )?),
    );
    reprepare_and_render(
        &mut renderer,
        &assets,
        &mut scene,
        camera,
        "dirty-transform",
    )?;
    events.push("dirty-transform");

    scene
        .mesh(base_geometry, accent_material)
        .transform(Transform::at(Vec3::new(0.35, 0.0, 0.0)))
        .add()
        .map_err(|error| JsValue::from_str(&format!("dirty material insert failed: {error:?}")))?;
    dirty_state.insert(
        "material".to_owned(),
        json!(expect_scene_changed(
            renderer.render(&scene, camera),
            "dirty-material"
        )?),
    );
    reprepare_and_render(&mut renderer, &assets, &mut scene, camera, "dirty-material")?;
    events.push("dirty-material");

    let instance_set = scene
        .add_instance_set(
            scene.root(),
            base_geometry,
            base_material,
            Transform::default(),
        )
        .map_err(|error| JsValue::from_str(&format!("dirty instance set failed: {error:?}")))?;
    scene
        .push_instance(instance_set, Transform::at(Vec3::new(-0.6, -0.1, 0.0)))
        .map_err(|error| JsValue::from_str(&format!("dirty instance failed: {error:?}")))?;
    dirty_state.insert(
        "instance".to_owned(),
        json!(expect_scene_changed(
            renderer.render(&scene, camera),
            "dirty-instance"
        )?),
    );
    reprepare_and_render(&mut renderer, &assets, &mut scene, camera, "dirty-instance")?;
    events.push("dirty-instance");

    let camera_node = scene
        .camera_node(camera)
        .ok_or_else(|| JsValue::from_str("dirty-camera: camera node not found"))?;
    scene
        .set_transform(camera_node, Transform::at(Vec3::new(0.0, 0.0, 2.4)))
        .map_err(|error| JsValue::from_str(&format!("dirty camera failed: {error:?}")))?;
    dirty_state.insert(
        "camera".to_owned(),
        json!(expect_scene_changed(
            renderer.render(&scene, camera),
            "dirty-camera"
        )?),
    );
    reprepare_and_render(&mut renderer, &assets, &mut scene, camera, "dirty-camera")?;
    events.push("dirty-camera");

    canvas.set_width(96);
    canvas.set_height(64);
    renderer
        .handle_surface_event(SurfaceEvent::Resize {
            width: 96,
            height: 64,
        })
        .map_err(|error| JsValue::from_str(&format!("dirty resize failed: {error:?}")))?;
    renderer
        .handle_surface_event(SurfaceEvent::ScaleFactorChanged { scale_factor: 2.0 })
        .map_err(|error| JsValue::from_str(&format!("dirty DPR failed: {error:?}")))?;
    dirty_state.insert(
        "resize_dpr".to_owned(),
        json!(expect_target_changed(
            renderer.render(&scene, camera),
            "dirty-resize-dpr"
        )?),
    );
    let resized_render = reprepare_and_render(
        &mut renderer,
        &assets,
        &mut scene,
        camera,
        "dirty-resize-dpr",
    )?;
    events.push("dirty-resize-dpr");

    scene
        .interaction_mut()
        .set_hover(Some(HitTarget::Node(node)));
    scene
        .interaction_mut()
        .set_primary_selection(Some(HitTarget::Node(node)));
    dirty_state.insert(
        "hover_selection".to_owned(),
        json!(expect_scene_changed(
            renderer.render(&scene, camera),
            "dirty-hover-selection"
        )?),
    );
    let selection_render = reprepare_and_render(
        &mut renderer,
        &assets,
        &mut scene,
        camera,
        "dirty-hover-selection",
    )?;
    events.push("dirty-hover-selection");

    let animation = verify_animation_dirty(&mut renderer, &mut events).await?;
    dirty_state.insert(
        "animation_mixer".to_owned(),
        animation["dirty_reason"].clone(),
    );
    let recovery =
        super::verify_context_recovery(&mut renderer, &assets, &mut scene, camera, &mut events)?;
    dirty_state.insert(
        "context_recovery".to_owned(),
        json!({
            "context": recovery,
            "device": null,
        }),
    );
    events.push("context-recovery");
    let renderer_readback = renderer
        .browser_readback_rgba8()
        .await?
        .map(|readback| renderer_readback_json(&readback));

    let stats = renderer.stats();
    let capabilities = renderer.capabilities();
    Ok(json!({
        "schema": "scena.m6.browser_state_lifecycle_probe.v1",
        "status": "rendered",
        "workflow": "state-lifetime-idle",
        "event_sequence": events,
        "dirty_state": dirty_state,
        "resource_lifetime": lifetime,
        "allocation_steady_state": {
            "render_mode": "OnChange",
            "idle_render_skipped": idle_render.skipped,
            "idle_draw_calls": idle_render.draw_calls,
            "skipped_frames": stats.skipped_frames,
        },
        "animation": animation,
        "initial": {
            "draw_calls": first_render.draw_calls,
            "primitives": first_render.primitives,
        },
        "resized": {
            "width": resized_render.width,
            "height": resized_render.height,
            "draw_calls": resized_render.draw_calls,
        },
        "selection": {
            "draw_calls": selection_render.draw_calls,
        },
        "capabilities": capabilities_json(*capabilities),
        "diagnostics": diagnostics_json(renderer.diagnostics()),
        "backend": format!("{:?}", capabilities.backend),
        "gpu_device": capabilities.gpu_device,
        "surface_attached": capabilities.surface_attached,
        "width": selection_render.width,
        "height": selection_render.height,
        "draw_calls": selection_render.draw_calls,
        "primitives": selection_render.primitives,
        "stats": stats_json(stats),
        "gpu_submissions": stats.gpu_submissions,
        "prepared_buffers": stats.buffers,
        "prepared_pipelines": stats.pipelines,
        "prepared_bind_groups": stats.bind_groups,
        "renderer_readback": renderer_readback,
    })
    .to_string())
}

fn verify_resource_lifetime(
    renderer: &mut Renderer,
    backend: Backend,
    events: &mut Vec<&'static str>,
) -> Result<serde_json::Value, JsValue> {
    let empty_assets = Assets::new();
    let mut empty_scene = Scene::new();
    empty_scene
        .add_default_camera()
        .map_err(|error| JsValue::from_str(&format!("lifetime camera insert failed: {error:?}")))?;
    renderer
        .prepare_with_assets(&mut empty_scene, &empty_assets)
        .map_err(|error| {
            JsValue::from_str(&format!("lifetime baseline prepare failed: {error:?}"))
        })?;
    let baseline_poll = renderer.poll_device();
    let baseline = renderer.stats();

    let heavy_assets = Assets::new();
    let geometry = heavy_assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material =
        heavy_assets.create_material(MaterialDesc::unlit(Color::from_srgb_u8(120, 180, 255)));
    let mut heavy_scene = Scene::new();
    heavy_scene
        .mesh(geometry, material)
        .add()
        .map_err(|error| JsValue::from_str(&format!("lifetime mesh insert failed: {error:?}")))?;
    let heavy_camera = heavy_scene
        .add_default_camera()
        .map_err(|error| JsValue::from_str(&format!("lifetime heavy camera failed: {error:?}")))?;
    renderer
        .prepare_with_assets(&mut heavy_scene, &heavy_assets)
        .map_err(|error| JsValue::from_str(&format!("lifetime heavy prepare failed: {error:?}")))?;
    renderer
        .render(&heavy_scene, heavy_camera)
        .map_err(|error| JsValue::from_str(&format!("lifetime heavy render failed: {error:?}")))?;
    let heavy = renderer.stats();
    if heavy.live_logical_handles <= baseline.live_logical_handles {
        return Err(JsValue::from_str(
            "resource-lifetime: prepared scene should increase live logical handles",
        ));
    }

    renderer
        .prepare_with_assets(&mut empty_scene, &empty_assets)
        .map_err(|error| {
            JsValue::from_str(&format!("lifetime release prepare failed: {error:?}"))
        })?;
    let submitted_poll = renderer.poll_device();
    let submitted_poll_observation = renderer.browser_device_poll_observation();
    let completion_poll = submitted_poll;
    let completion_poll_observation = submitted_poll_observation;
    let released = renderer.stats();
    if released.live_logical_handles != baseline.live_logical_handles {
        return Err(JsValue::from_str(
            "resource-lifetime: live logical handles should return to baseline",
        ));
    }
    if released.pending_destructions != baseline.pending_destructions {
        return Err(JsValue::from_str(&format!(
            "resource-lifetime: browser-managed logical destructions did not return to baseline; submitted={:?}/{submitted_poll_observation}, last={:?}/{completion_poll_observation}",
            submitted_poll.status, completion_poll.status,
        )));
    }
    events.push("resource-lifetime");

    Ok(json!({
        "baseline_live_logical_handles": baseline.live_logical_handles,
        "heavy_live_logical_handles": heavy.live_logical_handles,
        "released_live_logical_handles": released.live_logical_handles,
        "baseline_pending_destructions": baseline.pending_destructions,
        "released_pending_destructions": released.pending_destructions,
        "pending_returned_to_baseline": released.pending_destructions == baseline.pending_destructions,
        "baseline_poll_status": format!("{:?}", baseline_poll.status),
        "submitted_poll_status": format!("{:?}", submitted_poll.status),
        "submitted_wgpu_poll_observation": submitted_poll_observation,
        "completion_poll_status": format!("{:?}", completion_poll.status),
        "completion_wgpu_poll_observation": completion_poll_observation,
        "completion_confirmed": completion_poll.status == crate::DevicePollStatus::Confirmed,
        "retirement_mode": if backend == Backend::WebGl2 {
            "automatic-webgl2"
        } else {
            "automatic-webgpu"
        },
    }))
}

async fn verify_animation_dirty(
    renderer: &mut Renderer,
    events: &mut Vec<&'static str>,
) -> Result<serde_json::Value, JsValue> {
    let assets = Assets::new();
    let scene_asset = assets
        .load_scene("/fixtures/gltf/khronos/MorphCube/AnimatedMorphCube.gltf")
        .await
        .map_err(|error| JsValue::from_str(&format!("dirty animation load failed: {error:?}")))?;
    let mut scene = Scene::new();
    let import = scene.instantiate(&scene_asset).map_err(|error| {
        JsValue::from_str(&format!("dirty animation instantiate failed: {error:?}"))
    })?;
    let mixer = scene
        .create_animation_mixer(&import, "Square")
        .map_err(|error| JsValue::from_str(&format!("dirty animation mixer failed: {error:?}")))?;
    scene
        .play_animation(mixer)
        .map_err(|error| JsValue::from_str(&format!("dirty animation play failed: {error:?}")))?;
    let camera = scene
        .add_default_camera()
        .map_err(|error| JsValue::from_str(&format!("dirty animation camera failed: {error:?}")))?;
    if let Some(bounds) = import.bounds_world(&scene) {
        scene.frame(camera, bounds).map_err(|error| {
            JsValue::from_str(&format!("dirty animation frame failed: {error:?}"))
        })?;
    }
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .map_err(|error| {
            JsValue::from_str(&format!("dirty animation prepare failed: {error:?}"))
        })?;
    renderer
        .render(&scene, camera)
        .map_err(|error| JsValue::from_str(&format!("dirty animation render failed: {error:?}")))?;
    scene
        .update_animation(mixer, 1.0 / 30.0)
        .map_err(|error| JsValue::from_str(&format!("dirty animation update failed: {error:?}")))?;
    let dirty_reason =
        expect_scene_changed(renderer.render(&scene, camera), "dirty-animation-mixer")?;
    let render = reprepare_and_render(
        renderer,
        &assets,
        &mut scene,
        camera,
        "dirty-animation-mixer",
    )?;
    events.push("dirty-animation-mixer");

    Ok(json!({
        "clip": "Square",
        "dirty_reason": dirty_reason,
        "draw_calls": render.draw_calls,
        "primitives": render.primitives,
    }))
}

fn expect_scene_changed(
    result: Result<crate::RenderOutcome, RenderError>,
    label: &str,
) -> Result<String, JsValue> {
    if let Err(RenderError::NotPrepared {
        reason: NotPreparedReason::SceneChanged { .. },
    }) = result
    {
        return Ok("scene_changed".to_owned());
    }
    Err(JsValue::from_str(&format!(
        "{label}: render should require explicit prepare after scene change"
    )))
}

fn expect_target_changed(
    result: Result<crate::RenderOutcome, RenderError>,
    label: &str,
) -> Result<String, JsValue> {
    if let Err(RenderError::NotPrepared {
        reason: NotPreparedReason::TargetChanged { .. },
    }) = result
    {
        return Ok("target_changed".to_owned());
    }
    Err(JsValue::from_str(&format!(
        "{label}: render should require explicit prepare after target change"
    )))
}

fn reprepare_and_render<F>(
    renderer: &mut Renderer,
    assets: &Assets<F>,
    scene: &mut Scene,
    camera: crate::CameraKey,
    label: &str,
) -> Result<crate::RenderOutcome, JsValue> {
    renderer
        .prepare_with_assets(scene, assets)
        .map_err(|error| JsValue::from_str(&format!("{label} prepare failed: {error:?}")))?;
    renderer
        .render(scene, camera)
        .map_err(|error| JsValue::from_str(&format!("{label} render failed: {error:?}")))
}
