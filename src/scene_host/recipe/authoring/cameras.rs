use std::collections::BTreeMap;

use super::common::DiagnosticPathExt;
use super::transform::{TransformResolutionInput, transform_from_recipe};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildTargetV1, SceneRecipeCameraFramingV1, SceneRecipeCameraV1,
    SceneRecipeDiagnosticV1,
};
use crate::scene_host::SceneHostCore;
use crate::scene_host::camera::controls_from_scene_camera;
use crate::{Aabb, FramingOptions, NodeKey, OrbitControls, PerspectiveCamera, Vec3};

use super::super::{error_diagnostic, scene_host_error_diagnostic};

pub(in crate::scene_host::recipe) fn build_authored_cameras(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeCameraV1],
    node_keys: &BTreeMap<String, NodeKey>,
    manifest: &mut Vec<SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let root = host.scene.root();
    let root_handle = host.root_handle();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.cameras[{index}]");
        if recipe.kind != "perspective" {
            diagnostics.push(error_diagnostic(
                &path,
                "unsupported_feature",
                format!(
                    "camera kind '{}' is not implemented in this slice",
                    recipe.kind
                ),
                "use kind:\"perspective\"",
            ));
            continue;
        }
        if matches!(
            recipe
                .framing
                .as_ref()
                .and_then(|framing| framing.mode.as_deref()),
            Some("default_for_bounds")
        ) {
            let Some(bounds) = scene_bounds_for_camera(host, &path, diagnostics) else {
                continue;
            };
            let viewport = host.viewport.physical_size();
            let camera_key = match host
                .scene
                .add_perspective_camera_default_for(bounds, (viewport.width, viewport.height))
            {
                Ok(camera) => camera,
                Err(error) => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "camera_framing_failed",
                        format!(
                            "failed to create default framed camera '{}': {error}",
                            recipe.id
                        ),
                        "check the declared renderable bounds and camera framing options",
                    ));
                    continue;
                }
            };
            host.active_camera = camera_key;
            if let Some(camera_node) = host.scene.camera_node(camera_key) {
                let handle = host.register_node(camera_node);
                match controls_from_scene_camera(&host.scene, host.active_camera, bounds.center()) {
                    Ok(controls) => host.camera_controls = controls,
                    Err(error) => diagnostics.push(scene_host_error_diagnostic(
                        &path,
                        "camera_controls_failed",
                        error,
                    )),
                }
                manifest.push(SceneRecipeBuildTargetV1 {
                    id: recipe.id.clone(),
                    handle,
                    kind: "camera".to_owned(),
                    parent: Some(root_handle),
                    name: None,
                    visible: None,
                    active: Some(true),
                });
            } else {
                diagnostics.push(error_diagnostic(
                    &path,
                    "camera_create_failed",
                    format!("camera '{}' did not create a scene node", recipe.id),
                    "report this as a scena bug",
                ));
            }
            continue;
        }
        let camera = match camera_from_recipe(recipe, &path, diagnostics) {
            Some(camera) => {
                camera.with_aspect(host.viewport.logical_width() / host.viewport.logical_height())
            }
            None => continue,
        };
        let empty_imports = BTreeMap::new();
        let transform = match transform_from_recipe(
            recipe.transform.as_ref(),
            TransformResolutionInput {
                node_keys,
                imports: &empty_imports,
                parent: Some(root),
                current_bounds: None,
            },
            host,
        ) {
            Ok(transform) => transform,
            Err(diagnostic) => {
                diagnostics.push((*diagnostic).with_path(format!("{path}.transform")));
                continue;
            }
        };
        let camera_key = match host.scene.add_perspective_camera(root, camera, transform) {
            Ok(camera) => camera,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "camera_create_failed",
                    format!("failed to create camera '{}': {error}", recipe.id),
                    "check the camera transform and kind",
                ));
                continue;
            }
        };
        let Some(camera_node) = host.scene.camera_node(camera_key) else {
            diagnostics.push(error_diagnostic(
                &path,
                "camera_create_failed",
                format!("camera '{}' did not create a scene node", recipe.id),
                "report this as a scena bug",
            ));
            continue;
        };
        let handle = host.register_node(camera_node);
        let framed_controls = if let Some(framing) = &recipe.framing {
            let Some(bounds) = camera_framing_bounds(framing, host, &path, diagnostics) else {
                continue;
            };
            let viewport = host.viewport.physical_size();
            let options =
                framing_options_from_recipe(framing, viewport.width, viewport.height, bounds);
            match host.scene.frame_bounds(camera_key, bounds, options) {
                Ok(framing) => Some(OrbitControls::from_framing(framing)),
                Err(error) => {
                    diagnostics.push(error_diagnostic(
                        &path,
                        "camera_framing_failed",
                        format!("failed to frame camera '{}': {error}", recipe.id),
                        "check the declared renderable bounds and camera framing options",
                    ));
                    None
                }
            }
        } else {
            None
        };
        if recipe.active {
            if let Err(error) = host.scene.set_active_camera(camera_key) {
                diagnostics.push(error_diagnostic(
                    &path,
                    "camera_activate_failed",
                    format!("failed to activate camera '{}': {error}", recipe.id),
                    "check the camera handle",
                ));
            } else {
                host.active_camera = camera_key;
                if let Some(controls) = framed_controls {
                    host.camera_controls = controls;
                } else {
                    match controls_from_scene_camera(&host.scene, host.active_camera, Vec3::ZERO) {
                        Ok(controls) => host.camera_controls = controls,
                        Err(error) => diagnostics.push(scene_host_error_diagnostic(
                            &path,
                            "camera_controls_failed",
                            error,
                        )),
                    }
                }
            }
        }
        manifest.push(SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "camera".to_owned(),
            parent: Some(root_handle),
            name: None,
            visible: None,
            active: Some(recipe.active),
        });
    }
}

fn camera_framing_bounds(
    framing: &SceneRecipeCameraFramingV1,
    host: &SceneHostCore<DefaultAssetFetcher>,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Aabb> {
    if framing.mode.as_deref() == Some("target_region") || framing.target_region.is_some() {
        return target_region_bounds(framing, path, diagnostics);
    }
    scene_bounds_for_camera(host, path, diagnostics)
}

fn target_region_bounds(
    framing: &SceneRecipeCameraFramingV1,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Aabb> {
    let Some(target_region) = &framing.target_region else {
        diagnostics.push(error_diagnostic(
            format!("{path}.framing.target_region"),
            "camera_framing_failed",
            "target_region framing requires target_region bounds",
            "emit framing.target_region:{bounds:{min,max},centroid}",
        ));
        return None;
    };
    let min = vec3_from_array(target_region.bounds.min);
    let max = vec3_from_array(target_region.bounds.max);
    if !min.is_finite() || !max.is_finite() || min.x > max.x || min.y > max.y || min.z > max.z {
        diagnostics.push(error_diagnostic(
            format!("{path}.framing.target_region.bounds"),
            "camera_framing_failed",
            "target_region bounds must be finite with min <= max on every axis",
            "emit finite axis-aligned bounds around the target region",
        ));
        return None;
    }
    Some(Aabb::new(min, max))
}

fn vec3_from_array(value: [f64; 3]) -> Vec3 {
    Vec3::new(value[0] as f32, value[1] as f32, value[2] as f32)
}

fn camera_from_recipe(
    recipe: &SceneRecipeCameraV1,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<PerspectiveCamera> {
    if let Some(lens) = recipe.lens.as_deref() {
        return match PerspectiveCamera::from_lens_preset_name(lens) {
            Some(camera) => Some(camera),
            None => {
                diagnostics.push(error_diagnostic(
                    format!("{path}.lens"),
                    "invalid_camera_lens",
                    format!("camera lens preset '{lens}' is not supported"),
                    format!(
                        "use one of: {}",
                        PerspectiveCamera::LENS_PRESET_NAMES.join(", ")
                    ),
                ));
                None
            }
        };
    }
    Some(PerspectiveCamera::default().with_fov_degrees(recipe.fov_degrees.unwrap_or(60.0) as f32))
}

fn framing_options_from_recipe(
    framing: &SceneRecipeCameraFramingV1,
    width: u32,
    height: u32,
    bounds: Aabb,
) -> FramingOptions {
    let mut options = framing
        .preset
        .as_deref()
        .and_then(FramingOptions::from_preset_name)
        .unwrap_or_default()
        .viewport(width, height)
        .tighten_depth_range(true);
    if framing.mode.as_deref() == Some("principal_face") {
        options = options.look_from(principal_face_view_direction(bounds));
    }
    if let Some(fill) = framing.fill {
        options = options.fill(fill as f32);
    }
    if let Some(margin_px) = framing.margin_px {
        options = options.margin_px(margin_px as f32);
    }
    options
}

fn principal_face_view_direction(bounds: Aabb) -> Vec3 {
    let extent = (bounds.max - bounds.min).abs();
    let thin_axis = smallest_axis(extent);
    let face_normal = axis_vec(thin_axis);
    let reveal_axis = if thin_axis != 1 {
        Vec3::Y
    } else {
        axis_vec(strongest_non_parallel_axis(extent, thin_axis))
    };
    let context_axis = axis_vec(strongest_non_parallel_axis(extent, thin_axis));
    (face_normal + reveal_axis * 0.18 + context_axis * 0.08).normalize()
}

fn smallest_axis(extent: Vec3) -> usize {
    let values = [extent.x.abs(), extent.y.abs(), extent.z.abs()];
    values
        .iter()
        .enumerate()
        .min_by(|(left_index, left), (right_index, right)| {
            left.total_cmp(right).then(left_index.cmp(right_index))
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn strongest_non_parallel_axis(extent: Vec3, excluded: usize) -> usize {
    let values = [extent.x.abs(), extent.y.abs(), extent.z.abs()];
    values
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != excluded)
        .max_by(|(left_index, left), (right_index, right)| {
            left.total_cmp(right).then(left_index.cmp(right_index))
        })
        .map(|(index, _)| index)
        .unwrap_or(2)
}

fn axis_vec(axis: usize) -> Vec3 {
    match axis {
        0 => Vec3::X,
        1 => Vec3::Y,
        _ => Vec3::Z,
    }
}

fn scene_bounds_for_camera(
    host: &SceneHostCore<DefaultAssetFetcher>,
    path: &str,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) -> Option<Aabb> {
    match host
        .scene
        .node_world_bounds(host.scene.root(), &host.assets)
    {
        Ok(Some(bounds)) => Some(bounds),
        Ok(None) => {
            diagnostics.push(error_diagnostic(
                format!("{path}.framing"),
                "camera_framing_failed",
                "camera framing requires at least one declared renderable with bounds",
                "add a node/import/instance before using camera.framing",
            ));
            None
        }
        Err(error) => {
            diagnostics.push(error_diagnostic(
                format!("{path}.framing"),
                "camera_framing_failed",
                format!("failed to compute scene bounds for camera framing: {error}"),
                "check that declared renderables have valid finite bounds",
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scene, Transform};

    #[test]
    fn recipe_framing_tightens_depth_for_small_models() {
        let framing = SceneRecipeCameraFramingV1 {
            mode: None,
            preset: Some("isometric".to_owned()),
            fill: Some(0.72),
            margin_px: Some(48.0),
            target_region: None,
        };
        let bounds = Aabb::new(
            Vec3::new(-0.0016, -0.0012, -0.0008),
            Vec3::new(0.0016, 0.0012, 0.0008),
        );
        let mut scene = Scene::new();
        let camera = scene
            .add_perspective_camera(
                scene.root(),
                PerspectiveCamera::default().with_aspect(1.0),
                Transform::default(),
            )
            .expect("camera inserts");

        scene
            .frame_bounds(
                camera,
                bounds,
                framing_options_from_recipe(&framing, 512, 512, bounds),
            )
            .expect("small-model recipe framing must fit the camera depth range");
    }
}
