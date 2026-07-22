#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlCanvasElement;

wasm_bindgen_test_configure!(run_in_browser);

/// This WebGPU-only proof is intentionally separate from
/// `m6_browser_renderer_parity`: the Linux WebGL2 lane must not require a
/// WebGPU adapter, while the required WebGPU lane remains fail-closed when it
/// selects and executes this proof on a WebGPU-capable host.
#[cfg(feature = "browser-probe")]
#[wasm_bindgen_test(async)]
async fn m6_cpu_webgpu_parity_uses_the_headline_renderer_readback() {
    let canvas = browser_canvas(64, 64);
    let report = scena::browser_probe::m6_render_webgpu_probe(canvas)
        .await
        .expect("CPU/WebGPU parity probe runs");
    let report: serde_json::Value =
        serde_json::from_str(&report).expect("CPU/WebGPU parity report is JSON");
    let parity = report
        .get("parity")
        .expect("browser probe includes CPU/WebGPU parity evidence");
    let readback = report
        .get("renderer_readback")
        .expect("browser probe includes renderer-owned readback");

    assert_eq!(
        parity.get("schema").and_then(serde_json::Value::as_str),
        Some("scena.m6.cpu_webgpu_parity.v1")
    );
    assert_eq!(
        parity.get("backend").and_then(serde_json::Value::as_str),
        Some("WebGpu")
    );
    let gpu_frame = parity.get("gpu_frame").expect("parity has a GPU frame");
    assert_eq!(gpu_frame.get("width"), readback.get("width"));
    assert_eq!(gpu_frame.get("height"), readback.get("height"));
    assert_eq!(
        gpu_frame.get("rgba8_fnv1a64"),
        readback.get("rgba8_fnv1a64"),
        "required parity must evaluate the renderer headline readback"
    );
}

fn browser_canvas(width: u32, height: u32) -> HtmlCanvasElement {
    let window = web_sys::window().expect("browser window exists");
    let document = window.document().expect("browser document exists");
    let canvas = document
        .create_element("canvas")
        .expect("canvas element can be created")
        .dyn_into::<HtmlCanvasElement>()
        .expect("element is a canvas");
    canvas.set_width(width);
    canvas.set_height(height);
    document
        .body()
        .expect("document has body")
        .append_child(&canvas)
        .expect("canvas appends to document");
    canvas
}
