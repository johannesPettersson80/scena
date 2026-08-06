use super::visual_patch::{VisualPatchCameraEasedV1, VisualPatchResultV1, VisualPatchV1};
use super::{
    SceneHostCameraState, SceneHostCore, SceneHostEasing, SceneHostError, SceneHostErrorCode,
};
#[cfg(target_arch = "wasm32")]
use crate::OrbitControlAction;
use crate::{
    AssetFetcher, Camera, CameraBookmark, CameraKey, DepthRange, FramingOptions, LookupError,
    OrbitControlAction as HostOrbitControlAction, OrbitControls, OrthographicCamera,
    PerspectiveCamera, PointerButton, PointerEvent, PointerEventKind, Scene, Vec3,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostCameraProjection {
    Perspective,
    Orthographic,
}

impl SceneHostCameraProjection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Perspective => "perspective",
            Self::Orthographic => "orthographic",
        }
    }

    pub fn parse(value: &str) -> Result<Self, SceneHostError> {
        match value {
            "perspective" => Ok(Self::Perspective),
            "orthographic" => Ok(Self::Orthographic),
            _ => Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("unsupported camera projection '{value}'"),
            )),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) const fn orbit_action_name(action: OrbitControlAction) -> &'static str {
    match action {
        OrbitControlAction::None => "none",
        OrbitControlAction::BeginOrbit => "begin_orbit",
        OrbitControlAction::Orbit => "orbit",
        OrbitControlAction::Pan => "pan",
        OrbitControlAction::Zoom => "zoom",
        OrbitControlAction::End => "end",
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn camera_projection(&self) -> Result<SceneHostCameraProjection, SceneHostError> {
        match self.scene.camera(self.active_camera) {
            Some(Camera::Perspective(_)) => Ok(SceneHostCameraProjection::Perspective),
            Some(Camera::Orthographic(_)) => Ok(SceneHostCameraProjection::Orthographic),
            None => Err(SceneHostError::new(
                SceneHostErrorCode::Inspect,
                "active camera is unavailable",
            )),
        }
    }

    pub fn set_camera_projection(
        &mut self,
        projection: SceneHostCameraProjection,
    ) -> Result<(), SceneHostError> {
        self.ensure_active_camera()?;
        if self.camera_projection()? == projection {
            return Ok(());
        }

        let distance = self.camera_controls.distance();
        let aspect = self.viewport_aspect();
        let current = self
            .scene
            .camera(self.active_camera)
            .cloned()
            .ok_or(LookupError::CameraNotFound(self.active_camera))?;
        let replacement = match (current, projection) {
            (Camera::Perspective(camera), SceneHostCameraProjection::Orthographic) => {
                let half_height =
                    (distance * (camera.vertical_fov.radians() * 0.5).tan()).max(0.0001);
                let half_width = half_height * aspect;
                Camera::Orthographic(OrthographicCamera {
                    left: -half_width,
                    right: half_width,
                    bottom: -half_height,
                    top: half_height,
                    near: -camera.far.max(distance * 2.0),
                    far: camera.far.max(distance * 2.0),
                })
            }
            (Camera::Orthographic(camera), SceneHostCameraProjection::Perspective) => {
                let half_height = ((camera.top - camera.bottom).abs() * 0.5).max(0.0001);
                let fov_degrees = (2.0 * (half_height / distance).atan()).to_degrees();
                let far = camera.far.abs().max(distance * 2.0).max(1.0);
                Camera::Perspective(
                    PerspectiveCamera::default()
                        .with_fov_degrees(fov_degrees)
                        .with_aspect(aspect)
                        .with_depth_range(DepthRange::new(0.001, far)),
                )
            }
            (camera, _) => camera,
        };

        self.cancel_camera_transition();
        self.scene.set_camera(self.active_camera, replacement)?;
        self.camera_controls
            .apply_to_scene(&mut self.scene, self.active_camera)?;
        Ok(())
    }

    fn viewport_aspect(&self) -> f32 {
        (self.viewport.logical_width() / self.viewport.logical_height()).max(0.0001)
    }

    pub fn camera_state(&self) -> SceneHostCameraState {
        SceneHostCameraState::from_controls(&self.camera_controls)
    }

    pub fn get_camera(&self) -> SceneHostCameraState {
        self.camera_state()
    }

    pub fn camera_json(&self) -> Result<String, SceneHostError> {
        serde_json::to_string(&self.camera_state()).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("camera state serialization failed: {error}"),
            )
        })
    }

    pub fn set_camera(&mut self, state: SceneHostCameraState) -> Result<(), SceneHostError> {
        self.cancel_camera_transition();
        self.apply_camera_state(state)
    }

    pub(super) fn apply_camera_state(
        &mut self,
        state: SceneHostCameraState,
    ) -> Result<(), SceneHostError> {
        state.validate().map_err(|message| {
            SceneHostError::new(SceneHostErrorCode::InvalidInput, message.to_owned())
        })?;
        let controls = state.into_controls();
        controls.apply_to_scene(&mut self.scene, self.active_camera)?;
        self.camera_controls = controls;
        Ok(())
    }

    pub fn set_camera_json(&mut self, json: &str) -> Result<(), SceneHostError> {
        let state: SceneHostCameraState = serde_json::from_str(json).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("invalid camera JSON: {error}"),
            )
        })?;
        self.set_camera(state)
    }

    pub fn set_camera_bookmark(
        &mut self,
        bookmark: &CameraBookmark,
        duration_seconds: f64,
        easing: SceneHostEasing,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        self.apply_camera_eased_patch(bookmark.state(), duration_seconds, easing)
    }

    pub fn set_camera_bookmark_json(
        &mut self,
        json: &str,
        duration_seconds: f64,
        easing: SceneHostEasing,
    ) -> Result<String, SceneHostError> {
        let bookmark: CameraBookmark = serde_json::from_str(json).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("invalid camera bookmark JSON: {error}"),
            )
        })?;
        let result = self.set_camera_bookmark(&bookmark, duration_seconds, easing)?;
        serde_json::to_string(&result).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("camera bookmark result serialization failed: {error}"),
            )
        })
    }

    pub(super) fn apply_camera_eased_patch(
        &mut self,
        camera: SceneHostCameraState,
        duration_seconds: f64,
        easing: SceneHostEasing,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        self.apply_patch(&VisualPatchV1 {
            camera_eased: Some(VisualPatchCameraEasedV1 {
                camera,
                duration_seconds,
                easing,
            }),
            ..VisualPatchV1::default()
        })
    }

    pub fn camera_pointer_down(
        &mut self,
        x: f32,
        y: f32,
        button: PointerButton,
    ) -> Result<HostOrbitControlAction, SceneHostError> {
        self.apply_camera_pointer(PointerEvent {
            kind: PointerEventKind::Pressed,
            position: (x, y),
            button: Some(button),
            delta: (0.0, 0.0),
            scroll_delta: 0.0,
        })
    }

    pub fn camera_pointer_move(
        &mut self,
        x: f32,
        y: f32,
        delta_x: f32,
        delta_y: f32,
    ) -> Result<HostOrbitControlAction, SceneHostError> {
        self.apply_camera_pointer(PointerEvent::moved(x, y, delta_x, delta_y))
    }

    pub fn camera_pointer_up(
        &mut self,
        x: f32,
        y: f32,
    ) -> Result<HostOrbitControlAction, SceneHostError> {
        self.apply_camera_pointer(PointerEvent::released(x, y))
    }

    pub fn camera_wheel(
        &mut self,
        x: f32,
        y: f32,
        delta_y: f32,
    ) -> Result<HostOrbitControlAction, SceneHostError> {
        self.apply_camera_pointer(PointerEvent::wheel(x, y, delta_y))
    }

    pub fn frame_node(&mut self, node: u64) -> Result<(), SceneHostError> {
        self.ensure_active_camera()?;
        let node = self.resolve_node(node)?;
        let bounds = self
            .scene
            .node_world_bounds(node, &self.assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        self.cancel_camera_transition();
        self.scene.frame(self.active_camera, bounds)?;
        self.camera_controls =
            controls_from_scene_camera(&self.scene, self.active_camera, bounds.center())?;
        Ok(())
    }

    pub fn frame_node_product_view(&mut self, node: u64) -> Result<(), SceneHostError> {
        self.frame_node_with_preset(node, "product_viewer_default")
    }

    pub fn frame_node_with_preset(
        &mut self,
        node: u64,
        preset: &str,
    ) -> Result<(), SceneHostError> {
        self.ensure_active_camera()?;
        let node = self.resolve_node(node)?;
        let bounds = self
            .scene
            .node_world_bounds(node, &self.assets)?
            .ok_or(LookupError::ImportHasNoBounds)?;
        let width = self.viewport.logical_width().round().max(1.0) as u32;
        let height = self.viewport.logical_height().round().max(1.0) as u32;
        let min_viewport = width.min(height) as f32;
        let (options, fill, margin_px) = match preset {
            "cell_overview" => (FramingOptions::new().top(), 0.72, 48.0),
            "operator_review_default" => (
                FramingOptions::new().orbit(35.0_f32.to_radians(), 14.0_f32.to_radians()),
                0.78,
                48.0,
            ),
            "camera_behavior" | "product_hero" => (
                FramingOptions::new().three_quarter_front_right(),
                0.78,
                (min_viewport * 0.06).clamp(10.0, 48.0),
            ),
            "product_viewer_default" => (
                FramingOptions::new().three_quarter_front_right(),
                0.72,
                48.0,
            ),
            _ => {
                return Err(SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    format!("unsupported SceneHost camera preset {preset}"),
                ));
            }
        };
        let framing = self.scene.frame_bounds(
            self.active_camera,
            bounds,
            options
                .fill(fill)
                .margin_px(margin_px)
                .viewport(width, height),
        )?;
        self.cancel_camera_transition();
        self.camera_controls = OrbitControls::from_framing(framing);
        Ok(())
    }

    pub fn frame_all(&mut self) -> Result<(), SceneHostError> {
        self.ensure_active_camera()?;
        let width = self.viewport.logical_width().round().max(1.0) as u32;
        let height = self.viewport.logical_height().round().max(1.0) as u32;
        let framing = self.scene.frame_all_with_assets_and_options(
            self.active_camera,
            &self.assets,
            FramingOptions::new()
                .three_quarter_front_right()
                .tighten_depth_range(true)
                .viewport(width, height),
        )?;
        self.cancel_camera_transition();
        self.camera_controls = OrbitControls::from_framing(framing);
        Ok(())
    }

    pub fn frame_all_with_overlays(&mut self) -> Result<(), SceneHostError> {
        self.ensure_active_camera()?;
        let width = self.viewport.logical_width().round().max(1.0) as u32;
        let height = self.viewport.logical_height().round().max(1.0) as u32;
        let framing =
            self.scene
                .frame_all_with_overlays(self.active_camera, &self.assets, width, height)?;
        self.cancel_camera_transition();
        self.camera_controls = OrbitControls::from_framing(framing);
        Ok(())
    }

    fn apply_camera_pointer(
        &mut self,
        event: PointerEvent,
    ) -> Result<HostOrbitControlAction, SceneHostError> {
        let previous_distance = self.camera_controls.distance();
        let action = self.camera_controls.handle_pointer(event);
        if matches!(
            action,
            HostOrbitControlAction::Orbit
                | HostOrbitControlAction::Pan
                | HostOrbitControlAction::Zoom
        ) {
            self.cancel_camera_transition();
            self.scale_orthographic_projection(previous_distance, self.camera_controls.distance())?;
            self.camera_controls
                .apply_to_scene(&mut self.scene, self.active_camera)?;
        }
        Ok(action)
    }

    // An orthographic camera ignores its distance to the target, so orbit zoom
    // must rescale the frustum itself for the distance change to be visible.
    fn scale_orthographic_projection(
        &mut self,
        previous_distance: f32,
        next_distance: f32,
    ) -> Result<(), SceneHostError> {
        if !previous_distance.is_finite()
            || !next_distance.is_finite()
            || previous_distance <= 0.0
            || next_distance <= 0.0
        {
            return Ok(());
        }
        let scale = next_distance / previous_distance;
        if scale == 1.0 {
            return Ok(());
        }
        let Some(Camera::Orthographic(camera)) = self.scene.camera(self.active_camera).cloned()
        else {
            return Ok(());
        };
        let center_x = (camera.left + camera.right) * 0.5;
        let center_y = (camera.bottom + camera.top) * 0.5;
        let half_width = (camera.right - camera.left).abs() * 0.5 * scale;
        let half_height = (camera.top - camera.bottom).abs() * 0.5 * scale;
        self.scene.set_camera(
            self.active_camera,
            Camera::Orthographic(OrthographicCamera {
                left: center_x - half_width,
                right: center_x + half_width,
                bottom: center_y - half_height,
                top: center_y + half_height,
                ..camera
            }),
        )?;
        Ok(())
    }
}

pub(super) fn controls_from_scene_camera(
    scene: &Scene,
    camera: CameraKey,
    target: Vec3,
) -> Result<OrbitControls, SceneHostError> {
    let camera_node = scene
        .camera_node(camera)
        .ok_or(LookupError::CameraNotFound(camera))?;
    let camera_world = scene
        .world_transform(camera_node)
        .ok_or(LookupError::NodeNotFound(camera_node))?;
    let offset = camera_world.translation - target;
    let distance = offset.length().max(0.0001);
    let yaw_radians = offset.x.atan2(offset.z);
    let pitch_radians = (offset.y / distance).clamp(-1.0, 1.0).asin();
    Ok(OrbitControls::new(target, distance).with_angles(yaw_radians, pitch_radians))
}
