use crate::app::prelude::*;

pub(super) fn check_production_asset_profile(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "Cargo.toml",
        &[
            "default = []",
            "ktx2 = [\"dep:ktx2\", \"dep:basisu_c_sys\"]",
            "meshopt = [\"dep:meshopt\"]",
            "production-assets = [\"ktx2\", \"meshopt\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "docs/feature-flags.md",
        &[
            "`production-assets`",
            "enables `ktx2` + `meshopt`",
            "features = [\"production-assets\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "PRODUCTION-ASSET-PROFILE",
        "tests/production_asset_profile.rs",
        &["production_asset_profile_enables_compressed_asset_decoders_without_default_bloat"],
    );
}

pub(super) fn check_named_light_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "NAMED-LIGHT-PRESETS",
        "src/scene/lights.rs",
        &[
            "pub fn sun()",
            "pub fn key_light()",
            "pub fn fill_light()",
            "pub fn rim_light()",
            "pub fn softbox()",
            "pub fn bulb_warm()",
            "pub fn bulb_cool()",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-LIGHT-PRESETS",
        "tests/round_b_light_presets.rs",
        &[
            "named_directional_light_presets_are_public_and_ordered",
            "named_point_light_presets_are_kelvin_tinted_and_range_limited",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-LIGHT-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_b_light_preset_reference_docs_image",
            "round-b-light-preset-reference-docs-image",
            "reference-image+docs-image",
        ],
    );
}

pub(super) fn check_honest_material_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/material/presets.rs",
        &[
            "pub const fn matte(",
            "pub const fn plastic(",
            "pub const fn metal(",
            "pub const fn rubber()",
        ],
    );
    if fs::read_to_string(root.join("src/material/presets.rs")).is_ok_and(|text| {
        [
            "pub fn chrome(",
            "pub fn brushed_steel(",
            "pub fn clear_glass(",
            "pub fn frosted_glass(",
            "pub fn leather(",
        ]
        .into_iter()
        .any(|needle| text.contains(needle))
    }) {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            "material presets must not expose chrome/glass/leather names before the renderer supports their visual contract",
        ));
    }
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/round_b_material_presets.rs",
        &["honest_material_presets_are_public_pbr_shortcuts"],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_b_material_preset_reference_docs_image",
            "round-b-material-preset-reference-docs-image",
            "reference-image+docs-image",
        ],
    );
}

pub(super) fn check_named_background_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "NAMED-BACKGROUND-PRESETS",
        "src/render/background.rs",
        &[
            "pub enum Background",
            "Studio",
            "DarkStudio",
            "NeutralGray",
            "Transparent",
            "Custom(Color)",
            "pub const fn color(",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-BACKGROUND-PRESETS",
        "src/render/settings.rs",
        &["pub fn set_background(", "self.set_background_color("],
    );
    require_contains(
        root,
        findings,
        "NAMED-BACKGROUND-PRESETS",
        "tests/round_b_background_presets.rs",
        &[
            "named_background_presets_map_to_public_colors",
            "renderer_set_background_uses_named_scheme",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-BACKGROUND-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_b_background_preset_reference_docs_image",
            "round-b-background-preset-reference-docs-image",
            "reference-image+docs-image",
        ],
    );
}

pub(super) fn check_named_orbit_control_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "NAMED-ORBIT-CONTROL-PRESETS",
        "src/controls.rs",
        &[
            "pub fn cinematic(",
            "pub fn snappy(",
            "pub fn presentation(",
            "pub fn turntable(",
            "pub fn advance(",
            "pub const fn auto_rotate_rpm(",
            "pub fn auto_rotate_radians_per_second(",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-ORBIT-CONTROL-PRESETS",
        "tests/round_b_orbit_controls_presets.rs",
        &[
            "named_orbit_damping_presets_are_public_and_ordered",
            "turntable_presets_expose_explicit_frame_advance_semantics",
            "presentation_combines_medium_damping_with_slow_turntable_motion",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-ORBIT-CONTROL-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_b_orbit_control_preset_animated_docs_image",
            "round-b-orbit-control-preset-animated-docs-image",
            "animated-proof+docs-image",
        ],
    );

    for rel in ["src/demo_page.rs", "src/demo_page/imports.rs"] {
        let path = root.join(rel);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if text.contains(".with_damping(") {
            findings.push(Finding::new(
                "NAMED-ORBIT-CONTROL-PRESETS",
                format!(
                    "{rel} must use named OrbitControls presets instead of raw damping literals"
                ),
            ));
        }
    }
}

pub(super) fn check_named_auto_exposure_presets(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "NAMED-AUTO-EXPOSURE-PRESETS",
        "src/render/exposure.rs",
        &[
            "pub const fn product_studio(",
            "pub const fn indoor(",
            "pub const fn outdoor(",
            "pub const fn mixed(",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-AUTO-EXPOSURE-PRESETS",
        "tests/round_c_auto_exposure_presets.rs",
        &[
            "named_auto_exposure_scenarios_are_public_and_ordered",
            "scenario_presets_drive_different_ev_solutions",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-AUTO-EXPOSURE-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_c_auto_exposure_preset_reference_docs_image",
            "round-c-auto-exposure-preset-reference-docs-image",
            "reference-image+docs-image",
        ],
    );
    require_contains(
        root,
        findings,
        "NAMED-AUTO-EXPOSURE-PRESETS",
        "docs/guides/easy-scene-setup.md",
        &[
            "AutoExposureConfig::product_studio()",
            "AutoExposureConfig::indoor()",
            "AutoExposureConfig::outdoor()",
            "AutoExposureConfig::mixed()",
        ],
    );

    let rel = "src/demo_page.rs";
    if fs::read_to_string(root.join(rel))
        .is_ok_and(|text| text.contains("AutoExposureConfig::new("))
    {
        findings.push(Finding::new(
            "NAMED-AUTO-EXPOSURE-PRESETS",
            format!(
                "{rel} must use named AutoExposureConfig scenarios instead of raw exposure literals"
            ),
        ));
    }
}

pub(super) fn check_one_call_animation_playback(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ONE-CALL-ANIMATION-PLAYBACK",
        "src/scene/mixers.rs",
        &[
            "pub fn play_animation_by_name(",
            "self.create_animation_mixer(",
            "self.play_animation(mixer)",
        ],
    );
    require_contains(
        root,
        findings,
        "ONE-CALL-ANIMATION-PLAYBACK",
        "tests/round_c_animation_playback.rs",
        &["scene_play_animation_by_name_creates_and_starts_mixer"],
    );
    require_contains(
        root,
        findings,
        "ONE-CALL-ANIMATION-PLAYBACK",
        "tests/examples_visual_proof.rs",
        &[
            "round_c_animation_playback_reference_animated_docs_image",
            "round-c-animation-playback-reference-animated-docs-image",
            "animated-proof+docs-image",
        ],
    );
    require_contains(
        root,
        findings,
        "ONE-CALL-ANIMATION-PLAYBACK",
        "examples/animation.rs",
        &["play_animation_by_name(&import"],
    );
    require_contains(
        root,
        findings,
        "ONE-CALL-ANIMATION-PLAYBACK",
        "docs/guides/easy-scene-setup.md",
        &["play_animation_by_name(&import"],
    );
}

pub(super) fn check_connector_axial_gap(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "CONNECTOR-AXIAL-GAP",
        "src/scene/connectors/options.rs",
        &["pub fn with_axial_gap(", "pub const fn axial_gap("],
    );
    require_contains(
        root,
        findings,
        "CONNECTOR-AXIAL-GAP",
        "tests/round_d_connector_axial_gap.rs",
        &[
            "connect_options_axial_gap_offsets_along_target_forward_axis",
            "axial_gap_sanitizes_invalid_or_negative_values_to_zero",
        ],
    );
    require_contains(
        root,
        findings,
        "CONNECTOR-AXIAL-GAP",
        "docs/guides/place-and-connect-objects.md",
        &["with_axial_gap(0.4)", "target connector's forward axis"],
    );
    if fs::read_to_string(root.join("src/scene/connectors/options.rs"))
        .is_ok_and(|text| text.contains("with_clearance_mm("))
    {
        findings.push(Finding::new(
            "CONNECTOR-AXIAL-GAP",
            "with_clearance_mm must not ship until source-unit metadata can make it fail closed",
        ));
    }
}

pub(super) fn check_orbit_zoom_limits(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ORBIT-ZOOM-LIMITS",
        "src/controls.rs",
        &[
            "pub fn zoom_limits_bounds_relative(",
            "pub fn with_distance_limits(",
            "pub const fn min_distance(",
            "pub const fn max_distance(",
            "fn clamp_distance(",
        ],
    );
    require_contains(
        root,
        findings,
        "ORBIT-ZOOM-LIMITS",
        "tests/round_d_orbit_zoom_limits.rs",
        &[
            "orbit_zoom_limits_are_relative_to_current_framed_distance",
            "wheel_and_pinch_zoom_are_clamped_to_named_limits",
        ],
    );
    require_contains(
        root,
        findings,
        "ORBIT-ZOOM-LIMITS",
        "tests/examples_visual_proof.rs",
        &[
            "round_d_orbit_zoom_limit_animated_docs_image",
            "round-d-orbit-zoom-limit-animated-docs-image",
            "animated-proof+docs-image",
        ],
    );
    require_contains(
        root,
        findings,
        "ORBIT-ZOOM-LIMITS",
        "docs/guides/easy-scene-setup.md",
        &["zoom_limits_bounds_relative(0.5, 4.0)"],
    );
}

pub(super) fn check_viewer_pointer_callbacks(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VIEWER-POINTER-CALLBACKS",
        "src/viewer.rs",
        &[
            "mod interaction;",
            "click_callback: Option<ViewerPickCallback>",
            "hover_callback: Option<ViewerPickCallback>",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-POINTER-CALLBACKS",
        "src/viewer/interaction.rs",
        &[
            "pub fn on_click<",
            "pub fn on_hover<",
            "pub fn clear_click_callback(",
            "pub fn clear_hover_callback(",
            "pub fn click_at(",
            "pub fn hover_at(",
            "pick_and_select_at(",
            "pick_and_hover_at(",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-POINTER-CALLBACKS",
        "tests/round_d_viewer_pointer_callbacks.rs",
        &[
            "viewer_click_and_hover_callbacks_receive_hit_and_no_hit_results",
            "click_events",
            "hover_events",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-POINTER-CALLBACKS",
        "tests/examples_visual_proof.rs",
        &[
            "round_d_viewer_pointer_callback_animated_docs_image",
            "round-d-viewer-pointer-callback-animated-docs-image",
            "animated-proof+docs-image",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-POINTER-CALLBACKS",
        "docs/guides/easy-scene-setup.md",
        &[
            "viewer.on_click(",
            "viewer.on_hover(",
            "viewer.click_at(",
            "viewer.hover_at(",
        ],
    );
}

pub(super) fn check_viewer_capture_png(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VIEWER-CAPTURE-PNG",
        "src/viewer.rs",
        &["mod capture;", "pub use capture::ViewerCaptureError;"],
    );
    require_contains(
        root,
        findings,
        "VIEWER-CAPTURE-PNG",
        "src/viewer/capture.rs",
        &[
            "pub enum ViewerCaptureError",
            "pub fn capture_png_bytes(",
            "pub fn capture_png(",
            "png::Encoder::new",
            "png::ColorType::Rgba",
            "png::BitDepth::Eight",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-CAPTURE-PNG",
        "src/lib.rs",
        &["ViewerCaptureError"],
    );
    require_contains(
        root,
        findings,
        "VIEWER-CAPTURE-PNG",
        "tests/round_d_viewer_capture_png.rs",
        &[
            "viewer_capture_png_bytes_decode_to_current_frame",
            "viewer_capture_png_writes_reference_artifact",
            "viewer-capture-png-reference.png",
        ],
    );
    require_contains(
        root,
        findings,
        "VIEWER-CAPTURE-PNG",
        "docs/guides/easy-scene-setup.md",
        &[
            "viewer.capture_png(\"frame.png\")?",
            "viewer.capture_png_bytes()?",
        ],
    );
}

pub(super) fn check_asset_hot_reload(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSET-HOT-RELOAD",
        "Cargo.toml",
        &[
            "notify-debouncer-full",
            "optional = true",
            "hot-reload = [\"dep:notify-debouncer-full\"]",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-HOT-RELOAD",
        "src/assets.rs",
        &[
            "mod hot_reload;",
            "AssetHotReloadError",
            "AssetHotReloadWatcher",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-HOT-RELOAD",
        "src/assets/hot_reload.rs",
        &[
            "new_debouncer",
            "DebounceEventResult",
            "RecursiveMode::NonRecursive",
            "pub struct AssetHotReloadWatcher",
            "pub enum AssetHotReloadError",
            "watch_scene_for_hot_reload",
            "drain_changed_scenes",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-HOT-RELOAD",
        "src/lib.rs",
        &["AssetHotReloadError", "AssetHotReloadWatcher"],
    );
    require_contains(
        root,
        findings,
        "ASSET-HOT-RELOAD",
        "tests/round_d_asset_hot_reload.rs",
        &[
            "asset_hot_reload_watcher_reports_debounced_file_change_and_reload_updates_retained_asset",
            "asset-hot-reload-animated-proof.ppm",
            "reload_scene(&first)",
            "replace_import(&import, &reloaded)",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSET-HOT-RELOAD",
        "docs/guides/easy-scene-setup.md",
        &[
            "watch_scene_for_hot_reload",
            "drain_changed_scenes",
            "reload_scene(&scene_asset)",
            "replace_import(&import, &reloaded)",
        ],
    );
}
