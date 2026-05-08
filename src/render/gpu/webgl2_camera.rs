use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlUniformLocation};

#[derive(Debug)]
pub(super) struct WebGl2CameraUniformLocations {
    world_from_model: Option<WebGlUniformLocation>,
    normal_from_model: Option<WebGlUniformLocation>,
    view_from_world: Option<WebGlUniformLocation>,
    clip_from_view: Option<WebGlUniformLocation>,
    clip_from_world: Option<WebGlUniformLocation>,
    camera_position_exposure: Option<WebGlUniformLocation>,
    viewport_near_far: Option<WebGlUniformLocation>,
    color_management: Option<WebGlUniformLocation>,
}

pub(super) struct WebGl2CameraUniformUpload<'a> {
    pub(super) world_from_model: &'a [f32; 16],
    pub(super) normal_from_model: &'a [f32; 16],
    pub(super) view_from_world: &'a [f32; 16],
    pub(super) clip_from_view: &'a [f32; 16],
    pub(super) clip_from_world: &'a [f32; 16],
    pub(super) camera_position: [f32; 3],
    pub(super) viewport: [f32; 2],
    pub(super) near_far: [f32; 2],
    pub(super) exposure: f32,
    pub(super) color_management: [f32; 4],
}

pub(super) fn query_camera_uniform_locations(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
) -> WebGl2CameraUniformLocations {
    WebGl2CameraUniformLocations {
        world_from_model: gl.get_uniform_location(program, "world_from_model"),
        normal_from_model: gl.get_uniform_location(program, "normal_from_model"),
        view_from_world: gl.get_uniform_location(program, "view_from_world"),
        clip_from_view: gl.get_uniform_location(program, "clip_from_view"),
        clip_from_world: gl.get_uniform_location(program, "clip_from_world"),
        camera_position_exposure: gl.get_uniform_location(program, "camera_position_exposure"),
        viewport_near_far: gl.get_uniform_location(program, "viewport_near_far"),
        color_management: gl.get_uniform_location(program, "color_management"),
    }
}

pub(super) fn bind_camera_uniforms(
    gl: &WebGl2RenderingContext,
    locations: &WebGl2CameraUniformLocations,
    upload: WebGl2CameraUniformUpload<'_>,
) {
    gl.uniform_matrix4fv_with_f32_array(
        locations.world_from_model.as_ref(),
        false,
        upload.world_from_model,
    );
    gl.uniform_matrix4fv_with_f32_array(
        locations.normal_from_model.as_ref(),
        false,
        upload.normal_from_model,
    );
    gl.uniform_matrix4fv_with_f32_array(
        locations.view_from_world.as_ref(),
        false,
        upload.view_from_world,
    );
    gl.uniform_matrix4fv_with_f32_array(
        locations.clip_from_view.as_ref(),
        false,
        upload.clip_from_view,
    );
    gl.uniform_matrix4fv_with_f32_array(
        locations.clip_from_world.as_ref(),
        false,
        upload.clip_from_world,
    );
    gl.uniform4f(
        locations.camera_position_exposure.as_ref(),
        upload.camera_position[0],
        upload.camera_position[1],
        upload.camera_position[2],
        upload.exposure,
    );
    gl.uniform4f(
        locations.viewport_near_far.as_ref(),
        upload.viewport[0],
        upload.viewport[1],
        upload.near_far[0],
        upload.near_far[1],
    );
    gl.uniform4f(
        locations.color_management.as_ref(),
        upload.color_management[0],
        upload.color_management[1],
        upload.color_management[2],
        upload.color_management[3],
    );
}
