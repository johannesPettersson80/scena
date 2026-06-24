use std::collections::BTreeMap;

use super::common::{DiagnosticPathExt, authored_color};
use super::transform::{TransformResolutionInput, transform_from_recipe};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeBuildTargetV1, SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipeLightV1,
};
use crate::scene_host::SceneHostCore;
use crate::{Angle, AreaLight, AreaLightShape, Color, DirectionalLight, PointLight, SpotLight};

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
        if recipe.kind == "studio_rig" {
            if let Some(preset) = recipe.preset.as_deref()
                && preset != "studio_rig"
            {
                diagnostics.push(*invalid_light_preset(
                    format!("{path}.preset"),
                    format!("preset '{preset}' is not valid for studio_rig lights"),
                    "use studio_rig or omit preset for the default studio rig",
                ));
                continue;
            }
            let handles =
                match host.scene.add_studio_lighting() {
                    Ok(handles) => handles,
                    Err(error) => {
                        diagnostics.push(error_diagnostic(
                        &path,
                        "light_create_failed",
                        format!("failed to create studio lighting rig '{}': {error}", recipe.id),
                        "check scene light capacity and report persistent failures as a scena bug",
                    ));
                        continue;
                    }
                };
            for (suffix, node) in [
                ("key", handles.key),
                ("fill", handles.fill),
                ("rim", handles.rim),
            ] {
                let handle = host.register_node(node);
                manifest.push(SceneRecipeBuildTargetV1 {
                    id: format!("{}.{}", recipe.id, suffix),
                    handle,
                    kind: "light".to_owned(),
                    parent: Some(root_handle),
                    name: Some(format!("{} {suffix}", recipe.id)),
                    active: None,
                });
            }
            continue;
        }
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
            "area" => match authored_area_light(recipe, color, &path) {
                Ok(light) => host
                    .scene
                    .area_light(light)
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
                    "use directional, point, spot, or area",
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

fn authored_area_light(
    recipe: &SceneRecipeLightV1,
    color: Option<Color>,
    path: &str,
) -> Result<AreaLight, Box<SceneRecipeDiagnosticV1>> {
    let mut light = match recipe.preset.as_deref() {
        Some("softbox") => AreaLight::softbox(),
        Some(preset) => {
            return Err(invalid_light_preset(
                format!("{path}.preset"),
                format!("preset '{preset}' is not valid for area lights"),
                "use softbox",
            ));
        }
        None => AreaLight::default(),
    };
    if let Some(color) = color {
        light = light.with_color(color);
    }
    if let Some(luminous_flux) = recipe.luminous_flux_lumens {
        light = light.with_luminous_flux_lumens(luminous_flux as f32);
    }
    if let Some(range) = recipe.range {
        light = light.with_range(range as f32);
    }
    if let Some(shape) = authored_area_shape(recipe, path)? {
        light = light.with_shape(shape);
    }
    Ok(light)
}

fn authored_area_shape(
    recipe: &SceneRecipeLightV1,
    path: &str,
) -> Result<Option<AreaLightShape>, Box<SceneRecipeDiagnosticV1>> {
    match recipe.shape.as_deref() {
        Some("rect") => {
            if recipe.preset.as_deref() == Some("softbox")
                && recipe.width.is_none()
                && recipe.height.is_none()
            {
                Ok(None)
            } else {
                Ok(Some(AreaLightShape::rect(
                    recipe.width.unwrap_or(1.0) as f32,
                    recipe.height.unwrap_or(1.0) as f32,
                )))
            }
        }
        Some("disc") => Ok(Some(AreaLightShape::disc(
            recipe.radius.unwrap_or(0.5) as f32
        ))),
        Some("sphere") => Ok(Some(AreaLightShape::sphere(
            recipe.radius.unwrap_or(0.5) as f32
        ))),
        Some(shape) => Err(Box::new(error_diagnostic(
            format!("{path}.shape"),
            "invalid_area_light_shape",
            format!("area light shape '{shape}' is not supported"),
            "use rect, disc, or sphere",
        ))),
        None => {
            if recipe.width.is_some() || recipe.height.is_some() {
                Ok(Some(AreaLightShape::rect(
                    recipe.width.unwrap_or(1.0) as f32,
                    recipe.height.unwrap_or(1.0) as f32,
                )))
            } else if let Some(radius) = recipe.radius {
                Ok(Some(AreaLightShape::disc(radius as f32)))
            } else {
                Ok(None)
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn area_recipe_with_softbox_shape() -> SceneRecipeLightV1 {
        SceneRecipeLightV1 {
            id: "softbox".to_owned(),
            kind: "area".to_owned(),
            preset: Some("softbox".to_owned()),
            shape: Some("rect".to_owned()),
            color: None,
            illuminance_lux: None,
            intensity_candela: None,
            luminous_flux_lumens: None,
            range: None,
            width: None,
            height: None,
            radius: None,
            inner_cone_degrees: None,
            outer_cone_degrees: None,
            transform: None,
        }
    }

    #[test]
    fn softbox_rect_shape_without_dimensions_preserves_preset_extent() {
        let recipe = area_recipe_with_softbox_shape();

        let light = authored_area_light(&recipe, None, "$.lights[0]").expect("softbox builds");

        assert_eq!(
            light.shape(),
            AreaLightShape::rect(1.2, 0.6),
            "shape:\"rect\" without dimensions must not silently overwrite the softbox preset extent"
        );
    }
}
