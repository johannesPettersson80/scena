use serde::Deserialize;
use wasm_bindgen::prelude::*;

use super::inputs::{TRANSFORM_COMPONENT_COUNT, transform_from_components, transform_from_slices};
use super::wasm::{SceneHost, js_error};
use super::{SceneHostError, SceneHostErrorCode};

#[derive(Debug, Deserialize)]
struct WasmTransformUpdate {
    node: u64,
    translation: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = addEmpty)]
    pub fn add_empty(
        &mut self,
        parent: Option<u64>,
        translation: Box<[f32]>,
        rotation: Box<[f32]>,
        scale: Box<[f32]>,
        tag: Option<String>,
    ) -> Result<u64, JsValue> {
        self.core
            .add_empty(
                parent,
                transform_from_slices(&translation, &rotation, &scale).map_err(js_error)?,
                tag.as_deref(),
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setTransform)]
    pub fn set_transform(
        &mut self,
        node: u64,
        translation: Box<[f32]>,
        rotation: Box<[f32]>,
        scale: Box<[f32]>,
    ) -> Result<(), JsValue> {
        self.core
            .set_transform(
                node,
                transform_from_slices(&translation, &rotation, &scale).map_err(js_error)?,
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = setTransforms)]
    pub fn set_transforms(&mut self, batch_json: String) -> Result<(), JsValue> {
        let updates: Vec<WasmTransformUpdate> =
            serde_json::from_str(&batch_json).map_err(|error| {
                js_error(SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    format!("invalid setTransforms JSON: {error}"),
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
        self.core.set_transforms(&updates).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setTransformsTyped)]
    pub fn set_transforms_typed(
        &mut self,
        nodes: js_sys::BigUint64Array,
        components: js_sys::Float32Array,
    ) -> Result<(), JsValue> {
        let node_count = nodes.length() as usize;
        let expected_components = node_count
            .checked_mul(TRANSFORM_COMPONENT_COUNT)
            .ok_or_else(|| {
                js_error(SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    "setTransformsTyped component count overflow",
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
            updates.push((node, transform_components));
        }
        self.core
            .set_transforms_components(&updates)
            .map_err(js_error)
    }
}
