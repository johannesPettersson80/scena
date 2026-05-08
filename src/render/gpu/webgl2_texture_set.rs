use web_sys::{WebGl2RenderingContext, WebGlTexture};

use crate::render::prepare::PreparedMaterialSlot;

use super::webgl2_materials;

pub(super) struct WebGl2MaterialTextureSet {
    pub(super) base_color: WebGlTexture,
    pub(super) normal: WebGlTexture,
    pub(super) metallic_roughness: WebGlTexture,
    pub(super) occlusion: WebGlTexture,
    pub(super) emissive: WebGlTexture,
}

#[derive(Clone, Copy, Default)]
pub(super) struct WebGl2MaterialTextureHashes {
    base_color: Option<u64>,
    normal: Option<u64>,
    metallic_roughness: Option<u64>,
    occlusion: Option<u64>,
    emissive: Option<u64>,
}

impl WebGl2MaterialTextureSet {
    pub(super) fn new(gl: &WebGl2RenderingContext) -> Result<Self, wasm_bindgen::JsValue> {
        Ok(Self {
            base_color: webgl2_materials::create_material_texture(gl)?,
            normal: webgl2_materials::create_material_texture(gl)?,
            metallic_roughness: webgl2_materials::create_material_texture(gl)?,
            occlusion: webgl2_materials::create_material_texture(gl)?,
            emissive: webgl2_materials::create_material_texture(gl)?,
        })
    }
}

pub(super) fn upload_webgl2_material_texture_set(
    gl: &WebGl2RenderingContext,
    textures: &WebGl2MaterialTextureSet,
    hashes: &mut WebGl2MaterialTextureHashes,
    slot: Option<&PreparedMaterialSlot>,
) -> Result<(), wasm_bindgen::JsValue> {
    webgl2_materials::upload_material_texture_if_dirty(
        gl,
        &textures.base_color,
        &mut hashes.base_color,
        super::materials::MaterialTextureUpload::from_base_color_texture(
            slot.and_then(|slot| slot.base_color.as_ref())
                .map(|texture| &texture.desc),
        ),
    )?;
    webgl2_materials::upload_material_texture_if_dirty(
        gl,
        &textures.normal,
        &mut hashes.normal,
        super::materials::MaterialTextureUpload::from_normal_texture(
            slot.and_then(|slot| slot.normal.as_ref())
                .map(|texture| &texture.desc),
        ),
    )?;
    webgl2_materials::upload_material_texture_if_dirty(
        gl,
        &textures.metallic_roughness,
        &mut hashes.metallic_roughness,
        super::materials::MaterialTextureUpload::from_metallic_roughness_texture(
            slot.and_then(|slot| slot.metallic_roughness.as_ref())
                .map(|texture| &texture.desc),
        ),
    )?;
    webgl2_materials::upload_material_texture_if_dirty(
        gl,
        &textures.occlusion,
        &mut hashes.occlusion,
        super::materials::MaterialTextureUpload::from_occlusion_texture(
            slot.and_then(|slot| slot.occlusion.as_ref())
                .map(|texture| &texture.desc),
        ),
    )?;
    webgl2_materials::upload_material_texture_if_dirty(
        gl,
        &textures.emissive,
        &mut hashes.emissive,
        super::materials::MaterialTextureUpload::from_emissive_texture(
            slot.and_then(|slot| slot.emissive.as_ref())
                .map(|texture| &texture.desc),
        ),
    )?;
    Ok(())
}
