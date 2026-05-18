#[cfg(all(target_arch = "wasm32", feature = "demo-page"))]
pub(crate) fn browser_timing_enabled() -> bool {
    web_sys::window()
        .and_then(|window| {
            js_sys::Reflect::get(&window, &wasm_bindgen::JsValue::from_str("location")).ok()
        })
        .and_then(|location| {
            js_sys::Reflect::get(&location, &wasm_bindgen::JsValue::from_str("search")).ok()
        })
        .and_then(|search| search.as_string())
        .is_some_and(|search| {
            search
                .trim_start_matches('?')
                .split('&')
                .filter(|part| !part.is_empty())
                .any(|part| {
                    part == "perf"
                        || part == "timing"
                        || part
                            .strip_prefix("perf=")
                            .is_some_and(|value| value != "0" && value != "false")
                        || part
                            .strip_prefix("timing=")
                            .is_some_and(|value| value != "0" && value != "false")
                })
        })
}
