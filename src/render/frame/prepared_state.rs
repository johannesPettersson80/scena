use super::*;

impl Renderer {
    pub(in crate::render) fn prepared_state(
        &self,
        scene: &Scene,
    ) -> Result<&PreparedSceneState, RenderError> {
        let prepared = self.prepared.as_ref().ok_or(RenderError::NotPrepared {
            reason: NotPreparedReason::NeverPrepared,
        })?;

        if !prepared.scene.ptr_eq(&scene.identity()) {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::DifferentScene,
            });
        }

        let current_revision = scene.structure_revision();
        if prepared.structure_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.structure_revision,
                    current_revision,
                    change: ChangeKind::SceneStructure,
                },
            });
        }

        let current_revision = scene.transform_revision();
        if prepared.transform_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.transform_revision,
                    current_revision,
                    change: ChangeKind::Transform,
                },
            });
        }

        let current_revision = scene.camera_revision();
        if prepared.camera_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.camera_revision,
                    current_revision,
                    change: ChangeKind::Camera,
                },
            });
        }

        let current_revision = scene.appearance_revision();
        if prepared.appearance_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.appearance_revision,
                    current_revision,
                    change: ChangeKind::Appearance,
                },
            });
        }

        let current_revision = scene.visibility_revision();
        if prepared.visibility_revision != current_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::SceneChanged {
                    prepared_revision: prepared.visibility_revision,
                    current_revision,
                    change: ChangeKind::Visibility,
                },
            });
        }

        if prepared.environment_revision != self.environment_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::EnvironmentChanged {
                    prepared_revision: prepared.environment_revision,
                    current_revision: self.environment_revision,
                    change: ChangeKind::Environment,
                },
            });
        }

        if prepared.target_revision != self.target_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::TargetChanged {
                    prepared_revision: prepared.target_revision,
                    current_revision: self.target_revision,
                    change: ChangeKind::RenderTarget,
                },
            });
        }

        if prepared.output_resources_revision != self.output_resources_revision {
            return Err(RenderError::NotPrepared {
                reason: NotPreparedReason::OutputSettingsChanged {
                    prepared_revision: prepared.output_resources_revision,
                    current_revision: self.output_resources_revision,
                    change: ChangeKind::OutputSettings,
                },
            });
        }

        Ok(prepared)
    }
}
