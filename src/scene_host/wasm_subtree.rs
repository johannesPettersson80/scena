use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};
use crate::Color;

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = setNodeTint)]
    pub fn set_node_tint(
        &mut self,
        node: u64,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    ) -> Result<(), JsValue> {
        self.core
            .set_node_tint(node, Some(Color::from_linear_rgba(r, g, b, a)))
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearNodeTint)]
    pub fn clear_node_tint(&mut self, node: u64) -> Result<(), JsValue> {
        self.core.set_node_tint(node, None).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setVisible)]
    pub fn set_visible(&mut self, node: u64, visible: bool) -> Result<(), JsValue> {
        self.core.set_visible(node, visible).map_err(js_error)
    }

    #[wasm_bindgen(js_name = showOnly)]
    pub fn show_only(&mut self, nodes: js_sys::BigUint64Array) -> Result<(), JsValue> {
        self.core.show_only(&nodes.to_vec()).map_err(js_error)
    }

    #[wasm_bindgen(js_name = isolate)]
    pub fn isolate(&mut self, nodes: js_sys::BigUint64Array) -> Result<(), JsValue> {
        self.core.isolate(&nodes.to_vec()).map_err(js_error)
    }

    #[wasm_bindgen(js_name = ghost)]
    pub fn ghost(&mut self, node: u64, alpha: f32) -> Result<(), JsValue> {
        self.core.ghost(node, alpha).map_err(js_error)
    }

    #[wasm_bindgen(js_name = fitSelection)]
    pub fn fit_selection(&mut self, nodes: js_sys::BigUint64Array) -> Result<(), JsValue> {
        self.core.fit_selection(&nodes.to_vec()).map_err(js_error)
    }

    #[wasm_bindgen(js_name = subtreeNodesJson)]
    pub fn subtree_nodes_json(&mut self, root: u64) -> Result<String, JsValue> {
        self.core.subtree_nodes_json(root).map_err(js_error)
    }

    #[wasm_bindgen(js_name = setSubtreeTint)]
    pub fn set_subtree_tint(
        &mut self,
        root: u64,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        exclude_handles: js_sys::BigUint64Array,
    ) -> Result<(), JsValue> {
        self.core
            .set_subtree_tint(
                root,
                Some(Color::from_linear_rgba(r, g, b, a)),
                &exclude_handles.to_vec(),
            )
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = clearSubtreeTint)]
    pub fn clear_subtree_tint(
        &mut self,
        root: u64,
        exclude_handles: js_sys::BigUint64Array,
    ) -> Result<(), JsValue> {
        self.core
            .set_subtree_tint(root, None, &exclude_handles.to_vec())
            .map_err(js_error)
    }
}
