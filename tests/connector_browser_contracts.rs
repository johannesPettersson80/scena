#![cfg(feature = "scene-host")]

use scena::{CONNECTOR_BROWSER_SCHEMA_V1, ConnectorBrowserReportV1, SceneHostCore};

const SOURCE_ASSET: &str = "tests/assets/gltf/connector_debug_scene.gltf";
const TARGET_ASSET: &str = "tests/assets/gltf/connector_browser_targets.gltf";

#[test]
fn connector_browser_reports_import_connectors_and_metadata_candidates() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    let source = pollster::block_on(host.instantiate_url(SOURCE_ASSET)).expect("source imports");
    let targets = pollster::block_on(host.instantiate_url(TARGET_ASSET)).expect("targets import");

    let json = host
        .connector_browser_json(source, &[targets])
        .expect("connector browser serializes");
    let report: ConnectorBrowserReportV1 = serde_json::from_str(&json).expect("report decodes");

    assert_eq!(report.schema, CONNECTOR_BROWSER_SCHEMA_V1);
    assert_eq!(report.scope.kind, "import");
    assert_eq!(report.scope.import, Some(source));
    assert_eq!(report.scope.target_imports, vec![targets]);
    assert_eq!(report.summary.connector_count, 1);
    assert_eq!(report.summary.candidate_count, 2);
    assert_eq!(report.summary.compatible_count, 1);
    assert_eq!(report.summary.snap_ready_count, 1);
    assert_eq!(report.summary.invalid_count, 1);

    let source_connector = report
        .connectors
        .iter()
        .find(|connector| connector.name == "mount")
        .expect("source mount connector listed");
    assert_eq!(source_connector.kind.as_deref(), Some("mount"));
    assert_eq!(source_connector.allowed_mates, ["socket"]);
    assert!(source_connector.tags.contains(&"assembly".to_owned()));
    assert_eq!(source_connector.polarity.as_deref(), Some("plug"));
    assert_eq!(source_connector.roll_policy, "choose_nearest");
    assert_eq!(source_connector.snap_tolerance, Some(0.025));

    let ready = report
        .candidates
        .iter()
        .find(|candidate| candidate.target_name == "socket")
        .expect("compatible socket candidate exists");
    assert!(ready.compatible, "{ready:#?}");
    assert!(ready.snap_ready);
    assert_eq!(ready.invalid_reasons, Vec::<String>::new());
    assert!(ready.distance <= ready.tolerance);
    assert_eq!(ready.visual_cue.as_deref(), Some("scena-magnet-ready"));
    assert!(ready.ghost_transform.is_some());
    assert!(ready.connection_line.is_some());

    let invalid = report
        .candidates
        .iter()
        .find(|candidate| candidate.target_name == "cable")
        .expect("incompatible cable candidate exists");
    assert!(!invalid.compatible);
    assert!(!invalid.snap_ready);
    assert!(
        invalid
            .invalid_reasons
            .iter()
            .any(|reason| reason == "incompatible_kind" || reason == "polarity_mismatch"),
        "{invalid:#?}"
    );
    assert!(invalid.ghost_transform.is_none());
}

#[test]
fn connector_browser_reports_subtree_and_selection_scopes() {
    let mut host = SceneHostCore::headless(128, 128).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(TARGET_ASSET)).expect("targets import");
    let roots = host.import_roots(import).expect("import roots resolve");
    let socket = host
        .node_handle_by_name(import, "Socket")
        .expect("socket node handle resolves");

    let subtree_json = host
        .connector_browser_subtree_json(roots[0])
        .expect("subtree connector browser serializes");
    let subtree: ConnectorBrowserReportV1 =
        serde_json::from_str(&subtree_json).expect("subtree report decodes");
    assert_eq!(subtree.schema, CONNECTOR_BROWSER_SCHEMA_V1);
    assert_eq!(subtree.scope.kind, "subtree");
    assert_eq!(subtree.scope.root, Some(roots[0]));
    assert_eq!(subtree.summary.connector_count, 2);

    let selection_json = host
        .connector_browser_selection_json(&[socket])
        .expect("selection connector browser serializes");
    let selection: ConnectorBrowserReportV1 =
        serde_json::from_str(&selection_json).expect("selection report decodes");
    assert_eq!(selection.scope.kind, "selection");
    assert_eq!(selection.scope.selection, vec![socket]);
    assert_eq!(selection.summary.connector_count, 1);
    assert_eq!(selection.connectors[0].name, "socket");
}

#[test]
fn connector_browser_golden_fixture_matches_live_schema_serialization() {
    let fixture = include_str!("assets/stable-contracts/connector_browser.v1.json");
    let report: ConnectorBrowserReportV1 = serde_json::from_str(fixture).expect("fixture decodes");

    assert_eq!(report.schema, CONNECTOR_BROWSER_SCHEMA_V1);
    let value = serde_json::to_value(&report).expect("report serializes");
    let expected: serde_json::Value = serde_json::from_str(fixture).expect("fixture parses");
    assert_eq!(value, expected);
}
