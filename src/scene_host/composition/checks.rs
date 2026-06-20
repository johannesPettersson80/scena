use std::collections::BTreeMap;

use serde_json::Value;

use crate::{
    CaptureScreenRect, CaptureScreenRegion, RenderIntrospectionRectV1, SceneCompositionCheckV1,
    SceneCompositionRegionV1, SceneCompositionStatusV1,
};

pub(super) trait CompositionCheckExt {
    fn with_region(
        self,
        kind: &str,
        handle: Option<u64>,
        rect: Option<CaptureScreenRect>,
    ) -> SceneCompositionCheckV1;
    fn with_region_from_screen(
        self,
        kind: &str,
        handle: Option<u64>,
        region: CaptureScreenRegion,
    ) -> SceneCompositionCheckV1;
    fn with_region_value(self, region: SceneCompositionRegionV1) -> SceneCompositionCheckV1;
}

impl CompositionCheckExt for SceneCompositionCheckV1 {
    fn with_region(
        mut self,
        kind: &str,
        handle: Option<u64>,
        rect: Option<CaptureScreenRect>,
    ) -> SceneCompositionCheckV1 {
        self.region = rect.map(|rect| region_from_capture_rect(kind, handle, rect));
        self
    }

    fn with_region_from_screen(
        mut self,
        kind: &str,
        handle: Option<u64>,
        region: CaptureScreenRegion,
    ) -> SceneCompositionCheckV1 {
        self.region = Some(region_from_screen(kind, handle, region));
        self
    }

    fn with_region_value(mut self, region: SceneCompositionRegionV1) -> SceneCompositionCheckV1 {
        self.region = Some(region);
        self
    }
}

pub(super) fn checked_check(
    id: String,
    category: &str,
    code: &str,
    target_id: Option<String>,
    affected_handles: Vec<u64>,
    observed: BTreeMap<String, Value>,
    copy: (&str, &str),
) -> SceneCompositionCheckV1 {
    base_check(CheckParts {
        id,
        category: category.to_owned(),
        code: code.to_owned(),
        status: SceneCompositionStatusV1::Checked,
        severity: "info".to_owned(),
        target_id,
        affected_handles,
        observed,
        message: copy.0.to_owned(),
        fix_hint: copy.1.to_owned(),
    })
}

pub(super) fn error_check(
    id: String,
    category: &str,
    code: &str,
    target_id: Option<String>,
    affected_handles: Vec<u64>,
    observed: BTreeMap<String, Value>,
    copy: (&str, &str),
) -> SceneCompositionCheckV1 {
    base_check(CheckParts {
        id,
        category: category.to_owned(),
        code: code.to_owned(),
        status: SceneCompositionStatusV1::Failed,
        severity: "error".to_owned(),
        target_id,
        affected_handles,
        observed,
        message: copy.0.to_owned(),
        fix_hint: copy.1.to_owned(),
    })
}

pub(super) fn skip_check(
    id: String,
    category: &str,
    code: &str,
    status: SceneCompositionStatusV1,
    target_id: Option<String>,
    affected_handles: Vec<u64>,
    copy: (&str, &str),
) -> SceneCompositionCheckV1 {
    base_check(CheckParts {
        id,
        category: category.to_owned(),
        code: code.to_owned(),
        status,
        severity: "warning".to_owned(),
        target_id,
        affected_handles,
        observed: BTreeMap::new(),
        message: copy.0.to_owned(),
        fix_hint: copy.1.to_owned(),
    })
}

struct CheckParts {
    id: String,
    category: String,
    code: String,
    status: SceneCompositionStatusV1,
    severity: String,
    target_id: Option<String>,
    affected_handles: Vec<u64>,
    observed: BTreeMap<String, Value>,
    message: String,
    fix_hint: String,
}

fn base_check(parts: CheckParts) -> SceneCompositionCheckV1 {
    SceneCompositionCheckV1 {
        id: parts.id,
        category: parts.category,
        code: parts.code,
        status: parts.status,
        severity: parts.severity,
        region: None,
        target_id: parts.target_id,
        affected_handles: parts.affected_handles,
        observed: parts.observed,
        threshold: None,
        message: parts.message,
        fix_hint: parts.fix_hint,
    }
}

pub(super) fn observed_pairs<const N: usize>(
    pairs: [(&'static str, Value); N],
) -> BTreeMap<String, Value> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect()
}

fn region_from_capture_rect(
    kind: &str,
    handle: Option<u64>,
    rect: CaptureScreenRect,
) -> SceneCompositionRegionV1 {
    SceneCompositionRegionV1 {
        kind: kind.to_owned(),
        handle,
        rect_css_px: Some(RenderIntrospectionRectV1 {
            min_x: round3(rect.min_x),
            min_y: round3(rect.min_y),
            max_x: round3(rect.max_x),
            max_y: round3(rect.max_y),
            width: round3(rect.width),
            height: round3(rect.height),
        }),
    }
}

pub(super) fn region_from_screen(
    kind: &str,
    handle: Option<u64>,
    region: CaptureScreenRegion,
) -> SceneCompositionRegionV1 {
    let rect = CaptureScreenRect {
        min_x: region.x as f32,
        min_y: region.y as f32,
        max_x: (region.x + region.width) as f32,
        max_y: (region.y + region.height) as f32,
        width: region.width as f32,
        height: region.height as f32,
        center_x: region.x as f32 + region.width as f32 * 0.5,
        center_y: region.y as f32 + region.height as f32 * 0.5,
    };
    region_from_capture_rect(kind, handle, rect)
}

pub(super) fn round3(value: f32) -> f32 {
    if value.is_finite() {
        ((value as f64) * 1000.0).round() as f32 / 1000.0
    } else {
        value
    }
}
