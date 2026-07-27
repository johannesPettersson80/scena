use super::visual_patch::{VisualPatchCameraEasedV1, VisualPatchResultV1, VisualPatchV1};
use super::{
    SceneHostCameraState, SceneHostCore, SceneHostEasing, SceneHostError, SceneHostErrorCode,
};
#[cfg(target_arch = "wasm32")]
use crate::OrbitControlAction;
use crate::{
    AssetFetcher, CameraBookmark, CameraKey, FramingOptions, LookupError,
    OrbitControlAction as HostOrbitControlAction, OrbitControls, PointerButton, PointerEvent,
    PointerEventKind, Scene, Vec3,
};

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
        let action = self.camera_controls.handle_pointer(event);
        if matches!(
            action,
            HostOrbitControlAction::Orbit
                | HostOrbitControlAction::Pan
                | HostOrbitControlAction::Zoom
        ) {
            self.cancel_camera_transition();
            self.camera_controls
                .apply_to_scene(&mut self.scene, self.active_camera)?;
        }
        Ok(action)
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
