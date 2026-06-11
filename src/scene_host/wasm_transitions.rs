use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::inputs::{TRANSFORM_COMPONENT_COUNT, transform_from_components, transform_from_slices};
use super::wasm::{SceneHost, js_error};
use super::{SceneHostEasing, SceneHostError, SceneHostErrorCode};
use crate::Color;

#[derive(Debug, Deserialize)]
struct WasmEasedTransformUpdate {
    node: u64,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = setTransformEased)]
    pub fn set_transform_eased(
        &mut self,
        node: u64,
        translation: Box<[f32]>,
        rotation: Box<[f32]>,
        scale: Box<[f32]>,
        duration_seconds: f64,
        easing: String,
    ) -> Result<(), JsValue> {
        self.core
            .set_transform_eased(
                node,
                transform_from_slices(&translation, &rotation, &scale).map_err(js_error)?,
                duration_seconds,
                parse_easing(&easing).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setTransformsEased)]
    pub fn set_transforms_eased(
        &mut self,
        batch_json: String,
        duration_seconds: f64,
        easing: String,
    ) -> Result<(), JsValue> {
        let updates: Vec<WasmEasedTransformUpdate> =
            serde_json::from_str(&batch_json).map_err(|error| {
                js_error(SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    format!("invalid setTransformsEased JSON: {error}"),
                ))
            })?;
        let updates = updates
            .into_iter()
            .map(|update| {
                Ok((
                    update.node,
                    transform_from_components(update.translation, update.rotation, update.scale)?,
                ))
            })
            .collect::<Result<Vec<_>, SceneHostError>>()
            .map_err(js_error)?;
        self.core
            .set_transforms_eased(
                &updates,
                duration_seconds,
                parse_easing(&easing).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setTransformsEasedTyped)]
    pub fn set_transforms_eased_typed(
        &mut self,
        nodes: js_sys::BigUint64Array,
        components: js_sys::Float32Array,
        duration_seconds: f64,
        easing: String,
    ) -> Result<(), JsValue> {
        let node_count = nodes.length() as usize;
        let expected_components = node_count
            .checked_mul(TRANSFORM_COMPONENT_COUNT)
            .ok_or_else(|| {
                js_error(SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    "setTransformsEasedTyped component count overflow",
                ))
            })?;
        if components.length() as usize != expected_components {
            return Err(js_error(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "components must contain {} values for {} nodes, got {}",
                    expected_components,
                    node_count,
                    components.length()
                ),
            )));
        }
        let nodes = nodes.to_vec();
        let components = components.to_vec();
        let mut updates = Vec::with_capacity(node_count);
        for (index, node) in nodes.into_iter().enumerate() {
            let start = index * TRANSFORM_COMPONENT_COUNT;
            let mut transform_components = [0.0; TRANSFORM_COMPONENT_COUNT];
            transform_components
                .copy_from_slice(&components[start..start + TRANSFORM_COMPONENT_COUNT]);
            updates.push((
                node,
                super::inputs::transform_from_component_array(transform_components)
                    .map_err(js_error)?,
            ));
        }
        self.core
            .set_transforms_eased(
                &updates,
                duration_seconds,
                parse_easing(&easing).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setNodeTintEased)]
    pub fn set_node_tint_eased(
        &mut self,
        node: u64,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        duration_seconds: f64,
        easing: String,
    ) -> Result<(), JsValue> {
        self.core
            .set_node_tint_eased(
                node,
                Some(Color::from_linear_rgba(r, g, b, a)),
                duration_seconds,
                parse_easing(&easing).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearNodeTintEased)]
    pub fn clear_node_tint_eased(
        &mut self,
        node: u64,
        duration_seconds: f64,
        easing: String,
    ) -> Result<(), JsValue> {
        self.core
            .set_node_tint_eased(
                node,
                None,
                duration_seconds,
                parse_easing(&easing).map_err(js_error)?,
            )
            .map_err(js_error)
    }
}

fn parse_easing(value: &str) -> Result<SceneHostEasing, SceneHostError> {
    match value {
        "linear" => Ok(SceneHostEasing::Linear),
        "ease_in_out" | "easeInOut" => Ok(SceneHostEasing::EaseInOut),
        other => Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            format!("unsupported SceneHost easing {other}"),
        )),
    }
}
