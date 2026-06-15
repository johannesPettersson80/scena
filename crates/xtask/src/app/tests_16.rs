use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

pub(crate) fn write_scena_viewer_element_easy_scene_fixture(fixture_root: &Path) {
    fs::write(
        fixture_root.join("Cargo.toml"),
        format!(
            "{}\nviewer-element = []\nbrowser-probe = [\"viewer-element\"]",
            fs::read_to_string(fixture_root.join("Cargo.toml")).expect("manifest fixture")
        ),
    )
    .expect("manifest viewer-element fixture");
    fs::write(
        fixture_root.join("package.json"),
        r#"{"devDependencies":{"@google/model-viewer":"^4.2.0"}}"#,
    )
    .expect("package fixture");
    fs::write(
        fixture_root.join("src/lib.rs"),
        format!(
            "{} pub mod viewer_element; SCENA_VIEWER_TAG ScenaViewerAccessibilityDefaults ScenaViewerAnnotationAnchor ScenaViewerAnnotationError ScenaViewerAttributes ScenaViewerDropDecision ScenaViewerDropKind ScenaViewerDroppedFile ScenaViewerGestureAction ScenaViewerInspectorDiagnostic ScenaViewerInspectorSnapshot ScenaViewerKeyboardAction ScenaViewerProgress ScenaViewerProgressPhase ScenaViewerVariantOption ScenaViewerVariantSelection define_scena_viewer",
            fs::read_to_string(fixture_root.join("src/lib.rs")).expect("lib fixture")
        ),
    )
    .expect("lib viewer-element fixture");
    fs::write(
        fixture_root.join("src/viewer_element.rs"),
        r#"mod annotation_layout; mod annotations; mod inspector; mod model; pub use annotation_layout:: pub use annotations:: pub use inspector:: pub use model:: pub const SCENA_VIEWER_TAG defineScenaViewerElement /src/viewer_element/element.js"#,
    )
    .expect("viewer element fixture");
    fs::create_dir_all(fixture_root.join("src/viewer_element"))
        .expect("viewer element model fixture dir");
    fs::write(
        fixture_root.join("src/viewer_element/element.js"),
        r#"defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready setLoadProgress progress.part = "progress" role", "progressbar" scena-viewer-progress scena-viewer-progress-rendered dragover drop _handleDrop _isSupportedAssetFile scena-viewer-file-drop scena-viewer-drop-error setMaterialVariants variantPicker.part = "variant-picker" scena-viewer-variant-change scena-viewer-variants-ready tabIndex = 0 aria-roledescription _handleKeydown _keyboardAction scena-viewer-key-control pointerdown _handlePointerDown pointermove _handlePointerMove _handleWheel scena-viewer-gesture-control pinch-zoom setInspectorSnapshot setInspectorDiagnostics clearInspectorSnapshot inspector.part = "inspector" inspector-status inspector-list scena-viewer-inspector-rendered annotationSlot.name = "annotation" annotationAnchors requestAnnotationProjections setAnnotationProjections data-position data-normal data-surface scena-viewer-annotations-request scena-viewer-annotations-rendered"#,
    )
    .expect("viewer element JS fixture");
    fs::write(
        fixture_root.join("src/viewer_element/model.rs"),
        "pub struct ScenaViewerAccessibilityDefaults host_role host_label canvas_label touch_action host_is_keyboard_focusable pub enum ScenaViewerKeyboardAction from_key event_action pub enum ScenaViewerGestureAction pub struct ScenaViewerAttributes from_pairs pub enum ScenaViewerDropKind pub struct ScenaViewerDroppedFile pub struct ScenaViewerDropDecision from_file_names status_text pub enum ScenaViewerProgressPhase pub struct ScenaViewerProgress from_asset_event aria_text pub struct ScenaViewerVariantOption pub struct ScenaViewerVariantSelection with_active",
    )
    .expect("viewer element model fixture");
    fs::write(
        fixture_root.join("src/viewer_element/inspector.rs"),
        "pub struct ScenaViewerInspectorSnapshot from_renderer_state status_text warning_count error_count pub struct ScenaViewerInspectorDiagnostic DiagnosticSeverity RendererStats",
    )
    .expect("viewer element inspector fixture");
    fs::write(
        fixture_root.join("src/viewer_element/annotations.rs"),
        "pub struct ScenaViewerAnnotationAnchor pub enum ScenaViewerAnnotationError from_attributes data-position data-normal data-surface MissingPosition InvalidVector",
    )
    .expect("viewer element annotations fixture");
    fs::write(
        fixture_root.join("tests/scena_viewer_element.rs"),
        "scena_viewer_attributes_parse_model_viewer_style_surface scena_viewer_attributes_default_to_safe_drop_in_viewer_values scena_viewer_progress_maps_asset_events_to_accessible_details scena_viewer_drop_decision_accepts_gltf_and_reports_rejections scena_viewer_variant_selection_tracks_available_and_active_names scena_viewer_accessibility_defaults_define_mobile_and_keyboard_surface ScenaViewerGestureAction scena_viewer_inspector_snapshot_summarizes_diagnostics_and_render_state scena_viewer_annotation_anchor_parses_dataset_position_normal_and_surface camera-controls tone-mapping",
    )
    .expect("viewer element test fixture");
    fs::create_dir_all(fixture_root.join("tests/browser")).expect("viewer browser fixture dir");
    fs::create_dir_all(fixture_root.join("tests/assets/viewer")).expect("viewer fixture asset dir");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "assertScenaViewerElementProof runScenaViewerElementProof scena-viewer-element-browser-proof.png assertScenaViewerParityProof runScenaViewerParityProof scena-viewer-model-viewer-parity-browser-proof.png assertScenaViewerMobileA11yProof runScenaViewerMobileA11yProof scena-viewer-mobile-a11y-browser-proof.png assertCameraControlKitProof runCameraControlKitProof camera-control-kit-browser-proof.png @google/model-viewer model-viewer.min.js SCENA_BROWSER_VIEWER_ELEMENT_ONLY scena.scena_viewer_element_browser_proof.v1 scena.scena_viewer_model_viewer_parity_proof.v1 scena.scena_viewer_mobile_a11y_browser_proof.v1 scena.m6.camera_control_kit_browser_proof.v1 three_asset_side_by_side side-by-side-screenshot progress_sequence drop_render_status drop_render_auto_frame_status viewer-level-auto-framing scena-viewer-drop-render variant_render_status scena-viewer-material-variant-render annotation_tracking_sequence annotation_update_visible inspector_fixture_schema scena.scena_viewer_inspector_snapshot.v1",
    )
    .expect("viewer element browser runner fixture");
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe_page.js"),
        "defineScenaViewer m6CameraControlKitProbe m6RenderDroppedFileProbe m6RenderMaterialVariantProbe scenaViewerElementProbe scenaViewerModelViewerParityProbe scenaViewerMobileA11yProbe scenaCameraControlKitProbe SCENA_VIEWER_PARITY_ASSETS scena.scena_viewer_element_browser_proof.v1 scena.scena_viewer_model_viewer_parity_proof.v1 scena.scena_viewer_mobile_a11y_browser_proof.v1 scena.m6.camera_control_kit_browser_proof.v1 /model-viewer/model-viewer.min.js model-viewer source-gltf-materials AnimatedMorphCube.gltf WaterBottle.gltf three_asset_side_by_side side-by-side-screenshot model_viewer_loaded scena_pixels_nonblack loadInspectorSnapshot /fixtures/viewer/inspector_snapshot.json scena.scena_viewer_inspector_snapshot.v1 scena-viewer-progress-rendered progress_sequence scena-viewer-file-drop scena-viewer-drop-error renderDroppedFileIntoViewer drop_render_pixels_nonblack drop_render_auto_frame_status viewer-level-auto-framing renderSelectedVariantIntoViewer variant_render_green_dominant scena-viewer-material-variant-render scena-viewer-variant-change scena-viewer-annotations-rendered annotation_tracking_sequence annotation_update_visible scena-viewer-inspector-rendered scena-viewer-key-control scena-viewer-gesture-control pinch-zoom",
    )
    .expect("viewer element browser page fixture");
    fs::write(
        fixture_root.join("tests/assets/viewer/inspector_snapshot.json"),
        r#"{"schema":"scena.scena_viewer_inspector_snapshot.v1","source":"scena-viewer-inspector-fixture","overlay": "Diagnostics","diagnostics":[{"code":"FrameBounds"}],"stats":{"drawCalls":2}}"#,
    )
    .expect("viewer inspector JSON fixture");
    fs::write(
        fixture_root.join("src/browser_probe.rs"),
        "m6CameraControlKitProbe scena.m6.camera_control_kit_browser_proof.v1 FollowControls::behind_and_above FlyControls::new PointerEvent::primary_pressed PointerEvent::wheel",
    )
    .expect("camera control browser proof fixture");
    fs::write(
        fixture_root.join("docs/browser.md"),
        "<scena-viewer defineScenaViewer viewer-element shadow DOM canvas progressbar scena-viewer-progress drag-and-drop scena-viewer-file-drop scena-viewer-drop-error render-after-drop scena-viewer-drop-render viewer-level-auto-framing material variant picker scena-viewer-variant-change picker-to-rendered-variant scena-viewer-material-variant-render mobile accessibility keyboard scena-viewer-key-control scena-viewer-gesture-control pinch-zoom inspector/dev overlay setInspectorSnapshot scena-viewer-inspector-rendered annotation overlay data-position scena-viewer-annotations-request scena-viewer-annotations-rendered SCENA_BROWSER_VIEWER_ELEMENT_ONLY=1 scena-viewer-element-browser-proof.png scena-viewer-model-viewer-parity-browser-proof.png @google/model-viewer three-asset side-by-side",
    )
    .expect("browser docs fixture");
    let checklist_path =
        fixture_root.join("docs/checklists/next-release-easy-use-and-state-of-the-art.md");
    let mut checklist =
        fs::read_to_string(&checklist_path).expect("next release checklist fixture");
    checklist.push_str(
        " custom-element foundation, browser UI proof src/viewer_element.rs SCENA-VIEWER-ELEMENT ScenaViewerProgress scena-viewer-progress-rendered loading progress sequence proof **[shipped]** ScenaViewerDropDecision scena-viewer-file-drop render-after-drop proof **[shipped]** scena-viewer-drop-render viewer-level auto-framing browser proof **[shipped]** viewer-level-auto-framing ScenaViewerVariantSelection scena-viewer-variant-change picker-to-rendered-variant proof **[shipped]** scena-viewer-material-variant-render ScenaViewerAccessibilityDefaults scena-viewer-key-control ScenaViewerGestureAction scena-viewer-gesture-control mobile/a11y gesture proof **[shipped]** ScenaViewerInspectorSnapshot scena-viewer-inspector-rendered scena.scena_viewer_inspector_snapshot.v1 ScenaViewerAnnotationAnchor scena-viewer-annotations-rendered annotation tracking proof **[shipped]** scena.scena_viewer_element_browser_proof.v1 scena-viewer-element-browser-proof.png scena.scena_viewer_model_viewer_parity_proof.v1 scena-viewer-model-viewer-parity-browser-proof.png Three-asset side-by-side `<model-viewer>` parity proof **[shipped]** camera control browser proof **[shipped]** camera-control-kit-browser-proof.png scena.m6.camera_control_kit_browser_proof.v1",
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
        fixture_root.join("src/viewer_element/element.js"),
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
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_progress_sequence_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-progress-sequence");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe_page.js"),
        "defineScenaViewer m6CameraControlKitProbe scenaViewerElementProbe scenaCameraControlKitProbe scena.scena_viewer_element_browser_proof.v1 scena.m6.camera_control_kit_browser_proof.v1 loadInspectorSnapshot /fixtures/viewer/inspector_snapshot.json scena.scena_viewer_inspector_snapshot.v1 scena-viewer-progress-rendered scena-viewer-file-drop scena-viewer-drop-error scena-viewer-variant-change scena-viewer-annotations-rendered annotation_tracking_sequence annotation_update_visible scena-viewer-inspector-rendered scena-viewer-key-control",
    )
    .expect("viewer element fixture without progress sequence proof");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> progress proof without a phase sequence: {findings:?}",
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
        fixture_root.join("src/viewer_element/element.js"),
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
        fixture_root.join("src/viewer_element/element.js"),
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
        fixture_root.join("src/viewer_element/element.js"),
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
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_mobile_gesture_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-mobile-gesture-proof");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "assertScenaViewerElementProof runScenaViewerElementProof scena-viewer-element-browser-proof.png assertCameraControlKitProof runCameraControlKitProof camera-control-kit-browser-proof.png SCENA_BROWSER_VIEWER_ELEMENT_ONLY scena.scena_viewer_element_browser_proof.v1 scena.m6.camera_control_kit_browser_proof.v1 progress_sequence annotation_tracking_sequence annotation_update_visible inspector_fixture_schema scena.scena_viewer_inspector_snapshot.v1",
    )
    .expect("viewer element fixture without mobile gesture proof");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> mobile/a11y claims without gesture browser proof: {findings:?}",
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
        fixture_root.join("src/viewer_element/element.js"),
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

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_inspector_fixture() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-inspector-fixture");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    let _ = fs::remove_file(fixture_root.join("tests/assets/viewer/inspector_snapshot.json"));
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> inspector proof without the JSON fixture: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_annotation_overlay() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-annotation-overlay");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("src/viewer_element/element.js"),
        r#"mod inspector; mod model; pub use inspector:: pub use model:: pub const SCENA_VIEWER_TAG defineScenaViewerElement customElements.define attachShadow observedAttributes scena-viewer-ready setLoadProgress progress.part = "progress" role", "progressbar" scena-viewer-progress scena-viewer-progress-rendered dragover drop _handleDrop _isSupportedAssetFile scena-viewer-file-drop scena-viewer-drop-error setMaterialVariants variantPicker.part = "variant-picker" scena-viewer-variant-change scena-viewer-variants-ready tabIndex = 0 aria-roledescription _handleKeydown _keyboardAction scena-viewer-key-control setInspectorSnapshot setInspectorDiagnostics clearInspectorSnapshot inspector.part = "inspector" inspector-status inspector-list scena-viewer-inspector-rendered"#,
    )
    .expect("viewer element fixture without annotation overlay");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> that drops the annotation overlay surface: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_annotation_tracking_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-annotation-tracking");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe_page.js"),
        "defineScenaViewer scenaViewerElementProbe scena.scena_viewer_element_browser_proof.v1 scena-viewer-progress-rendered scena-viewer-file-drop scena-viewer-drop-error scena-viewer-variant-change scena-viewer-annotations-rendered scena-viewer-inspector-rendered scena-viewer-key-control",
    )
    .expect("viewer element fixture without annotation tracking proof");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> annotation proof without a tracking sequence: {findings:?}",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_missing_scena_viewer_parity_proof() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root =
        root.join("target/xtask-doctor-regressions/missing-scena-viewer-parity-proof");
    write_easy_scene_fixture(
        &fixture_root,
        VALID_GUIDE,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    write_scena_viewer_element_easy_scene_fixture(&fixture_root);
    fs::write(
        fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js"),
        "assertScenaViewerElementProof runScenaViewerElementProof scena-viewer-element-browser-proof.png assertScenaViewerMobileA11yProof runScenaViewerMobileA11yProof scena-viewer-mobile-a11y-browser-proof.png SCENA_BROWSER_VIEWER_ELEMENT_ONLY scena.scena_viewer_element_browser_proof.v1 progress_sequence",
    )
    .expect("viewer element fixture without parity proof");
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "SCENA-VIEWER-ELEMENT"),
        "doctor must reject <scena-viewer> parity claims without three-asset side-by-side browser proof: {findings:?}",
    );
}
