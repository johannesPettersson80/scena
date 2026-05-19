use crate::app::prelude::*;

pub(super) fn check_round_a_easy_use_primitives(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ROUND-A-EASY-USE-PRIMITIVES",
        "src/material.rs",
        &[
            "pub const TRANSPARENT",
            "pub const GRAY",
            "pub const BLUE",
            "pub fn from_hex(",
            "pub fn from_kelvin(",
        ],
    );
    require_contains(
        root,
        findings,
        "ROUND-A-EASY-USE-PRIMITIVES",
        "src/scene/camera.rs",
        &[
            "pub fn standard()",
            "pub fn wide_angle()",
            "pub fn portrait()",
            "pub fn telephoto()",
            "pub fn with_fov_degrees(",
        ],
    );
    require_contains(
        root,
        findings,
        "ROUND-A-EASY-USE-PRIMITIVES",
        "src/scene/math.rs",
        &["pub fn looking_at("],
    );
    require_contains(
        root,
        findings,
        "ROUND-A-EASY-USE-PRIMITIVES",
        "tests/round_a_easy_use.rs",
        &[
            "round_a_color_named_constants_and_hex_alias_are_public",
            "round_a_color_kelvin_helper_is_clamped_and_ordered",
            "round_a_perspective_camera_lens_presets_are_named_degree_surfaces",
            "round_a_transform_looking_at_faces_target_with_requested_up",
        ],
    );
    require_contains(
        root,
        findings,
        "ROUND-A-EASY-USE-PRIMITIVES",
        "tests/examples_visual_proof.rs",
        &[
            "round_a_named_color_swatch_docs_image",
            "round-a-named-color-swatch-docs-image",
            "round_a_lens_preset_comparison_docs_image",
            "round-a-lens-preset-comparison-docs-image",
        ],
    );

    for rel in ROUND_A_CAMERA_FIRST_PATH_FILES {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if text.contains("PerspectiveCamera::default().with_aspect(") {
            findings.push(Finding::new(
                "ROUND-A-EASY-USE-PRIMITIVES",
                format!(
                    "{rel} must use a named PerspectiveCamera lens preset and let frame_bounds/viewport own aspect"
                ),
            ));
        }
        if contains_raw_camera_fov_literal(&text) {
            findings.push(Finding::new(
                "ROUND-A-EASY-USE-PRIMITIVES",
                format!(
                    "{rel} must use named PerspectiveCamera lens presets in first-path examples; keep raw FOV setters in the dedicated escape hatch"
                ),
            ));
        }
    }

    for rel in ROUND_A_COLOR_FIRST_PATH_FILES {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if contains_raw_color_literal(&text) {
            findings.push(Finding::new(
                "ROUND-A-EASY-USE-PRIMITIVES",
                format!("{rel} must use named colors or Color::from_hex for first-path examples"),
            ));
        }
    }
    check_example_quat_literals(root, findings);
}

const ROUND_A_CAMERA_FIRST_PATH_FILES: &[&str] = &[
    "README.md",
    "docs/getting-started.md",
    "docs/api.md",
    "docs/guides/easy-scene-setup.md",
    "docs/guides/migrating-from-threejs.md",
    "examples/easy_model_viewer.rs",
    "examples/camera_framing.rs",
    "examples/connector_auto_framing.rs",
    "src/demo_page.rs",
    "src/demo_page/imports.rs",
];

const ROUND_A_COLOR_FIRST_PATH_FILES: &[&str] = &[
    "README.md",
    "docs/getting-started.md",
    "docs/guides/easy-scene-setup.md",
    "examples/camera_framing.rs",
    "examples/easy_model_viewer.rs",
    "src/demo_page.rs",
    "src/demo_page/imports.rs",
];

const ROUND_A_QUAT_ESCAPE_HATCH_EXAMPLES: &[&str] = &["examples/transform_escape_hatch.rs"];

fn contains_raw_color_literal(text: &str) -> bool {
    [
        "Color::from_srgb(",
        "Color::from_srgb_u8(",
        "Color::from_linear_rgb(",
        "Color::from_linear_rgba(",
    ]
    .into_iter()
    .any(|needle| text.contains(needle))
}

fn contains_raw_camera_fov_literal(text: &str) -> bool {
    text.contains("vertical_fov: Angle::from_degrees(")
        || text.contains(".with_fov_degrees(")
        || text.contains("with_fov_degrees(")
}

fn check_example_quat_literals(root: &Path, findings: &mut Vec<Finding>) {
    let examples_dir = root.join("examples");
    let Ok(entries) = fs::read_dir(examples_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let rel = Path::new("examples").join(entry.file_name());
        if ROUND_A_QUAT_ESCAPE_HATCH_EXAMPLES
            .iter()
            .any(|allowed| rel == Path::new(allowed))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if contains_raw_quat_literal(&text) {
            findings.push(Finding::new(
                "ROUND-A-EASY-USE-PRIMITIVES",
                format!(
                    "{} must use Transform rotation helpers or looking_at; raw Quat::from_* literals belong only in the transform escape hatch",
                    rel.display()
                ),
            ));
        }
    }
}

fn contains_raw_quat_literal(text: &str) -> bool {
    let mut rest = text;
    while let Some(index) = rest.find("Quat::from_") {
        let after_prefix = &rest[index + "Quat::from_".len()..];
        if let Some(open_paren) = after_prefix.find('(') {
            let args = after_prefix[open_paren + 1..].trim_start();
            if starts_with_float_literal(args) {
                return true;
            }
        }
        rest = after_prefix;
    }
    false
}

fn starts_with_float_literal(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    let mut chars = value.chars().peekable();
    let mut saw_before_decimal = false;
    while matches!(chars.peek(), Some(char) if char.is_ascii_digit()) {
        saw_before_decimal = true;
        chars.next();
    }
    if chars.next() != Some('.') || !saw_before_decimal {
        return false;
    }
    let mut saw_after_decimal = false;
    while matches!(chars.peek(), Some(char) if char.is_ascii_digit()) {
        saw_after_decimal = true;
        chars.next();
    }
    saw_after_decimal
}
