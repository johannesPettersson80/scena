use std::collections::BTreeMap;

use crate::RenderIntrospectionRectV1;

use super::types::{RenderQualityCheckV1, RenderQualityRegion, round3};

pub(super) struct ThresholdCheck<'a> {
    pub(super) id: &'a str,
    pub(super) code: &'a str,
    pub(super) severity: &'a str,
    pub(super) region: RenderQualityRegion,
    pub(super) observed_key: &'a str,
    pub(super) observed: f32,
    pub(super) threshold_key: &'a str,
    pub(super) threshold: f32,
    pub(super) fails: bool,
    pub(super) fix_hint: &'a str,
}

pub(super) fn push_threshold_check(
    checks: &mut Vec<RenderQualityCheckV1>,
    check: ThresholdCheck<'_>,
) {
    if check.fails {
        checks.push(single_value_check(SingleValueCheck {
            id: check.id,
            code: check.code,
            severity: check.severity,
            region: check.region,
            observed_key: check.observed_key,
            observed: check.observed,
            threshold_key: check.threshold_key,
            threshold: check.threshold,
            fix_hint: check.fix_hint,
        }));
    }
}

pub(super) struct SingleValueCheck<'a> {
    pub(super) id: &'a str,
    pub(super) code: &'a str,
    pub(super) severity: &'a str,
    pub(super) region: RenderQualityRegion,
    pub(super) observed_key: &'a str,
    pub(super) observed: f32,
    pub(super) threshold_key: &'a str,
    pub(super) threshold: f32,
    pub(super) fix_hint: &'a str,
}

pub(super) fn single_value_check(check: SingleValueCheck<'_>) -> RenderQualityCheckV1 {
    RenderQualityCheckV1 {
        id: check.id.to_owned(),
        code: check.code.to_owned(),
        severity: check.severity.to_owned(),
        region: check.region.to_report(),
        observed: BTreeMap::from([(check.observed_key.to_owned(), round3(check.observed))]),
        threshold: BTreeMap::from([(check.threshold_key.to_owned(), round3(check.threshold))]),
        fix_hint: check.fix_hint.to_owned(),
    }
}

pub(super) fn region_from_rect(
    kind: &'static str,
    rect: RenderIntrospectionRectV1,
    width: u32,
    height: u32,
) -> RenderQualityRegion {
    let x = rect.min_x.floor().max(0.0) as u32;
    let y = rect.min_y.floor().max(0.0) as u32;
    let max_x = rect.max_x.ceil().max(rect.min_x).min(width as f32) as u32;
    let max_y = rect.max_y.ceil().max(rect.min_y).min(height as f32) as u32;
    RenderQualityRegion {
        kind,
        handle: None,
        x: x.min(width),
        y: y.min(height),
        width: max_x.saturating_sub(x).max(1),
        height: max_y.saturating_sub(y).max(1),
    }
}
