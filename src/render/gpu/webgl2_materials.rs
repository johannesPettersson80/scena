use web_sys::{WebGl2RenderingContext, WebGlTexture};

use crate::assets::{TextureFilter, TextureSamplerDesc, TextureWrap};

use super::materials::MaterialTextureUpload;

pub(super) fn create_material_texture(
    gl: &WebGl2RenderingContext,
) -> Result<WebGlTexture, wasm_bindgen::JsValue> {
    let texture = gl
        .create_texture()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 texture allocation failed"))?;
    let mut initial_hash = None;
    upload_material_texture_if_dirty(
        gl,
        &texture,
        &mut initial_hash,
        MaterialTextureUpload::from_base_color_texture(None),
    )?;
    Ok(texture)
}

pub(super) fn upload_material_texture_if_dirty(
    gl: &WebGl2RenderingContext,
    texture: &WebGlTexture,
    last_texture_hash: &mut Option<u64>,
    upload: MaterialTextureUpload<'_>,
) -> Result<(), wasm_bindgen::JsValue> {
    let next_hash = material_texture_hash(upload);
    if *last_texture_hash == Some(next_hash) {
        return Ok(());
    }

    gl.active_texture(WebGl2RenderingContext::TEXTURE0);
    gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MIN_FILTER,
        webgl2_filter_mode(upload.sampler.min_filter(), true) as i32,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_MAG_FILTER,
        webgl2_filter_mode(upload.sampler.mag_filter(), false) as i32,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_S,
        webgl2_wrap_mode(upload.sampler.wrap_s()) as i32,
    );
    gl.tex_parameteri(
        WebGl2RenderingContext::TEXTURE_2D,
        WebGl2RenderingContext::TEXTURE_WRAP_T,
        webgl2_wrap_mode(upload.sampler.wrap_t()) as i32,
    );
    gl.pixel_storei(WebGl2RenderingContext::UNPACK_ALIGNMENT, 1);
    gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        WebGl2RenderingContext::TEXTURE_2D,
        0,
        WebGl2RenderingContext::RGBA as i32,
        upload.width as i32,
        upload.height as i32,
        0,
        WebGl2RenderingContext::RGBA,
        WebGl2RenderingContext::UNSIGNED_BYTE,
        Some(upload.rgba8),
    )?;
    if webgl2_filter_uses_mipmaps(upload.sampler.min_filter()) {
        gl.generate_mipmap(WebGl2RenderingContext::TEXTURE_2D);
    }
    *last_texture_hash = Some(next_hash);
    Ok(())
}

fn webgl2_wrap_mode(wrap: TextureWrap) -> u32 {
    match wrap {
        TextureWrap::ClampToEdge => WebGl2RenderingContext::CLAMP_TO_EDGE,
        TextureWrap::MirroredRepeat => WebGl2RenderingContext::MIRRORED_REPEAT,
        TextureWrap::Repeat => WebGl2RenderingContext::REPEAT,
    }
}

fn webgl2_filter_mode(filter: Option<TextureFilter>, allow_mipmaps: bool) -> u32 {
    match filter {
        Some(TextureFilter::Nearest) => WebGl2RenderingContext::NEAREST,
        Some(TextureFilter::Linear) | None => WebGl2RenderingContext::LINEAR,
        Some(TextureFilter::NearestMipmapNearest) if allow_mipmaps => {
            WebGl2RenderingContext::NEAREST_MIPMAP_NEAREST
        }
        Some(TextureFilter::LinearMipmapNearest) if allow_mipmaps => {
            WebGl2RenderingContext::LINEAR_MIPMAP_NEAREST
        }
        Some(TextureFilter::NearestMipmapLinear) if allow_mipmaps => {
            WebGl2RenderingContext::NEAREST_MIPMAP_LINEAR
        }
        Some(TextureFilter::LinearMipmapLinear) if allow_mipmaps => {
            WebGl2RenderingContext::LINEAR_MIPMAP_LINEAR
        }
        Some(TextureFilter::NearestMipmapNearest | TextureFilter::NearestMipmapLinear) => {
            WebGl2RenderingContext::NEAREST
        }
        Some(TextureFilter::LinearMipmapNearest | TextureFilter::LinearMipmapLinear) => {
            WebGl2RenderingContext::LINEAR
        }
    }
}

fn webgl2_filter_uses_mipmaps(filter: Option<TextureFilter>) -> bool {
    matches!(
        filter,
        Some(
            TextureFilter::NearestMipmapNearest
                | TextureFilter::LinearMipmapNearest
                | TextureFilter::NearestMipmapLinear
                | TextureFilter::LinearMipmapLinear
        )
    )
}

fn material_texture_hash(upload: MaterialTextureUpload<'_>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    hash ^= u64::from(upload.width);
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= u64::from(upload.height);
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash ^= if upload.uses_decoded_texture { 1 } else { 0 };
    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    hash_sampler_desc(&mut hash, upload.sampler);
    for byte in upload.rgba8 {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn hash_sampler_desc(hash: &mut u64, sampler: TextureSamplerDesc) {
    for value in [
        texture_filter_hash(sampler.mag_filter()),
        texture_filter_hash(sampler.min_filter()),
        texture_wrap_hash(sampler.wrap_s()),
        texture_wrap_hash(sampler.wrap_t()),
    ] {
        *hash ^= value;
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn texture_filter_hash(filter: Option<TextureFilter>) -> u64 {
    match filter {
        None => 0,
        Some(TextureFilter::Nearest) => 1,
        Some(TextureFilter::Linear) => 2,
        Some(TextureFilter::NearestMipmapNearest) => 3,
        Some(TextureFilter::LinearMipmapNearest) => 4,
        Some(TextureFilter::NearestMipmapLinear) => 5,
        Some(TextureFilter::LinearMipmapLinear) => 6,
    }
}

fn texture_wrap_hash(wrap: TextureWrap) -> u64 {
    match wrap {
        TextureWrap::ClampToEdge => 1,
        TextureWrap::MirroredRepeat => 2,
        TextureWrap::Repeat => 3,
    }
}
