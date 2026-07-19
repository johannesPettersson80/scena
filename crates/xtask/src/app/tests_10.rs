use crate::app::prelude::*;

#[test]
pub(crate) fn m7_ergonomics_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_m7_ergonomics_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn easy_scene_setup_contracts_are_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_inline_look_from_literal_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-inline-look-from");
    write_minimal_easy_scene_fixture(
        &fixture_root,
        "frame_bounds(()) bounds_for_transforms add_grid_floor FramingOptions::new().look_from(Vec3::new(-0.4398, 0.3051, 0.8447))",
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-CAMERA-VIEWS-NAMED"),
        "doctor must reject inline Vec3 literal look_from camera views in the demo: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_post_framing_angle_patch() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-angle-patch");
    write_minimal_easy_scene_fixture(
        &fixture_root,
        "frame_bounds(()) bounds_for_transforms add_grid_floor .focus_on_framing(framing).with_angles(-0.4, 0.3)",
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-CAMERA-VIEWS-NAMED"),
        "doctor must reject .with_angles() pose patches after focus_on_framing(): {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_open_diagnostics_and_public_frame_text() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-diagnostics");
    write_minimal_easy_scene_fixture(
        &fixture_root,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
    );
    fs::write(
        fixture_root.join("demo/index.html"),
        r#"<details id="diagnostics" class="diagnostics" open><strong id="metric-frame">0</strong></details>"#,
    )
    .expect("demo html fixture");
    fs::write(
        fixture_root.join("demo/main.js"),
        "setStatus(activeAsset.label, `frame ${frameCount}`);",
    )
    .expect("demo js fixture");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-DIAGNOSTICS"),
        "doctor must reject public frame-counter diagnostics regressions: {findings:?}",
    );
}

pub(crate) fn write_minimal_easy_scene_fixture(fixture_root: &Path, demo_page_rs: &str) {
    let _ = fs::remove_dir_all(fixture_root);
    for dir in "src/demo_page docs/guides docs/release-notes docs/checklists demo examples src/assets src/capture src/viewer src/controls src/scene src/scene/connectors src/material src/render src/geometry tests tests/assets/gltf".split_whitespace() {
        fs::create_dir_all(fixture_root.join(dir)).expect("fixture dir");
    }
    crate::app::tests_17::write_shared_capture_fixture(fixture_root);
    fs::write(
        fixture_root.join("docs/guides/easy-scene-setup.md"),
        "frame_bounds add_perspective_camera_default_for add_studio_lighting add_grid_floor set_auto_exposure scene.mate project_world_point Camera views azimuth_elevation three_quarter_front_right AutoExposureConfig::product_studio() AutoExposureConfig::indoor() AutoExposureConfig::outdoor() AutoExposureConfig::mixed() play_animation_by_name(&import viewer.play_clip( zoom_limits_bounds_relative(0.5, 4.0) FollowControls::behind_and_above FlyControls::new move_local with_yaw_pitch_degrees viewer.on_click( viewer.on_hover( viewer.click_at( viewer.hover_at( InteractionStyle::outline renderer.set_hover_style renderer.set_selection_style selection or hover state updates viewer.capture_png(\"frame.png\")? viewer.capture_png_bytes()? render_png_bytes() CPU headless renderer without requesting a GPU adapter Reference-image regression ReferenceImage::from_rgba8 regress_with_tolerance ReferenceImageTolerance::new().with_max_abs_diff AssetLoadProgress build_with_progress load_progress_events Material variants viewer.material_variants() viewer.set_active_material_variant(Some(\"blue\"))? viewer.set_active_material_variant(None)? watch_scene_for_hot_reload drain_changed_scenes reload_scene(&scene_asset) replace_import(&import, &reloaded) controls.url_state().to_query_string() CameraOrbitUrlState::from_url_query controls.with_url_state(state) framing.url_state().to_query_string() EnvironmentPreset::Studio load_environment_preset EnvironmentPreset::ALL KTX2 cubemap presets are still future work khronos-samples assets.khronos().water_bottle().await? KhronosSample::ALL\n```rust\nlet mut scene = Scene::new();\nscene.add_studio_lighting()?;\nscene.add_grid_floor(&assets, GridFloorOptions::new())?;\nscene.add_perspective_camera_default_for(bounds, (width, height))?;\n```",
    )
    .expect("guide fixture");
    fs::write(
        fixture_root.join("Cargo.toml"),
        "notify-debouncer-full = { version = \"0.7.0\", optional = true }\nhot-reload = [\"dep:notify-debouncer-full\"]\nkhronos-samples = []\ndefault = []\nktx2 = [\"dep:ktx2\", \"dep:basisu_c_sys\"]\nmeshopt = [\"dep:meshopt\"]\nproduction-assets = [\"ktx2\", \"meshopt\"]\nserde = { version = \"1\", features = [\"derive\"] }\nurlencoding = \"2\"",
    )
    .expect("manifest fixture");
    fs::write(
        fixture_root.join("docs/feature-flags.md"),
        "khronos-samples Khronos glTF sample-asset catalog `production-assets` enables `ktx2` + `meshopt` cargo add scena --features production-assets",
    )
    .expect("feature flags fixture");
    fs::write(
        fixture_root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md"),
        "Production-grade asset pipeline complete and production-profile ready Status: **[shipped]** for the production profile tests/m8_compressed_asset_release_proof.rs target/gate-artifacts/m8-compressed-assets CPU rasterizer fallback for no-GPU screenshots Status: **[shipped]** render_png_bytes() Reference-image regression as a public API Status:\n  **[shipped]** ReferenceImage::from_rgba8 REFERENCE-IMAGE-REGRESSION Follow/Fly library primitives **[shipped]** tests/camera_control_kit.rs CAMERA-CONTROL-KIT Picking + hover + selection Status: **[shipped]** PICKING-OUTLINE-HOVER examples_visual_picking_selection_hover_renders_pick_state_to_ppm",
    )
    .expect("next release checklist fixture");
    fs::write(
        fixture_root.join("docs/api.md"),
        "ReferenceImage::from_rgba8 regress regress_with_tolerance ReferenceImageError",
    )
    .expect("api fixture");
    fs::write(
        fixture_root.join("docs/guides/migrating-from-threejs.md"),
        "new THREE.Box3 controls.target.copy OrbitControls::from_framing spherical.theta spherical.phi azimuth_elevation",
    )
    .expect("migration fixture");
    fs::write(
        fixture_root.join("docs/guides/place-and-connect-objects.md"),
        "with_axial_gap(0.4) target connector's forward axis",
    )
    .expect("connect guide fixture");
    fs::write(
        fixture_root.join("docs/release-notes/v1.3.0.md"),
        "Status: ready OrbitControls::from_framing Aabb::union ScreenRect ProjectedPoint GridFloorHandles LookupError::InvalidBounds LookupError::UnsupportedCameraType FramingOptions::azimuth_elevation FramingOptions::front FramingOptions::back FramingOptions::left FramingOptions::right FramingOptions::top FramingOptions::bottom FramingOptions::three_quarter_front_left FramingOptions::three_quarter_front_right FramingOptions::three_quarter_back_left FramingOptions::three_quarter_back_right",
    )
    .expect("release notes fixture");
    fs::write(
        fixture_root.join("docs/README.md"),
        "Easy scene setup guides/easy-scene-setup.md",
    )
    .expect("docs readme fixture");
    fs::write(
        fixture_root.join("README.md"),
        "## Easy Scene Setup\ndocs/guides/easy-scene-setup.md docs/release-notes/v1.3.0.md",
    )
    .expect("readme fixture");
    fs::write(fixture_root.join("src/demo_page.rs"), demo_page_rs).expect("demo fixture");
    fs::write(
        fixture_root.join("src/controls.rs"),
        "mod camera_kit; mod url_state; pub use camera_kit::{FlyControls, FollowControls}; CameraOrbitUrlState CameraOrbitUrlStateError pub fn cinematic() {} pub fn snappy() {} pub fn presentation() {} pub fn turntable() {} pub fn focus() {} pub fn advance() {} pub const fn auto_rotate_rpm() {} pub fn auto_rotate_radians_per_second() {} pub fn zoom_limits_bounds_relative() {} pub fn with_distance_limits() {} pub const fn min_distance() {} pub const fn max_distance() {} fn clamp_distance() {}",
    )
    .expect("controls fixture");
    fs::write(fixture_root.join("src/controls/camera_kit.rs"), "pub struct FollowControls; pub fn behind_and_above() {} pub fn with_target_offset() {} pub struct FlyControls; pub fn with_yaw_pitch_degrees() {} pub fn move_local() {} pub fn look_delta() {} pub fn apply_to_scene() {}")
        .expect("camera control kit fixture");
    fs::write(
        fixture_root.join("src/controls/url_state.rs"),
        "pub struct CameraOrbitUrlState #[derive(Serialize, Deserialize)] pub enum CameraOrbitUrlStateError from_url_query to_query_string camera-orbit camera-target urlencoding::encode urlencoding::decode",
    )
    .expect("controls url state fixture");
    for (path, text) in [
        (
            "src/assets.rs",
            "mod environment_preset; mod hot_reload; mod khronos; pub use environment_preset::{EnvironmentPreset, EnvironmentPresetMetadata}; pub use hot_reload::{AssetHotReloadError, AssetHotReloadWatcher}; pub use khronos::{KhronosSample, KhronosSampleMetadata, KhronosSamples};",
        ),
        (
            "src/assets/environment_preset.rs",
            "pub enum EnvironmentPreset { NeutralStudio, Studio } pub struct EnvironmentPresetMetadata pub const ALL: &[EnvironmentPreset] = &[]; PACKAGE_SIZE_BUDGET_BYTES pub async fn load_environment_preset() {} source_sha256 source_url license environment-preset-reference-docs-image.ppm",
        ),
        (
            "src/assets/khronos.rs",
            "pub enum KhronosSample {} pub const ALL: &[KhronosSample] = &[]; PACKAGE_SIZE_BUDGET_BYTES pub fn khronos(&self) {} pub async fn water_bottle() {} pub async fn transmission_test() {} pub async fn rigged_simple() {} primary_sha256 license_reference",
        ),
        (
            "src/assets/hot_reload.rs",
            "use notify_debouncer_full::{new_debouncer, DebounceEventResult}; use notify_debouncer_full::notify::RecursiveMode; pub struct AssetHotReloadWatcher; pub enum AssetHotReloadError {} fn watch_scene_for_hot_reload() { new_debouncer; RecursiveMode::NonRecursive; } fn drain_changed_scenes() {}",
        ),
        (
            "src/viewer.rs",
            "mod animation; mod capture; mod interaction; mod load_progress; mod material_variants; pub use capture::{ViewerCaptureError, ViewerPngError}; click_callback: Option<ViewerPickCallback> hover_callback: Option<ViewerPickCallback> load_progress_events: Vec<AssetLoadProgress>",
        ),
        (
            "src/viewer/animation.rs",
            "pub fn play_clip() { self.scene.play_animation_by_name(&self.import, name); }",
        ),
        (
            "src/viewer/load_progress.rs",
            "pub async fn build_with_progress<T>() {} pub async fn render_with_progress<T>() {} pub fn build_with_progress<T>() {} pub async fn build_async_with_progress<T>() {} pub fn load_progress_events(&self) -> &[AssetLoadProgress] { &[] }",
        ),
    ] {
        fs::write(fixture_root.join(path), text).expect("fixture write");
    }
    fs::write(
        fixture_root.join("src/viewer/material_variants.rs"),
        "pub fn material_variants(&self) -> &[String] { &[] } pub fn active_material_variant(&self) -> Option<String> { None } pub fn set_active_material_variant(&mut self, name: Option<&str>) -> crate::Result<()> { self.scene.set_active_variant(&self.import, name)?; self.prepare() }",
    )
    .expect("viewer material variants fixture");
    fs::write(
        fixture_root.join("src/viewer/capture.rs"),
        "pub enum ViewerCaptureError { Capture(CaptureError) } pub enum ViewerPngError {} CPU headless renderer does not request a GPU adapter pub fn capture_png_bytes() { self.capture()? .to_png_bytes() } pub fn capture_png(path) { self.capture()? .write_png(path) } pub async fn render_png_bytes() {} pub async fn render_png() {}",
    )
    .expect("viewer capture fixture");
    fs::write(
        fixture_root.join("src/viewer/interaction.rs"),
        "pub fn on_click<T>() {} pub fn on_hover<T>() {} pub fn clear_click_callback() {} pub fn clear_hover_callback() {} pub fn click_at() { pick_and_select_at(); } pub fn hover_at() { pick_and_hover_at(); } fn pick_and_select_at() {} fn pick_and_hover_at() {}",
    )
    .expect("viewer interaction fixture");
    fs::write(
        fixture_root.join("src/demo_page/connectors.rs"),
        "project_world_point",
    )
    .expect("connector projection fixture");
    fs::write(
        fixture_root.join("src/diagnostics.rs"),
        "InvalidBounds InvalidFramingOption UnsupportedCameraType A viewport width or height was zero Bounds were empty A named framing option failed validation does not support the camera type",
    )
    .expect("diagnostics fixture");
    fs::write(
        fixture_root.join("src/scene/framing.rs"),
        "pre-existing aspect add_perspective_camera_default_for # Examples # Errors LookupError::UnsupportedCameraType LookupError::InvalidFramingOption",
    )
    .expect("framing fixture");
    fs::write(
        fixture_root.join("src/scene/lights.rs"),
        "studio docs pub fn sun() {} pub fn key_light() {} pub fn fill_light() {} pub fn rim_light() {} pub fn softbox() {} pub fn bulb_warm() {} pub fn bulb_cool() {}",
    )
    .expect("lights fixture");
    fs::write(
        fixture_root.join("src/scene/mixers.rs"),
        "pub fn play_animation_by_name() { self.create_animation_mixer(); self.play_animation(mixer); }",
    )
    .expect("mixers fixture");
    fs::write(
        fixture_root.join("src/scene/connectors/options.rs"),
        "pub fn with_axial_gap() {} pub const fn axial_gap() {}",
    )
    .expect("connector options fixture");
    fs::write(
        fixture_root.join("examples/animation.rs"),
        "play_animation_by_name(&import",
    )
    .expect("animation example fixture");
    fs::write(
        fixture_root.join("src/lib.rs"),
        "pub mod reference_image; ReferenceImage ReferenceImageError ReferenceImageTolerance regress regress_with_tolerance ViewerCaptureError ViewerPngError AssetHotReloadError AssetHotReloadWatcher AssetLoadProgress CameraOrbitUrlState CameraOrbitUrlStateError FollowControls FlyControls EnvironmentPreset EnvironmentPresetMetadata KhronosSample KhronosSampleMetadata KhronosSamples",
    )
    .expect("lib fixture");
    fs::write(
        fixture_root.join("src/reference_image.rs"),
        "pub struct ReferenceImage; pub struct ReferenceImageTolerance; pub struct ReferenceImageReport; pub enum ReferenceImageError { DiffExceeded(ReferenceImageReport) } pub fn regress() {} pub fn regress_with_tolerance() {}",
    )
    .expect("reference image fixture");
    fs::write(fixture_root.join("src/geometry.rs"), "").expect("geometry fixture");
    fs::write(fixture_root.join("src/geometry/bounds.rs"), "").expect("bounds fixture");
    fs::write(
        fixture_root.join("tests/examples_visual_proof.rs"),
        "frame_bounds_rendered_output_proves_fill_center_and_unclipped_object frame-bounds-rendered-output computed_distance projected_rect nonblack_pixel_rect round_a_named_color_swatch_docs_image round-a-named-color-swatch-docs-image round_a_lens_preset_comparison_docs_image round-a-lens-preset-comparison-docs-image round_b_light_preset_reference_docs_image round-b-light-preset-reference-docs-image round_b_material_preset_reference_docs_image round-b-material-preset-reference-docs-image round_b_background_preset_reference_docs_image round-b-background-preset-reference-docs-image round_b_orbit_control_preset_animated_docs_image round-b-orbit-control-preset-animated-docs-image round_c_auto_exposure_preset_reference_docs_image round-c-auto-exposure-preset-reference-docs-image round_c_animation_playback_reference_animated_docs_image round-c-animation-playback-reference-animated-docs-image round_d_orbit_zoom_limit_animated_docs_image round-d-orbit-zoom-limit-animated-docs-image round_d_viewer_pointer_callback_animated_docs_image round-d-viewer-pointer-callback-animated-docs-image examples_visual_picking_selection_hover_renders_pick_state_to_ppm picking_selection_hover pick_and_select_with_assets InteractionStyle::outline viewer_material_variant_reference_docs_image viewer-material-variant-reference-docs-image reference-image+docs-image animated-proof+docs-image",
    )
    .expect("visual proof fixture");
    let color_fixture = "pub const TRANSPARENT: Color = Color; pub const GRAY: Color = Color; pub const BLUE: Color = Color; pub fn from_hex(value: &str) {} pub fn from_kelvin(kelvin: f32) {}";
    fs::write(fixture_root.join("src/material.rs"), color_fixture).expect("material fixture");
    fs::write(fixture_root.join("src/material/color.rs"), color_fixture)
        .expect("material color fixture");
    fs::write(
        fixture_root.join("src/material/presets.rs"),
        "pub const fn matte(color: Color) {} pub const fn plastic(color: Color) {} pub const fn metal(color: Color) {} pub const fn rubber() {}",
    )
    .expect("material presets fixture");
    fs::write(
        fixture_root.join("src/render/background.rs"),
        "pub enum Background { Studio, DarkStudio, NeutralGray, White, Black, Sky, Transparent, Custom(Color) } impl Background { pub const fn color(self) -> Color {} }",
    )
    .expect("background fixture");
    fs::write(
        fixture_root.join("src/render/settings.rs"),
        "pub fn set_background(background: Background) { self.set_background_color(background.color()); } pub fn set_hover_style() {} pub fn set_selection_style() {}",
    )
    .expect("render settings fixture");
    fs::write(
        fixture_root.join("src/picking.rs"),
        "pub struct InteractionContext; pub struct InteractionStyle; pub const fn outline() {} pub fn set_hover() {} pub fn set_primary_selection() {}",
    )
    .expect("picking fixture");
    fs::write(
        fixture_root.join("src/scene/picking.rs"),
        "pub fn pick_and_select_with_assets() {} pub fn pick_and_hover_with_assets() {} pub fn set_hover_target() {} pub fn set_primary_selection_target() {}",
    )
    .expect("scene picking fixture");
    fs::write(
        fixture_root.join("src/render/exposure.rs"),
        "pub const fn product_studio() {} pub const fn indoor() {} pub const fn outdoor() {} pub const fn mixed() {}",
    )
    .expect("auto exposure fixture");
    fs::write(
        fixture_root.join("src/scene/camera.rs"),
        "pub fn standard() {} pub fn wide_angle() {} pub fn portrait() {} pub fn telephoto() {} pub fn with_fov_degrees(degrees: f32) {}",
    )
    .expect("camera fixture");
    fs::write(
        fixture_root.join("src/scene/math.rs"),
        "pub fn looking_at() {}",
    )
    .expect("math fixture");
    for (path, text) in [
        (
            "tests/round_a_easy_use.rs",
            "round_a_color_named_constants_and_hex_alias_are_public round_a_color_kelvin_helper_is_clamped_and_ordered round_a_perspective_camera_lens_presets_are_named_degree_surfaces round_a_transform_looking_at_faces_target_with_requested_up",
        ),
        (
            "tests/production_asset_profile.rs",
            "production_asset_profile_enables_compressed_asset_decoders_without_default_bloat",
        ),
        (
            "tests/m8_compressed_asset_release_proof.rs",
            "m8_ktx2_material_role_visual_rows_write_release_artifacts m8_meshopt_visual_rows_write_release_artifacts m8_ext_mesh_gpu_instancing_visual_row_writes_release_artifacts m8_compressed_native_gpu_lane_records_fail_closed_unavailable_artifact render_native_gpu_compressed_asset_lane SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS browser-webgpu browser-webgl2 release_evidence scena.compressed_asset_visual_proof.v1",
        ),
        (
            "tests/round_b_light_presets.rs",
            "named_directional_light_presets_are_public_and_ordered named_point_light_presets_are_kelvin_tinted_and_range_limited",
        ),
        (
            "tests/round_b_material_presets.rs",
            "honest_material_presets_are_public_pbr_shortcuts",
        ),
        (
            "tests/round_b_background_presets.rs",
            "named_background_presets_map_to_public_colors renderer_set_background_uses_named_scheme",
        ),
        (
            "tests/round_b_orbit_controls_presets.rs",
            "named_orbit_damping_presets_are_public_and_ordered turntable_presets_expose_explicit_frame_advance_semantics presentation_sets_slow_turntable_motion",
        ),
        (
            "tests/round_c_auto_exposure_presets.rs",
            "named_auto_exposure_scenarios_are_public_and_ordered scenario_presets_drive_different_ev_solutions",
        ),
        (
            "tests/round_c_animation_playback.rs",
            "scene_play_animation_by_name_creates_and_starts_mixer headless_viewer_play_clip_starts_loaded_import_animation",
        ),
        (
            "tests/round_d_connector_axial_gap.rs",
            "connect_options_axial_gap_offsets_along_target_forward_axis axial_gap_sanitizes_invalid_or_negative_values_to_zero",
        ),
        (
            "tests/round_d_orbit_zoom_limits.rs",
            "orbit_zoom_limits_are_relative_to_current_framed_distance wheel_and_pinch_zoom_are_clamped_to_named_limits",
        ),
        (
            "tests/round_d_viewer_pointer_callbacks.rs",
            "viewer_click_and_hover_callbacks_receive_hit_and_no_hit_results click_events hover_events",
        ),
        (
            "tests/round_d_viewer_capture_png.rs",
            "viewer_capture_png_bytes_decode_to_current_frame viewer_capture_png_writes_reference_artifact viewer_capture_png_uses_shared_capture_stale_frame_guard headless_viewer_builder_renders_gltf_to_png_bytes_without_gpu_setup headless_viewer_builder_renders_gltf_to_png_file_without_gpu_setup .render_png_bytes() .render_png( visible CPU-rendered pixels viewer-capture-png-reference.png",
        ),
        (
            "tests/reference_image_regression_api.rs",
            "reference_image_regression_accepts_exact_rgba8_match reference_image_regression_reports_tolerance_failure reference_image_regression_rejects_invalid_rgba_length reference_image_regression_rejects_dimension_mismatch",
        ),
        (
            "tests/first_render_api.rs",
            "headless_gltf_viewer_surfaces_asset_load_progress .build_with_progress(|event| observed.push(event)) viewer.load_progress_events() AssetLoadProgress::LoadStarted AssetLoadProgress::Parsed AssetLoadProgress::Cached headless_gltf_viewer_switches_material_variants_and_reprepares material_variants_scene.gltf viewer.material_variants() viewer.set_active_material_variant(Some(\"midnight\")) viewer.active_material_variant()",
        ),
        (
            "tests/m7_interactive_viewer.rs",
            "interactive_gltf_viewer_surfaces_asset_load_progress .build_with_progress(|event| observed.push(event)) viewer.load_progress_events() AssetLoadProgress::LoadStarted AssetLoadProgress::Parsed interactive_gltf_viewer_switches_material_variants_and_reprepares material_variants_scene.gltf viewer.material_variants() viewer.set_active_material_variant(Some(\"noon\")) viewer.active_material_variant()",
        ),
        (
            "tests/assets/gltf/material_variants_scene.gltf",
            "KHR_materials_variants \"midnight\" \"noon\"",
        ),
        (
            "tests/round_d_asset_hot_reload.rs",
            "asset_hot_reload_watcher_reports_debounced_file_change_and_reload_updates_retained_asset asset-hot-reload-animated-proof.ppm reload_scene(&first) replace_import(&import, &reloaded)",
        ),
        (
            "tests/round_c_khronos_samples.rs",
            "khronos_sample_catalog_exposes_manifest_metadata_and_package_budget khronos_sample_loader_loads_every_catalog_entry_without_user_paths khronos_sample_loader_has_named_shortcuts_for_headline_assets khronos_sample_loader_renders_rigged_sample_reference_artifact rigged-simple-sample-loader-reference.ppm",
        ),
        (
            "tests/round_c_environment_presets.rs",
            "environment_preset_catalog_exposes_metadata_and_package_budget environment_presets_load_without_user_supplied_paths environment_presets_render_reference_contact_sheet environment-preset-reference-docs-image.ppm",
        ),
        (
            "tests/round_d_viewer_url_state.rs",
            "camera_orbit_url_state_round_trips_orbit_controls camera_orbit_url_state_accepts_compact_checklist_query_shape camera_orbit_url_state_omits_asset_urls_and_secrets framing_outcome_exports_camera_orbit_url_state",
        ),
        (
            "tests/camera_control_kit.rs",
            "follow_controls_track_node_with_named_offset fly_controls_move_in_camera_local_axes_and_apply_to_scene",
        ),
    ] {
        fs::write(fixture_root.join(path), text).expect("test fixture");
    }
    fs::write(
        fixture_root.join("demo/index.html"),
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    )
    .expect("demo html fixture");
    fs::write(
        fixture_root.join("demo/main.js"),
        "setStatus('demo', 'rendered');",
    )
    .expect("demo js fixture");
    crate::app::tests_15::write_asset_validation_easy_scene_fixture(fixture_root);
    crate::app::tests_16::write_scena_viewer_element_easy_scene_fixture(fixture_root);
}

#[test]
pub(crate) fn demo_build_heartbeat_contract_is_source_enforced() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let mut findings = Vec::new();

    check_demo_build_heartbeat_contract(&root, &mut findings);

    assert_eq!(findings, Vec::new());
}

#[test]
pub(crate) fn demo_build_heartbeat_contract_rejects_direct_wasm_pack_script() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/direct-demo-wasm-pack");
    fs::create_dir_all(fixture_root.join("scripts")).expect("fixture scripts");
    fs::write(
        fixture_root.join("package.json"),
        r#"{"scripts":{"demo:build":"wasm-pack build --release --target web --out-dir demo/pkg . --features demo-page"}}"#,
    )
    .expect("package fixture");
    fs::write(
        fixture_root.join("scripts/build_demo_wasm.js"),
        "wasm-pack\n",
    )
    .expect("script fixture");
    let mut findings = Vec::new();

    check_demo_build_heartbeat_contract(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-BUILD-HEARTBEAT"),
        "doctor must reject a silent direct wasm-pack demo build script: {findings:?}",
    );
}

/// Plan line 1588 / Phase 4: doctor rule regression coverage. Each of the
/// following tests writes a fixture whose required documentation
/// substring is missing and asserts the matching DOCS-* rule fires.
/// Closes the regression gap for the documentation contracts that were
/// previously enforced only by the live tree.
#[test]
pub(crate) fn doctor_rejects_render_lifecycle_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/lifecycle-doc-stub");
    let doc_path = fixture_root.join("docs/lifecycle.md");
    fs::create_dir_all(doc_path.parent().expect("lifecycle parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Render lifecycle\n\nThis stub deliberately omits the contract substrings.\n",
    )
    .expect("lifecycle fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-LIFECYCLE"),
        "doctor must reject docs/lifecycle.md when the contract substrings \
         are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_asset_gltf_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/gltf-doc-stub");
    let doc_path = fixture_root.join("docs/assets.md");
    fs::create_dir_all(doc_path.parent().expect("gltf parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# glTF contract\n\nStub that omits the connector and stale-import contract terms.\n",
    )
    .expect("gltf fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-GLTF"),
        "doctor must reject docs/assets.md when its required \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_visual_quality_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/visual-doc-stub");
    let doc_path = fixture_root.join("docs/headless-rendering.md");
    fs::create_dir_all(doc_path.parent().expect("visual parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Visual quality\n\nStub without color management or determinism clauses.\n",
    )
    .expect("visual fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-VISUAL"),
        "doctor must reject docs/headless-rendering.md when its required \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_platform_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/platform-doc-stub");
    let doc_path = fixture_root.join("docs/platforms.md");
    fs::create_dir_all(doc_path.parent().expect("platform parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Platforms\n\nStub without the required browser backend substrings.\n",
    )
    .expect("platform fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-PLATFORM"),
        "doctor must reject docs/platforms.md when its required platform \
         substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_errors_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/errors-doc-stub");
    let doc_path = fixture_root.join("docs/errors.md");
    fs::create_dir_all(doc_path.parent().expect("errors parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Errors\n\nStub without the renderer error contract terms.\n",
    )
    .expect("errors fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| finding.rule == "DOCS-ERRORS"),
        "doctor must reject docs/errors.md when its required substrings \
         are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_public_api_doc_missing_substring_regression() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/public-api-doc-stub");
    let doc_path = fixture_root.join("docs/api.md");
    fs::create_dir_all(doc_path.parent().expect("public-api parent")).expect("fixture dir");
    fs::write(
        &doc_path,
        "# Public API\n\nStub without the public-API surface contract terms.\n",
    )
    .expect("public-api fixture");
    let mut findings = Vec::new();

    check_required_doc_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-PUBLIC-API"),
        "doctor must reject docs/api.md when the public-API surface \
         contract substrings are missing: {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_required_docs_missing_file_regression() {
    // DOCS-REQUIRED: the doctor walks `REQUIRED_DOCS` and asserts every
    // file is present + non-empty. A fixture missing one such doc
    // regresses the rule. Picking `docs/decisions/ADR-0005-local-release-candidate-deferrals.md`
    // because it is part of the canonical required set and silent
    // deletion would breach the local-release-candidate paperwork.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/required-doc-missing");
    // Empty fixture root → every required doc is missing.
    fs::create_dir_all(&fixture_root).expect("fixture dir");
    let mut findings = Vec::new();

    require_files(&fixture_root, &mut findings, "DOCS-REQUIRED", REQUIRED_DOCS);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-REQUIRED"),
        "doctor must reject the repo when REQUIRED_DOCS files are missing: \
         {findings:?}",
    );
}

#[test]
pub(crate) fn doctor_rejects_m5_release_cargo_missing_metadata_regression() {
    // ARCH-M5-RELEASE: Cargo.toml must keep the rust-version, docs.rs
    // documentation pointer, keywords, categories, include list, and
    // hybrid `["rlib", "cdylib"]` crate type. A stub manifest without
    // those entries regresses the v1 release contract.
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/m5-release-cargo-stub");
    let manifest_path = fixture_root.join("Cargo.toml");
    fs::create_dir_all(manifest_path.parent().expect("manifest parent")).expect("fixture dir");
    fs::write(
        &manifest_path,
        "[package]\nname = \"scena\"\nversion = \"0.0.0\"\n",
    )
    .expect("manifest fixture");
    let mut findings = Vec::new();

    require_contains(
        &fixture_root,
        &mut findings,
        "ARCH-M5-RELEASE",
        "Cargo.toml",
        &[
            "version = \"1.7.2\"",
            "rust-version = ",
            "documentation = \"https://docs.rs/scena\"",
            "keywords = [",
            "categories = [",
            "include = [",
            "crate-type = [\"rlib\", \"cdylib\"]",
        ],
    );

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ARCH-M5-RELEASE"),
        "doctor must reject Cargo.toml stubs that drop the v1 release-metadata \
         surface: {findings:?}",
    );
}
