pub(super) fn verify_target_fit(
    expectations: &[scena::SceneRecipeTargetFitExpectationV1],
    manifest: &scena::SceneRecipeBuildV1,
    capture: &scena::CaptureRgba8,
    background: scena::Color,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> usize {
    let mut checks = 0;
    for expectation in expectations {
        checks += 1;
        let affected_handles =
            match super::resolve_target_handles(&expectation.target, manifest, true) {
                Ok(handles) => handles,
                Err(message) => {
                    push_reason(
                        reasons,
                        "target_not_found",
                        Some(expectation.id.clone()),
                        Vec::new(),
                        message,
                    );
                    continue;
                }
            };
        let bounds = bounds_from_recipe(expectation.bounds);
        let projection = match project_target_region(capture, bounds, expectation.centroid) {
            Ok(projection) => projection,
            Err((code, message)) => {
                push_reason(
                    reasons,
                    code,
                    Some(expectation.id.clone()),
                    affected_handles.clone(),
                    message,
                );
                continue;
            }
        };
        if let Some(min_fit) = expectation.min_fit
            && f64::from(projection.fit_fraction) < min_fit
        {
            push_reason(
                reasons,
                "target_fit_below_min",
                Some(expectation.id.clone()),
                affected_handles.clone(),
                format!(
                    "expected target fit_fraction >= {min_fit}, observed {}",
                    projection.fit_fraction
                ),
            );
        }
        if let Some(max_fit) = expectation.max_fit
            && f64::from(projection.fit_fraction) > max_fit
        {
            push_reason(
                reasons,
                "target_fit_above_max",
                Some(expectation.id.clone()),
                affected_handles.clone(),
                format!(
                    "expected target fit_fraction <= {max_fit}, observed {}",
                    projection.fit_fraction
                ),
            );
        }
        if let Some(min_visible_coverage) = expectation.min_visible_coverage {
            match visible_coverage_for_rect(capture, projection.rect, background) {
                Some(coverage) if f64::from(coverage) >= min_visible_coverage => {}
                Some(coverage) => push_reason(
                    reasons,
                    "target_visible_coverage_below_min",
                    Some(expectation.id.clone()),
                    affected_handles.clone(),
                    format!(
                        "expected target visible coverage >= {min_visible_coverage}, observed {coverage}"
                    ),
                ),
                None => push_reason(
                    reasons,
                    "target_visible_coverage_unavailable",
                    Some(expectation.id.clone()),
                    affected_handles.clone(),
                    "target projected to an empty screen region".to_owned(),
                ),
            }
        }
    }
    checks
}

#[derive(Debug, Clone, Copy)]
struct TargetProjection {
    rect: scena::CaptureScreenRect,
    fit_fraction: f32,
}

fn project_target_region(
    capture: &scena::CaptureRgba8,
    bounds: scena::Aabb,
    centroid: [f64; 3],
) -> Result<TargetProjection, (&'static str, String)> {
    let centroid = vec3_from_array(centroid);
    if !centroid.is_finite() {
        return Err((
            "target_region_projection_missing",
            "target region centroid is not finite".to_owned(),
        ));
    }
    let Some(center) = scena::project_world_point_from_capture(capture, centroid) else {
        return Err((
            "target_region_projection_missing",
            "target region centroid did not project into the active camera frustum".to_owned(),
        ));
    };
    if !ndc_inside(center.ndc_x) || !ndc_inside(center.ndc_y) {
        return Err((
            "target_region_centroid_out_of_frame",
            format!(
                "target region centroid projected outside the viewport: ndc=({}, {})",
                center.ndc_x, center.ndc_y
            ),
        ));
    }

    let mut projected = Vec::with_capacity(8);
    for corner in aabb_corners(bounds) {
        let Some(point) = scena::project_world_point_from_capture(capture, corner) else {
            return Err((
                "target_region_projection_missing",
                "one or more target region corners did not project into the active camera frustum"
                    .to_owned(),
            ));
        };
        if !ndc_inside(point.ndc_x) || !ndc_inside(point.ndc_y) {
            return Err((
                "target_region_clipped",
                format!(
                    "target region corner projected outside the viewport: ndc=({}, {})",
                    point.ndc_x, point.ndc_y
                ),
            ));
        }
        projected.push(point);
    }

    let rect = screen_rect_from_points(&projected).ok_or_else(|| {
        (
            "target_region_projection_missing",
            "target region projected to an invalid screen rectangle".to_owned(),
        )
    })?;
    let fit_fraction = (rect.width / capture.descriptor.width as f32)
        .max(rect.height / capture.descriptor.height as f32);
    Ok(TargetProjection { rect, fit_fraction })
}

fn bounds_from_recipe(bounds: scena::SceneRecipeTargetBoundsV1) -> scena::Aabb {
    scena::Aabb::new(vec3_from_array(bounds.min), vec3_from_array(bounds.max))
}

fn vec3_from_array(value: [f64; 3]) -> scena::Vec3 {
    scena::Vec3::new(value[0] as f32, value[1] as f32, value[2] as f32)
}

fn aabb_corners(bounds: scena::Aabb) -> [scena::Vec3; 8] {
    let min = bounds.min;
    let max = bounds.max;
    [
        scena::Vec3::new(min.x, min.y, min.z),
        scena::Vec3::new(max.x, min.y, min.z),
        scena::Vec3::new(min.x, max.y, min.z),
        scena::Vec3::new(max.x, max.y, min.z),
        scena::Vec3::new(min.x, min.y, max.z),
        scena::Vec3::new(max.x, min.y, max.z),
        scena::Vec3::new(min.x, max.y, max.z),
        scena::Vec3::new(max.x, max.y, max.z),
    ]
}

fn screen_rect_from_points(
    points: &[scena::CaptureProjectedPoint],
) -> Option<scena::CaptureScreenRect> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }
    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    Some(scena::CaptureScreenRect {
        min_x,
        min_y,
        max_x,
        max_y,
        width,
        height,
        center_x: (min_x + max_x) * 0.5,
        center_y: (min_y + max_y) * 0.5,
    })
}

fn visible_coverage_for_rect(
    capture: &scena::CaptureRgba8,
    rect: scena::CaptureScreenRect,
    background: scena::Color,
) -> Option<f32> {
    let region =
        scena::screen_region_from_rect(rect, capture.descriptor.width, capture.descriptor.height)?;
    let background = linear_rgba_to_srgb8(background);
    let mut foreground_pixels = 0_u64;
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y as usize) * (capture.descriptor.width as usize) + (x as usize)) * 4;
            let Some(pixel) = capture.rgba8.get(offset..offset + 4) else {
                continue;
            };
            if pixel_differs_from_background(pixel, background) {
                foreground_pixels = foreground_pixels.saturating_add(1);
            }
        }
    }
    let region_pixels = u64::from(region.width).saturating_mul(u64::from(region.height));
    (region_pixels > 0).then_some(foreground_pixels as f32 / region_pixels as f32)
}

fn linear_rgba_to_srgb8(color: scena::Color) -> [u8; 4] {
    [
        linear_channel_to_srgb_u8(color.r),
        linear_channel_to_srgb_u8(color.g),
        linear_channel_to_srgb_u8(color.b),
        (color.a.clamp(0.0, 1.0) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

fn linear_channel_to_srgb_u8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round().clamp(0.0, 255.0) as u8
}

fn pixel_differs_from_background(pixel: &[u8], background: [u8; 4]) -> bool {
    const TOLERANCE_RGBA8: u8 = 2;
    (0..3).any(|channel| pixel[channel].abs_diff(background[channel]) > TOLERANCE_RGBA8)
}

fn ndc_inside(value: f32) -> bool {
    const TOLERANCE: f32 = 0.001;
    value.is_finite() && (-1.0 - TOLERANCE..=1.0 + TOLERANCE).contains(&value)
}

fn push_reason(
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
    code: &str,
    expectation_id: Option<String>,
    affected_handles: Vec<u64>,
    message: String,
) {
    reasons.push(scena::SceneRecipeVerificationReasonV1 {
        code: code.to_owned(),
        severity: "error".to_owned(),
        source: "render".to_owned(),
        expectation_id,
        affected_handles,
        message,
    });
}
