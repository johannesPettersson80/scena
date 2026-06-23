use super::area::area_light_shape_code;
use super::{
    MAX_GPU_AREA_LIGHTS, MAX_GPU_LIGHTS_PER_TYPE, PreparedLights, directional_illuminance_lux,
    punctual_intensity_candela,
};
use crate::render::prepare::PreparedEnvironmentLighting;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct PreparedGpuLightUniform {
    pub(in crate::render) directional_light_direction_intensity:
        [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) directional_light_color: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) directional_shadow_control: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) point_light_position_intensity: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) point_light_color_range: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_position_intensity: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_direction_cones: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_cone_range: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) spot_light_color_range: [[f32; 4]; MAX_GPU_LIGHTS_PER_TYPE],
    pub(in crate::render) area_light_position_flux: [[f32; 4]; MAX_GPU_AREA_LIGHTS],
    pub(in crate::render) area_light_axis_x_shape: [[f32; 4]; MAX_GPU_AREA_LIGHTS],
    pub(in crate::render) area_light_axis_y_range: [[f32; 4]; MAX_GPU_AREA_LIGHTS],
    pub(in crate::render) area_light_color: [[f32; 4]; MAX_GPU_AREA_LIGHTS],
    pub(in crate::render) light_counts: [f32; 4],
    pub(in crate::render) environment_diffuse_intensity: [f32; 4],
    pub(in crate::render) environment_specular_intensity: [f32; 4],
}

impl Default for PreparedGpuLightUniform {
    fn default() -> Self {
        Self {
            directional_light_direction_intensity: [[0.0, 0.0, -1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            directional_light_color: [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            directional_shadow_control: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            point_light_position_intensity: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            point_light_color_range: [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_position_intensity: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_direction_cones: [[0.0, 0.0, -1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_cone_range: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            spot_light_color_range: [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_LIGHTS_PER_TYPE],
            area_light_position_flux: [[0.0, 0.0, 0.0, 0.0]; MAX_GPU_AREA_LIGHTS],
            area_light_axis_x_shape: [[1.0, 0.0, 0.0, 0.0]; MAX_GPU_AREA_LIGHTS],
            area_light_axis_y_range: [[0.0, 1.0, 0.0, 0.0]; MAX_GPU_AREA_LIGHTS],
            area_light_color: [[1.0, 1.0, 1.0, 0.0]; MAX_GPU_AREA_LIGHTS],
            light_counts: [0.0, 0.0, 0.0, 0.0],
            environment_diffuse_intensity: [0.0, 0.0, 0.0, 0.0],
            environment_specular_intensity: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl PreparedLights {
    pub(super) fn gpu_uniform(
        &self,
        environment: PreparedEnvironmentLighting,
        tiled_assignment_active: bool,
    ) -> PreparedGpuLightUniform {
        let mut uniform = PreparedGpuLightUniform {
            light_counts: [
                if tiled_assignment_active {
                    0.0
                } else {
                    self.directional.len().min(MAX_GPU_LIGHTS_PER_TYPE) as f32
                },
                if tiled_assignment_active {
                    0.0
                } else {
                    self.point.len().min(MAX_GPU_LIGHTS_PER_TYPE) as f32
                },
                if tiled_assignment_active {
                    0.0
                } else {
                    self.spot.len().min(MAX_GPU_LIGHTS_PER_TYPE) as f32
                },
                self.area.len().min(MAX_GPU_AREA_LIGHTS) as f32,
            ],
            ..Default::default()
        };
        let mut shadow_slot_used = false;
        for (index, light) in self
            .directional
            .iter()
            .take(MAX_GPU_LIGHTS_PER_TYPE)
            .enumerate()
        {
            uniform.directional_light_direction_intensity[index] = [
                light.direction.x,
                light.direction.y,
                light.direction.z,
                directional_illuminance_lux(light.illuminance_lux),
            ];
            uniform.directional_light_color[index] =
                [light.color.r, light.color.g, light.color.b, 0.0];
            if light.casts_shadows && !shadow_slot_used {
                uniform.directional_shadow_control[index] = [1.0, 0.0, 0.0, 0.0];
                shadow_slot_used = true;
            }
        }
        for (index, light) in self.point.iter().take(MAX_GPU_LIGHTS_PER_TYPE).enumerate() {
            uniform.point_light_position_intensity[index] = [
                light.position.x,
                light.position.y,
                light.position.z,
                punctual_intensity_candela(light.intensity_candela),
            ];
            uniform.point_light_color_range[index] = [
                light.color.r,
                light.color.g,
                light.color.b,
                light.range.unwrap_or(0.0).max(0.0),
            ];
        }
        for (index, light) in self.spot.iter().take(MAX_GPU_LIGHTS_PER_TYPE).enumerate() {
            uniform.spot_light_position_intensity[index] = [
                light.position.x,
                light.position.y,
                light.position.z,
                punctual_intensity_candela(light.intensity_candela),
            ];
            uniform.spot_light_direction_cones[index] =
                [light.direction.x, light.direction.y, light.direction.z, 0.0];
            uniform.spot_light_cone_range[index] = [
                light.inner_cone_cos,
                light.outer_cone_cos,
                light.range.unwrap_or(0.0).max(0.0),
                0.0,
            ];
            uniform.spot_light_color_range[index] =
                [light.color.r, light.color.g, light.color.b, 0.0];
        }
        for (index, light) in self.area.iter().take(MAX_GPU_AREA_LIGHTS).enumerate() {
            uniform.area_light_position_flux[index] = [
                light.position.x,
                light.position.y,
                light.position.z,
                light.luminous_flux_lumens,
            ];
            uniform.area_light_axis_x_shape[index] = [
                light.axis_x.x,
                light.axis_x.y,
                light.axis_x.z,
                area_light_shape_code(light.shape),
            ];
            uniform.area_light_axis_y_range[index] = [
                light.axis_y.x,
                light.axis_y.y,
                light.axis_y.z,
                light.range.unwrap_or(0.0).max(0.0),
            ];
            uniform.area_light_color[index] = [light.color.r, light.color.g, light.color.b, 0.0];
        }
        if environment.is_active() {
            uniform.environment_diffuse_intensity = environment.gpu_diffuse_intensity();
            uniform.environment_specular_intensity = environment.gpu_specular_intensity();
        }
        uniform
    }
}
