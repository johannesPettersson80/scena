use std::collections::BTreeMap;

use super::common::DiagnosticPathExt;
use super::transform::{TransformResolutionInput, transform_from_recipe};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildTargetV1, SceneRecipeCameraV1, SceneRecipeDiagnosticV1,
};
use crate::scene_host::SceneHostCore;
use crate::scene_host::camera::controls_from_scene_camera;
use crate::{NodeKey, PerspectiveCamera, Vec3};

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
        let camera = PerspectiveCamera::default()
            .with_fov_degrees(recipe.fov_degrees.unwrap_or(60.0) as f32)
            .with_aspect(host.viewport.logical_width() / host.viewport.logical_height());
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
        manifest.push(SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "camera".to_owned(),
            parent: Some(root_handle),
            name: None,
            active: Some(recipe.active),
        });
    }
}
