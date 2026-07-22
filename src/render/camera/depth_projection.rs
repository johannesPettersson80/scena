use super::*;

impl CameraProjection {
    /// Projects a vertex already clipped to the camera depth slab. The supplied
    /// depth is the exact clipping result, avoiding a second range rejection
    /// from floating-point roundoff at the near or far plane.
    pub(in crate::render) fn project_clipped(
        &self,
        world_position: Vec3,
        view_depth: f32,
    ) -> Option<ProjectedVertex> {
        let view = self.world_to_view(world_position)?;
        let [near, far] = self.near_far();
        if !view_depth.is_finite() || view_depth < near || view_depth > far {
            return None;
        }
        match self.camera {
            Camera::Perspective(camera) => {
                let aspect = positive_or(
                    camera.aspect,
                    self.target.width.max(1) as f32 / self.target.height.max(1) as f32,
                );
                let focal = (camera.vertical_fov.radians() * 0.5).tan().recip();
                if !focal.is_finite() || view_depth <= 0.0 {
                    return None;
                }
                Some(ProjectedVertex {
                    ndc_x: (view.x * focal / aspect) / view_depth,
                    ndc_y: (view.y * focal) / view_depth,
                    depth: perspective_depth_buffer_value(view_depth, near, far)?,
                    view_depth,
                })
            }
            Camera::Orthographic(camera) => {
                let width = camera.right - camera.left;
                let height = camera.top - camera.bottom;
                if width.abs() <= f32::EPSILON || height.abs() <= f32::EPSILON {
                    return None;
                }
                Some(ProjectedVertex {
                    ndc_x: (view.x - camera.left) / width * 2.0 - 1.0,
                    ndc_y: (view.y - camera.bottom) / height * 2.0 - 1.0,
                    depth: orthographic_depth_buffer_value(view_depth, near, far)?,
                    view_depth,
                })
            }
        }
    }
}
