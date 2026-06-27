use std::collections::BTreeMap;

use super::rendered_coverage::{RenderedNodeCoverage, changed_node_coverage};

#[derive(Debug, Clone)]
pub(super) struct AnimationObservation {
    pub(super) time_seconds: f32,
    pub(super) transform_revision: u64,
    pub(super) appearance_revision: u64,
    pub(super) payload_fnv1a64: String,
    pub(super) capture: scena::CaptureRgba8,
    pub(super) inspection: scena::SceneInspectionReportV1,
    pub(super) node_transforms: Vec<(u64, scena::Transform)>,
    pub(super) node_rendered_coverage: Vec<(u64, RenderedNodeCoverage)>,
}

pub(super) fn samples_from_observations(
    observations: &[AnimationObservation],
    tolerance: f32,
    expected_translations: Option<&[scena::Vec3]>,
    selected_node: Option<u64>,
    expected_tolerance: f32,
) -> Vec<scena::AnimationSampleV1> {
    let baseline = observations.first().map(|observation| {
        observation
            .node_transforms
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>()
    });
    observations
        .iter()
        .enumerate()
        .map(|(index, observation)| {
            let invalid_node_count = observation
                .node_transforms
                .iter()
                .filter(|(_, transform)| !scena::transform_is_finite(*transform))
                .count();
            let moving_node_count = baseline
                .as_ref()
                .map(|baseline| {
                    observation
                        .node_transforms
                        .iter()
                        .filter(|(handle, transform)| {
                            baseline.get(handle).is_some_and(|baseline_transform| {
                                scena::transform_differs(*baseline_transform, *transform, tolerance)
                            })
                        })
                        .count()
                })
                .unwrap_or(0);
            scena::AnimationSampleV1 {
                time_seconds: round3(observation.time_seconds),
                transform_revision: observation.transform_revision,
                appearance_revision: observation.appearance_revision,
                payload_fnv1a64: observation.payload_fnv1a64.clone(),
                moving_node_count,
                invalid_node_count,
                observed_values: observed_values_for_sample(
                    observations,
                    index,
                    observation,
                    selected_node,
                    expected_translations.and_then(|expected| expected.get(index).copied()),
                    expected_tolerance,
                ),
            }
        })
        .collect()
}

pub(super) fn selected_observed_node(
    observations: &[AnimationObservation],
    tolerance: f32,
) -> Option<u64> {
    let baseline = observations.first()?;
    let mut moving = Vec::new();
    for (handle, baseline_transform) in &baseline.node_transforms {
        if observations.iter().skip(1).any(|observation| {
            observation
                .node_transforms
                .iter()
                .find(|(candidate, _)| candidate == handle)
                .is_some_and(|(_, transform)| {
                    scena::transform_differs(*baseline_transform, *transform, tolerance)
                })
        }) {
            moving.push(*handle);
        }
    }
    for handle in &moving {
        if observations.iter().any(|observation| {
            observation
                .node_rendered_coverage
                .iter()
                .any(|(candidate, coverage)| candidate == handle && coverage.coverage_px > 0)
        }) {
            return Some(*handle);
        }
    }
    if let Some(handle) = moving.first() {
        return Some(*handle);
    }
    baseline.node_transforms.first().map(|(handle, _)| *handle)
}

fn observed_values_for_sample(
    observations: &[AnimationObservation],
    observation_index: usize,
    observation: &AnimationObservation,
    selected_node: Option<u64>,
    expected_translation: Option<scena::Vec3>,
    expected_tolerance: f32,
) -> Vec<scena::AnimationObservedValueV1> {
    let Some(node) = selected_node else {
        return Vec::new();
    };
    let Some((_, transform)) = observation
        .node_transforms
        .iter()
        .find(|(candidate, _)| *candidate == node)
    else {
        return Vec::new();
    };
    let within_tolerance = expected_translation.map(|expected| {
        (transform.translation.x - expected.x).abs() <= expected_tolerance
            && (transform.translation.y - expected.y).abs() <= expected_tolerance
            && (transform.translation.z - expected.z).abs() <= expected_tolerance
    });
    let rendered =
        rendered_coverage_for_sample(observations, observation_index, node).or_else(|| {
            observation
                .node_rendered_coverage
                .iter()
                .find(|(candidate, _)| *candidate == node)
                .map(|(_, coverage)| *coverage)
        });
    vec![scena::AnimationObservedValueV1 {
        id: "selected-transform".to_owned(),
        node,
        kind: "transform".to_owned(),
        transform: *transform,
        rendered_centroid_css_px: rendered.and_then(|coverage| coverage.centroid_css_px),
        rendered_coverage_px: rendered.map(|coverage| coverage.coverage_px),
        expected_translation,
        within_tolerance,
    }]
}

fn rendered_coverage_for_sample(
    observations: &[AnimationObservation],
    observation_index: usize,
    node: u64,
) -> Option<RenderedNodeCoverage> {
    let observation = observations.get(observation_index)?;
    let direct = observation
        .node_rendered_coverage
        .iter()
        .find(|(candidate, _)| *candidate == node)
        .map(|(_, coverage)| *coverage);
    if direct.is_some_and(coverage_is_measured) {
        return direct;
    }
    let reference = reference_observation_for_changed_pixels(observations, observation_index)?;
    let changed = changed_node_coverage(
        &observation.capture,
        &observation.inspection,
        &reference.capture,
        node,
    );
    if coverage_is_measured(changed) {
        return Some(changed);
    }
    direct.or(Some(changed))
}

fn coverage_is_measured(coverage: RenderedNodeCoverage) -> bool {
    coverage.coverage_px > 0 && coverage.centroid_css_px.is_some()
}

fn reference_observation_for_changed_pixels(
    observations: &[AnimationObservation],
    observation_index: usize,
) -> Option<&AnimationObservation> {
    let observation = observations.get(observation_index)?;
    observations
        .iter()
        .enumerate()
        .find(|(index, candidate)| {
            *index != observation_index && candidate.payload_fnv1a64 != observation.payload_fnv1a64
        })
        .map(|(_, candidate)| candidate)
}

fn round3(value: f32) -> f32 {
    if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    }
}
