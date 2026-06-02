use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostics::LookupError;

use super::{CameraKey, NodeKey, Scene, Transform, Vec3};

pub const SCENE_ANNOTATION_PROJECTION_SCHEMA_V1: &str = "scena.annotation_projection.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationAnchor {
    id: String,
    target: AnnotationAnchorTarget,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnnotationAnchorTarget {
    World { position: Vec3 },
    Node { node: NodeKey, local_offset: Vec3 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotationProjectionReportV1 {
    pub schema: String,
    pub coordinate_space: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub annotations: Vec<AnnotationProjectionV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotationProjectionV1 {
    pub id: String,
    #[serde(default)]
    pub node_handle: Option<u64>,
    pub x: f32,
    pub y: f32,
    pub visible: bool,
}

impl AnnotationAnchor {
    pub fn world(id: impl Into<String>, position: Vec3) -> Self {
        Self {
            id: id.into(),
            target: AnnotationAnchorTarget::World { position },
        }
    }

    pub fn node(id: impl Into<String>, node: NodeKey, local_offset: Vec3) -> Self {
        Self {
            id: id.into(),
            target: AnnotationAnchorTarget::Node { node, local_offset },
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn target(&self) -> AnnotationAnchorTarget {
        self.target
    }

    pub(crate) const fn target_node(&self) -> Option<NodeKey> {
        match self.target {
            AnnotationAnchorTarget::Node { node, .. } => Some(node),
            AnnotationAnchorTarget::World { .. } => None,
        }
    }
}

impl AnnotationProjectionReportV1 {
    pub fn to_schema_json(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("annotation projection report is JSON-serializable")
    }
}

impl Scene {
    pub fn set_annotation_anchor(&mut self, anchor: AnnotationAnchor) -> Result<(), LookupError> {
        if let Some(node) = anchor.target_node()
            && !self.nodes.contains_key(node)
        {
            return Err(LookupError::NodeNotFound(node));
        }

        let changed = self.annotations.get(anchor.id()) != Some(&anchor);
        self.annotations.insert(anchor.id.clone(), anchor);
        if changed {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }

    pub fn clear_annotation_anchor(&mut self, id: &str) -> bool {
        let removed = self.annotations.remove(id).is_some();
        if removed {
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        removed
    }

    pub fn annotation_anchor(&self, id: &str) -> Option<&AnnotationAnchor> {
        self.annotations.get(id)
    }

    pub fn annotation_anchors(&self) -> impl Iterator<Item = &AnnotationAnchor> {
        self.annotations.values()
    }

    pub fn annotation_projection_report(
        &self,
        camera: CameraKey,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<AnnotationProjectionReportV1, LookupError> {
        if viewport_width == 0 || viewport_height == 0 {
            return Err(LookupError::InvalidViewport {
                width: viewport_width,
                height: viewport_height,
            });
        }
        if !self.cameras.contains_key(camera) {
            return Err(LookupError::CameraNotFound(camera));
        }

        let annotations = self
            .annotations
            .values()
            .map(|anchor| self.project_annotation(anchor, camera, viewport_width, viewport_height))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AnnotationProjectionReportV1 {
            schema: SCENE_ANNOTATION_PROJECTION_SCHEMA_V1.to_owned(),
            coordinate_space: "viewport_pixels".to_owned(),
            viewport_width,
            viewport_height,
            annotations,
        })
    }

    pub fn annotation_projection_report_with_node_handles(
        &self,
        camera: CameraKey,
        viewport_width: u32,
        viewport_height: u32,
        node_handles: &BTreeMap<NodeKey, u64>,
    ) -> Result<AnnotationProjectionReportV1, LookupError> {
        let mut report =
            self.annotation_projection_report(camera, viewport_width, viewport_height)?;
        for projection in &mut report.annotations {
            projection.node_handle = self
                .annotation_anchor(&projection.id)
                .and_then(|anchor| anchor.target_node())
                .and_then(|node| node_handles.get(&node).copied());
        }
        Ok(report)
    }

    fn project_annotation(
        &self,
        anchor: &AnnotationAnchor,
        camera: CameraKey,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<AnnotationProjectionV1, LookupError> {
        let world_position = match anchor.target {
            AnnotationAnchorTarget::World { position } => position,
            AnnotationAnchorTarget::Node { node, local_offset } => {
                let world_transform = self
                    .world_transform(node)
                    .ok_or(LookupError::NodeNotFound(node))?;
                Transform::compose(world_transform, Transform::at(local_offset)).translation
            }
        };

        let projected =
            self.project_world_point(camera, world_position, viewport_width, viewport_height)?;
        let (x, y, visible) = match projected {
            Some(point) => (
                point.x,
                point.y,
                point.ndc_x.abs() <= 1.0 && point.ndc_y.abs() <= 1.0,
            ),
            None => (0.0, 0.0, false),
        };

        Ok(AnnotationProjectionV1 {
            id: anchor.id.clone(),
            node_handle: None,
            x,
            y,
            visible,
        })
    }
}
