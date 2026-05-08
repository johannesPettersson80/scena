use web_sys::{WebGl2RenderingContext, WebGlProgram, WebGlUniformLocation};

use crate::render::prepare::PreparedGpuLightUniform;

#[derive(Debug)]
pub(super) struct WebGl2LightingUniformLocations {
    directional_light_direction_intensity: Option<WebGlUniformLocation>,
    directional_light_color_count: Option<WebGlUniformLocation>,
    point_light_position_intensity: Option<WebGlUniformLocation>,
    point_light_color_range: Option<WebGlUniformLocation>,
    spot_light_position_intensity: Option<WebGlUniformLocation>,
    spot_light_direction_cones: Option<WebGlUniformLocation>,
    spot_light_cone_range: Option<WebGlUniformLocation>,
    spot_light_color_range: Option<WebGlUniformLocation>,
    environment_diffuse_intensity: Option<WebGlUniformLocation>,
    environment_specular_intensity: Option<WebGlUniformLocation>,
}

pub(super) fn query_lighting_uniform_locations(
    gl: &WebGl2RenderingContext,
    program: &WebGlProgram,
) -> WebGl2LightingUniformLocations {
    WebGl2LightingUniformLocations {
        directional_light_direction_intensity: gl
            .get_uniform_location(program, "directional_light_direction_intensity"),
        directional_light_color_count: gl
            .get_uniform_location(program, "directional_light_color_count"),
        point_light_position_intensity: gl
            .get_uniform_location(program, "point_light_position_intensity"),
        point_light_color_range: gl.get_uniform_location(program, "point_light_color_range"),
        spot_light_position_intensity: gl
            .get_uniform_location(program, "spot_light_position_intensity"),
        spot_light_direction_cones: gl.get_uniform_location(program, "spot_light_direction_cones"),
        spot_light_cone_range: gl.get_uniform_location(program, "spot_light_cone_range"),
        spot_light_color_range: gl.get_uniform_location(program, "spot_light_color_range"),
        environment_diffuse_intensity: gl
            .get_uniform_location(program, "environment_diffuse_intensity"),
        environment_specular_intensity: gl
            .get_uniform_location(program, "environment_specular_intensity"),
    }
}

pub(super) fn bind_lighting_uniforms(
    gl: &WebGl2RenderingContext,
    locations: &WebGl2LightingUniformLocations,
    lighting: PreparedGpuLightUniform,
) {
    uniform4(
        gl,
        &locations.directional_light_direction_intensity,
        lighting.directional_light_direction_intensity,
    );
    uniform4(
        gl,
        &locations.directional_light_color_count,
        lighting.directional_light_color_count,
    );
    uniform4(
        gl,
        &locations.point_light_position_intensity,
        lighting.point_light_position_intensity,
    );
    uniform4(
        gl,
        &locations.point_light_color_range,
        lighting.point_light_color_range,
    );
    uniform4(
        gl,
        &locations.spot_light_position_intensity,
        lighting.spot_light_position_intensity,
    );
    uniform4(
        gl,
        &locations.spot_light_direction_cones,
        lighting.spot_light_direction_cones,
    );
    uniform4(
        gl,
        &locations.spot_light_cone_range,
        lighting.spot_light_cone_range,
    );
    uniform4(
        gl,
        &locations.spot_light_color_range,
        lighting.spot_light_color_range,
    );
    uniform4(
        gl,
        &locations.environment_diffuse_intensity,
        lighting.environment_diffuse_intensity,
    );
    uniform4(
        gl,
        &locations.environment_specular_intensity,
        lighting.environment_specular_intensity,
    );
}

fn uniform4(gl: &WebGl2RenderingContext, location: &Option<WebGlUniformLocation>, value: [f32; 4]) {
    gl.uniform4f(location.as_ref(), value[0], value[1], value[2], value[3]);
}
