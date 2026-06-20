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
        let Some(bounds) =
            scena::project_aabb_from_capture(capture, draw.local_bounds, draw.world_transform)
        else {
            continue;
        };
        min_x = min_x.min(bounds.min_x);
        min_y = min_y.min(bounds.min_y);
        max_x = max_x.max(bounds.max_x);
        max_y = max_y.max(bounds.max_y);
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
