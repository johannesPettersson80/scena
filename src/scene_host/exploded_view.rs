use serde::{Deserialize, Serialize};

use super::visual_patch::{VisualPatchTransformEasedV1, VisualPatchTransformV1, VisualPatchV1};
use super::{SceneHostCore, SceneHostEasing, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, ExplodedView, Vec3};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostExplodedViewOptionsV1 {
    #[serde(default)]
    pub mode: SceneHostExplodedViewModeV1,
    #[serde(default)]
    pub axis: Option<[f32; 3]>,
    #[serde(default = "default_factor")]
    pub factor: f32,
    #[serde(default = "default_distance")]
    pub distance: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(default)]
    pub easing: SceneHostEasing,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneHostExplodedViewModeV1 {
    #[default]
    DirectChildren,
    HierarchyDepth,
    Axis,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn exploded_view_patch(
        &mut self,
        root: u64,
        options: SceneHostExplodedViewOptionsV1,
    ) -> Result<VisualPatchV1, SceneHostError> {
        let root = self.resolve_node(root)?;
        validate_host_exploded_options(&options)?;
        let mut view = ExplodedView::from_node(root)
            .factor(options.factor)
            .distance(options.distance);
        match options.mode {
            SceneHostExplodedViewModeV1::DirectChildren => {}
            SceneHostExplodedViewModeV1::HierarchyDepth => {
                view = view.by_hierarchy_depth();
            }
            SceneHostExplodedViewModeV1::Axis => {
                let axis = options.axis.unwrap_or([1.0, 0.0, 0.0]);
                view = view.along_axis(Vec3::new(axis[0], axis[1], axis[2]));
            }
        }
        let plan = view.transforms(&self.scene, &self.assets)?;
        let mut patch = VisualPatchV1::default();

        match options.duration_seconds {
            Some(duration_seconds) if duration_seconds > 0.0 => {
                for update in plan.updates() {
                    let handle = self.register_node(update.node);
                    patch.transforms_eased.push(VisualPatchTransformEasedV1 {
                        node: handle,
                        transform: update.transform,
                        duration_seconds,
                        easing: options.easing,
                    });
                }
            }
            _ => {
                for update in plan.updates() {
                    let handle = self.register_node(update.node);
                    patch.transforms.push(VisualPatchTransformV1 {
                        node: handle,
                        transform: update.transform,
                    });
                }
            }
        }
        Ok(patch)
    }

    pub fn exploded_view_patch_json(
        &mut self,
        root: u64,
        options_json: &str,
    ) -> Result<String, SceneHostError> {
        let options: SceneHostExplodedViewOptionsV1 =
            serde_json::from_str(options_json).map_err(|error| {
                SceneHostError::new(
                    SceneHostErrorCode::InvalidInput,
                    format!("invalid exploded view options JSON: {error}"),
                )
            })?;
        let patch = self.exploded_view_patch(root, options)?;
        serde_json::to_string(&patch).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("exploded view patch serialization failed: {error}"),
            )
        })
    }
}

fn validate_host_exploded_options(
    options: &SceneHostExplodedViewOptionsV1,
) -> Result<(), SceneHostError> {
    if let Some(duration_seconds) = options.duration_seconds
        && (!duration_seconds.is_finite() || duration_seconds < 0.0)
    {
        return Err(SceneHostError::new(
            SceneHostErrorCode::InvalidInput,
            "exploded view duration_seconds must be finite and non-negative",
        ));
    }
    Ok(())
}

fn default_factor() -> f32 {
    1.0
}

fn default_distance() -> f32 {
    1.0
}
