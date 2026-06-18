use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color};
use super::transform::{TransformResolutionInput, transform_from_recipe};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildTargetV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeLightV1,
};
use crate::scene_host::SceneHostCore;
use crate::{Angle, Color, DirectionalLight, PointLight, SpotLight};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) fn build_authored_lights(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    recipes: &[SceneRecipeLightV1],
    manifest: &mut Vec<SceneRecipeBuildTargetV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let root = host.scene.root();
    let root_handle = host.root_handle();
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.lights[{index}]");
        let empty_nodes = BTreeMap::new();
        let empty_imports = BTreeMap::new();
        let transform = match transform_from_recipe(
            recipe.transform.as_ref(),
            TransformResolutionInput {
                node_keys: &empty_nodes,
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
        let color = match recipe.color.as_deref() {
            Some(color) => match authored_color(colors, color) {
                Ok(color) => Some(color),
                Err(diagnostic) => {
                    diagnostics.push((*diagnostic).with_path(format!("{path}.color")));
                    continue;
                }
            },
            None => None,
        };
        let node = match recipe.kind.as_str() {
            "directional" => match authored_directional_light(recipe, color, &path) {
                Ok(light) => host
                    .scene
                    .directional_light(light)
                    .parent(root)
                    .transform(transform)
                    .add(),
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            },
            "point" => match authored_point_light(recipe, color, &path) {
                Ok(light) => host
                    .scene
                    .point_light(light)
                    .parent(root)
                    .transform(transform)
                    .add(),
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            },
            "spot" => match authored_spot_light(recipe, color, &path) {
                Ok(light) => host
                    .scene
                    .spot_light(light)
                    .parent(root)
                    .transform(transform)
                    .add(),
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    continue;
                }
            },
            kind => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "unsupported_feature",
                    format!("light kind '{kind}' is not supported"),
                    "use directional, point, or spot",
                ));
                continue;
            }
        };
        let node = match node {
            Ok(node) => node,
            Err(error) => {
                diagnostics.push(error_diagnostic(
                    &path,
                    "light_create_failed",
                    format!("failed to create light '{}': {error}", recipe.id),
                    "check the light transform and parent",
                ));
                continue;
            }
        };
        let handle = host.register_node(node);
        manifest.push(SceneRecipeBuildTargetV1 {
            id: recipe.id.clone(),
            handle,
            kind: "light".to_owned(),
            parent: Some(root_handle),
            name: None,
            active: None,
        });
    }
}

fn authored_directional_light(
    recipe: &SceneRecipeLightV1,
    color: Option<Color>,
    path: &str,
) -> Result<DirectionalLight, Box<SceneRecipeDiagnosticV1>> {
    let mut light = match recipe.preset.as_deref() {
        Some("sun") => DirectionalLight::sun(),
        Some("key") => DirectionalLight::key_light(),
        Some("fill") => DirectionalLight::fill_light(),
        Some("rim") => DirectionalLight::rim_light(),
        Some(preset) => {
            return Err(invalid_light_preset(
                format!("{path}.preset"),
                format!("preset '{preset}' is not valid for directional lights"),
                "use sun, key, fill, or rim",
            ));
        }
        None => DirectionalLight::default(),
    };
    if let Some(color) = color {
        light = light.with_color(color);
    }
    if let Some(lux) = recipe.illuminance_lux {
        light = light.with_illuminance_lux(lux as f32);
    }
    Ok(light)
}

fn authored_point_light(
    recipe: &SceneRecipeLightV1,
    color: Option<Color>,
    path: &str,
) -> Result<PointLight, Box<SceneRecipeDiagnosticV1>> {
    let mut light = match recipe.preset.as_deref() {
        Some("softbox") => PointLight::softbox(),
        Some("bulb_warm") => PointLight::bulb_warm(),
        Some("bulb_cool") => PointLight::bulb_cool(),
        Some(preset) => {
            return Err(invalid_light_preset(
                format!("{path}.preset"),
                format!("preset '{preset}' is not valid for point lights"),
                "use softbox, bulb_warm, or bulb_cool",
            ));
        }
        None => PointLight::default(),
    };
    if let Some(color) = color {
        light = light.with_color(color);
    }
    if let Some(intensity) = recipe.intensity_candela {
        light = light.with_intensity_candela(intensity as f32);
    }
    if let Some(range) = recipe.range {
        light = light.with_range(range as f32);
    }
    Ok(light)
}

fn authored_spot_light(
    recipe: &SceneRecipeLightV1,
    color: Option<Color>,
    path: &str,
) -> Result<SpotLight, Box<SceneRecipeDiagnosticV1>> {
    if recipe.preset.is_some() {
        return Err(Box::new(error_diagnostic(
            format!("{path}.preset"),
            "unsupported_feature",
            "spot light presets are not supported",
            "omit preset and set spot light intensity, range, and cone angles explicitly",
        )));
    }
    let mut light = SpotLight::default();
    if let Some(color) = color {
        light = light.with_color(color);
    }
    if let Some(intensity) = recipe.intensity_candela {
        light = light.with_intensity_candela(intensity as f32);
    }
    if let Some(range) = recipe.range {
        light = light.with_range(range as f32);
    }
    if let Some(inner) = recipe.inner_cone_degrees {
        light = light.with_inner_cone_angle(Angle::from_degrees(inner as f32));
    }
    if let Some(outer) = recipe.outer_cone_degrees {
        light = light.with_outer_cone_angle(Angle::from_degrees(outer as f32));
    }
    Ok(light)
}

fn invalid_light_preset(
    path: String,
    message: String,
    help: &'static str,
) -> Box<SceneRecipeDiagnosticV1> {
    Box::new(error_diagnostic(
        path,
        "invalid_light_preset",
        message,
        help,
    ))
}
