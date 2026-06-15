use serde::{Deserialize, Serialize};

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    Aabb, AssetFetcher, ClippingPlane, Color, GeometryDesc, MaterialDesc, SectionBox, Vec3,
};

pub const SCENE_HOST_SECTION_BOX_SCHEMA_V1: &str = "scena.scene_host_section_box.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneHostSectionBoxReportV1 {
    pub schema: String,
    pub enabled: bool,
    pub inverted: bool,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub margin: f32,
    pub planes: Vec<SceneHostClippingPlaneV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub helper_node: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SceneHostClippingPlaneV1 {
    pub normal: [f32; 3],
    pub distance: f32,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_section_box_json(
        &mut self,
        bounds: Aabb,
        margin: f32,
        inverted: bool,
        helper_wireframe: bool,
    ) -> Result<String, SceneHostError> {
        let section = SectionBox::from_bounds(bounds)
            .with_margin(margin)
            .with_inverted(inverted);
        self.set_section_box_state(section, helper_wireframe)?;
        self.section_box_report_json()
    }

    pub fn clear_section_box_json(&mut self) -> Result<String, SceneHostError> {
        self.clear_section_box_state()?;
        self.section_box_report_json()
    }

    pub fn invert_section_box_json(&mut self, inverted: bool) -> Result<String, SceneHostError> {
        self.invert_section_box_state(inverted)?;
        self.section_box_report_json()
    }

    pub(super) fn set_section_box_state(
        &mut self,
        section: SectionBox,
        helper_wireframe: bool,
    ) -> Result<bool, SceneHostError> {
        let previous = self.scene.section_box();
        let mut changed = self.scene.set_section_box(section)?;
        if helper_wireframe {
            if self.section_box_helper.is_none() || previous != Some(section) {
                self.remove_section_box_helper()?;
                let helper = self.add_section_box_helper(section)?;
                self.section_box_helper = Some(helper);
                changed = true;
            }
        } else {
            changed |= self.remove_section_box_helper()?;
        }
        Ok(changed)
    }

    pub(super) fn clear_section_box_state(&mut self) -> Result<bool, SceneHostError> {
        let changed = self.scene.clear_section_box();
        Ok(changed | self.remove_section_box_helper()?)
    }

    pub(super) fn invert_section_box_state(
        &mut self,
        inverted: bool,
    ) -> Result<bool, SceneHostError> {
        Ok(self.scene.invert_section_box(inverted)?)
    }

    fn section_box_report_json(&self) -> Result<String, SceneHostError> {
        serde_json::to_string(&self.section_box_report()).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("failed to serialize section box report: {error}"),
            )
        })
    }

    fn section_box_report(&self) -> SceneHostSectionBoxReportV1 {
        let section = self.scene.section_box();
        let planes = self
            .scene
            .section_box_planes()
            .unwrap_or_default()
            .into_iter()
            .map(SceneHostClippingPlaneV1::from)
            .collect();
        let (enabled, inverted, min, max, margin) =
            section.map_or((false, false, [0.0; 3], [0.0; 3], 0.0), |section| {
                (
                    true,
                    section.inverted(),
                    section.bounds().min.to_array(),
                    section.bounds().max.to_array(),
                    section.margin(),
                )
            });
        SceneHostSectionBoxReportV1 {
            schema: SCENE_HOST_SECTION_BOX_SCHEMA_V1.to_owned(),
            enabled,
            inverted,
            min,
            max,
            margin,
            planes,
            helper_node: self.section_box_helper,
        }
    }

    fn add_section_box_helper(&mut self, section: SectionBox) -> Result<u64, SceneHostError> {
        let geometry = self
            .assets
            .create_geometry(GeometryDesc::bounding_box(section.expanded_bounds()));
        let material = self
            .assets
            .create_material(MaterialDesc::line(Color::YELLOW, 1.0));
        let root = self.scene.root();
        let node = self.scene.mesh(geometry, material).parent(root).add()?;
        self.scene.set_helper_on_top(node, true)?;
        self.scene.add_tag(node, "scena:section_box:helper")?;
        Ok(self.register_node(node))
    }

    fn remove_section_box_helper(&mut self) -> Result<bool, SceneHostError> {
        let Some(helper) = self.section_box_helper.take() else {
            return Ok(false);
        };
        match self.remove_node(helper) {
            Ok(()) => Ok(true),
            Err(error)
                if matches!(
                    error.code(),
                    SceneHostErrorCode::NodeHandleNotFound | SceneHostErrorCode::StaleNodeHandle
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }
}

impl From<ClippingPlane> for SceneHostClippingPlaneV1 {
    fn from(plane: ClippingPlane) -> Self {
        Self {
            normal: plane.normal().to_array(),
            distance: plane.distance(),
        }
    }
}

pub(super) fn aabb_from_arrays(min: [f32; 3], max: [f32; 3]) -> Aabb {
    Aabb::new(Vec3::from_array(min), Vec3::from_array(max))
}
