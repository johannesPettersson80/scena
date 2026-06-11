use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::animation::{invalid_input, validate_time_seconds};
use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, Color, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostEasing {
    #[default]
    Linear,
    EaseInOut,
}

#[derive(Debug, Default)]
pub(super) struct HostTransitions {
    transforms: BTreeMap<u64, TransformTransition>,
    tints: BTreeMap<u64, TintTransition>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TransformTransition {
    start: Transform,
    target: Transform,
    elapsed_seconds: f32,
    duration_seconds: f32,
    easing: SceneHostEasing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TintTransition {
    start: Option<Color>,
    target: Option<Color>,
    elapsed_seconds: f32,
    duration_seconds: f32,
    easing: SceneHostEasing,
}

#[derive(Debug, Clone, Copy)]
struct TransformSample {
    handle: u64,
    transform: Transform,
    complete: bool,
}

#[derive(Debug, Clone, Copy)]
struct TintSample {
    handle: u64,
    tint: Option<Color>,
    complete: bool,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_transform_eased(
        &mut self,
        node: u64,
        transform: Transform,
        duration_seconds: f64,
        easing: SceneHostEasing,
    ) -> Result<(), SceneHostError> {
        self.set_transforms_eased(&[(node, transform)], duration_seconds, easing)
    }

    pub fn set_transforms_eased(
        &mut self,
        transforms: &[(u64, Transform)],
        duration_seconds: f64,
        easing: SceneHostEasing,
    ) -> Result<(), SceneHostError> {
        let duration_seconds =
            validate_time_seconds("transition duration_seconds", duration_seconds)?;
        let mut transitions = Vec::with_capacity(transforms.len());
        for (node, transform) in transforms {
            let transform = super::inputs::validate_transform(*transform)?;
            let start = self.current_host_transform(*node)?;
            transitions.push((
                *node,
                TransformTransition {
                    start,
                    target: transform,
                    elapsed_seconds: 0.0,
                    duration_seconds,
                    easing,
                },
            ));
        }

        if duration_seconds == 0.0 {
            let immediate = transitions
                .into_iter()
                .map(|(node, transition)| (node, transition.target))
                .collect::<Vec<_>>();
            return self.set_transforms(&immediate);
        }

        for (node, transition) in transitions {
            self.transitions.transforms.insert(node, transition);
        }
        Ok(())
    }

    pub fn set_node_tint_eased(
        &mut self,
        node: u64,
        tint: Option<Color>,
        duration_seconds: f64,
        easing: SceneHostEasing,
    ) -> Result<(), SceneHostError> {
        validate_opaque_tint(tint)?;
        let duration_seconds =
            validate_time_seconds("transition duration_seconds", duration_seconds)?;
        let start = self.current_host_tint(node)?;
        if start.is_some_and(|color| color.a < 1.0) {
            return Err(invalid_input(
                "eased tint requires the current tint to be opaque or unset",
            ));
        }

        if duration_seconds == 0.0 {
            return self.set_node_tint(node, tint);
        }

        self.transitions.tints.insert(
            node,
            TintTransition {
                start,
                target: tint,
                elapsed_seconds: 0.0,
                duration_seconds,
                easing,
            },
        );
        Ok(())
    }

    pub(super) fn cancel_transform_transition(&mut self, node: u64) {
        self.transitions.transforms.remove(&node);
    }

    pub(super) fn cancel_tint_transition(&mut self, node: u64) {
        self.transitions.tints.remove(&node);
    }

    pub(super) fn advance_transitions(&mut self, delta_seconds: f32) -> Result<(), SceneHostError> {
        let mut transform_samples = Vec::new();
        for (handle, transition) in &mut self.transitions.transforms {
            transition.elapsed_seconds =
                (transition.elapsed_seconds + delta_seconds).min(transition.duration_seconds);
            let complete = transition.elapsed_seconds >= transition.duration_seconds;
            transform_samples.push(TransformSample {
                handle: *handle,
                transform: transition.sample(),
                complete,
            });
        }
        for sample in &transform_samples {
            self.apply_transform_transition_sample(sample.handle, sample.transform)?;
        }
        for sample in transform_samples {
            if sample.complete {
                self.transitions.transforms.remove(&sample.handle);
            }
        }

        let mut tint_samples = Vec::new();
        for (handle, transition) in &mut self.transitions.tints {
            transition.elapsed_seconds =
                (transition.elapsed_seconds + delta_seconds).min(transition.duration_seconds);
            let complete = transition.elapsed_seconds >= transition.duration_seconds;
            tint_samples.push(TintSample {
                handle: *handle,
                tint: transition.sample(complete),
                complete,
            });
        }
        for sample in &tint_samples {
            self.apply_tint_transition_sample(sample.handle, sample.tint)?;
        }
        for sample in tint_samples {
            if sample.complete {
                self.transitions.tints.remove(&sample.handle);
            }
        }
        Ok(())
    }

    fn current_host_transform(&self, handle: u64) -> Result<Transform, SceneHostError> {
        if self.is_instance_root_handle(handle) {
            return self
                .instance_handles
                .get(
                    handle,
                    SceneHostErrorCode::NodeHandleNotFound,
                    SceneHostErrorCode::StaleNodeHandle,
                )
                .map(|binding| binding.root_transform);
        }
        let node = self.resolve_node(handle)?;
        self.scene
            .node(node)
            .map(|node| node.transform())
            .ok_or_else(|| {
                SceneHostError::new(
                    SceneHostErrorCode::NodeHandleNotFound,
                    format!("host node handle {handle} no longer resolves to a scene node"),
                )
            })
    }

    fn current_host_tint(&self, handle: u64) -> Result<Option<Color>, SceneHostError> {
        if self.is_instance_root_handle(handle) {
            return self
                .instance_handles
                .get(
                    handle,
                    SceneHostErrorCode::NodeHandleNotFound,
                    SceneHostErrorCode::StaleNodeHandle,
                )
                .map(|binding| binding.tint);
        }
        let node = self.resolve_node(handle)?;
        Ok(self.scene.node_tint(node)?)
    }

    fn apply_transform_transition_sample(
        &mut self,
        handle: u64,
        transform: Transform,
    ) -> Result<(), SceneHostError> {
        if self.is_instance_root_handle(handle) {
            return self.set_instance_root_transform(handle, transform);
        }
        let node = self.resolve_node(handle)?;
        self.scene.set_transform(node, transform)?;
        Ok(())
    }

    fn apply_tint_transition_sample(
        &mut self,
        handle: u64,
        tint: Option<Color>,
    ) -> Result<(), SceneHostError> {
        if self.is_instance_root_handle(handle) {
            return self.set_instance_root_tint(handle, tint);
        }
        let node = self.resolve_node(handle)?;
        self.scene.set_node_tint(node, tint)?;
        Ok(())
    }
}

impl TransformTransition {
    fn sample(self) -> Transform {
        let amount = eased_amount(
            self.elapsed_seconds / self.duration_seconds.max(f32::EPSILON),
            self.easing,
        );
        Transform {
            translation: self.start.translation.lerp(self.target.translation, amount),
            rotation: self.start.rotation.slerp(self.target.rotation, amount),
            scale: self.start.scale.lerp(self.target.scale, amount),
        }
    }
}

impl TintTransition {
    fn sample(self, complete: bool) -> Option<Color> {
        if complete {
            return self.target;
        }
        let amount = eased_amount(
            self.elapsed_seconds / self.duration_seconds.max(f32::EPSILON),
            self.easing,
        );
        let start = self.start.unwrap_or(Color::WHITE);
        let target = self.target.unwrap_or(Color::WHITE);
        Some(lerp_color(start, target, amount))
    }
}

fn validate_opaque_tint(tint: Option<Color>) -> Result<(), SceneHostError> {
    let Some(tint) = tint else {
        return Ok(());
    };
    let components = [tint.r, tint.g, tint.b, tint.a];
    if !components.into_iter().all(f32::is_finite) {
        return Err(invalid_input("eased tint target must be finite"));
    }
    if tint.a < 1.0 {
        return Err(invalid_input(
            "eased tint target must be opaque in this release",
        ));
    }
    Ok(())
}

fn eased_amount(amount: f32, easing: SceneHostEasing) -> f32 {
    let amount = amount.clamp(0.0, 1.0);
    match easing {
        SceneHostEasing::Linear => amount,
        SceneHostEasing::EaseInOut => {
            if amount < 0.5 {
                4.0 * amount * amount * amount
            } else {
                1.0 - (-2.0 * amount + 2.0).powi(3) / 2.0
            }
        }
    }
}

fn lerp_color(start: Color, target: Color, amount: f32) -> Color {
    let mix = |left: f32, right: f32| left + (right - left) * amount;
    Color::from_linear_rgba(
        mix(start.r, target.r),
        mix(start.g, target.g),
        mix(start.b, target.b),
        1.0,
    )
}
