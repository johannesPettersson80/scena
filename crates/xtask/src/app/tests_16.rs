use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

pub(crate) fn write_scena_viewer_element_easy_scene_fixture(fixture_root: &Path) {
    fs::write(
        fixture_root.join("Cargo.toml"),
        format!(
            "{}\nviewer-element = []",
            fs::read_to_string(fixture_root.join("Cargo.toml")).expect("manifest fixture")
        ),
    )
    .expect("manifest viewer-element fixture");
    fs::write(
        fixture_root.join("src/lib.rs"),
        format!(
            "{} pub mod viewer_element; SCENA_VIEWER_TAG ScenaViewerAccessibilityDefaults ScenaViewerAttributes ScenaViewerDropDecision ScenaViewerDropKind ScenaViewerDroppedFile ScenaViewerInspectorDiagnostic ScenaViewerInspectorSnapshot ScenaViewerKeyboardAction ScenaViewerProgress ScenaViewerProgressPhase ScenaViewerVariantOption ScenaViewerVariantSelection define_scena_viewer",
            fs::read_to_string(fixture_root.join("src/lib.rs")).expect("lib fixture")
        ),
    )
    .expect("lib viewer-element fixture");
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        r#"mod inspector; mod model; pub use inspector:: pub use model:: pub const SCENA_VIEWER_TAG defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready setLoadProgress progress.part = "progress" role", "progressbar" scena-viewer-progress scena-viewer-progress-rendered dragover drop _handleDrop _isSupportedAssetFile scena-viewer-file-drop scena-viewer-drop-error setMaterialVariants variantPicker.part = "variant-picker" scena-viewer-variant-change scena-viewer-variants-ready tabIndex = 0 aria-roledescription _handleKeydown _keyboardAction scena-viewer-key-control setInspectorSnapshot setInspectorDiagnostics clearInspectorSnapshot inspector.part = "inspector" inspector-status inspector-list scena-viewer-inspector-rendered"#,
    )
    .expect("viewer element fixture");
    fs::create_dir_all(fixture_root.join("src/viewer_element"))
        .expect("viewer element model fixture dir");
    fs::write(
        fixture_root.join("src/viewer_element/model.rs"),
        "pub struct ScenaViewerAccessibilityDefaults host_role host_label canvas_label touch_action host_is_keyboard_focusable pub enum ScenaViewerKeyboardAction from_key event_action pub struct ScenaViewerAttributes from_pairs pub enum ScenaViewerDropKind pub struct ScenaViewerDroppedFile pub struct ScenaViewerDropDecision from_file_names status_text pub enum ScenaViewerProgressPhase pub struct ScenaViewerProgress from_asset_event aria_text pub struct ScenaViewerVariantOption pub struct ScenaViewerVariantSelection with_active",
    )
    .expect("viewer element model fixture");
    fs::write(
        fixture_root.join("src/viewer_element/inspector.rs"),
        "pub struct ScenaViewerInspectorSnapshot from_renderer_state status_text warning_count error_count pub struct ScenaViewerInspectorDiagnostic DiagnosticSeverity RendererStats",
    )
    .expect("viewer element inspector fixture");
    fs::write(
        fixture_root.join("tests/scena_viewer_element.rs"),
        "scena_viewer_attributes_parse_model_viewer_style_surface scena_viewer_attributes_default_to_safe_drop_in_viewer_values scena_viewer_progress_maps_asset_events_to_accessible_details scena_viewer_drop_decision_accepts_gltf_and_reports_rejections scena_viewer_variant_selection_tracks_available_and_active_names scena_viewer_accessibility_defaults_define_mobile_and_keyboard_surface scena_viewer_inspector_snapshot_summarizes_diagnostics_and_render_state camera-controls tone-mapping",
    )
    .expect("viewer element test fixture");
    fs::write(
        fixture_root.join("docs/browser.md"),
        "<scena-viewer defineScenaViewer viewer-element shadow DOM canvas progressbar scena-viewer-progress drag-and-drop scena-viewer-file-drop scena-viewer-drop-error material variant picker scena-viewer-variant-change mobile accessibility keyboard scena-viewer-key-control inspector/dev overlay setInspectorSnapshot scena-viewer-inspector-rendered",
    )
    .expect("browser docs fixture");
    let checklist_path =
        fixture_root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md");
    let mut checklist =
        fs::read_to_string(&checklist_path).expect("next release checklist fixture");
    checklist.push_str(
        " custom-element\nfoundation **[shipped]** src/viewer_element.rs SCENA-VIEWER-ELEMENT ScenaViewerProgress scena-viewer-progress-rendered ScenaViewerDropDecision scena-viewer-file-drop ScenaViewerVariantSelection scena-viewer-variant-change ScenaViewerAccessibilityDefaults scena-viewer-key-control ScenaViewerInspectorSnapshot scena-viewer-inspector-rendered Full\n  asset loading/rendering parity remains open under bet 1.1",
    );
    fs::write(checklist_path, checklist).expect("next release checklist viewer element fixture");
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_element_foundation() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-scena-viewer-element");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let _ = fs::remove_file(fixture_root.join("src/viewer_element.rs"));
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> foundation claims without the source, docs, and tests: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_progress_ui() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-progress-ui");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        "pub const SCENA_VIEWER_TAG pub struct ScenaViewerAttributes from_pairs defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready",
    )
    .expect("viewer element fixture without progress ui");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> that drops the progressbar and structured progress events: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_drag_drop_surface() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-scena-viewer-drag-drop");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        r#"pub const SCENA_VIEWER_TAG pub struct ScenaViewerAttributes from_pairs pub enum ScenaViewerProgressPhase pub struct ScenaViewerProgress from_asset_event aria_text defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready setLoadProgress progress.part = "progress" role", "progressbar" scena-viewer-progress scena-viewer-progress-rendered"#,
    )
    .expect("viewer element fixture without drag-drop");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> that drops drag/drop validation and events: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_variant_picker() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-variant-picker");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        r#"pub const SCENA_VIEWER_TAG pub struct ScenaViewerAttributes from_pairs pub enum ScenaViewerDropKind pub struct ScenaViewerDroppedFile pub struct ScenaViewerDropDecision from_file_names status_text pub enum ScenaViewerProgressPhase pub struct ScenaViewerProgress from_asset_event aria_text defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready setLoadProgress progress.part = "progress" role", "progressbar" scena-viewer-progress scena-viewer-progress-rendered dragover drop _handleDrop _isSupportedAssetFile scena-viewer-file-drop scena-viewer-drop-error"#,
    )
    .expect("viewer element fixture without variant picker");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> that drops material variant picker and events: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_a11y_defaults() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/missing-scena-viewer-a11y");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        r#"mod model; pub use model:: pub const SCENA_VIEWER_TAG defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready setLoadProgress progress.part = "progress" role", "progressbar" scena-viewer-progress scena-viewer-progress-rendered dragover drop _handleDrop _isSupportedAssetFile scena-viewer-file-drop scena-viewer-drop-error setMaterialVariants variantPicker.part = "variant-picker" scena-viewer-variant-change scena-viewer-variants-ready"#,
    )
    .expect("viewer element fixture without a11y defaults");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> that drops mobile/a11y defaults and keyboard events: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_inspector_overlay() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-inspector-overlay");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        r#"mod model; pub use model:: pub const SCENA_VIEWER_TAG defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready setLoadProgress progress.part = "progress" role", "progressbar" scena-viewer-progress scena-viewer-progress-rendered dragover drop _handleDrop _isSupportedAssetFile scena-viewer-file-drop scena-viewer-drop-error setMaterialVariants variantPicker.part = "variant-picker" scena-viewer-variant-change scena-viewer-variants-ready tabIndex = 0 aria-roledescription _handleKeydown _keyboardAction scena-viewer-key-control"#,
    )
    .expect("viewer element fixture without inspector overlay");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> that drops the inspector/dev overlay surface: {findings:?}",
    );
}
