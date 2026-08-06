use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use slotmap::Key as _;

use super::{SceneHostCore, SceneHostError, SceneHostErrorCode};
use crate::{
    AssetFetcher, AssetPath, MaterialHandle, PhotoQualityAnalysisReportV1,
    PhotographicMaterialResolutionV1, select_photographic_material_resolution,
};

pub const PHOTOGRAPHIC_MATERIAL_RESOLUTION_SELECTION_SCHEMA_V1: &str =
    "scena.photographic_material_resolution_selection.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicMaterialResolutionSelectionReportV1 {
    pub schema: String,
    pub decoded_texture_budget_bytes: u64,
    pub decoded_texture_plan_bytes: u64,
    pub selections: Vec<PhotographicMaterialResolutionSelectionV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotographicMaterialResolutionSelectionV1 {
    pub pack_id: String,
    pub material_handle_before: u64,
    pub material_handle_after: u64,
    pub measured_texels_per_pixel_p50: f64,
    pub one_k_texels_per_pixel_p50: f64,
    pub previous_resolution: PhotographicMaterialResolutionV1,
    pub selected_resolution: PhotographicMaterialResolutionV1,
    pub changed: bool,
}

struct PlannedSelection {
    material: MaterialHandle,
    nodes: Vec<crate::NodeKey>,
    pack_id: String,
    manifest_path: AssetPath,
    measured_density: f64,
    one_k_density: f64,
    previous: PhotographicMaterialResolutionV1,
    selected: PhotographicMaterialResolutionV1,
}

impl<F: AssetFetcher> SceneHostCore<F> {
    /// Selects the smallest available 1K/2K/4K material tier that preserves at
    /// least one source texel per output pixel for each measured visible
    /// material, then rebinds the affected mesh nodes before the next prepare.
    pub async fn select_photographic_material_resolutions(
        &mut self,
        analysis: &PhotoQualityAnalysisReportV1,
        decoded_texture_budget_bytes: u64,
    ) -> Result<PhotographicMaterialResolutionSelectionReportV1, SceneHostError> {
        let inspection = self.scene.inspect_with_assets(&self.assets);
        let mut nodes_by_material = BTreeMap::<MaterialHandle, Vec<crate::NodeKey>>::new();
        for draw in inspection.draw_list() {
            nodes_by_material
                .entry(draw.material())
                .or_default()
                .push(draw.node());
        }

        let mut planned = Vec::new();
        for metric in &analysis.materials {
            let Some(density) = metric.projected_texture_density.as_ref() else {
                continue;
            };
            let Some((&material, nodes)) = nodes_by_material
                .iter()
                .find(|(material, _)| material.data().as_ffi() == metric.material_handle)
            else {
                continue;
            };
            let Some(binding) = self.assets.photographic_material_pack_binding(material) else {
                continue;
            };
            let one_k_density =
                density.texels_per_pixel_p50 / binding.resolution.scale_from_one_k();
            let Some(selected) = select_photographic_material_resolution(one_k_density) else {
                continue;
            };
            let selected = selected.max(binding.resolution);
            planned.push(PlannedSelection {
                material,
                nodes: nodes.clone(),
                pack_id: binding.pack_id,
                manifest_path: binding.manifest_path,
                measured_density: density.texels_per_pixel_p50,
                one_k_density,
                previous: binding.resolution,
                selected,
            });
        }
        planned.sort_by_key(|selection| selection.material.data().as_ffi());

        let selected_by_material = planned
            .iter()
            .map(|selection| (selection.material, selection.selected))
            .collect::<BTreeMap<_, _>>();
        let mut planned_packs = BTreeSet::new();
        for material in nodes_by_material.keys().copied() {
            let Some(binding) = self.assets.photographic_material_pack_binding(material) else {
                continue;
            };
            let resolution = selected_by_material
                .get(&material)
                .copied()
                .unwrap_or(binding.resolution);
            planned_packs.insert((binding.pack_id, resolution));
        }
        let decoded_texture_plan_bytes = planned_packs
            .iter()
            .map(|(_, resolution)| canonical_pack_decoded_bytes(*resolution))
            .sum::<u64>();
        if decoded_texture_plan_bytes > decoded_texture_budget_bytes {
            return Err(SceneHostError::new(
                SceneHostErrorCode::InvalidInput,
                format!(
                    "density-selected material variants require {decoded_texture_plan_bytes} decoded texture bytes, exceeding the explicit budget {decoded_texture_budget_bytes}; raise the material texture budget or reduce visible texture density"
                ),
            ));
        }

        let mut selections = Vec::with_capacity(planned.len());
        for selection in planned {
            let before = selection.material;
            let mut after = before;
            if selection.selected > selection.previous {
                let manifest_path = resolution_variant_manifest_path(
                    &selection.manifest_path,
                    selection.previous,
                    selection.selected,
                )?;
                let loaded = self
                    .assets
                    .load_photographic_material_pack(manifest_path)
                    .await?;
                if loaded.pack().id != selection.pack_id
                    || loaded.resolution() != selection.selected
                {
                    return Err(SceneHostError::new(
                        SceneHostErrorCode::Asset,
                        format!(
                            "material variant resolved to pack '{}' at {}, expected '{}' at {}",
                            loaded.pack().id,
                            loaded.resolution().as_str(),
                            selection.pack_id,
                            selection.selected.as_str()
                        ),
                    ));
                }
                let material = self
                    .assets
                    .try_material(before)?
                    .with_base_color_texture(loaded.base_color_texture())
                    .with_normal_texture(loaded.normal_texture())
                    .with_metallic_roughness_texture(loaded.metallic_roughness_texture())
                    .with_occlusion_texture(loaded.metallic_roughness_texture());
                after = self
                    .assets
                    .create_photographic_material_pack_derivative(loaded.material(), material)?;
                for node in selection.nodes {
                    self.scene.set_mesh_material(node, after)?;
                }
            }
            selections.push(PhotographicMaterialResolutionSelectionV1 {
                pack_id: selection.pack_id,
                material_handle_before: before.data().as_ffi(),
                material_handle_after: after.data().as_ffi(),
                measured_texels_per_pixel_p50: selection.measured_density,
                one_k_texels_per_pixel_p50: selection.one_k_density,
                previous_resolution: selection.previous,
                selected_resolution: selection.selected,
                changed: after != before,
            });
        }

        Ok(PhotographicMaterialResolutionSelectionReportV1 {
            schema: PHOTOGRAPHIC_MATERIAL_RESOLUTION_SELECTION_SCHEMA_V1.to_owned(),
            decoded_texture_budget_bytes,
            decoded_texture_plan_bytes,
            selections,
        })
    }
}

fn canonical_pack_decoded_bytes(resolution: PhotographicMaterialResolutionV1) -> u64 {
    let dimension = u64::from(resolution.dimension_px());
    3 * dimension * dimension * 4
}

fn resolution_variant_manifest_path(
    current: &AssetPath,
    current_resolution: PhotographicMaterialResolutionV1,
    selected_resolution: PhotographicMaterialResolutionV1,
) -> Result<AssetPath, SceneHostError> {
    let value = current.as_str();
    if value.contains("://") {
        let needle = format!("/{}/", current_resolution.as_str());
        let Some(index) = value.rfind(&needle) else {
            return Err(resolution_layout_error(value, current_resolution));
        };
        let mut selected = String::with_capacity(value.len());
        selected.push_str(&value[..index + 1]);
        selected.push_str(selected_resolution.as_str());
        selected.push_str(&value[index + needle.len() - 1..]);
        return Ok(AssetPath::from(selected));
    }

    let path = Path::new(value);
    let Some(file_name) = path.file_name() else {
        return Err(resolution_layout_error(value, current_resolution));
    };
    let Some(resolution_dir) = path.parent() else {
        return Err(resolution_layout_error(value, current_resolution));
    };
    if resolution_dir.file_name().and_then(|name| name.to_str())
        != Some(current_resolution.as_str())
    {
        return Err(resolution_layout_error(value, current_resolution));
    }
    let Some(family_dir) = resolution_dir.parent() else {
        return Err(resolution_layout_error(value, current_resolution));
    };
    Ok(AssetPath::from(
        family_dir
            .join(selected_resolution.as_str())
            .join(file_name),
    ))
}

fn resolution_layout_error(
    path: &str,
    resolution: PhotographicMaterialResolutionV1,
) -> SceneHostError {
    SceneHostError::new(
        SceneHostErrorCode::InvalidInput,
        format!(
            "resolution-aware material pack '{path}' must live below a '{}' directory with sibling 1k, 2k, and 4k pack directories",
            resolution.as_str()
        ),
    )
}
