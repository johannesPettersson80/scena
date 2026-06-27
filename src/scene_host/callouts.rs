use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError};
use crate::{AssetFetcher, Callout, Color, Vec3};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneHostCalloutReportV1 {
    pub id: String,
    pub anchor_id: String,
    pub leader_line_node: u64,
    pub label_node: u64,
    pub text: String,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn add_node_callout(
        &mut self,
        id: &str,
        node: u64,
        local_offset: [f32; 3],
        label_offset: [f32; 3],
        text: &str,
    ) -> Result<SceneHostCalloutReportV1, SceneHostError> {
        let node_key = self.resolve_node(node)?;
        let report = self.scene.add_callout(
            &self.assets,
            Callout::node(id, node_key, vec3(local_offset), text)
                .with_label_offset(vec3(label_offset))
                .with_color(Color::CYAN),
        )?;
        Ok(SceneHostCalloutReportV1 {
            id: report.id,
            anchor_id: report.anchor_id,
            leader_line_node: self.register_node(report.leader_line_node),
            label_node: self.register_node(report.label_node),
            text: report.text,
        })
    }

    pub fn add_world_callout(
        &mut self,
        id: &str,
        position: [f32; 3],
        label_offset: [f32; 3],
        text: &str,
    ) -> Result<SceneHostCalloutReportV1, SceneHostError> {
        let report = self.scene.add_callout(
            &self.assets,
            Callout::world(id, vec3(position), text).with_label_offset(vec3(label_offset)),
        )?;
        Ok(SceneHostCalloutReportV1 {
            id: report.id,
            anchor_id: report.anchor_id,
            leader_line_node: self.register_node(report.leader_line_node),
            label_node: self.register_node(report.label_node),
            text: report.text,
        })
    }

    pub fn clear_callout(&mut self, id: &str) -> bool {
        let Some(callout) = self.scene.callout_state(id).cloned() else {
            return false;
        };
        let removed = self.scene.clear_callout(id);
        if removed {
            self.invalidate_node_handles(&[callout.leader_line_node(), callout.label_node()]);
        }
        removed
    }
}

const fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}
