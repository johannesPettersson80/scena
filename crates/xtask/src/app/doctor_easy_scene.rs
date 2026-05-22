use crate::app::prelude::*;

mod asset_validation;
mod camera_control_kit;
mod environment_presets;
mod khronos_samples;
mod material_presets;
mod next_release;
mod picking_outline_hover;
mod production_asset_profile;
mod reference_image_regression;
mod round_a_primitives;
mod scena_viewer_element;
mod state_url;
mod viewer_headless_png;
mod viewer_load_progress;
mod viewer_material_variants;

pub(crate) fn check_easy_scene_setup_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "DOCS-EASY-SCENE-SETUP",
        "docs/guides/easy-scene-setup.md",
        &[
            "frame_bounds",
            "add_studio_lighting",
            "add_grid_floor",
            "add_perspective_camera_default_for",
            "set_auto_exposure",
            "scene.mate",
            "project_world_point",
            "Camera views",
            "azimuth_elevation",
            "three_quarter_front_right",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-EASY-SCENE-SETUP",
        "docs/README.md",
        &["Easy scene setup", "guides/easy-scene-setup.md"],
    );
    require_contains(
        root,
        findings,
        "DOCS-EASY-SCENE-SETUP",
        "README.md",
        &[
            "## Easy Scene Setup",
            "docs/guides/easy-scene-setup.md",
            "docs/release-notes/v1.3.0.md",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-EASY-FRAMING",
        "src/demo_page.rs",
        &["frame_bounds(", "bounds_for_transforms", "add_grid_floor"],
    );
    require_contains(
        root,
        findings,
        "DEMO-EASY-FRAMING",
        "src/demo_page/connectors.rs",
        &["project_world_point"],
    );
    check_easy_scene_guide_snippet(root, findings);
    require_contains(
        root,
        findings,
        "DOCS-EASY-SCENE-SETUP",
        "docs/guides/migrating-from-threejs.md",
        &[
            "new THREE.Box3",
            "controls.target.copy",
            "OrbitControls::from_framing",
            "spherical.theta",
            "spherical.phi",
            "azimuth_elevation",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-EASY-SCENE-SETUP",
        "docs/release-notes/v1.3.0.md",
        &[
            "Status: ready",
            "OrbitControls::from_framing",
            "Aabb::union",
            "ScreenRect",
            "ProjectedPoint",
            "GridFloorHandles",
            "LookupError::InvalidBounds",
            "LookupError::UnsupportedCameraType",
            "FramingOptions::azimuth_elevation",
            "FramingOptions::front",
            "FramingOptions::back",
            "FramingOptions::left",
            "FramingOptions::right",
            "FramingOptions::top",
            "FramingOptions::bottom",
            "FramingOptions::three_quarter_front_left",
            "FramingOptions::three_quarter_front_right",
            "FramingOptions::three_quarter_back_left",
            "FramingOptions::three_quarter_back_right",
        ],
    );
    if fs::read_to_string(root.join("docs/release-notes/v1.3.0.md"))
        .is_ok_and(|text| text.contains("Status: draft"))
    {
        findings.push(Finding::new(
            "DOCS-EASY-SCENE-SETUP",
            "v1.3.0 release notes must not be left in draft status",
        ));
    }
    require_contains(
        root,
        findings,
        "DOCS-EASY-SCENE-SETUP",
        "src/scene/framing.rs",
        &[
            "pre-existing aspect",
            "add_perspective_camera_default_for",
            "# Examples",
            "# Errors",
            "LookupError::UnsupportedCameraType",
            "LookupError::InvalidFramingOption",
        ],
    );
    require_contains(
        root,
        findings,
        "DOCS-EASY-SCENE-SETUP",
        "src/diagnostics.rs",
        &[
            "A viewport width or height was zero",
            "Bounds were empty",
            "A named framing option failed validation",
            "does not support the camera type",
        ],
    );
    if fs::read_to_string(root.join("src/scene/lights.rs"))
        .is_ok_and(|text| text.contains("Phase 5.3"))
    {
        findings.push(Finding::new(
            "DOCS-EASY-SCENE-SETUP",
            "public studio-lighting docs must not contain internal phase labels",
        ));
    }
    require_contains(
        root,
        findings,
        "DEMO-EASY-FRAMING",
        "src/diagnostics.rs",
        &[
            "InvalidBounds",
            "InvalidFramingOption",
            "UnsupportedCameraType",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-EASY-FRAMING",
        "tests/examples_visual_proof.rs",
        &[
            "frame_bounds_rendered_output_proves_fill_center_and_unclipped_object",
            "frame-bounds-rendered-output",
            "computed_distance",
            "projected_rect",
            "nonblack_pixel_rect",
        ],
    );
    round_a_primitives::check_round_a_easy_use_primitives(root, findings);
    next_release::check_named_light_presets(root, findings);
    material_presets::check_honest_material_presets(root, findings);
    next_release::check_named_background_presets(root, findings);
    environment_presets::check_environment_presets(root, findings);
    next_release::check_named_orbit_control_presets(root, findings);
    next_release::check_named_auto_exposure_presets(root, findings);
    next_release::check_one_call_animation_playback(root, findings);
    next_release::check_connector_axial_gap(root, findings);
    next_release::check_orbit_zoom_limits(root, findings);
    next_release::check_viewer_pointer_callbacks(root, findings);
    next_release::check_viewer_capture_png(root, findings);
    viewer_headless_png::check_viewer_headless_png(root, findings);
    viewer_load_progress::check_viewer_load_progress(root, findings);
    viewer_material_variants::check_viewer_material_variants(root, findings);
    camera_control_kit::check_camera_control_kit(root, findings);
    asset_validation::check_asset_validation_doctor(root, findings);
    picking_outline_hover::check_picking_outline_hover(root, findings);
    reference_image_regression::check_reference_image_regression(root, findings);
    scena_viewer_element::check_scena_viewer_element(root, findings);
    next_release::check_asset_hot_reload(root, findings);
    khronos_samples::check_khronos_samples(root, findings);
    production_asset_profile::check_production_asset_profile(root, findings);
    state_url::check_state_via_url(root, findings);

    for rel in ["src/lib.rs", "src/geometry.rs"] {
        if fs::read_to_string(root.join(rel)).is_ok_and(|text| text.contains("FramingAngles")) {
            findings.push(Finding::new(
                "DEMO-EASY-FRAMING",
                format!("{rel} must not re-export legacy FramingAngles; use Scene::frame_bounds"),
            ));
        }
    }
    if fs::read_to_string(root.join("src/geometry/bounds.rs"))
        .is_ok_and(|text| text.contains("framing_transform"))
    {
        findings.push(Finding::new(
            "DEMO-EASY-FRAMING",
            "src/geometry/bounds.rs must not expose legacy Aabb::framing_transform",
        ));
    }
    check_demo_camera_views_named(root, findings);
    check_demo_diagnostics_contract(root, findings);
}

fn check_easy_scene_guide_snippet(root: &Path, findings: &mut Vec<Finding>) {
    let rel = "docs/guides/easy-scene-setup.md";
    let Ok(text) = fs::read_to_string(root.join(rel)) else {
        findings.push(Finding::new(
            "DOCS-EASY-SCENE-SETUP",
            format!("could not read {rel}"),
        ));
        return;
    };
    let required = [
        "Scene::new()",
        "scene.add_studio_lighting()",
        "scene.add_grid_floor(",
    ];
    let mut in_rust = false;
    let mut block = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```rust") {
            in_rust = true;
            block.clear();
            continue;
        }
        if in_rust && line.trim_start().starts_with("```") {
            if required.iter().all(|needle| block.contains(needle))
                && (block.contains("scene.frame_bounds(")
                    || block.contains("scene.add_perspective_camera_default_for("))
            {
                return;
            }
            in_rust = false;
            continue;
        }
        if in_rust {
            block.push_str(line);
            block.push('\n');
        }
    }
    findings.push(Finding::new(
        "DOCS-EASY-SCENE-SETUP",
        "easy scene setup guide must contain one Rust block with Scene::new, add_studio_lighting, add_grid_floor, and camera framing",
    ));
}

fn check_demo_camera_views_named(root: &Path, findings: &mut Vec<Finding>) {
    for rel in demo_page_source_files(root) {
        let path = root.join(&rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if contains_inline_look_from_vec3_literal(&text) {
            findings.push(Finding::new(
                "DEMO-CAMERA-VIEWS-NAMED",
                format!(
                    "{} must use named camera views or azimuth_elevation(), not inline Vec3::new(...) look_from literals",
                    rel.display()
                ),
            ));
        }
        if contains_inline_orbit_float_literals(&text) {
            findings.push(Finding::new(
                "DEMO-CAMERA-VIEWS-NAMED",
                format!(
                    "{} must use named camera views or azimuth_elevation(), not inline .orbit(float, float) literals",
                    rel.display()
                ),
            ));
        }
        if text.contains(".with_angles") {
            findings.push(Finding::new(
                "DEMO-CAMERA-VIEWS-NAMED",
                format!(
                    "{} must not patch framed orbit yaw/pitch with .with_angles()",
                    rel.display()
                ),
            ));
        }
        for line in text.lines() {
            if let Some(name) = const_name(line)
                && (name.contains("VIEW_DIRECTION")
                    || (name.contains("APPROVED") && name.contains("VIEW")))
            {
                findings.push(Finding::new(
                    "DEMO-CAMERA-VIEWS-NAMED",
                    format!(
                        "{} must not hide demo camera views in opaque constants named {name}",
                        rel.display()
                    ),
                ));
            }
        }
    }
}

fn contains_inline_look_from_vec3_literal(text: &str) -> bool {
    let mut rest = text;
    while let Some(index) = rest.find(".look_from(") {
        let after_call = &rest[index + ".look_from(".len()..];
        let trimmed = after_call.trim_start();
        if let Some(after_vec3) = trimmed.strip_prefix("Vec3::new(")
            && starts_with_float_literal(after_vec3.trim_start())
        {
            return true;
        }
        rest = after_call;
    }
    false
}

fn contains_inline_orbit_float_literals(text: &str) -> bool {
    let mut rest = text;
    while let Some(index) = rest.find(".orbit(") {
        let after_call = &rest[index + ".orbit(".len()..];
        if let Some(end) = after_call.find(')') {
            let args = &after_call[..end];
            let mut parts = args.split(',').map(str::trim);
            if parts.next().is_some_and(starts_with_float_literal)
                && parts.next().is_some_and(starts_with_float_literal)
            {
                return true;
            }
        }
        rest = after_call;
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

fn const_name(line: &str) -> Option<&str> {
    let mut words = line.split(|ch: char| ch.is_whitespace() || ch == ':');
    while let Some(word) = words.next() {
        if word == "const" {
            return words.find(|candidate| !candidate.is_empty());
        }
    }
    None
}

fn check_demo_diagnostics_contract(root: &Path, findings: &mut Vec<Finding>) {
    let html_path = root.join("demo/index.html");
    let Ok(html) = fs::read_to_string(&html_path) else {
        findings.push(Finding::new(
            "DEMO-DIAGNOSTICS",
            "demo/index.html could not be read",
        ));
        return;
    };
    let Some(diagnostics_tag) = find_tag_with_id(&html, "details", "diagnostics") else {
        findings.push(Finding::new(
            "DEMO-DIAGNOSTICS",
            "demo diagnostics must be a closed <details id=\"diagnostics\"> element",
        ));
        return;
    };
    if !diagnostics_tag.contains("class=\"diagnostics\"") {
        findings.push(Finding::new(
            "DEMO-DIAGNOSTICS",
            "demo diagnostics <details> must keep class=\"diagnostics\"",
        ));
    }
    if diagnostics_tag
        .split_whitespace()
        .any(|part| part.trim_end_matches('>') == "open")
    {
        findings.push(Finding::new(
            "DEMO-DIAGNOSTICS",
            "demo diagnostics must be collapsed by default",
        ));
    }
    let details_index = html.find("id=\"diagnostics\"").unwrap_or_default();
    if let Some(frame_index) = html.find("id=\"metric-frame\"") {
        if frame_index < details_index {
            findings.push(Finding::new(
                "DEMO-DIAGNOSTICS",
                "frame counter must live inside collapsed diagnostics",
            ));
        }
    } else {
        findings.push(Finding::new(
            "DEMO-DIAGNOSTICS",
            "collapsed diagnostics must own the frame counter metric",
        ));
    }
    if fs::read_to_string(root.join("demo/main.js"))
        .is_ok_and(|text| text.contains("frame ${") || text.contains("frame \""))
    {
        findings.push(Finding::new(
            "DEMO-DIAGNOSTICS",
            "public status text must not expose frame-counter wording outside diagnostics",
        ));
    }
}

fn find_tag_with_id<'a>(html: &'a str, tag: &str, id: &str) -> Option<&'a str> {
    let mut rest = html;
    let needle = format!("<{tag}");
    while let Some(start) = rest.find(&needle) {
        let after_start = &rest[start..];
        let end = after_start.find('>')?;
        let candidate = &after_start[..=end];
        if candidate.contains(&format!("id=\"{id}\"")) {
            return Some(candidate);
        }
        rest = &after_start[end + 1..];
    }
    None
}

fn demo_page_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![PathBuf::from("src/demo_page.rs")];
    let dir = root.join("src/demo_page");
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                files.push(PathBuf::from("src/demo_page").join(entry.file_name()));
            }
        }
    }
    files
}
