use wasm_bindgen::{JsCast, JsValue};

use super::{SceneHostError, SceneHostErrorCode};

pub(super) fn browser_canvas_rgba8(
    canvas: &web_sys::HtmlCanvasElement,
) -> Result<Option<(u32, u32, Vec<u8>)>, SceneHostError> {
    let width = canvas.width();
    let height = canvas.height();
    let len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| {
            SceneHostError::new(
                SceneHostErrorCode::Capture,
                format!("canvas readback dimensions overflow: {width}x{height}"),
            )
        })?;
    if len == 0 {
        return Ok(None);
    }

    let get_context = js_sys::Reflect::get(canvas.as_ref(), &JsValue::from_str("getContext"))
        .map_err(browser_readback_error)?
        .dyn_into::<js_sys::Function>()
        .map_err(browser_readback_error)?;
    let options = js_sys::Object::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("preserveDrawingBuffer"),
        &JsValue::TRUE,
    )
    .map_err(browser_readback_error)?;
    let context = get_context
        .call2(
            canvas.as_ref(),
            &JsValue::from_str("webgl2"),
            options.as_ref(),
        )
        .map_err(browser_readback_error)?;
    if context.is_null() || context.is_undefined() {
        return Ok(None);
    }

    let rgba = js_sys::Reflect::get(&context, &JsValue::from_str("RGBA"))
        .map_err(browser_readback_error)?;
    let unsigned_byte = js_sys::Reflect::get(&context, &JsValue::from_str("UNSIGNED_BYTE"))
        .map_err(browser_readback_error)?;
    let read_pixels = js_sys::Reflect::get(&context, &JsValue::from_str("readPixels"))
        .map_err(browser_readback_error)?
        .dyn_into::<js_sys::Function>()
        .map_err(browser_readback_error)?;

    let bytes = js_sys::Uint8Array::new_with_length(len);
    let args = js_sys::Array::new();
    args.push(&JsValue::from_f64(0.0));
    args.push(&JsValue::from_f64(0.0));
    args.push(&JsValue::from_f64(f64::from(width)));
    args.push(&JsValue::from_f64(f64::from(height)));
    args.push(&rgba);
    args.push(&unsigned_byte);
    args.push(bytes.as_ref());
    read_pixels
        .apply(&context, &args)
        .map_err(browser_readback_error)?;

    let mut rgba8 = vec![0; len as usize];
    bytes.copy_to(rgba8.as_mut_slice());
    Ok(Some((width, height, rgba8)))
}

fn browser_readback_error(error: JsValue) -> SceneHostError {
    SceneHostError::new(
        SceneHostErrorCode::Capture,
        error
            .as_string()
            .unwrap_or_else(|| format!("browser canvas readback failed: {error:?}")),
    )
}
