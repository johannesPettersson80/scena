use crate::app::prelude::*;

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_prose_only_guide_snippets() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-prose-only-guide");
    write_easy_scene_fixture(
        &fixture_root,
        "Scene::new() scene.add_studio_lighting() scene.add_grid_floor( scene.frame_bounds(\n```rust\nlet scene = Scene::new();\n```",
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DOCS-EASY-SCENE-SETUP"),
        "doctor must reject guide snippets when required calls are only prose substrings: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_demo_orbit_literal_residue() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-orbit-literals");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor FramingOptions::new().orbit(-0.48, 0.31)",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-CAMERA-VIEWS-NAMED"),
        "doctor must reject inline demo orbit literal residue instead of only dead names: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_reordered_open_diagnostics() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/easy-scene-reordered-open");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details class="diagnostics" open id="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "DEMO-DIAGNOSTICS"),
        "doctor must reject open diagnostics regardless of attribute order: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_round_a_raw_camera_aspect_in_first_path() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/round-a-raw-camera-aspect");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::create_dir_all(fixture_root.join("examples")).expect("examples fixture dir");
    fs::write(
        fixture_root.join("examples/easy_model_viewer.rs"),
        "PerspectiveCamera::default().with_aspect(width as f32 / height as f32)",
    )
    .expect("round-a raw camera fixture");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ROUND-A-EASY-USE-PRIMITIVES"),
        "doctor must reject raw with_aspect camera construction in first-path examples: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_round_a_raw_color_literals_in_first_path() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/round-a-raw-color-literal");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("README.md"),
        "## Easy Scene Setup docs/guides/easy-scene-setup.md docs/release-notes/v1.3.0.md Color::from_srgb_u8(80, 160, 255)",
    )
    .expect("round-a raw color fixture");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ROUND-A-EASY-USE-PRIMITIVES"),
        "doctor must reject raw color literals in first-path docs: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_demo_raw_color_literals() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/demo-raw-color-literal");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor Color::from_linear_rgba(0.0, 0.0, 0.0, 0.0)",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ROUND-A-EASY-USE-PRIMITIVES"),
        "doctor must reject raw color literals in src/demo_page* first-path code: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_first_path_camera_fov_literals() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/raw-camera-fov-literal");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("examples/easy_model_viewer.rs"),
        "PerspectiveCamera::standard().with_fov_degrees(42.0)",
    )
    .expect("raw fov fixture");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ROUND-A-EASY-USE-PRIMITIVES"),
        "doctor must reject raw camera FOV literals in first-path examples: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_example_quat_literals() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/example-quat-literal");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("examples/easy_model_viewer.rs"),
        "Transform::default().with_rotation(Quat::from_xyzw(0.0, 0.0, 0.0, 1.0))",
    )
    .expect("quat literal fixture");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "ROUND-A-EASY-USE-PRIMITIVES"),
        "doctor must reject raw quaternion literals in examples except the transform escape hatch: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_production_asset_profile() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-production-assets");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("Cargo.toml"),
        "default = []\nktx2 = [\"dep:ktx2\", \"dep:basisu_c_sys\"]\nmeshopt = [\"dep:meshopt\"]",
    )
    .expect("manifest fixture");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "PRODUCTION-ASSET-PROFILE"),
        "doctor must reject manifests without the named production-assets feature: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_viewer_load_progress_surface() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-viewer-load-progress");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer.rs"),
        "mod capture; mod interaction; pub use capture::{ViewerCaptureError, ViewerPngError}; click_callback: Option<ViewerPickCallback> hover_callback: Option<ViewerPickCallback>",
    )
    .expect("viewer fixture without load progress");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VIEWER-LOAD-PROGRESS"),
        "doctor must reject viewer surfaces that do not expose asset load progress: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_headless_png_one_liner() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-headless-png");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer/capture.rs"),
        "pub enum ViewerCaptureError {} pub fn capture_png_bytes() { png::Encoder::new(); png::ColorType::Rgba; png::BitDepth::Eight; } pub fn capture_png() {}",
    )
    .expect("viewer capture fixture without headless PNG one-liner");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "VIEWER-HEADLESS-PNG"),
        "doctor must reject capture surfaces that do not expose the headless glTF-to-PNG one-liner: {findings:?}",
    );
}

pub(crate) const VALID_GUIDE: &str = "frame_bounds add_perspective_camera_default_for add_studio_lighting add_grid_floor set_auto_exposure scene.mate project_world_point Camera views azimuth_elevation three_quarter_front_right AutoExposureConfig::product_studio() AutoExposureConfig::indoor() AutoExposureConfig::outdoor() AutoExposureConfig::mixed() play_animation_by_name(&import zoom_limits_bounds_relative(0.5, 4.0) FollowControls::behind_and_above FlyControls::new move_local with_yaw_pitch_degrees viewer.on_click( viewer.on_hover( viewer.click_at( viewer.hover_at( InteractionStyle::outline renderer.set_hover_style renderer.set_selection_style viewer.capture_png(\"frame.png\")? viewer.capture_png_bytes()? render_png_bytes() CPU headless renderer without requesting a GPU adapter Reference-image regression ReferenceImage::from_rgba8 regress_with_tolerance ReferenceImageTolerance::new().with_max_abs_diff AssetLoadProgress build_with_progress load_progress_events Material variants viewer.material_variants() viewer.set_active_material_variant(Some(\"blue\"))? viewer.set_active_material_variant(None)? watch_scene_for_hot_reload drain_changed_scenes reload_scene(&scene_asset) replace_import(&import, &reloaded) controls.url_state().to_query_string() CameraOrbitUrlState::from_url_query controls.with_url_state(state) framing.url_state().to_query_string() EnvironmentPreset::Studio load_environment_preset EnvironmentPreset::ALL KTX2 cubemap presets are still future work khronos-samples assets.khronos().water_bottle().await? KhronosSample::ALL\n```rust\nlet mut scene = Scene::new();\nscene.add_studio_lighting()?;\nscene.add_grid_floor(&assets, GridFloorOptions::new())?;\nscene.add_perspective_camera_default_for(bounds, (width, height))?;\n```";

pub(crate) fn write_easy_scene_fixture(
    fixture_root: &Path,
    guide: &str,
    demo_rs: &str,
    diagnostics_html: &str,
) {
    let _ = fs::remove_dir_all(fixture_root);
    for dir in [
        "demo",
        "examples",
        "docs/guides",
        "docs/release-notes",
        "docs/checklists",
        "src",
        "src/assets",
        "src/viewer",
        "src/material",
        "src/render",
        "src/demo_page",
        "src/scene",
        "src/scene/connectors",
        "tests",
        "tests/assets/gltf",
    ] {
        fs::create_dir_all(fixture_root.join(dir)).expect("fixture dir");
    }
    fs::write(fixture_root.join("docs/guides/easy-scene-setup.md"), guide).expect("guide fixture");
    fs::write(
        fixture_root.join("Cargo.toml"),
        "notify-debouncer-full = { version = \"0.7.0\", optional = true }\nhot-reload = [\"dep:notify-debouncer-full\"]\nkhronos-samples = []\ndefault = []\nktx2 = [\"dep:ktx2\", \"dep:basisu_c_sys\"]\nmeshopt = [\"dep:meshopt\"]\nproduction-assets = [\"ktx2\", \"meshopt\"]\nserde = { version = \"1\", features = [\"derive\"] }\nurlencoding = \"2\"",
    )
    .expect("manifest fixture");
    fs::write(
        fixture_root.join("docs/feature-flags.md"),
        "khronos-samples Khronos glTF sample-asset catalog `production-assets` enables `ktx2` + `meshopt` features = [\"production-assets\"]",
    )
    .expect("feature flags fixture");
    fs::write(
        fixture_root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md"),
        "Production-grade asset pipeline complete and production-profile ready Status: **[shipped]** for the production profile tests/m8_compressed_asset_release_proof.rs target/gate-artifacts/m8-compressed-assets CPU rasterizer fallback for no-GPU screenshots Status: **[shipped]** render_png_bytes() Reference-image regression as a public API Status:\n  **[shipped]** ReferenceImage::from_rgba8 REFERENCE-IMAGE-REGRESSION Follow/Fly library primitives **[shipped]** tests/camera_control_kit.rs CAMERA-CONTROL-KIT Picking + outline + hover Status: **[shipped]** PICKING-OUTLINE-HOVER examples_visual_picking_selection_hover_renders_styled_pick_to_ppm",
    )
    .expect("next release checklist fixture");
    crate::app::tests_15::write_asset_validation_easy_scene_fixture(fixture_root);
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
        "## Easy Scene Setup docs/guides/easy-scene-setup.md docs/release-notes/v1.3.0.md",
    )
    .expect("readme fixture");
    fs::write(fixture_root.join("src/demo_page.rs"), demo_rs).expect("demo fixture");
    fs::write(
        fixture_root.join("src/controls.rs"),
        "mod camera_kit; mod url_state; pub use camera_kit::{FlyControls, FollowControls}; CameraOrbitUrlState CameraOrbitUrlStateError pub fn cinematic() {} pub fn snappy() {} pub fn presentation() {} pub fn turntable() {} pub fn advance() {} pub const fn auto_rotate_rpm() {} pub fn auto_rotate_radians_per_second() {} pub fn zoom_limits_bounds_relative() {} pub fn with_distance_limits() {} pub const fn min_distance() {} pub const fn max_distance() {} fn clamp_distance() {}",
    )
    .expect("controls fixture");
    fs::create_dir_all(fixture_root.join("src/controls")).expect("controls module dir");
    fs::write(
        fixture_root.join("src/controls/camera_kit.rs"),
        "pub struct FollowControls; pub fn behind_and_above() {} pub fn with_target_offset() {} pub struct FlyControls; pub fn with_yaw_pitch_degrees() {} pub fn move_local() {} pub fn look_delta() {} pub fn apply_to_scene() {}",
    )
    .expect("camera control kit fixture");
    fs::write(
        fixture_root.join("src/controls/url_state.rs"),
        "pub struct CameraOrbitUrlState #[derive(Serialize, Deserialize)] pub enum CameraOrbitUrlStateError from_url_query to_query_string camera-orbit camera-target urlencoding::encode urlencoding::decode",
    )
    .expect("controls url state fixture");
    fs::write(
        fixture_root.join("src/assets.rs"),
        "mod environment_preset; mod hot_reload; mod khronos; pub use environment_preset::{EnvironmentPreset, EnvironmentPresetMetadata}; pub use hot_reload::{AssetHotReloadError, AssetHotReloadWatcher}; pub use khronos::{KhronosSample, KhronosSampleMetadata, KhronosSamples};",
    )
    .expect("assets fixture");
    fs::write(
        fixture_root.join("src/assets/environment_preset.rs"),
        "pub enum EnvironmentPreset { NeutralStudio, Studio } pub struct EnvironmentPresetMetadata pub const ALL: &[EnvironmentPreset] = &[]; PACKAGE_SIZE_BUDGET_BYTES pub async fn load_environment_preset() {} source_sha256 source_url license environment-preset-reference-docs-image.ppm",
    )
    .expect("environment preset fixture");
    fs::write(
        fixture_root.join("src/assets/khronos.rs"),
        "pub enum KhronosSample {} pub const ALL: &[KhronosSample] = &[]; PACKAGE_SIZE_BUDGET_BYTES pub fn khronos(&self) {} pub async fn water_bottle() {} pub async fn transmission_test() {} pub async fn rigged_simple() {} primary_sha256 license_reference",
    )
    .expect("khronos samples fixture");
    fs::write(
        fixture_root.join("src/assets/hot_reload.rs"),
        "use notify_debouncer_full::{new_debouncer, DebounceEventResult}; use notify_debouncer_full::notify::RecursiveMode; pub struct AssetHotReloadWatcher; pub enum AssetHotReloadError {} fn watch_scene_for_hot_reload() { new_debouncer; RecursiveMode::NonRecursive; } fn drain_changed_scenes() {}",
    )
    .expect("asset hot reload fixture");
    fs::write(
        fixture_root.join("src/viewer.rs"),
        "mod capture; mod interaction; mod load_progress; mod material_variants; pub use capture::{ViewerCaptureError, ViewerPngError}; click_callback: Option<ViewerPickCallback> hover_callback: Option<ViewerPickCallback> load_progress_events: Vec<AssetLoadProgress>",
    )
    .expect("viewer fixture");
    fs::write(
        fixture_root.join("src/viewer/load_progress.rs"),
        "pub async fn build_with_progress<T>() {} pub async fn render_with_progress<T>() {} pub fn build_with_progress<T>() {} pub async fn build_async_with_progress<T>() {} pub fn load_progress_events(&self) -> &[AssetLoadProgress] { &[] }",
    )
    .expect("viewer progress fixture");
    fs::write(
        fixture_root.join("src/viewer/material_variants.rs"),
        "pub fn material_variants(&self) -> &[String] { &[] } pub fn active_material_variant(&self) -> Option<String> { None } pub fn set_active_material_variant(&mut self, name: Option<&str>) -> crate::Result<()> { self.scene.set_active_variant(&self.import, name)?; self.prepare() }",
    )
    .expect("viewer material variants fixture");
    fs::write(
        fixture_root.join("src/viewer/capture.rs"),
        "pub enum ViewerCaptureError {} pub enum ViewerPngError {} CPU headless renderer does not request a GPU adapter pub fn capture_png_bytes() { png::Encoder::new(); png::ColorType::Rgba; png::BitDepth::Eight; } pub fn capture_png() {} pub async fn render_png_bytes() {} pub async fn render_png() {}",
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
        fixture_root.join("tests/examples_visual_proof.rs"),
        "frame_bounds_rendered_output_proves_fill_center_and_unclipped_object frame-bounds-rendered-output computed_distance projected_rect nonblack_pixel_rect round_a_named_color_swatch_docs_image round-a-named-color-swatch-docs-image round_a_lens_preset_comparison_docs_image round-a-lens-preset-comparison-docs-image round_b_light_preset_reference_docs_image round-b-light-preset-reference-docs-image round_b_material_preset_reference_docs_image round-b-material-preset-reference-docs-image round_b_background_preset_reference_docs_image round-b-background-preset-reference-docs-image round_b_orbit_control_preset_animated_docs_image round-b-orbit-control-preset-animated-docs-image round_c_auto_exposure_preset_reference_docs_image round-c-auto-exposure-preset-reference-docs-image round_c_animation_playback_reference_animated_docs_image round-c-animation-playback-reference-animated-docs-image round_d_orbit_zoom_limit_animated_docs_image round-d-orbit-zoom-limit-animated-docs-image round_d_viewer_pointer_callback_animated_docs_image round-d-viewer-pointer-callback-animated-docs-image examples_visual_picking_selection_hover_renders_styled_pick_to_ppm picking_selection_hover InteractionStyle::outline viewer_material_variant_reference_docs_image viewer-material-variant-reference-docs-image reference-image+docs-image animated-proof+docs-image",
    )
    .expect("visual proof fixture");
    fs::write(
        fixture_root.join("src/material.rs"),
        "pub const TRANSPARENT: Color = Color; pub const GRAY: Color = Color; pub const BLUE: Color = Color; pub fn from_hex(value: &str) {} pub fn from_kelvin(kelvin: f32) {}",
    )
    .expect("material fixture");
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
        "pub struct InteractionStyle; pub const fn outline() {} pub fn set_hover() {} pub fn set_primary_selection() {}",
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
    fs::write(
        fixture_root.join("tests/round_a_easy_use.rs"),
        "round_a_color_named_constants_and_hex_alias_are_public round_a_color_kelvin_helper_is_clamped_and_ordered round_a_perspective_camera_lens_presets_are_named_degree_surfaces round_a_transform_looking_at_faces_target_with_requested_up",
    )
    .expect("round-a test fixture");
    fs::write(
        fixture_root.join("tests/production_asset_profile.rs"),
        "production_asset_profile_enables_compressed_asset_decoders_without_default_bloat",
    )
    .expect("production asset profile test fixture");
    fs::write(
        fixture_root.join("tests/m8_compressed_asset_release_proof.rs"),
        "m8_ktx2_material_role_visual_rows_write_release_artifacts m8_meshopt_visual_rows_write_release_artifacts m8_ext_mesh_gpu_instancing_visual_row_writes_release_artifacts m8_compressed_native_gpu_lane_records_fail_closed_unavailable_artifact scena.compressed_asset_visual_proof.v1",
    )
    .expect("compressed asset proof test fixture");
    fs::write(
        fixture_root.join("tests/round_b_light_presets.rs"),
        "named_directional_light_presets_are_public_and_ordered named_point_light_presets_are_kelvin_tinted_and_range_limited",
    )
    .expect("light preset test fixture");
    fs::write(
        fixture_root.join("tests/round_b_material_presets.rs"),
        "honest_material_presets_are_public_pbr_shortcuts",
    )
    .expect("material preset test fixture");
    fs::write(
        fixture_root.join("tests/round_b_background_presets.rs"),
        "named_background_presets_map_to_public_colors renderer_set_background_uses_named_scheme",
    )
    .expect("background preset test fixture");
    fs::write(
        fixture_root.join("tests/round_b_orbit_controls_presets.rs"),
        "named_orbit_damping_presets_are_public_and_ordered turntable_presets_expose_explicit_frame_advance_semantics presentation_combines_medium_damping_with_slow_turntable_motion",
    )
    .expect("orbit preset test fixture");
    fs::write(
        fixture_root.join("tests/round_c_auto_exposure_presets.rs"),
        "named_auto_exposure_scenarios_are_public_and_ordered scenario_presets_drive_different_ev_solutions",
    )
    .expect("auto exposure preset test fixture");
    fs::write(
        fixture_root.join("tests/round_c_animation_playback.rs"),
        "scene_play_animation_by_name_creates_and_starts_mixer",
    )
    .expect("animation playback test fixture");
    fs::write(
        fixture_root.join("tests/round_d_connector_axial_gap.rs"),
        "connect_options_axial_gap_offsets_along_target_forward_axis axial_gap_sanitizes_invalid_or_negative_values_to_zero",
    )
    .expect("axial gap test fixture");
    fs::write(
        fixture_root.join("tests/round_d_orbit_zoom_limits.rs"),
        "orbit_zoom_limits_are_relative_to_current_framed_distance wheel_and_pinch_zoom_are_clamped_to_named_limits",
    )
    .expect("orbit zoom-limit test fixture");
    fs::write(
        fixture_root.join("tests/round_d_viewer_pointer_callbacks.rs"),
        "viewer_click_and_hover_callbacks_receive_hit_and_no_hit_results click_events hover_events",
    )
    .expect("viewer pointer callback test fixture");
    fs::write(
        fixture_root.join("tests/round_d_viewer_capture_png.rs"),
        "viewer_capture_png_bytes_decode_to_current_frame viewer_capture_png_writes_reference_artifact headless_viewer_builder_renders_gltf_to_png_bytes_without_gpu_setup headless_viewer_builder_renders_gltf_to_png_file_without_gpu_setup .render_png_bytes() .render_png( visible CPU-rendered pixels viewer-capture-png-reference.png",
    )
    .expect("viewer PNG capture test fixture");
    fs::write(
        fixture_root.join("tests/reference_image_regression_api.rs"),
        "reference_image_regression_accepts_exact_rgba8_match reference_image_regression_reports_tolerance_failure reference_image_regression_rejects_invalid_rgba_length reference_image_regression_rejects_dimension_mismatch",
    )
    .expect("reference image regression test fixture");
    fs::write(
        fixture_root.join("tests/first_render_api.rs"),
        "headless_gltf_viewer_surfaces_asset_load_progress .build_with_progress(|event| observed.push(event)) viewer.load_progress_events() AssetLoadProgress::LoadStarted AssetLoadProgress::Parsed AssetLoadProgress::Cached headless_gltf_viewer_switches_material_variants_and_reprepares material_variants_scene.gltf viewer.material_variants() viewer.set_active_material_variant(Some(\"midnight\")) viewer.active_material_variant()",
    )
    .expect("viewer progress headless test fixture");
    fs::write(
        fixture_root.join("tests/m7_interactive_viewer.rs"),
        "interactive_gltf_viewer_surfaces_asset_load_progress .build_with_progress(|event| observed.push(event)) viewer.load_progress_events() AssetLoadProgress::LoadStarted AssetLoadProgress::Parsed interactive_gltf_viewer_switches_material_variants_and_reprepares material_variants_scene.gltf viewer.material_variants() viewer.set_active_material_variant(Some(\"noon\")) viewer.active_material_variant()",
    )
    .expect("viewer progress interactive test fixture");
    fs::write(
        fixture_root.join("tests/assets/gltf/material_variants_scene.gltf"),
        "KHR_materials_variants \"midnight\" \"noon\"",
    )
    .expect("material variants asset fixture");
    fs::write(
        fixture_root.join("tests/round_d_asset_hot_reload.rs"),
        "asset_hot_reload_watcher_reports_debounced_file_change_and_reload_updates_retained_asset asset-hot-reload-animated-proof.ppm reload_scene(&first) replace_import(&import, &reloaded)",
    )
    .expect("asset hot reload test fixture");
    fs::write(
        fixture_root.join("tests/round_c_khronos_samples.rs"),
        "khronos_sample_catalog_exposes_manifest_metadata_and_package_budget khronos_sample_loader_loads_every_catalog_entry_without_user_paths khronos_sample_loader_has_named_shortcuts_for_headline_assets khronos_sample_loader_renders_rigged_sample_reference_artifact rigged-simple-sample-loader-reference.ppm",
    )
    .expect("khronos sample test fixture");
    fs::write(
        fixture_root.join("tests/round_c_environment_presets.rs"),
        "environment_preset_catalog_exposes_metadata_and_package_budget environment_presets_load_without_user_supplied_paths environment_presets_render_reference_contact_sheet environment-preset-reference-docs-image.ppm",
    )
    .expect("environment preset test fixture");
    fs::write(
        fixture_root.join("tests/round_d_viewer_url_state.rs"),
        "camera_orbit_url_state_round_trips_orbit_controls camera_orbit_url_state_accepts_compact_checklist_query_shape camera_orbit_url_state_omits_asset_urls_and_secrets framing_outcome_exports_camera_orbit_url_state",
    )
    .expect("url state test fixture");
    fs::write(
        fixture_root.join("tests/camera_control_kit.rs"),
        "follow_controls_track_node_with_named_offset fly_controls_move_in_camera_local_axes_and_apply_to_scene",
    )
    .expect("camera control kit test fixture");
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
    fs::write(fixture_root.join("demo/index.html"), diagnostics_html).expect("demo html fixture");
    fs::write(
        fixture_root.join("demo/main.js"),
        "setStatus('demo', 'rendered');",
    )
    .expect("demo js fixture");
    crate::app::tests_16::write_scena_viewer_element_easy_scene_fixture(fixture_root);
}
