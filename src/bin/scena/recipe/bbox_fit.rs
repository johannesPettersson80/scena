use std::collections::BTreeSet;

pub(super) fn verify_bbox_fit(
    expectation: Option<scena::SceneRecipeBboxFitExpectationV1>,
    manifest: &scena::SceneRecipeBuildV1,
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
    introspection: &scena::RenderIntrospectionReportV1,
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
) -> usize {
    let Some(expectation) = expectation else {
        return 0;
    };
    let observed = subject_fit_fraction(manifest, capture, inspection)
        .unwrap_or(introspection.framing.fit_fraction);
    if let Some(min) = expectation.min
        && f64::from(observed) < min
    {
        push_reason(
            reasons,
            "fit_fraction_below_min",
            Some("expect_bbox_fit".to_owned()),
            format!("expected subject fit_fraction >= {min}, observed {observed}"),
        );
    }
    if let Some(max) = expectation.max
        && f64::from(observed) > max
    {
        push_reason(
            reasons,
            "fit_fraction_above_max",
            Some("expect_bbox_fit".to_owned()),
            format!("expected subject fit_fraction <= {max}, observed {observed}"),
        );
    }
    1
}

fn subject_fit_fraction(
    manifest: &scena::SceneRecipeBuildV1,
    capture: &scena::CaptureRgba8,
    inspection: &scena::SceneInspectionReportV1,
) -> Option<f32> {
    let handles = subject_handles(manifest);
    if handles.is_empty() || capture.descriptor.width == 0 || capture.descriptor.height == 0 {
        return None;
    }

    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut found = false;
    for draw in &inspection.draw_list {
        if !handles.contains(&draw.node) {
            continue;
        }
        let Some(bounds) = projected_draw_bounds(capture, draw) else {
            continue;
        };
        min_x = min_x.min(bounds[0]);
        min_y = min_y.min(bounds[1]);
        max_x = max_x.max(bounds[2]);
        max_y = max_y.max(bounds[3]);
        found = true;
    }
    if !found {
        return None;
    }

    let width = (max_x - min_x).max(0.0);
    let height = (max_y - min_y).max(0.0);
    Some((width / capture.descriptor.width as f32).max(height / capture.descriptor.height as f32))
}

fn subject_handles(manifest: &scena::SceneRecipeBuildV1) -> BTreeSet<u64> {
    let mut handles = BTreeSet::new();
    for node in &manifest.nodes {
        if matches!(node.kind.as_str(), "node" | "instance_set" | "particle_set") {
            handles.insert(node.handle);
        }
    }
    for import in &manifest.imports {
        handles.extend(import.nodes_by_path.values().copied());
        handles.extend(import.root_handles.iter().copied());
    }
    handles
}

fn projected_draw_bounds(
    capture: &scena::CaptureRgba8,
    draw: &scena::SceneDrawInspectionV1,
) -> Option<[f32; 4]> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut found = false;
    for corner in aabb_corners(draw.local_bounds) {
        let world = transform_point(draw.world_transform, corner);
        let Some(point) = project_world_point(capture, world) else {
            continue;
        };
        min_x = min_x.min(point[0]);
        min_y = min_y.min(point[1]);
        max_x = max_x.max(point[0]);
        max_y = max_y.max(point[1]);
        found = true;
    }
    found.then_some([min_x, min_y, max_x, max_y])
}

fn project_world_point(capture: &scena::CaptureRgba8, world: scena::Vec3) -> Option<[f32; 2]> {
    let world_from_camera = capture.descriptor.camera.world_transform?;
    let projection = capture.descriptor.camera.projection?;
    let view = world_to_view(world, world_from_camera)?;
    let (ndc_x, ndc_y) = match projection {
        scena::CaptureProjection::Perspective {
            vertical_fov_radians,
            aspect,
            near,
            far,
        } => {
            let depth = -view.z;
            if !depth.is_finite() || depth < near || depth > far {
                return None;
            }
            let aspect = if aspect.is_finite() && aspect > 0.0 {
                aspect
            } else {
                capture.descriptor.width as f32 / capture.descriptor.height as f32
            };
            let focal = (vertical_fov_radians * 0.5).tan().recip();
            if !focal.is_finite() {
                return None;
            }
            (view.x * focal / (aspect * depth), view.y * focal / depth)
        }
        scena::CaptureProjection::Orthographic {
            left,
            right,
            bottom,
            top,
            near,
            far,
        } => {
            let depth = -view.z;
            if !depth.is_finite() || depth < near || depth > far {
                return None;
            }
            let width = right - left;
            let height = top - bottom;
            if width.abs() <= f32::EPSILON || height.abs() <= f32::EPSILON {
                return None;
            }
            (
                (view.x - left) / width * 2.0 - 1.0,
                (view.y - bottom) / height * 2.0 - 1.0,
            )
        }
    };
    if !ndc_x.is_finite() || !ndc_y.is_finite() {
        return None;
    }
    Some([
        (ndc_x * 0.5 + 0.5) * capture.descriptor.width as f32,
        (1.0 - (ndc_y * 0.5 + 0.5)) * capture.descriptor.height as f32,
    ])
}

fn world_to_view(world: scena::Vec3, world_from_camera: scena::Transform) -> Option<scena::Vec3> {
    if !world_from_camera.translation.is_finite()
        || !world_from_camera.rotation.is_finite()
        || !world_from_camera.scale.is_finite()
        || world_from_camera.scale.x.abs() <= f32::EPSILON
        || world_from_camera.scale.y.abs() <= f32::EPSILON
        || world_from_camera.scale.z.abs() <= f32::EPSILON
    {
        return None;
    }
    let translated = world - world_from_camera.translation;
    let rotated = world_from_camera.rotation.inverse() * translated;
    Some(scena::Vec3::new(
        rotated.x / world_from_camera.scale.x,
        rotated.y / world_from_camera.scale.y,
        rotated.z / world_from_camera.scale.z,
    ))
}

fn transform_point(transform: scena::Transform, point: scena::Vec3) -> scena::Vec3 {
    transform.translation + transform.rotation * (point * transform.scale)
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

fn push_reason(
    reasons: &mut Vec<scena::SceneRecipeVerificationReasonV1>,
    code: &str,
    expectation_id: Option<String>,
    message: String,
) {
    reasons.push(scena::SceneRecipeVerificationReasonV1 {
        code: code.to_owned(),
        severity: "error".to_owned(),
        source: "render".to_owned(),
        expectation_id,
        affected_handles: Vec::new(),
        message,
    });
}
