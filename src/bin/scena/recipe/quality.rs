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
    (profile == scena::RenderQualityProfile::Product && expectation_is_profile_only(expectation))
        .then(|| scena::SceneRecipeQualityGeometryV1 {
            min_intermediate_edge_fraction: Some(
                profile.default_min_geometry_intermediate_edge_fraction() as f64,
            ),
        })
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

fn expectation_is_profile_only(expectation: &scena::SceneRecipeQualityExpectationV1) -> bool {
    expectation.exposure.is_none()
        && expectation.contrast.is_none()
        && expectation.noise.is_none()
        && expectation.text.is_none()
        && expectation.line.is_none()
        && expectation.geometry.is_none()
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
