#![cfg(target_arch = "wasm32")]

pub(super) fn trace_browser_readback(stage: &'static str, detail: serde_json::Value) {
    let enabled = web_sys::window()
        .and_then(|window| {
            js_sys::Reflect::get(
                window.as_ref(),
                &wasm_bindgen::JsValue::from_str("__SCENA_BROWSER_GPU_DIAGNOSTICS__"),
            )
            .ok()
        })
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    if !enabled {
        return;
    }
    let message = serde_json::json!({
        "schema": "scena.browser_readback_trace.v1",
        "stage": stage,
        "detail": detail,
    })
    .to_string();
    web_sys::console::info_1(&wasm_bindgen::JsValue::from_str(&message));
}
