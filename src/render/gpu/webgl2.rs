use std::cell::RefCell;

use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext, WebGlBuffer, WebGlProgram, WebGlUniformLocation,
};

use crate::render::prepare::{PreparedGpuLightUniform, PreparedMaterialSlot};

use super::material_uniform::MaterialUniformUpload;
use super::vertices::PrimitiveDrawBatch;
use super::webgl2_camera::{
    WebGl2CameraUniformLocations, WebGl2CameraUniformUpload, bind_camera_uniforms,
    query_camera_uniform_locations,
};
use super::webgl2_lighting::{
    WebGl2LightingUniformLocations, bind_lighting_uniforms, query_lighting_uniform_locations,
};
use super::webgl2_program::{
    FRAGMENT_SHADER, VERTEX_SHADER, compile_shader, context_options, draw_batch_hash, link_program,
    vertex_stream_hash,
};
use super::webgl2_texture_set::{
    WebGl2MaterialTextureHashes, WebGl2MaterialTextureSet, upload_webgl2_material_texture_set,
};
use super::webgl2_vertices::configure_vertex_attributes;

thread_local! {
    static WEBGL2_RENDER_CACHE: RefCell<Option<WebGl2RenderCache>> = const { RefCell::new(None) };
}

pub(super) use super::webgl2_vertices::encode_vertices;

pub(super) fn render_canvas(
    canvas: &HtmlCanvasElement,
    vertices: &[f32],
    draw_batches: &[PrimitiveDrawBatch],
    world_from_model: &[f32; 16],
    normal_from_model: &[f32; 16],
    view_from_world: &[f32; 16],
    clip_from_view: &[f32; 16],
    clip_from_world: &[f32; 16],
    camera_position: [f32; 3],
    viewport: [f32; 2],
    near_far: [f32; 2],
    exposure: f32,
    color_management: [f32; 4],
    lighting: PreparedGpuLightUniform,
) -> Result<(), wasm_bindgen::JsValue> {
    WEBGL2_RENDER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if !cache
            .as_ref()
            .is_some_and(|existing| existing.matches_canvas(canvas))
        {
            return Err(wasm_bindgen::JsValue::from_str(
                "webgl2 resources were not prepared; call Renderer::prepare before render",
            ));
        }
        let cache = cache.as_mut().expect("cache match was checked");
        cache.render(
            vertices,
            draw_batches,
            world_from_model,
            normal_from_model,
            view_from_world,
            clip_from_view,
            clip_from_world,
            camera_position,
            viewport,
            near_far,
            exposure,
            color_management,
            lighting,
        )
    })
}

pub(super) fn prepare_canvas(canvas: &HtmlCanvasElement) -> Result<(), wasm_bindgen::JsValue> {
    WEBGL2_RENDER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|existing| !existing.matches_canvas(canvas))
        {
            *cache = Some(WebGl2RenderCache::new(canvas)?);
        }
        Ok(())
    })
}

pub(super) fn clear_render_cache() {
    WEBGL2_RENDER_CACHE.with(|cache| {
        *cache.borrow_mut() = None;
    });
}

struct WebGl2RenderCache {
    canvas: HtmlCanvasElement,
    gl: WebGl2RenderingContext,
    program: WebGlProgram,
    buffer: WebGlBuffer,
    camera_uniforms: WebGl2CameraUniformLocations,
    lighting_uniforms: WebGl2LightingUniformLocations,
    base_color_texture_uniform: Option<WebGlUniformLocation>,
    normal_texture_uniform: Option<WebGlUniformLocation>,
    metallic_roughness_texture_uniform: Option<WebGlUniformLocation>,
    occlusion_texture_uniform: Option<WebGlUniformLocation>,
    emissive_texture_uniform: Option<WebGlUniformLocation>,
    base_color_uv_offset_scale_uniform: Option<WebGlUniformLocation>,
    base_color_uv_rotation_uniform: Option<WebGlUniformLocation>,
    base_color_factor_uniform: Option<WebGlUniformLocation>,
    emissive_strength_uniform: Option<WebGlUniformLocation>,
    metallic_roughness_alpha_uniform: Option<WebGlUniformLocation>,
    material_textures: Vec<WebGl2MaterialTextureSet>,
    material_uniforms: Vec<MaterialUniformUpload>,
    vertex_capacity_f32: usize,
    last_vertex_hash: Option<u64>,
    last_vertex_len: usize,
    last_texture_hashes: Vec<WebGl2MaterialTextureHashes>,
    last_draw_batch_hash: Option<u64>,
    draw_batches: Vec<PrimitiveDrawBatch>,
}

impl WebGl2RenderCache {
    fn new(canvas: &HtmlCanvasElement) -> Result<Self, wasm_bindgen::JsValue> {
        let gl = canvas
            .get_context_with_context_options("webgl2", &context_options())?
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 context unavailable"))?
            .dyn_into::<WebGl2RenderingContext>()?;
        let vertex_shader =
            compile_shader(&gl, WebGl2RenderingContext::VERTEX_SHADER, VERTEX_SHADER)?;
        let fragment_shader = compile_shader(
            &gl,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            FRAGMENT_SHADER,
        )?;
        let program = link_program(&gl, &vertex_shader, &fragment_shader)?;
        gl.delete_shader(Some(&vertex_shader));
        gl.delete_shader(Some(&fragment_shader));
        let buffer = gl
            .create_buffer()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 buffer allocation failed"))?;
        let camera_uniforms = query_camera_uniform_locations(&gl, &program);
        let lighting_uniforms = query_lighting_uniform_locations(&gl, &program);
        let base_color_texture_uniform = gl.get_uniform_location(&program, "base_color_texture");
        let normal_texture_uniform = gl.get_uniform_location(&program, "normal_texture");
        let metallic_roughness_texture_uniform =
            gl.get_uniform_location(&program, "metallic_roughness_texture");
        let occlusion_texture_uniform = gl.get_uniform_location(&program, "occlusion_texture");
        let emissive_texture_uniform = gl.get_uniform_location(&program, "emissive_texture");
        let base_color_uv_offset_scale_uniform =
            gl.get_uniform_location(&program, "base_color_uv_offset_scale");
        let base_color_uv_rotation_uniform =
            gl.get_uniform_location(&program, "base_color_uv_rotation");
        let base_color_factor_uniform = gl.get_uniform_location(&program, "base_color_factor");
        let emissive_strength_uniform = gl.get_uniform_location(&program, "emissive_strength");
        let metallic_roughness_alpha_uniform =
            gl.get_uniform_location(&program, "metallic_roughness_alpha");
        let material_textures = vec![WebGl2MaterialTextureSet::new(&gl)?];

        Ok(Self {
            canvas: canvas.clone(),
            gl,
            program,
            buffer,
            camera_uniforms,
            lighting_uniforms,
            base_color_texture_uniform,
            normal_texture_uniform,
            metallic_roughness_texture_uniform,
            occlusion_texture_uniform,
            emissive_texture_uniform,
            base_color_uv_offset_scale_uniform,
            base_color_uv_rotation_uniform,
            base_color_factor_uniform,
            emissive_strength_uniform,
            metallic_roughness_alpha_uniform,
            material_textures,
            material_uniforms: vec![MaterialUniformUpload::identity()],
            vertex_capacity_f32: 0,
            last_vertex_hash: None,
            last_vertex_len: 0,
            last_texture_hashes: vec![WebGl2MaterialTextureHashes::default()],
            last_draw_batch_hash: None,
            draw_batches: Vec::new(),
        })
    }

    fn matches_canvas(&self, canvas: &HtmlCanvasElement) -> bool {
        js_sys::Object::is(self.canvas.as_ref(), canvas.as_ref())
    }

    fn render(
        &mut self,
        vertices: &[f32],
        draw_batches: &[PrimitiveDrawBatch],
        world_from_model: &[f32; 16],
        normal_from_model: &[f32; 16],
        view_from_world: &[f32; 16],
        clip_from_view: &[f32; 16],
        clip_from_world: &[f32; 16],
        camera_position: [f32; 3],
        viewport: [f32; 2],
        near_far: [f32; 2],
        exposure: f32,
        color_management: [f32; 4],
        lighting: PreparedGpuLightUniform,
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        self.gl.clear_color(0.0, 0.0, 0.0, 1.0);
        self.gl.enable(WebGl2RenderingContext::DEPTH_TEST);
        self.gl.depth_mask(true);
        self.gl.depth_func(WebGl2RenderingContext::LEQUAL);
        self.gl.clear_depth(1.0);
        self.gl.clear(
            WebGl2RenderingContext::COLOR_BUFFER_BIT | WebGl2RenderingContext::DEPTH_BUFFER_BIT,
        );
        self.gl.use_program(Some(&self.program));
        bind_camera_uniforms(
            &self.gl,
            &self.camera_uniforms,
            WebGl2CameraUniformUpload {
                world_from_model,
                normal_from_model,
                view_from_world,
                clip_from_view,
                clip_from_world,
                camera_position,
                viewport,
                near_far,
                exposure,
                color_management,
            },
        );
        bind_lighting_uniforms(&self.gl, &self.lighting_uniforms, lighting);
        self.ensure_vertices_prepared(vertices)?;
        configure_vertex_attributes(&self.gl, &self.program)?;
        self.ensure_draw_batches_prepared(draw_batches)?;
        for batch in draw_batches {
            self.bind_material_texture(batch.material_slot);
            self.gl.draw_arrays(
                WebGl2RenderingContext::TRIANGLES,
                batch.start_vertex as i32,
                batch.vertex_count as i32,
            );
        }
        self.gl.flush();
        Ok(())
    }

    fn ensure_vertices_prepared(&self, vertices: &[f32]) -> Result<(), wasm_bindgen::JsValue> {
        let next_hash = vertex_stream_hash(vertices);
        if self.last_vertex_hash != Some(next_hash) || self.last_vertex_len != vertices.len() {
            return Err(wasm_bindgen::JsValue::from_str(
                "webgl2 vertex stream was not prepared; call Renderer::prepare after scene changes",
            ));
        }
        self.gl
            .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer));
        Ok(())
    }

    fn upload_prepared_resources_if_dirty(
        &mut self,
        vertices: &[f32],
        draw_batches: &[PrimitiveDrawBatch],
        material_slots: &[PreparedMaterialSlot],
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.upload_material_textures_if_dirty(material_slots)?;
        self.record_draw_batches(draw_batches);
        self.upload_vertices_if_dirty(vertices);
        Ok(())
    }

    fn upload_vertices_if_dirty(&mut self, vertices: &[f32]) {
        let next_hash = vertex_stream_hash(vertices);
        if self.last_vertex_hash == Some(next_hash) && self.last_vertex_len == vertices.len() {
            self.gl
                .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer));
            return;
        }

        let vertex_array = js_sys::Float32Array::from(vertices);
        self.gl
            .bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&self.buffer));
        if self.vertex_capacity_f32 == 0 || vertices.len() > self.vertex_capacity_f32 {
            self.gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &vertex_array,
                WebGl2RenderingContext::STATIC_DRAW,
            );
            self.vertex_capacity_f32 = vertices.len();
        } else {
            self.gl.buffer_sub_data_with_i32_and_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                0,
                &vertex_array,
            );
        }
        self.last_vertex_hash = Some(next_hash);
        self.last_vertex_len = vertices.len();
    }

    fn upload_material_textures_if_dirty(
        &mut self,
        material_slots: &[PreparedMaterialSlot],
    ) -> Result<(), wasm_bindgen::JsValue> {
        let required_len = material_slots.len() + 1;
        while self.material_textures.len() < required_len {
            self.material_textures
                .push(WebGl2MaterialTextureSet::new(&self.gl)?);
            self.last_texture_hashes
                .push(WebGl2MaterialTextureHashes::default());
        }
        self.material_uniforms.clear();
        self.material_uniforms
            .push(MaterialUniformUpload::identity());
        for index in required_len..self.material_textures.len() {
            if let Some(hashes) = self.last_texture_hashes.get_mut(index) {
                *hashes = WebGl2MaterialTextureHashes::default();
            }
        }

        upload_webgl2_material_texture_set(
            &self.gl,
            &self.material_textures[0],
            &mut self.last_texture_hashes[0],
            None,
        )?;
        for (index, material_slot) in material_slots.iter().enumerate() {
            let slot = index + 1;
            upload_webgl2_material_texture_set(
                &self.gl,
                &self.material_textures[slot],
                &mut self.last_texture_hashes[slot],
                Some(material_slot),
            )?;
            let transform = material_slot
                .base_color
                .as_ref()
                .and_then(|texture| texture.transform);
            self.material_uniforms
                .push(MaterialUniformUpload::from_material(
                    Some(&material_slot.material),
                    transform,
                ));
        }
        Ok(())
    }

    fn bind_material_texture(&self, material_slot: u32) {
        let textures = self
            .material_textures
            .get(material_slot as usize)
            .unwrap_or_else(|| {
                self.material_textures
                    .first()
                    .expect("fallback material texture is always prepared")
            });
        self.bind_texture_unit(
            WebGl2RenderingContext::TEXTURE0,
            &textures.base_color,
            self.base_color_texture_uniform.as_ref(),
            0,
        );
        self.bind_texture_unit(
            WebGl2RenderingContext::TEXTURE1,
            &textures.normal,
            self.normal_texture_uniform.as_ref(),
            1,
        );
        self.bind_texture_unit(
            WebGl2RenderingContext::TEXTURE2,
            &textures.metallic_roughness,
            self.metallic_roughness_texture_uniform.as_ref(),
            2,
        );
        self.bind_texture_unit(
            WebGl2RenderingContext::TEXTURE3,
            &textures.occlusion,
            self.occlusion_texture_uniform.as_ref(),
            3,
        );
        self.bind_texture_unit(
            WebGl2RenderingContext::TEXTURE4,
            &textures.emissive,
            self.emissive_texture_uniform.as_ref(),
            4,
        );
        let uniform = self
            .material_uniforms
            .get(material_slot as usize)
            .unwrap_or_else(|| {
                self.material_uniforms
                    .first()
                    .expect("fallback material uniform is always prepared")
            });
        self.gl.uniform4f(
            self.base_color_uv_offset_scale_uniform.as_ref(),
            uniform.offset_scale[0],
            uniform.offset_scale[1],
            uniform.offset_scale[2],
            uniform.offset_scale[3],
        );
        self.gl.uniform4f(
            self.base_color_uv_rotation_uniform.as_ref(),
            uniform.rotation[0],
            uniform.rotation[1],
            uniform.rotation[2],
            uniform.rotation[3],
        );
        self.gl.uniform4f(
            self.base_color_factor_uniform.as_ref(),
            uniform.base_color_factor[0],
            uniform.base_color_factor[1],
            uniform.base_color_factor[2],
            uniform.base_color_factor[3],
        );
        self.gl.uniform4f(
            self.emissive_strength_uniform.as_ref(),
            uniform.emissive_strength[0],
            uniform.emissive_strength[1],
            uniform.emissive_strength[2],
            uniform.emissive_strength[3],
        );
        self.gl.uniform4f(
            self.metallic_roughness_alpha_uniform.as_ref(),
            uniform.metallic_roughness_alpha[0],
            uniform.metallic_roughness_alpha[1],
            uniform.metallic_roughness_alpha[2],
            uniform.metallic_roughness_alpha[3],
        );
    }

    fn bind_texture_unit(
        &self,
        texture_unit: u32,
        texture: &web_sys::WebGlTexture,
        uniform: Option<&WebGlUniformLocation>,
        uniform_index: i32,
    ) {
        self.gl.active_texture(texture_unit);
        self.gl
            .bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(texture));
        self.gl.uniform1i(uniform, uniform_index);
    }

    fn record_draw_batches(&mut self, draw_batches: &[PrimitiveDrawBatch]) {
        self.last_draw_batch_hash = Some(draw_batch_hash(draw_batches));
        self.draw_batches.clear();
        self.draw_batches.extend_from_slice(draw_batches);
    }

    fn ensure_draw_batches_prepared(
        &self,
        draw_batches: &[PrimitiveDrawBatch],
    ) -> Result<(), wasm_bindgen::JsValue> {
        let expected_hash = self.last_draw_batch_hash.ok_or_else(|| {
            wasm_bindgen::JsValue::from_str("webgl2 draw batches were not prepared")
        })?;
        if expected_hash == draw_batch_hash(draw_batches) && draw_batches == self.draw_batches {
            return Ok(());
        }
        Err(wasm_bindgen::JsValue::from_str(
            "webgl2 draw batch data is not available in render cache",
        ))
    }
}

pub(super) fn prepare_canvas_vertices(
    canvas: &HtmlCanvasElement,
    vertices: &[f32],
    draw_batches: &[PrimitiveDrawBatch],
    material_slots: &[PreparedMaterialSlot],
) -> Result<(), wasm_bindgen::JsValue> {
    prepare_canvas(canvas)?;
    WEBGL2_RENDER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let cache = cache
            .as_mut()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("webgl2 resources were not prepared"))?;
        cache.upload_prepared_resources_if_dirty(vertices, draw_batches, material_slots)?;
        Ok(())
    })
}
