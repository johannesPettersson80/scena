use serde::{Deserialize, Serialize};

use super::{SceneHostClippingPlaneV1, SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{AssetFetcher, ClippingPlane, ClippingPlaneSet, Vec3};

pub const SCENE_HOST_CLIPPING_PLANES_SCHEMA_V1: &str = "scena.scene_host_clipping_planes.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneHostClippingPlanesV1 {
    pub schema: String,
    pub planes: Vec<SceneHostClippingPlaneV1>,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    pub fn set_clipping_planes_json(&mut self, json: &str) -> Result<String, SceneHostError> {
        let request: SceneHostClippingPlanesV1 = serde_json::from_str(json).map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!("invalid clipping-plane request: {error}"),
            )
        })?;
        if request.schema != SCENE_HOST_CLIPPING_PLANES_SCHEMA_V1 {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "unsupported clipping-plane schema '{}'; expected '{SCENE_HOST_CLIPPING_PLANES_SCHEMA_V1}'",
                    request.schema
                ),
            ));
        }
        if request.planes.len() > self.recipe_max_clipping_planes() {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "{} clipping planes exceed renderer limit {}",
                    request.planes.len(),
                    self.recipe_max_clipping_planes()
                ),
            ));
        }

        let planes = request
            .planes
            .iter()
            .map(|plane| {
                let normal = Vec3::from_array(plane.normal);
                if !normal.is_finite()
                    || normal.length_squared() <= f32::EPSILON
                    || !plane.distance.is_finite()
                {
                    return Err(SceneHostError::new(
                        SceneHostErrorCode::InvalidInput,
                        "clipping planes require finite non-zero normals and finite distances",
                    ));
                }
                Ok(ClippingPlane::new(normal.normalize(), plane.distance))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for key in self.host_clipping_planes.drain(..) {
            self.scene.remove_clipping_plane(key);
        }
        let mut active = ClippingPlaneSet::new();
        for plane in planes {
            let key = self.scene.add_clipping_plane(plane);
            active = active.with_plane(key);
            self.host_clipping_planes.push(key);
        }
        self.scene.set_clipping_planes(active)?;

        serde_json::to_string(&SceneHostClippingPlanesV1 {
            schema: SCENE_HOST_CLIPPING_PLANES_SCHEMA_V1.to_owned(),
            planes: self
                .scene
                .active_clipping_plane_values()
                .map(SceneHostClippingPlaneV1::from)
                .collect(),
        })
        .map_err(|error| {
            SceneHostError::new(
                SceneHostErrorCode::Inspect,
                format!("failed to serialize clipping-plane report: {error}"),
            )
        })
    }
}
