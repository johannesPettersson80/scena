use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::HtmlCanvasElement;

use crate::diagnostics::{Backend, OutputColorSpace};

use super::GpuDeviceState;

#[wasm_bindgen(inline_js = r#"
const STATE_KEY = "__scenaOutputColorSpace";

function baseState(backend, requested) {
  return {
    api: backend,
    property: backend === "webgpu"
      ? "GPUCanvasConfiguration.colorSpace"
      : "drawingBufferColorSpace",
    requested,
    supported: false,
    configured: false,
    effective: backend === "webgpu" ? null : "srgb",
    display_p3: false,
    injected_by: "RendererOptions::with_output_color_space",
    error: null,
  };
}

function installWebGpuDisplayP3Hook(canvas, requested) {
  const state = baseState("webgpu", requested);
  const context = canvas.getContext("webgpu");
  if (!context || typeof context.configure !== "function") {
    state.error = "webgpu context unavailable";
    canvas[STATE_KEY] = state;
    return state;
  }
  if (context.__scenaDisplayP3HookInstalled) {
    const existing = context[STATE_KEY] || canvas[STATE_KEY] || state;
    canvas[STATE_KEY] = existing;
    return existing;
  }
  const original = context.configure.bind(context);
  const hooked = function(config) {
    const displayP3Config = Object.assign({}, config, { colorSpace: "display-p3" });
    try {
      original(displayP3Config);
      const configured = baseState("webgpu", "display-p3");
      configured.supported = true;
      configured.configured = true;
      configured.effective = "display-p3";
      configured.display_p3 = true;
      context[STATE_KEY] = configured;
      canvas[STATE_KEY] = configured;
    } catch (caught) {
      original(config);
      const fallback = baseState("webgpu", "display-p3");
      fallback.supported = false;
      fallback.configured = false;
      fallback.effective = null;
      fallback.display_p3 = false;
      fallback.error = String(caught && caught.message ? caught.message : caught);
      context[STATE_KEY] = fallback;
      canvas[STATE_KEY] = fallback;
    }
  };
  try {
    context.configure = hooked;
  } catch (caught) {
    try {
      Object.defineProperty(context, "configure", { value: hooked, configurable: true });
    } catch (defineCaught) {
      state.error = String(defineCaught && defineCaught.message ? defineCaught.message : caught);
      canvas[STATE_KEY] = state;
      return state;
    }
  }
  context.__scenaDisplayP3HookInstalled = true;
  state.supported = true;
  state.effective = "pending-configure";
  canvas[STATE_KEY] = state;
  return state;
}

function configureWebGl2DisplayP3(canvas, requested) {
  const state = baseState("webgl2", requested);
  const gl = canvas.getContext("webgl2");
  if (!gl) {
    state.error = "webgl2 context unavailable";
    canvas[STATE_KEY] = state;
    return state;
  }
  state.supported = "drawingBufferColorSpace" in gl;
  if (!state.supported) {
    state.error = "drawingBufferColorSpace unavailable";
    canvas[STATE_KEY] = state;
    return state;
  }
  try {
    if (requested === "display-p3") {
      gl.drawingBufferColorSpace = "display-p3";
    }
    state.effective = gl.drawingBufferColorSpace || null;
    state.configured = state.effective === requested;
    state.display_p3 = state.effective === "display-p3";
  } catch (caught) {
    state.error = String(caught && caught.message ? caught.message : caught);
  }
  canvas[STATE_KEY] = state;
  return state;
}

export function scenaPrepareBrowserCanvasOutputColorSpace(canvas, backend, requested) {
  if (requested !== "display-p3") {
    const state = baseState(backend, requested);
    state.supported = true;
    state.configured = true;
    state.effective = "srgb";
    canvas[STATE_KEY] = state;
    return state;
  }
  if (backend === "webgpu") {
    return installWebGpuDisplayP3Hook(canvas, requested);
  }
  return baseState(backend, requested);
}

export function scenaRefreshBrowserCanvasOutputColorSpace(canvas, backend, requested) {
  if (requested !== "display-p3") {
    return scenaPrepareBrowserCanvasOutputColorSpace(canvas, backend, requested);
  }
  if (backend === "webgl2") {
    return configureWebGl2DisplayP3(canvas, requested);
  }
  const context = canvas.getContext("webgpu");
  const state = (context && context[STATE_KEY]) || canvas[STATE_KEY] || baseState("webgpu", requested);
  canvas[STATE_KEY] = state;
  return state;
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = scenaPrepareBrowserCanvasOutputColorSpace)]
    fn js_prepare_browser_canvas_output_color_space(
        canvas: &HtmlCanvasElement,
        backend: &str,
        requested: &str,
    ) -> JsValue;

    #[wasm_bindgen(js_name = scenaRefreshBrowserCanvasOutputColorSpace)]
    fn js_refresh_browser_canvas_output_color_space(
        canvas: &HtmlCanvasElement,
        backend: &str,
        requested: &str,
    ) -> JsValue;
}

pub(super) fn prepare_browser_canvas_output_color_space(
    backend: Backend,
    canvas: &HtmlCanvasElement,
    output_color_space: OutputColorSpace,
) {
    let _ = js_prepare_browser_canvas_output_color_space(
        canvas,
        backend_name(backend),
        output_color_space_name(output_color_space),
    );
}

impl GpuDeviceState {
    pub(super) fn refresh_browser_canvas_output_color_space(&mut self, backend: Backend) {
        self.display_p3_canvas_configured = false;
        if self.output_color_space != OutputColorSpace::DisplayP3 {
            return;
        }
        let Some(canvas) = &self.browser_canvas else {
            return;
        };
        let state = js_refresh_browser_canvas_output_color_space(
            canvas,
            backend_name(backend),
            output_color_space_name(self.output_color_space),
        );
        self.display_p3_canvas_configured =
            js_sys::Reflect::get(&state, &JsValue::from_str("display_p3"))
                .ok()
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
    }
}

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::WebGpu => "webgpu",
        Backend::WebGl2 => "webgl2",
        Backend::Headless
        | Backend::HeadlessGpu
        | Backend::SurfaceDescriptor
        | Backend::NativeSurface => "unsupported",
    }
}

fn output_color_space_name(output_color_space: OutputColorSpace) -> &'static str {
    match output_color_space {
        OutputColorSpace::Srgb => "srgb",
        OutputColorSpace::DisplayP3 => "display-p3",
    }
}
