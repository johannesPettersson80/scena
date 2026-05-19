use scena::{
    AssetLoadProgress, AssetPath, SCENA_VIEWER_TAG, ScenaViewerAttributes, ScenaViewerProgress,
    ScenaViewerProgressPhase, Tonemapper,
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
