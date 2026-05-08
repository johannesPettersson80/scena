use crate::diagnostics::LookupError;

use super::{Camera, CameraKey, Scene};

impl Scene {
    pub(crate) fn ensure_camera_depth_reaches(
        &mut self,
        camera: CameraKey,
        far_distance: f32,
    ) -> Result<(), LookupError> {
        if !far_distance.is_finite() || far_distance <= 0.0 {
            return Ok(());
        }

        let required_far = far_distance + far_distance.max(1.0);
        let camera = self
            .cameras
            .get_mut(camera)
            .ok_or(LookupError::CameraNotFound(camera))?;
        let changed = match camera {
            Camera::Perspective(camera) => {
                let far = camera.far.max(required_far);
                let changed = far != camera.far;
                camera.far = far;
                changed
            }
            Camera::Orthographic(camera) => {
                let far = camera.far.max(required_far);
                let near = camera.near.min(-required_far);
                let changed = far != camera.far || near != camera.near;
                camera.far = far;
                camera.near = near;
                changed
            }
        };

        if changed {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }
}
