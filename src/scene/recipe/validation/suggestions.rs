pub(super) fn nearest_root_field(input: &str) -> Option<&'static str> {
    nearest(input, ROOT_FIELDS)
}

pub(super) fn nearest_import_field(input: &str) -> Option<&'static str> {
    nearest(input, IMPORT_FIELDS)
}

pub(super) fn nearest_capture_field(input: &str) -> Option<&'static str> {
    nearest(input, CAPTURE_FIELDS)
}

fn nearest(input: &str, candidates: &'static [&'static str]) -> Option<&'static str> {
    candidates
        .iter()
        .map(|candidate| (*candidate, edit_distance(input, candidate)))
        .filter(|(_, distance)| *distance <= 4)
        .min_by(|(left_name, left), (right_name, right)| {
            left.cmp(right).then_with(|| left_name.cmp(right_name))
        })
        .map(|(candidate, _)| candidate)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = usize::from(left_char != *right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
}

pub(super) const ROOT_FIELDS: &[&str] = &[
    "schema",
    "imports",
    "colors",
    "geometries",
    "materials",
    "nodes",
    "instance_sets",
    "fonts",
    "labels",
    "clipping_planes",
    "animations",
    "cameras",
    "lights",
    "scene",
    "render",
    "expect",
    "section_box",
    "measurements",
    "callouts",
    "exploded_view",
    "capture",
    "metadata",
];
pub(super) const IMPORT_FIELDS: &[&str] =
    &["id", "uri", "optional", "transform", "expected_extent"];
pub(super) const CAPTURE_FIELDS: &[&str] = &["width", "height"];
pub(super) const EXPECTED_EXTENT_FIELDS: &[&str] = &["min", "max", "unit"];
pub(super) const UNSUPPORTED_WORKFLOW_FIELDS: &[&str] = &[
    "steps",
    "sequence",
    "sequences",
    "loop",
    "loops",
    "branch",
    "branches",
    "timeline",
    "script",
];
pub(super) const UNSUPPORTED_SECTION_FIELDS: &[&str] = &[
    "primitives",
    "skins",
    "morphs",
    "particles",
    "viewer_profile",
    "environment",
    "placements",
    "anchors",
    "connectors",
    "bounds",
    "named_states",
];
