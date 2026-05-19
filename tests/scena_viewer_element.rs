use scena::{
    AssetLoadProgress, AssetPath, DebugOverlay, Diagnostic, DiagnosticCode, DiagnosticSeverity,
    RendererStats, SCENA_VIEWER_TAG, ScenaViewerAccessibilityDefaults, ScenaViewerAttributes,
    ScenaViewerDropDecision, ScenaViewerDropKind, ScenaViewerInspectorSnapshot,
    ScenaViewerKeyboardAction, ScenaViewerProgress, ScenaViewerProgressPhase,
    ScenaViewerVariantSelection, Tonemapper,
};

#[test]
fn scena_viewer_attributes_parse_model_viewer_style_surface() {
    let attrs = ScenaViewerAttributes::from_pairs([
        ("src", "machine.glb"),
        ("environment", "studio"),
        ("tone-mapping", "neutral"),
        ("camera-controls", ""),
        ("auto-rotate", "true"),
        ("ar", "false"),
    ]);

    assert_eq!(SCENA_VIEWER_TAG, "scena-viewer");
    assert_eq!(attrs.src(), Some("machine.glb"));
    assert_eq!(attrs.environment(), Some("studio"));
    assert_eq!(attrs.tonemapper(), Tonemapper::PbrNeutral);
    assert!(attrs.camera_controls());
    assert!(attrs.auto_rotate());
    assert!(!attrs.ar());
}

#[test]
fn scena_viewer_attributes_default_to_safe_drop_in_viewer_values() {
    let attrs = ScenaViewerAttributes::default();

    assert_eq!(attrs.src(), None);
    assert_eq!(attrs.environment(), None);
    assert_eq!(attrs.tonemapper(), Tonemapper::PbrNeutral);
    assert!(!attrs.camera_controls());
    assert!(!attrs.auto_rotate());
    assert!(!attrs.ar());
}

#[test]
fn scena_viewer_progress_maps_asset_events_to_accessible_details() {
    let started = ScenaViewerProgress::from_asset_event(&AssetLoadProgress::LoadStarted {
        path: AssetPath::from("memory://machine.glb"),
    });
    assert_eq!(started.phase(), ScenaViewerProgressPhase::Loading);
    assert_eq!(started.path(), Some("memory://machine.glb"));
    assert_eq!(started.aria_text(), "Loading memory://machine.glb");
    assert!(!started.is_complete());

    let fetched = ScenaViewerProgress::from_asset_event(&AssetLoadProgress::AssetFetched {
        path: AssetPath::from("memory://machine.glb"),
        bytes: 42,
    });
    assert_eq!(fetched.phase(), ScenaViewerProgressPhase::Fetching);
    assert_eq!(fetched.loaded_bytes(), Some(42));
    assert_eq!(
        fetched.aria_text(),
        "Fetched 42 bytes from memory://machine.glb"
    );

    let parsed = ScenaViewerProgress::from_asset_event(&AssetLoadProgress::Parsed {
        path: AssetPath::from("memory://machine.glb"),
        nodes: 3,
        meshes: 2,
    });
    assert_eq!(parsed.phase(), ScenaViewerProgressPhase::Parsing);
    assert_eq!(parsed.nodes(), Some(3));
    assert_eq!(parsed.meshes(), Some(2));
    assert_eq!(
        parsed.aria_text(),
        "Parsed memory://machine.glb with 3 nodes and 2 meshes"
    );

    let cached = ScenaViewerProgress::from_asset_event(&AssetLoadProgress::Cached {
        path: AssetPath::from("memory://machine.glb"),
    });
    assert_eq!(cached.phase(), ScenaViewerProgressPhase::Complete);
    assert!(cached.is_complete());
    assert_eq!(cached.aria_text(), "Loaded memory://machine.glb");
}

#[test]
fn scena_viewer_drop_decision_accepts_gltf_and_reports_rejections() {
    let decision =
        ScenaViewerDropDecision::from_file_names(["machine.glb", "assembly.gltf", "notes.txt"]);

    assert_eq!(decision.accepted().len(), 2);
    assert_eq!(decision.accepted()[0].name(), "machine.glb");
    assert_eq!(decision.accepted()[0].kind(), ScenaViewerDropKind::Glb);
    assert_eq!(decision.accepted()[1].name(), "assembly.gltf");
    assert_eq!(decision.accepted()[1].kind(), ScenaViewerDropKind::Gltf);
    assert_eq!(decision.rejected(), &["notes.txt".to_string()]);
    assert!(decision.has_accepted_files());
    assert!(decision.has_rejections());

    let empty = ScenaViewerDropDecision::from_file_names(std::iter::empty::<&str>());
    assert!(!empty.has_accepted_files());
    assert_eq!(empty.status_text(), "Drop a .glb or .gltf file");

    assert_eq!(
        decision.status_text(),
        "Accepted 2 glTF files; rejected notes.txt"
    );
}

#[test]
fn scena_viewer_variant_selection_tracks_available_and_active_names() {
    let selection =
        ScenaViewerVariantSelection::from_names(["midnight", "noon"]).with_active("noon");

    assert_eq!(selection.options().len(), 2);
    assert_eq!(selection.options()[0].name(), "midnight");
    assert_eq!(selection.options()[0].label(), "midnight");
    assert_eq!(selection.options()[1].name(), "noon");
    assert_eq!(selection.active(), Some("noon"));
    assert!(selection.has_active_variant());
    assert_eq!(selection.status_text(), "2 material variants; active noon");

    let unknown =
        ScenaViewerVariantSelection::from_names(["midnight", "noon"]).with_active("missing");
    assert_eq!(unknown.active(), None);
    assert!(!unknown.has_active_variant());
    assert_eq!(unknown.status_text(), "2 material variants");

    let empty = ScenaViewerVariantSelection::from_names(std::iter::empty::<&str>());
    assert!(empty.options().is_empty());
    assert_eq!(empty.status_text(), "No material variants");
}

#[test]
fn scena_viewer_accessibility_defaults_define_mobile_and_keyboard_surface() {
    let defaults = ScenaViewerAccessibilityDefaults::default();

    assert_eq!(defaults.host_role(), "img");
    assert_eq!(defaults.host_label(), "3D model viewer");
    assert_eq!(defaults.canvas_label(), "scena 3D viewer canvas");
    assert_eq!(defaults.min_width_px(), 160);
    assert_eq!(defaults.min_height_px(), 120);
    assert_eq!(defaults.touch_action(), "none");
    assert!(defaults.host_is_keyboard_focusable());

    assert_eq!(
        ScenaViewerKeyboardAction::from_key("ArrowLeft"),
        Some(ScenaViewerKeyboardAction::OrbitLeft)
    );
    assert_eq!(
        ScenaViewerKeyboardAction::from_key("+"),
        Some(ScenaViewerKeyboardAction::ZoomIn)
    );
    assert_eq!(
        ScenaViewerKeyboardAction::from_key("Escape"),
        Some(ScenaViewerKeyboardAction::ResetView)
    );
    assert_eq!(ScenaViewerKeyboardAction::from_key("Tab"), None);
}

#[test]
fn scena_viewer_inspector_snapshot_summarizes_diagnostics_and_render_state() {
    let diagnostics = [
        Diagnostic::warning(
            DiagnosticCode::MissingLightingOrEnvironment,
            "scene has no visible lighting",
            "add a default light or environment preset",
        ),
        Diagnostic::error(
            DiagnosticCode::MissingActiveCamera,
            "scene has no active camera",
            "call Scene::set_active_camera",
        ),
    ];
    let stats = RendererStats {
        draw_calls: 7,
        triangles: 42,
        target_width: 320,
        target_height: 180,
        ..RendererStats::default()
    };

    let snapshot = ScenaViewerInspectorSnapshot::from_renderer_state(
        DebugOverlay::BoundingBoxes,
        &diagnostics,
        stats,
    );

    assert_eq!(snapshot.overlay(), DebugOverlay::BoundingBoxes);
    assert_eq!(snapshot.diagnostics().len(), 2);
    assert_eq!(
        snapshot.diagnostics()[0].severity(),
        DiagnosticSeverity::Warning
    );
    assert_eq!(
        snapshot.diagnostics()[0].code(),
        "MissingLightingOrEnvironment"
    );
    assert_eq!(snapshot.warning_count(), 1);
    assert_eq!(snapshot.error_count(), 1);
    assert!(snapshot.has_errors());
    assert_eq!(
        snapshot.status_text(),
        "BoundingBoxes overlay; 1 error, 1 warning; 7 draws; 42 triangles at 320x180"
    );
}
