use super::super::section_box::aabb_from_arrays;
use super::super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use super::types::{
    VISUAL_PATCH_SCHEMA_V1, VisualPatchAnimationTimeModeV1, VisualPatchEntryErrorV1,
    VisualPatchLabelTargetV1, VisualPatchLabelV1, VisualPatchResultV1, VisualPatchRevisionDeltaV1,
    VisualPatchSectionBoxV1, VisualPatchV1,
};
use crate::SectionBox;
use crate::{AnnotationAnchor, AssetFetcher, Color, HitTarget, SceneDirtyState, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RevisionSnapshot {
    structure: u64,
    transform: u64,
    camera: u64,
    appearance: u64,
    visibility: u64,
    interaction: u64,
}

impl RevisionSnapshot {
    fn from_dirty(dirty: SceneDirtyState) -> Self {
        Self {
            structure: dirty.structure_revision,
            transform: dirty.transform_revision,
            camera: dirty.camera_revision,
            appearance: dirty.appearance_revision,
            visibility: dirty.visibility_revision,
            interaction: dirty.interaction_revision,
        }
    }

    fn delta_since(self, before: Self) -> VisualPatchRevisionDeltaV1 {
        VisualPatchRevisionDeltaV1 {
            structure: self.structure.saturating_sub(before.structure),
            transform: self.transform.saturating_sub(before.transform),
            camera: self.camera.saturating_sub(before.camera),
            appearance: self.appearance.saturating_sub(before.appearance),
            visibility: self.visibility.saturating_sub(before.visibility),
            interaction: self.interaction.saturating_sub(before.interaction),
        }
    }
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn apply_patch(
        &mut self,
        patch: &VisualPatchV1,
    ) -> Result<VisualPatchResultV1, SceneHostError> {
        if patch.schema != VISUAL_PATCH_SCHEMA_V1 {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "unsupported visual patch schema {}; expected {}",
                    patch.schema, VISUAL_PATCH_SCHEMA_V1
                ),
            ));
        }

        let before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
        let mut result = VisualPatchResultV1::new();
        if patch.echo_metadata {
            result.metadata = patch.metadata.clone();
        }

        for (index, entry) in patch.transforms.iter().enumerate() {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.set_transform(entry.node, entry.transform) {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if entry_after.transform != entry_before.transform {
                        result.applied.transforms += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "transforms",
                    index,
                    Some(entry.node),
                    error,
                )),
            }
        }

        for (index, entry) in patch.tints.iter().enumerate() {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match validate_tint(entry.tint)
                .and_then(|()| self.set_node_tint(entry.node, entry.tint))
            {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if entry_after.structure != entry_before.structure
                        || entry_after.appearance != entry_before.appearance
                    {
                        result.applied.tints += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "tints",
                    index,
                    Some(entry.node),
                    error,
                )),
            }
        }

        for (index, entry) in patch.visibility.iter().enumerate() {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.set_visible(entry.node, entry.visible) {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if entry_after.visibility != entry_before.visibility {
                        result.applied.visibility += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "visibility",
                    index,
                    Some(entry.node),
                    error,
                )),
            }
        }

        if let Some(camera) = patch.camera {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            let camera_before = self.camera_state();
            if camera != camera_before {
                match self.set_camera(camera) {
                    Ok(()) => {
                        let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                        if self.camera_state() != camera_before
                            || entry_after.transform != entry_before.transform
                        {
                            result.applied.camera = 1;
                        }
                    }
                    Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                        "camera", 0, None, error,
                    )),
                }
            }
        }

        for (index, entry) in patch.transforms_eased.iter().enumerate() {
            let target_changed = match self.current_host_transform(entry.node) {
                Ok(start) => start != entry.transform,
                Err(error) => {
                    result.failed.push(VisualPatchEntryErrorV1::from_error(
                        "transforms_eased",
                        index,
                        Some(entry.node),
                        error,
                    ));
                    continue;
                }
            };
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.set_transform_eased(
                entry.node,
                entry.transform,
                entry.duration_seconds,
                entry.easing,
            ) {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if target_changed || entry_after.transform != entry_before.transform {
                        result.applied.transforms_eased += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "transforms_eased",
                    index,
                    Some(entry.node),
                    error,
                )),
            }
        }

        for (index, entry) in patch.tints_eased.iter().enumerate() {
            let target_changed = match self.current_host_tint(entry.node) {
                Ok(start) => start != entry.tint,
                Err(error) => {
                    result.failed.push(VisualPatchEntryErrorV1::from_error(
                        "tints_eased",
                        index,
                        Some(entry.node),
                        error,
                    ));
                    continue;
                }
            };
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.set_node_tint_eased(
                entry.node,
                entry.tint,
                entry.duration_seconds,
                entry.easing,
            ) {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if target_changed
                        || entry_after.structure != entry_before.structure
                        || entry_after.appearance != entry_before.appearance
                    {
                        result.applied.tints_eased += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "tints_eased",
                    index,
                    Some(entry.node),
                    error,
                )),
            }
        }

        if let Some(entry) = patch.camera_eased {
            let target_changed = self.camera_state() != entry.camera;
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.set_camera_eased(entry.camera, entry.duration_seconds, entry.easing) {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if target_changed || entry_after.transform != entry_before.transform {
                        result.applied.camera_eased += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "camera_eased",
                    0,
                    None,
                    error,
                )),
            }
        }

        for (index, entry) in patch.animation_time.iter().enumerate() {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            let operation = match entry.mode {
                VisualPatchAnimationTimeModeV1::Seek => {
                    self.seek_animation(entry.mixer, entry.seconds)
                }
                VisualPatchAnimationTimeModeV1::Advance => {
                    self.advance_animation(entry.mixer, entry.seconds)
                }
            };
            match operation {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if entry_after != entry_before {
                        result.applied.animation_time += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "animation_time",
                    index,
                    Some(entry.mixer),
                    error,
                )),
            }
        }

        if let Some(entry) = patch.selection {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.patch_hit_target(entry.node) {
                Ok(target) => {
                    self.scene.set_primary_selection_target(target);
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if entry_after.interaction != entry_before.interaction {
                        result.applied.selection = 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "selection",
                    0,
                    entry.node,
                    error,
                )),
            }
        }

        if let Some(entry) = patch.hover {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.patch_hit_target(entry.node) {
                Ok(target) => {
                    self.scene.set_hover_target(target);
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if entry_after.interaction != entry_before.interaction {
                        result.applied.hover = 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "hover", 0, entry.node, error,
                )),
            }
        }

        for (index, entry) in patch.material_variants.iter().enumerate() {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            let variant_before = match self.active_material_variant(entry.import) {
                Ok(variant) => variant,
                Err(error) => {
                    result.failed.push(VisualPatchEntryErrorV1::from_error(
                        "material_variants",
                        index,
                        Some(entry.import),
                        error,
                    ));
                    continue;
                }
            };
            match self.set_active_material_variant(entry.import, entry.variant.as_deref()) {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    let variant_after = self
                        .active_material_variant(entry.import)
                        .expect("material variant import was just resolved");
                    if variant_after != variant_before
                        || entry_after.structure != entry_before.structure
                    {
                        result.applied.material_variants += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "material_variants",
                    index,
                    Some(entry.import),
                    error,
                )),
            }
        }

        for (index, entry) in patch.labels.iter().enumerate() {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            match self.apply_patch_label(entry) {
                Ok(()) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if entry_after.structure != entry_before.structure {
                        result.applied.labels += 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "labels",
                    index,
                    entry.target.handle(),
                    error,
                )),
            }
        }

        if let Some(entry) = patch.section_box {
            let entry_before = RevisionSnapshot::from_dirty(self.scene.dirty_state());
            let operation = match entry {
                VisualPatchSectionBoxV1::Set {
                    min,
                    max,
                    margin,
                    inverted,
                    helper_wireframe,
                } => self.set_section_box_state(
                    SectionBox::from_bounds(aabb_from_arrays(min, max))
                        .with_margin(margin)
                        .with_inverted(inverted),
                    helper_wireframe,
                ),
                VisualPatchSectionBoxV1::Invert { inverted } => {
                    self.invert_section_box_state(inverted)
                }
                VisualPatchSectionBoxV1::Disable => self.clear_section_box_state(),
            };
            match operation {
                Ok(changed) => {
                    let entry_after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
                    if changed || entry_after.structure != entry_before.structure {
                        result.applied.section_box = 1;
                    }
                }
                Err(error) => result.failed.push(VisualPatchEntryErrorV1::from_error(
                    "section_box",
                    0,
                    None,
                    error,
                )),
            }
        }

        let after = RevisionSnapshot::from_dirty(self.scene.dirty_state());
        result.revisions = after.delta_since(before);
        Ok(result)
    }

    pub fn apply_patch_json(&mut self, json: &str) -> Result<String, SceneHostError> {
        let patch: VisualPatchV1 = serde_json::from_str(json).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("invalid visual patch JSON: {error}"),
            )
        })?;
        let result = self.apply_patch(&patch)?;
        serde_json::to_string(&result).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("visual patch result serialization failed: {error}"),
            )
        })
    }

    fn patch_hit_target(&self, node: Option<u64>) -> Result<Option<HitTarget>, SceneHostError> {
        node.map(|handle| self.resolve_node(handle).map(HitTarget::Node))
            .transpose()
    }

    fn apply_patch_label(&mut self, entry: &VisualPatchLabelV1) -> Result<(), SceneHostError> {
        validate_label_id(&entry.id)?;
        match &entry.target {
            VisualPatchLabelTargetV1::Node { node, local_offset } => {
                let node = self.resolve_node(*node)?;
                let local_offset = vec3_from_components("label local_offset", *local_offset)?;
                self.scene.set_annotation_anchor(AnnotationAnchor::node(
                    &entry.id,
                    node,
                    local_offset,
                ))?;
            }
            VisualPatchLabelTargetV1::World { position } => {
                let position = vec3_from_components("label position", *position)?;
                self.scene
                    .set_annotation_anchor(AnnotationAnchor::world(&entry.id, position))?;
            }
            VisualPatchLabelTargetV1::Clear => {
                self.scene.clear_annotation_anchor(&entry.id);
            }
        }
        Ok(())
    }
}

fn validate_tint(tint: Option<Color>) -> Result<(), SceneHostError> {
    let Some(tint) = tint else {
        return Ok(());
    };
    let components = [tint.r, tint.g, tint.b, tint.a];
    if components.iter().all(|component| component.is_finite()) {
        return Ok(());
    }
    Err(SceneHostError::new(
        SceneHostErrorCode::InvalidInput,
        "tint must contain only finite values",
    ))
}

fn validate_label_id(id: &str) -> Result<(), SceneHostError> {
    if !id.trim().is_empty() {
        return Ok(());
    }
    Err(SceneHostError::new(
        SceneHostErrorCode::InvalidInput,
        "label id must be a non-empty string",
    ))
}

fn vec3_from_components(field: &str, values: [f32; 3]) -> Result<Vec3, SceneHostError> {
    if values.iter().all(|value| value.is_finite()) {
        return Ok(Vec3::new(values[0], values[1], values[2]));
    }
    Err(SceneHostError::new(
        SceneHostErrorCode::InvalidInput,
        format!("{field} must contain only finite values"),
    ))
}
