use scena::{SCENA_VIEWER_TAG, ScenaViewerAttributes, Tonemapper};

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
