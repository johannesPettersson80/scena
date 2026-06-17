use std::collections::BTreeMap;

use crate::animation::{
    AnimationChannel, AnimationClip, AnimationInterpolation, AnimationOutput, AnimationTarget,
};
use crate::assets::DefaultAssetFetcher;
use crate::scene::recipe::{
    SceneRecipeAnimationV1, SceneRecipeBuildAnimationV1, SceneRecipeDiagnosticV1,
    SceneRecipeTargetV1,
};
use crate::scene_host::SceneHostCore;
use crate::{NodeKey, Quat, Vec3};

use super::super::error_diagnostic;

pub(in crate::scene_host::recipe) fn build_authored_animations(
    host: &mut SceneHostCore<DefaultAssetFetcher>,
    recipes: &[SceneRecipeAnimationV1],
    node_keys: &BTreeMap<String, NodeKey>,
    manifest: &mut Vec<SceneRecipeBuildAnimationV1>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    for (index, recipe) in recipes.iter().enumerate() {
        let path = format!("$.animations[{index}]");
        let mut channels = Vec::new();
        let mut animation_failed = false;
        for (channel_index, channel) in recipe.channels.iter().enumerate() {
            let channel_path = format!("{path}.channels[{channel_index}]");
            match channel_from_recipe(&channel_path, channel, node_keys) {
                Ok(channel) => channels.push(channel),
                Err(diagnostic) => {
                    diagnostics.push(*diagnostic);
                    animation_failed = true;
                }
            }
        }
        if animation_failed || channels.is_empty() {
            if channels.is_empty() {
                diagnostics.push(error_diagnostic(
                    &path,
                    "animation_create_failed",
                    format!("animation '{}' has no valid channels", recipe.id),
                    "fix the animation channels and retry",
                ));
            }
            continue;
        }
        let clip =
            AnimationClip::authored(Some(recipe.id.clone()), channels, recipe.duration as f32);
        let duration_seconds = round3(clip.duration_seconds());
        let channel_count = clip.channels().len();
        let mixer = host.scene.create_authored_animation_mixer(clip);
        let handle = host.animation_handles.insert(mixer);
        manifest.push(SceneRecipeBuildAnimationV1 {
            id: recipe.id.clone(),
            handle,
            duration_seconds,
            channel_count,
        });
    }
}

fn channel_from_recipe(
    path: &str,
    channel: &crate::SceneRecipeAnimationChannelV1,
    node_keys: &BTreeMap<String, NodeKey>,
) -> Result<AnimationChannel, Box<SceneRecipeDiagnosticV1>> {
    let SceneRecipeTargetV1::Node { id } = &channel.target else {
        return Err(Box::new(error_diagnostic(
            format!("{path}.target"),
            "unsupported_feature",
            "animation channels can only target nodes",
            "use target:{kind:\"node\",id:\"...\"}",
        )));
    };
    let Some(node) = node_keys.get(id.as_str()).copied() else {
        return Err(Box::new(error_diagnostic(
            format!("{path}.target.id"),
            "unknown_animation_target",
            format!("animation target references unknown node id '{id}'"),
            "target a node from the build manifest",
        )));
    };
    let target = match channel.path.as_str() {
        "translation" => AnimationTarget::Translation,
        "rotation" => AnimationTarget::Rotation,
        "scale" => AnimationTarget::Scale,
        "weights" => AnimationTarget::Weights,
        _ => {
            return Err(Box::new(error_diagnostic(
                format!("{path}.path"),
                "invalid_animation_path",
                format!("animation path '{}' is not supported", channel.path),
                "use translation, rotation, scale, or weights",
            )));
        }
    };
    let interpolation = match channel.interpolation.as_str() {
        "linear" => AnimationInterpolation::Linear,
        "step" => AnimationInterpolation::Step,
        "cubic_spline" => AnimationInterpolation::CubicSpline,
        _ => {
            return Err(Box::new(error_diagnostic(
                format!("{path}.interpolation"),
                "invalid_animation_interpolation",
                format!(
                    "animation interpolation '{}' is not supported",
                    channel.interpolation
                ),
                "use linear, step, or cubic_spline",
            )));
        }
    };
    let times = f32_values(&format!("{path}.times"), &channel.times)?;
    let output = match target {
        AnimationTarget::Translation | AnimationTarget::Scale => {
            AnimationOutput::Vec3(vec3_values(&format!("{path}.values"), &channel.values)?)
        }
        AnimationTarget::Rotation => {
            AnimationOutput::Quat(quat_values(&format!("{path}.values"), &channel.values)?)
        }
        AnimationTarget::Weights => {
            if !channel.values.iter().all(|value| !value.is_empty()) {
                return Err(Box::new(error_diagnostic(
                    format!("{path}.values"),
                    "invalid_animation_values",
                    "weights animation values must include at least one weight",
                    "emit one numeric component per morph target",
                )));
            }
            AnimationOutput::Weights(weight_values(&format!("{path}.values"), &channel.values)?)
        }
    };
    Ok(AnimationChannel::new(
        node,
        target,
        times,
        output,
        interpolation,
    ))
}

fn f32_values(path: &str, values: &[f64]) -> Result<Vec<f32>, Box<SceneRecipeDiagnosticV1>> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| f32_value(&format!("{path}[{index}]"), *value))
        .collect()
}

fn vec3_values(path: &str, values: &[Vec<f64>]) -> Result<Vec<Vec3>, Box<SceneRecipeDiagnosticV1>> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let [x, y, z] = value.as_slice() else {
                return Err(Box::new(error_diagnostic(
                    format!("{path}[{index}]"),
                    "invalid_animation_value",
                    "translation and scale values must have exactly three components",
                    "emit [x,y,z]",
                )));
            };
            Ok(Vec3::new(
                f32_value(&format!("{path}[{index}][0]"), *x)?,
                f32_value(&format!("{path}[{index}][1]"), *y)?,
                f32_value(&format!("{path}[{index}][2]"), *z)?,
            ))
        })
        .collect()
}

fn quat_values(path: &str, values: &[Vec<f64>]) -> Result<Vec<Quat>, Box<SceneRecipeDiagnosticV1>> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let [x, y, z, w] = value.as_slice() else {
                return Err(Box::new(error_diagnostic(
                    format!("{path}[{index}]"),
                    "invalid_animation_value",
                    "rotation values must have exactly four components",
                    "emit [x,y,z,w]",
                )));
            };
            Ok(Quat::from_xyzw(
                f32_value(&format!("{path}[{index}][0]"), *x)?,
                f32_value(&format!("{path}[{index}][1]"), *y)?,
                f32_value(&format!("{path}[{index}][2]"), *z)?,
                f32_value(&format!("{path}[{index}][3]"), *w)?,
            )
            .normalize())
        })
        .collect()
}

fn weight_values(
    path: &str,
    values: &[Vec<f64>],
) -> Result<Vec<Vec<f32>>, Box<SceneRecipeDiagnosticV1>> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .iter()
                .enumerate()
                .map(|(component, value)| {
                    f32_value(&format!("{path}[{index}][{component}]"), *value)
                })
                .collect()
        })
        .collect()
}

fn f32_value(path: &str, value: f64) -> Result<f32, Box<SceneRecipeDiagnosticV1>> {
    if value.is_finite() && value.abs() <= f64::from(f32::MAX) {
        Ok(value as f32)
    } else {
        Err(Box::new(error_diagnostic(
            path,
            "invalid_animation_value",
            format!("animation value must be finite f32-compatible, got {value}"),
            "emit finite numeric values",
        )))
    }
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}
