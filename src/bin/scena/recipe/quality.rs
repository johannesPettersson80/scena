mod depth_of_field;
mod verification;

pub(super) use depth_of_field::append_depth_of_field_checks;
pub(super) use verification::{QualityVerificationInput, verify_quality_expectations};

pub(super) fn expectation_without_region_specific_checks(
    expect: Option<&scena::SceneRecipeExpectV1>,
) -> Option<scena::SceneRecipeQualityExpectationV1> {
    expect
        .and_then(|expect| expect.expect_quality.as_ref())
        .cloned()
        .map(|mut expectation| {
            expectation.text = None;
            expectation.line = None;
            expectation.geometry = None;
            expectation.reflection = None;
            expectation.area_light = None;
            expectation.grounding = None;
            expectation.depth_of_field = None;
            expectation
        })
}

pub(super) fn geometry_expectation(
    expectation: Option<&scena::SceneRecipeQualityExpectationV1>,
) -> Option<scena::SceneRecipeQualityGeometryV1> {
    let expectation = expectation?;
    if let Some(geometry) = expectation.geometry {
        return Some(geometry);
    }
    let profile = scena::RenderQualityProfile::parse(&expectation.profile)?;
    Some(scena::SceneRecipeQualityGeometryV1 {
        min_intermediate_edge_fraction: Some(
            profile.default_min_geometry_intermediate_edge_fraction() as f64,
        ),
    })
}

pub(super) struct GridLineQualityThresholds {
    pub(super) min_intermediate_px_per_edge: f32,
    pub(super) min_unique_luma_levels: f32,
    pub(super) max_halo_overshoot: f32,
    pub(super) min_contrast_range: f32,
}

pub(super) fn grid_line_quality_thresholds(
    recipe: &scena::SceneRecipeV1,
    expectation: Option<&scena::SceneRecipeQualityExpectationV1>,
) -> Option<GridLineQualityThresholds> {
    if !recipe
        .scene
        .as_ref()
        .and_then(|scene| scene.grid.as_ref())
        .is_some_and(|grid| grid.enabled)
    {
        return None;
    }
    let expectation = expectation?;
    let profile = scena::RenderQualityProfile::parse(&expectation.profile)
        .unwrap_or(scena::RenderQualityProfile::Product);
    Some(match profile {
        scena::RenderQualityProfile::Product => GridLineQualityThresholds {
            min_intermediate_px_per_edge: 3.8,
            min_unique_luma_levels: 32.0,
            max_halo_overshoot: 0.10,
            min_contrast_range: 0.70,
        },
        scena::RenderQualityProfile::Documentation => GridLineQualityThresholds {
            min_intermediate_px_per_edge: 2.8,
            min_unique_luma_levels: 24.0,
            max_halo_overshoot: 0.08,
            min_contrast_range: 0.60,
        },
        scena::RenderQualityProfile::Cad | scena::RenderQualityProfile::Dashboard => {
            GridLineQualityThresholds {
                min_intermediate_px_per_edge: 2.5,
                min_unique_luma_levels: 20.0,
                max_halo_overshoot: 0.10,
                min_contrast_range: 0.35,
            }
        }
        scena::RenderQualityProfile::Twin => GridLineQualityThresholds {
            min_intermediate_px_per_edge: 2.8,
            min_unique_luma_levels: 24.0,
            max_halo_overshoot: 0.08,
            min_contrast_range: 0.60,
        },
    })
}

pub(super) fn grid_floor_region(capture: &scena::CaptureRgba8) -> scena::RenderQualityRegion {
    let width = capture.descriptor.width;
    let height = capture.descriptor.height;
    let region_width = ((width as f32) * 0.56).round() as u32;
    let region_height = ((height as f32) * 0.34).round() as u32;
    let x = width.saturating_sub(region_width) / 2;
    let y = ((height as f32) * 0.58).floor() as u32;
    scena::RenderQualityRegion {
        kind: "grid_floor",
        handle: None,
        x: x.min(width),
        y: y.min(height),
        width: region_width.min(width.saturating_sub(x)).max(1),
        height: region_height.min(height.saturating_sub(y)).max(1),
    }
}

pub(super) fn subject_region(
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
) -> scena::RenderQualityRegion {
    introspection
        .content_bbox_css_px
        .map(|rect| {
            region_from_rect(
                "subject",
                rect,
                capture.descriptor.width,
                capture.descriptor.height,
            )
        })
        .unwrap_or_else(|| {
            scena::RenderQualityRegion::full_frame(
                capture.descriptor.width,
                capture.descriptor.height,
            )
        })
}

pub(super) fn geometry_region(
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
) -> scena::RenderQualityRegion {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for check in &composition.checks {
        if check.status != scena::SceneCompositionStatusV1::Checked
            || check.category != "placement"
            || check.code != "projected_bbox_available"
        {
            continue;
        }
        let Some(region) = &check.region else {
            continue;
        };
        if region.kind != "node" {
            continue;
        }
        let Some(rect) = region.rect_css_px else {
            continue;
        };
        min_x = min_x.min(rect.min_x);
        min_y = min_y.min(rect.min_y);
        max_x = max_x.max(rect.max_x);
        max_y = max_y.max(rect.max_y);
    }
    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        return region_from_rect(
            "subject",
            scena::RenderIntrospectionRectV1 {
                min_x,
                min_y,
                max_x,
                max_y,
                width: (max_x - min_x).max(0.0),
                height: (max_y - min_y).max(0.0),
            },
            capture.descriptor.width,
            capture.descriptor.height,
        );
    }
    subject_region(capture, introspection)
}

pub(super) fn reflection_region(capture: &scena::CaptureRgba8) -> scena::RenderQualityRegion {
    let height = capture.descriptor.height;
    let y = ((height as f32) * 0.78).floor() as u32;
    scena::RenderQualityRegion {
        kind: "reflection_surface",
        handle: None,
        x: 0,
        y: y.min(height),
        width: capture.descriptor.width.max(1),
        height: height.saturating_sub(y).max(1),
    }
}

pub(super) fn reflection_region_for_expectation(
    capture: &scena::CaptureRgba8,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    reflection: &scena::scene::recipe::SceneRecipeQualityReflectionV1,
) -> Result<(String, scena::RenderQualityRegion), String> {
    let Some(target) = reflection.target.as_ref() else {
        return Ok((
            "expect_quality.reflection".to_owned(),
            reflection_region(capture),
        ));
    };
    let handles = super::resolve_target_handles(target, manifest, false)?;
    for handle in handles {
        if let Some(region) = projected_region_for_handle(capture, composition, handle) {
            return Ok((
                "expect_quality.reflection.target".to_owned(),
                inset_reflection_target_region(region),
            ));
        }
    }
    Err("reflection target did not have a projected screen region".to_owned())
}

pub(super) fn area_light_region_for_expectation(
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    area_light: &scena::scene::recipe::SceneRecipeQualityAreaLightV1,
) -> Result<(String, scena::RenderQualityRegion), String> {
    let Some(target) = area_light.target.as_ref() else {
        return Ok((
            "expect_quality.area_light".to_owned(),
            geometry_region(capture, introspection, composition),
        ));
    };
    let handles = super::resolve_target_handles(target, manifest, false)?;
    for handle in handles {
        if let Some(mut region) = projected_region_for_handle(capture, composition, handle) {
            region.kind = "area_light_shadow_target";
            return Ok(("expect_quality.area_light.target".to_owned(), region));
        }
    }
    Err("area-light quality target did not have a projected screen region".to_owned())
}

pub(super) fn grounding_region_for_expectation(
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    grounding: &scena::scene::recipe::SceneRecipeQualityGroundingV1,
) -> Result<(String, scena::RenderQualityRegion), String> {
    let Some(target) = grounding.target.as_ref() else {
        return Ok((
            "expect_quality.grounding".to_owned(),
            geometry_region(capture, introspection, composition),
        ));
    };
    let handles = super::resolve_target_handles(target, manifest, false)?;
    for handle in handles {
        if let Some(mut region) = projected_region_for_handle(capture, composition, handle) {
            region.kind = "contact_shadow_target";
            return Ok(("expect_quality.grounding.target".to_owned(), region));
        }
    }
    Err("grounding quality target did not have a projected screen region".to_owned())
}

pub(super) struct DepthOfFieldRegions {
    pub(super) focal: scena::RenderQualityRegion,
    pub(super) background: scena::RenderQualityRegion,
}

pub(super) fn depth_of_field_regions_for_expectation(
    capture: &scena::CaptureRgba8,
    introspection: &scena::RenderIntrospectionReportV1,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    depth_of_field: &scena::scene::recipe::SceneRecipeQualityDepthOfFieldV1,
) -> Result<DepthOfFieldRegions, String> {
    let focal = if let Some(target) = depth_of_field.target.as_ref() {
        first_projected_region_for_target(capture, composition, manifest, target, "dof_focal")?
    } else {
        geometry_region(capture, introspection, composition)
    };
    let background = if let Some(target) = depth_of_field.background_target.as_ref() {
        first_projected_region_for_target(capture, composition, manifest, target, "dof_background")?
    } else {
        largest_background_region(capture, focal)
    };
    Ok(DepthOfFieldRegions { focal, background })
}

pub(super) fn max_area_light_emitter_extent_meters(recipe: &scena::SceneRecipeV1) -> f32 {
    recipe
        .lights
        .iter()
        .filter(|light| light.kind == "area")
        .map(|light| match light.shape.as_deref().unwrap_or("rect") {
            "disc" | "sphere" => light.radius.unwrap_or(0.5).max(0.0) * 2.0,
            _ => light
                .width
                .unwrap_or(1.0)
                .min(light.height.unwrap_or(1.0))
                .max(0.0),
        })
        .fold(0.0_f64, f64::max) as f32
}

fn inset_reflection_target_region(
    mut region: scena::RenderQualityRegion,
) -> scena::RenderQualityRegion {
    let inset_x = (region.width / 5).min(region.width.saturating_sub(1) / 2);
    let inset_y = (region.height / 5).min(region.height.saturating_sub(1) / 2);
    region.x = region.x.saturating_add(inset_x);
    region.y = region.y.saturating_add(inset_y);
    region.width = region
        .width
        .saturating_sub(inset_x.saturating_mul(2))
        .max(1);
    region.height = region
        .height
        .saturating_sub(inset_y.saturating_mul(2))
        .max(1);
    region
}

fn projected_region_for_handle(
    capture: &scena::CaptureRgba8,
    composition: &scena::SceneCompositionReportV1,
    handle: u64,
) -> Option<scena::RenderQualityRegion> {
    projected_region_for_handle_kind(capture, composition, handle, "reflection_target")
}

fn projected_region_for_handle_kind(
    capture: &scena::CaptureRgba8,
    composition: &scena::SceneCompositionReportV1,
    handle: u64,
    kind: &'static str,
) -> Option<scena::RenderQualityRegion> {
    composition.checks.iter().find_map(|check| {
        if check.status != scena::SceneCompositionStatusV1::Checked
            || check.category != "placement"
            || check.code != "projected_bbox_available"
        {
            return None;
        }
        let region = check.region.as_ref()?;
        if region.handle != Some(handle) || region.kind != "node" {
            return None;
        }
        let rect = region.rect_css_px?;
        Some(region_from_rect_with_handle(
            kind,
            Some(handle),
            rect,
            capture.descriptor.width,
            capture.descriptor.height,
        ))
    })
}

fn first_projected_region_for_target(
    capture: &scena::CaptureRgba8,
    composition: &scena::SceneCompositionReportV1,
    manifest: &scena::SceneRecipeBuildV1,
    target: &scena::SceneRecipeTargetV1,
    kind: &'static str,
) -> Result<scena::RenderQualityRegion, String> {
    let handles = super::resolve_target_handles(target, manifest, false)?;
    for handle in handles {
        if let Some(region) = projected_region_for_handle_kind(capture, composition, handle, kind) {
            return Ok(region);
        }
    }
    Err(format!(
        "{kind} target did not have a projected screen region"
    ))
}

fn largest_background_region(
    capture: &scena::CaptureRgba8,
    focal: scena::RenderQualityRegion,
) -> scena::RenderQualityRegion {
    let width = capture.descriptor.width;
    let height = capture.descriptor.height;
    let right_width = width.saturating_sub(focal.x.saturating_add(focal.width));
    let below_height = height.saturating_sub(focal.y.saturating_add(focal.height));
    let candidates = [
        scena::RenderQualityRegion {
            kind: "dof_background",
            handle: None,
            x: 0,
            y: 0,
            width: focal.x,
            height,
        },
        scena::RenderQualityRegion {
            kind: "dof_background",
            handle: None,
            x: focal.x.saturating_add(focal.width).min(width),
            y: 0,
            width: right_width,
            height,
        },
        scena::RenderQualityRegion {
            kind: "dof_background",
            handle: None,
            x: 0,
            y: 0,
            width,
            height: focal.y,
        },
        scena::RenderQualityRegion {
            kind: "dof_background",
            handle: None,
            x: 0,
            y: focal.y.saturating_add(focal.height).min(height),
            width,
            height: below_height,
        },
    ];
    candidates
        .into_iter()
        .max_by_key(|region| region.width.saturating_mul(region.height))
        .filter(|region| region.width > 0 && region.height > 0)
        .unwrap_or_else(|| reflection_region(capture))
}

fn region_from_rect(
    kind: &'static str,
    rect: scena::RenderIntrospectionRectV1,
    width: u32,
    height: u32,
) -> scena::RenderQualityRegion {
    let x = rect.min_x.floor().max(0.0) as u32;
    let y = rect.min_y.floor().max(0.0) as u32;
    let max_x = rect.max_x.ceil().max(rect.min_x).min(width as f32) as u32;
    let max_y = rect.max_y.ceil().max(rect.min_y).min(height as f32) as u32;
    scena::RenderQualityRegion {
        kind,
        handle: None,
        x: x.min(width),
        y: y.min(height),
        width: max_x.saturating_sub(x).max(1),
        height: max_y.saturating_sub(y).max(1),
    }
}

fn region_from_rect_with_handle(
    kind: &'static str,
    handle: Option<u64>,
    rect: scena::RenderIntrospectionRectV1,
    width: u32,
    height: u32,
) -> scena::RenderQualityRegion {
    let mut region = region_from_rect(kind, rect, width, height);
    region.handle = handle;
    region
}
