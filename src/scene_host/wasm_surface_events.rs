use wasm_bindgen::prelude::*;

use super::wasm::{SceneHost, js_error};
use crate::SurfaceEvent;

#[wasm_bindgen]
impl SceneHost {
    #[wasm_bindgen(js_name = handleSurfaceContextLost)]
    pub fn handle_surface_context_lost(&mut self, recoverable: bool) -> Result<(), JsValue> {
        self.core
            .handle_surface_event(SurfaceEvent::ContextLost { recoverable })
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = handleSurfaceContextRestored)]
    pub fn handle_surface_context_restored(&mut self) -> Result<(), JsValue> {
        self.core
            .handle_surface_event(SurfaceEvent::ContextRestored)
            .map_err(js_error)
    }
}
