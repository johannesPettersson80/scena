use super::observations::AnimationObservation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExpectedNodeIssue {
    Missing,
    Static,
}

pub(super) fn expected_node_status(
    observations: &[AnimationObservation],
    handle: u64,
    tolerance: f32,
    expect_change: bool,
) -> Option<ExpectedNodeIssue> {
    let Some(first) = observations.first() else {
        return Some(ExpectedNodeIssue::Missing);
    };
    let Some(baseline) = first
        .node_transforms
        .iter()
        .find(|(candidate, _)| *candidate == handle)
        .map(|(_, transform)| *transform)
    else {
        return Some(ExpectedNodeIssue::Missing);
    };
    let mut moved = false;
    for observation in observations.iter().skip(1) {
        let Some((_, transform)) = observation
            .node_transforms
            .iter()
            .find(|(candidate, _)| *candidate == handle)
        else {
            return Some(ExpectedNodeIssue::Missing);
        };
        if scena::transform_differs(baseline, *transform, tolerance) {
            moved = true;
        }
    }
    if expect_change && !moved {
        Some(ExpectedNodeIssue::Static)
    } else {
        None
    }
}

pub(super) fn apply_expected_node_status(
    report: &mut scena::AnimationIntrospectionReportV1,
    handle: u64,
    issue: Option<ExpectedNodeIssue>,
) {
    let Some(issue) = issue else {
        return;
    };
    let (code, message, action, help) = match issue {
        ExpectedNodeIssue::Missing => (
            "expected_node_missing",
            format!(
                "expected animation target node handle {handle} was not present in every sample"
            ),
            "inspect_expected_node_handle",
            "run scena inspect and pass a stable node handle that exists in every sampled frame",
        ),
        ExpectedNodeIssue::Static => (
            "expected_node_static",
            format!(
                "expected animation target node handle {handle} did not move across sampled times"
            ),
            "choose_animated_node_handle",
            "run scena inspect and bind the expectation to the node that should animate",
        ),
    };
    report.reasons.push(scena::AnimationIntrospectionReasonV1 {
        code: code.to_owned(),
        severity: "error".to_owned(),
        message,
    });
    report.fixes.push(scena::AnimationIntrospectionFixV1 {
        action: action.to_owned(),
        help: help.to_owned(),
    });
    report.ok = false;
}
