//! Stage C2: KHR_lights_punctual parsing now uses the `gltf` crate's
//! typed `khr_lights_punctual::Light` accessors.

use ::gltf::Document;
use ::gltf::khr_lights_punctual::Kind as PunctualKind;

use crate::material::Color;
use crate::scene::{Angle, DirectionalLight, Light, PointLight, SpotLight};

use super::SceneAssetLight;

pub(super) fn parse_punctual_lights(document: &Document) -> Vec<SceneAssetLight> {
    let Some(lights) = document.lights() else {
        return Vec::new();
    };
    lights
        .map(|light| {
            let color = Color::from_linear_rgb(
                light.color()[0],
                light.color()[1],
                light.color()[2],
            );
            let intensity = light.intensity();
            let range = light.range();
            let light = match light.kind() {
                PunctualKind::Directional => Light::Directional(
                    DirectionalLight::default()
                        .with_color(color)
                        .with_illuminance_lux(intensity),
                ),
                PunctualKind::Point => {
                    let mut point = PointLight::default()
                        .with_color(color)
                        .with_intensity_candela(intensity);
                    if let Some(range) = range {
                        point = point.with_range(range);
                    }
                    Light::Point(point)
                }
                PunctualKind::Spot {
                    inner_cone_angle,
                    outer_cone_angle,
                } => {
                    let mut spot = SpotLight::default()
                        .with_color(color)
                        .with_intensity_candela(intensity)
                        .with_inner_cone_angle(Angle::from_radians(inner_cone_angle))
                        .with_outer_cone_angle(Angle::from_radians(outer_cone_angle));
                    if let Some(range) = range {
                        spot = spot.with_range(range);
                    }
                    Light::Spot(spot)
                }
            };
            SceneAssetLight { light }
        })
        .collect()
}
