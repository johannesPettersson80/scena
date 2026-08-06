use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::SceneHostSemanticAovLegendEntryV1;
use crate::CaptureProjection;

pub const PHOTO_QUALITY_ANALYSIS_SCHEMA_V1: &str = "scena.photo_quality_analysis.v1";

#[derive(Debug, Clone, Copy)]
pub struct PhotoQualityAnalysisInputV1<'a> {
    pub width: u32,
    pub height: u32,
    pub rgba8: &'a [u8],
    pub beauty_id_indices: &'a [u32],
    pub depth_meters: &'a [f32],
    pub projection: Option<CaptureProjection>,
    pub legend: &'a [SceneHostSemanticAovLegendEntryV1],
    pub subject_handles: &'a [u64],
    pub support_handles: &'a [u64],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoQualityAnalysisReportV1 {
    pub schema: String,
    pub mode: String,
    pub identity_source: String,
    pub materials: Vec<PhotoMaterialQualityMetricsV1>,
    pub grounding: PhotoGroundingQualityMetricsV1,
    pub contour: PhotoContourQualityMetricsV1,
    pub unavailable_metrics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoMaterialQualityMetricsV1 {
    pub material_handle: u64,
    pub material_kind: String,
    pub material_class: String,
    pub material_class_basis: String,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub effective_metallic_mean: f32,
    pub effective_roughness_mean: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_texture_min_dimension_px: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_tile_size_m: Option<f32>,
    pub sample_count: u64,
    pub interior_sample_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflection_structure_rms_srgb8: Option<f64>,
    pub luminance_p99_srgb8: f64,
    pub near_white_fraction: f64,
    pub clipped_fraction: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projected_texture_density: Option<PhotoProjectedTextureDensityV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoProjectedTextureDensityV1 {
    pub method: String,
    pub sample_count: u64,
    pub texels_per_pixel_p10: f64,
    pub texels_per_pixel_p50: f64,
    pub texels_per_pixel_p90: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoGroundingQualityMetricsV1 {
    pub method: String,
    pub boundary_sample_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_shadow_delta_mean_srgb8: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached_fraction: Option<f64>,
    pub contact_shadow_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoContourQualityMetricsV1 {
    pub method: String,
    pub boundary_sample_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curved_turn_diversity: Option<f64>,
}

pub fn analyze_photo_quality(
    input: PhotoQualityAnalysisInputV1<'_>,
) -> Result<PhotoQualityAnalysisReportV1, &'static str> {
    let width = input.width as usize;
    let height = input.height as usize;
    let pixel_count = width
        .checked_mul(height)
        .ok_or("photo_quality_dimensions_overflow")?;
    if width == 0 || height == 0 {
        return Err("photo_quality_dimensions_empty");
    }
    if input.rgba8.len() != pixel_count.saturating_mul(4) {
        return Err("photo_quality_rgba8_length_mismatch");
    }
    if input.beauty_id_indices.len() != pixel_count {
        return Err("photo_quality_beauty_id_length_mismatch");
    }
    if input.depth_meters.len() != pixel_count {
        return Err("photo_quality_depth_length_mismatch");
    }

    let subject_handles = input
        .subject_handles
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let support_handles = input
        .support_handles
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let subject_entries = input
        .legend
        .iter()
        .filter(|entry| legend_matches_handles(entry, &subject_handles))
        .collect::<Vec<_>>();
    if subject_entries.is_empty() {
        return Err("photo_quality_subject_palette_missing");
    }
    let subject_palette = subject_entries
        .iter()
        .map(|entry| entry.palette_index)
        .collect::<BTreeSet<_>>();
    let support_palette = input
        .legend
        .iter()
        .filter(|entry| legend_matches_handles(entry, &support_handles))
        .map(|entry| entry.palette_index)
        .collect::<BTreeSet<_>>();

    let mut material_entries = BTreeMap::<u64, MaterialPixels<'_>>::new();
    for entry in subject_entries {
        let Some(material_handle) = entry.material_handle else {
            continue;
        };
        let group = material_entries
            .entry(material_handle)
            .or_insert_with(|| MaterialPixels {
                legend: entry,
                palette: BTreeSet::new(),
            });
        group.palette.insert(entry.palette_index);
    }
    let materials: Vec<PhotoMaterialQualityMetricsV1> = material_entries
        .into_iter()
        .map(|(material_handle, group)| measure_material(material_handle, group, &input))
        .collect();
    let grounding = measure_grounding(
        width,
        height,
        input.rgba8,
        input.beauty_id_indices,
        &subject_palette,
        &support_palette,
    );
    let contour = measure_contour(width, height, input.beauty_id_indices, &subject_palette);

    let texture_density_applicable = materials
        .iter()
        .any(|material| material.surface_texture_min_dimension_px.is_some());
    let texture_density_measured = materials
        .iter()
        .any(|material| material.projected_texture_density.is_some());
    let unavailable_metrics = if texture_density_applicable && !texture_density_measured {
        vec!["projected_texture_density_requires_physical_tile_and_linear_depth".to_owned()]
    } else {
        Vec::new()
    };

    Ok(PhotoQualityAnalysisReportV1 {
        schema: PHOTO_QUALITY_ANALYSIS_SCHEMA_V1.to_owned(),
        mode: "report_only".to_owned(),
        identity_source: "same_pass_beauty_semantic".to_owned(),
        materials,
        grounding,
        contour,
        unavailable_metrics,
    })
}

struct MaterialPixels<'a> {
    legend: &'a SceneHostSemanticAovLegendEntryV1,
    palette: BTreeSet<u32>,
}

fn legend_matches_handles(
    entry: &SceneHostSemanticAovLegendEntryV1,
    handles: &BTreeSet<u64>,
) -> bool {
    handles.contains(&entry.node_handle)
        || entry
            .instance_handle
            .is_some_and(|handle| handles.contains(&handle))
}

fn measure_material(
    material_handle: u64,
    group: MaterialPixels<'_>,
    input: &PhotoQualityAnalysisInputV1<'_>,
) -> PhotoMaterialQualityMetricsV1 {
    let width = input.width as usize;
    let height = input.height as usize;
    let material_kind = group
        .legend
        .material_kind
        .clone()
        .unwrap_or_else(|| "unknown".to_owned());
    let metallic_factor = group.legend.metallic_factor.unwrap_or(0.0);
    let roughness_factor = group.legend.roughness_factor.unwrap_or(1.0);
    let effective_metallic_mean = group
        .legend
        .effective_metallic_mean
        .unwrap_or(metallic_factor);
    let effective_roughness_mean = group
        .legend
        .effective_roughness_mean
        .unwrap_or(roughness_factor);
    let material_class = material_class(
        &material_kind,
        effective_metallic_mean,
        effective_roughness_mean,
    );
    let measure_reflection_structure = matches!(material_class, "smooth_metal" | "rough_metal");
    let density_scale = projected_texture_density_scale(
        group.legend.surface_texture_min_dimension_px,
        group.legend.surface_tile_size_m,
    );
    let mut lumas = Vec::new();
    let mut projected_texture_densities = Vec::new();
    let mut clipped = 0_u64;
    let mut near_white = 0_u64;
    let mut residual_sum_sq = 0.0_f64;
    let mut interior_sample_count = 0_u64;

    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if !group.palette.contains(&input.beauty_id_indices[index]) {
                continue;
            }
            let pixel = &input.rgba8[index * 4..index * 4 + 4];
            let luma = pixel_luminance(pixel);
            lumas.push(luma);
            near_white += u64::from(luma >= 250.0);
            clipped += u64::from(pixel[..3].contains(&u8::MAX));

            if let (Some(projection), Some(density_scale)) = (input.projection, density_scale)
                && let Some(world_units_per_pixel) =
                    world_units_per_pixel(projection, input.depth_meters[index], height)
            {
                projected_texture_densities.push(world_units_per_pixel * density_scale);
            }

            if !measure_reflection_structure
                || x < 2
                || y < 2
                || x + 2 >= width
                || y + 2 >= height
                || luma >= 245.0
            {
                continue;
            }
            let mut neighborhood_sum = 0.0;
            let mut interior = true;
            for sample_y in y - 2..=y + 2 {
                for sample_x in x - 2..=x + 2 {
                    let sample_index = sample_y * width + sample_x;
                    if !group
                        .palette
                        .contains(&input.beauty_id_indices[sample_index])
                    {
                        interior = false;
                        break;
                    }
                    neighborhood_sum +=
                        pixel_luminance(&input.rgba8[sample_index * 4..sample_index * 4 + 4]);
                }
                if !interior {
                    break;
                }
            }
            if interior {
                let residual = luma - neighborhood_sum / 25.0;
                residual_sum_sq += residual * residual;
                interior_sample_count = interior_sample_count.saturating_add(1);
            }
        }
    }
    lumas.sort_by(f64::total_cmp);
    projected_texture_densities.sort_by(f64::total_cmp);
    let sample_count = lumas.len() as u64;
    let p99_index = lumas
        .len()
        .saturating_sub(1)
        .min((lumas.len() as f64 * 0.99).floor() as usize);
    PhotoMaterialQualityMetricsV1 {
        material_handle,
        material_kind,
        material_class: material_class.to_owned(),
        material_class_basis: "effective_surface".to_owned(),
        metallic_factor,
        roughness_factor,
        effective_metallic_mean,
        effective_roughness_mean,
        surface_texture_min_dimension_px: group.legend.surface_texture_min_dimension_px,
        surface_tile_size_m: group.legend.surface_tile_size_m,
        sample_count,
        interior_sample_count,
        reflection_structure_rms_srgb8: (measure_reflection_structure && interior_sample_count > 0)
            .then(|| (residual_sum_sq / interior_sample_count as f64).sqrt()),
        luminance_p99_srgb8: lumas.get(p99_index).copied().unwrap_or(0.0),
        near_white_fraction: near_white as f64 / sample_count.max(1) as f64,
        clipped_fraction: clipped as f64 / sample_count.max(1) as f64,
        projected_texture_density: (!projected_texture_densities.is_empty()).then(|| {
            PhotoProjectedTextureDensityV1 {
                method: "beauty_identity_linear_depth_physical_tile".to_owned(),
                sample_count: projected_texture_densities.len() as u64,
                texels_per_pixel_p10: percentile(&projected_texture_densities, 0.10),
                texels_per_pixel_p50: percentile(&projected_texture_densities, 0.50),
                texels_per_pixel_p90: percentile(&projected_texture_densities, 0.90),
            }
        }),
    }
}

fn projected_texture_density_scale(
    texture_min_dimension_px: Option<u32>,
    tile_size_m: Option<f32>,
) -> Option<f64> {
    let texture_min_dimension_px = texture_min_dimension_px?;
    let tile_size_m = tile_size_m?;
    if texture_min_dimension_px == 0 || !tile_size_m.is_finite() || tile_size_m <= 0.0 {
        return None;
    }
    Some(f64::from(texture_min_dimension_px) / f64::from(tile_size_m))
}

fn world_units_per_pixel(
    projection: CaptureProjection,
    depth_meters: f32,
    height: usize,
) -> Option<f64> {
    if height == 0 {
        return None;
    }
    let world_height = match projection {
        CaptureProjection::Perspective {
            vertical_fov_radians,
            ..
        } => {
            if !depth_meters.is_finite()
                || depth_meters <= 0.0
                || !vertical_fov_radians.is_finite()
                || vertical_fov_radians <= 0.0
            {
                return None;
            }
            2.0 * f64::from(depth_meters) * (f64::from(vertical_fov_radians) * 0.5).tan()
        }
        CaptureProjection::Orthographic { bottom, top, .. } => {
            let world_height = f64::from(top - bottom).abs();
            if !world_height.is_finite() || world_height <= 0.0 {
                return None;
            }
            world_height
        }
    };
    let value = world_height / height as f64;
    (value.is_finite() && value > 0.0).then_some(value)
}

fn percentile(sorted: &[f64], quantile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * quantile.clamp(0.0, 1.0)).round() as usize;
    sorted[index]
}

fn material_class(kind: &str, metallic: f32, roughness: f32) -> &'static str {
    if kind != "pbr_metallic_roughness" {
        return "non_pbr";
    }
    if metallic >= 0.5 && roughness <= 0.4 {
        "smooth_metal"
    } else if metallic >= 0.5 {
        "rough_metal"
    } else {
        "dielectric"
    }
}

fn measure_grounding(
    width: usize,
    height: usize,
    rgba8: &[u8],
    ids: &[u32],
    subject_palette: &BTreeSet<u32>,
    support_palette: &BTreeSet<u32>,
) -> PhotoGroundingQualityMetricsV1 {
    const MIN_BOUNDARY_SAMPLES: u64 = 32;
    const MIN_CONTACT_DELTA_SRGB8: f64 = 4.0;
    const MIN_ATTACHED_FRACTION: f64 = 0.20;

    let mut deltas = Vec::new();
    if height > 5 && !support_palette.is_empty() {
        for y in 1..height - 4 {
            for x in 0..width {
                let index = y * width + x;
                if !support_palette.contains(&ids[index])
                    || !subject_palette.contains(&ids[index - width])
                {
                    continue;
                }
                let far = (y + 4) * width + x;
                if !support_palette.contains(&ids[far]) {
                    continue;
                }
                let boundary_luma = pixel_luminance(&rgba8[index * 4..index * 4 + 4]);
                let far_luma = pixel_luminance(&rgba8[far * 4..far * 4 + 4]);
                deltas.push((far_luma - boundary_luma).max(0.0));
            }
        }
    }
    let boundary_sample_count = deltas.len() as u64;
    let contact_shadow_delta_mean_srgb8 =
        (!deltas.is_empty()).then(|| deltas.iter().sum::<f64>() / deltas.len() as f64);
    let attached_fraction = (!deltas.is_empty()).then(|| {
        deltas
            .iter()
            .filter(|delta| **delta >= MIN_CONTACT_DELTA_SRGB8)
            .count() as f64
            / deltas.len() as f64
    });
    let contact_shadow_confirmed = boundary_sample_count >= MIN_BOUNDARY_SAMPLES
        && contact_shadow_delta_mean_srgb8.is_some_and(|delta| delta >= MIN_CONTACT_DELTA_SRGB8)
        && attached_fraction.is_some_and(|fraction| fraction >= MIN_ATTACHED_FRACTION);
    PhotoGroundingQualityMetricsV1 {
        method: "same_pass_subject_support_boundary_downward_4px".to_owned(),
        boundary_sample_count,
        contact_shadow_delta_mean_srgb8,
        attached_fraction,
        contact_shadow_confirmed,
    }
}

fn measure_contour(
    width: usize,
    height: usize,
    ids: &[u32],
    subject_palette: &BTreeSet<u32>,
) -> PhotoContourQualityMetricsV1 {
    let mut left = vec![None; height];
    let mut right = vec![None; height];
    for y in 0..height {
        for x in 0..width {
            if !subject_palette.contains(&ids[y * width + x]) {
                continue;
            }
            left[y].get_or_insert(x as i32);
            right[y] = Some(x as i32);
        }
    }
    let mut turn_samples = 0_u64;
    let mut changing_turns = 0_u64;
    for edge in [&left, &right] {
        for y in 2..height.saturating_sub(2) {
            let (Some(before), Some(center), Some(after)) = (edge[y - 2], edge[y], edge[y + 2])
            else {
                continue;
            };
            turn_samples = turn_samples.saturating_add(1);
            changing_turns += u64::from((after - center) != (center - before));
        }
    }
    PhotoContourQualityMetricsV1 {
        method: "semantic_row_extent_turn_diversity_4px".to_owned(),
        boundary_sample_count: turn_samples,
        curved_turn_diversity: (turn_samples > 0)
            .then(|| changing_turns as f64 / turn_samples as f64),
    }
}

fn pixel_luminance(pixel: &[u8]) -> f64 {
    0.2126 * f64::from(pixel[0]) + 0.7152 * f64::from(pixel[1]) + 0.0722 * f64::from(pixel[2])
}
