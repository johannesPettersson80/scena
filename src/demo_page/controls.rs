use std::io::Cursor;

use wasm_bindgen::prelude::*;

use crate::{AntiAliasing, AutoExposureConfig, Background, Color, PostBloomConfig};

use super::DemoApp;

/// Switch the renderer background to a named [`Background`] scheme.
///
/// Accepted scheme keys (case-insensitive):
/// `studio`, `dark_studio`, `neutral_gray`, `white`, `black`, `sky`,
/// `transparent`.
#[wasm_bindgen]
pub fn set_background_scheme(app: &mut DemoApp, scheme: &str) -> Result<(), JsValue> {
    let background = background_from_scheme(scheme)?;
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_background_scheme")
    })?;
    renderer.set_background(background);
    Ok(())
}

/// Returns the CSS color for a named [`Background`] scheme.
///
/// Browser WebGL2 canvases can remain alpha-transparent even when the wgpu
/// surface requests opaque compositing. The demo mirrors the renderer
/// background into CSS so browser screenshots and human-visible pages show the
/// same background color as the render pass.
#[wasm_bindgen]
pub fn background_scheme_css_color(scheme: &str) -> Result<String, JsValue> {
    let color = background_from_scheme(scheme)?.color();
    if color.a < 1.0 {
        let r = linear_channel_to_srgb_u8(color.r);
        let g = linear_channel_to_srgb_u8(color.g);
        let b = linear_channel_to_srgb_u8(color.b);
        Ok(format!(
            "rgba({r}, {g}, {b}, {:.3})",
            color.a.clamp(0.0, 1.0)
        ))
    } else {
        Ok(color_to_css_rgb(color))
    }
}

fn background_from_scheme(scheme: &str) -> Result<Background, JsValue> {
    Ok(match scheme.to_ascii_lowercase().as_str() {
        "studio" => Background::Studio,
        "dark_studio" | "darkstudio" | "dark-studio" => Background::DarkStudio,
        "neutral_gray" | "neutralgray" | "neutral-gray" => Background::NeutralGray,
        "white" => Background::White,
        "black" => Background::Black,
        "sky" => Background::Sky,
        "transparent" => Background::Transparent,
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown background scheme: {other}"
            )));
        }
    })
}

fn linear_channel_to_srgb_u8(channel: f32) -> u8 {
    let linear = channel.clamp(0.0, 1.0);
    let srgb = if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).round().clamp(0.0, 255.0) as u8
}

fn color_to_css_rgb(color: Color) -> String {
    let r = linear_channel_to_srgb_u8(color.r);
    let g = linear_channel_to_srgb_u8(color.g);
    let b = linear_channel_to_srgb_u8(color.b);
    format!("rgb({r}, {g}, {b})")
}

/// Switch the renderer's auto-exposure to a named scenario.
///
/// Accepted preset keys: `product_studio`, `indoor`, `outdoor`, `mixed`.
#[wasm_bindgen]
pub fn set_auto_exposure_preset(app: &mut DemoApp, preset: &str) -> Result<(), JsValue> {
    let config = match preset.to_ascii_lowercase().as_str() {
        "product_studio" | "productstudio" | "product-studio" => {
            AutoExposureConfig::product_studio()
        }
        "indoor" => AutoExposureConfig::indoor(),
        "outdoor" => AutoExposureConfig::outdoor(),
        "mixed" | "default" => AutoExposureConfig::mixed(),
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown auto-exposure preset: {other}"
            )));
        }
    };
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_auto_exposure_preset")
    })?;
    renderer.set_auto_exposure(config);
    Ok(())
}

/// Pins the renderer to a fixed exposure and disables managed auto exposure.
#[wasm_bindgen]
pub fn set_fixed_exposure_ev(app: &mut DemoApp, ev: f32) -> Result<(), JsValue> {
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_fixed_exposure_ev")
    })?;
    renderer.clear_auto_exposure();
    renderer.set_exposure_ev(ev);
    Ok(())
}

/// Switch the renderer's anti-aliasing mode.
///
/// Accepted mode keys: `fxaa` (the default), `none`.
#[wasm_bindgen]
pub fn set_anti_aliasing_mode(app: &mut DemoApp, mode: &str) -> Result<(), JsValue> {
    let setting = match mode.to_ascii_lowercase().as_str() {
        "fxaa" | "on" | "true" => AntiAliasing::Fxaa,
        "none" | "off" | "false" => AntiAliasing::None,
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown anti-aliasing mode: {other}"
            )));
        }
    };
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_anti_aliasing_mode")
    })?;
    renderer.set_anti_aliasing(setting);
    Ok(())
}

/// Toggle the subtle post bloom pass on or off.
#[wasm_bindgen]
pub fn set_bloom_enabled(app: &mut DemoApp, enabled: bool) -> Result<(), JsValue> {
    let renderer = app.renderer.as_mut().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before set_bloom_enabled")
    })?;
    renderer.set_bloom(if enabled {
        Some(PostBloomConfig::subtle())
    } else {
        None
    });
    Ok(())
}

/// Encode the latest rendered frame as PNG bytes for download or upload.
///
/// Requires that `tick` has been called at least once after `attach_to_canvas`
/// so the renderer holds a populated readback buffer.
#[wasm_bindgen]
pub fn capture_png_bytes(app: &DemoApp) -> Result<Box<[u8]>, JsValue> {
    let renderer = app.renderer.as_ref().ok_or_else(|| {
        JsValue::from_str("attach_to_canvas must be called before capture_png_bytes")
    })?;
    let stats = renderer.stats();
    let width = stats.target_width;
    let height = stats.target_height;
    let frame = renderer.frame_rgba8();
    let expected = width as usize * height as usize * 4;
    if frame.len() != expected {
        return Err(JsValue::from_str(&format!(
            "renderer frame buffer is {} bytes; expected {} RGBA8 bytes",
            frame.len(),
            expected
        )));
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|err| JsValue::from_str(&format!("png header failed: {err:?}")))?;
        writer
            .write_image_data(frame)
            .map_err(|err| JsValue::from_str(&format!("png body failed: {err:?}")))?;
    }
    Ok(bytes.into_boxed_slice())
}
