use scena::{
    AnchorFrame, Assets, Callout, Color, ConnectorFrame, LabelKey, NodeKind, Scene, Transform, Vec3,
};
#[cfg(feature = "scene-host")]
use scena::{
    AnnotationProjectionReportV1, SceneHostCore, VISUAL_PATCH_SCHEMA_V1, VisualPatchResultV1,
};

#[test]
fn callouts_attach_to_node_world_anchor_and_connector_and_clean_up() {
    let assets = Assets::new();
    let mut scene = Scene::new();
    let node = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("target inserts");
    let anchor = scene
        .add_anchor(AnchorFrame::new(
            node,
            Transform::at(Vec3::new(0.1, 0.0, 0.0)),
        ))
        .expect("anchor inserts");
    let connector = scene
        .add_connector(ConnectorFrame::new(
            node,
            Transform::at(Vec3::new(-0.1, 0.0, 0.0)),
        ))
        .expect("connector inserts");

    let node_callout = scene
        .add_callout(
            &assets,
            Callout::node("node-callout", node, Vec3::ZERO, "Node")
                .with_label_offset(Vec3::new(0.35, 0.2, 0.0))
                .with_color(Color::YELLOW),
        )
        .expect("node callout inserts");
    let world_callout = scene
        .add_callout(
            &assets,
            Callout::world("world-callout", Vec3::new(0.0, 0.0, 0.0), "World")
                .with_label_offset(Vec3::new(0.0, 0.35, 0.0)),
        )
        .expect("world callout inserts");
    let anchor_callout = scene
        .add_callout(
            &assets,
            Callout::anchor("anchor-callout", anchor, "Anchor")
                .with_label_offset(Vec3::new(0.0, 0.2, 0.0)),
        )
        .expect("anchor callout inserts");
    let connector_callout = scene
        .add_callout(
            &assets,
            Callout::connector("connector-callout", connector, "Connector")
                .with_label_offset(Vec3::new(0.0, -0.2, 0.0)),
        )
        .expect("connector callout inserts");

    for report in [
        &node_callout,
        &world_callout,
        &anchor_callout,
        &connector_callout,
    ] {
        assert_eq!(report.anchor_id, report.id);
        assert!(scene.annotation_anchor(&report.anchor_id).is_some());
        assert!(matches!(
            scene
                .node(report.leader_line_node)
                .expect("leader line node exists")
                .kind(),
            NodeKind::Mesh(_)
        ));
        assert_label_text(&scene, report.label, report.text.as_str());
    }

    assert!(scene.clear_callout("world-callout"));
    assert!(scene.annotation_anchor("world-callout").is_none());
    assert!(scene.node(world_callout.leader_line_node).is_none());
    assert!(scene.node(world_callout.label_node).is_none());

    scene.remove_node(node).expect("target removes");
    assert!(scene.annotation_anchor("node-callout").is_none());
    assert!(scene.callout("node-callout").is_none());
    assert!(scene.callout("anchor-callout").is_none());
    assert!(scene.callout("connector-callout").is_none());
}

#[test]
#[cfg(feature = "scene-host")]
fn scene_host_callout_uses_stable_handles_and_label_patch_anchor_id() {
    let mut host = SceneHostCore::headless(160, 120).expect("host builds");
    let target = host
        .add_empty(None, Transform::IDENTITY, Some("callout-target"))
        .expect("target inserts");

    let report = host
        .add_node_callout(
            "motor-callout",
            target,
            [0.0, 0.0, 0.0],
            [0.45, 0.2, 0.0],
            "Motor",
        )
        .expect("host callout inserts");
    assert_eq!(report.id, "motor-callout");
    assert_eq!(report.anchor_id, "motor-callout");
    assert_ne!(report.leader_line_node, target);
    assert_ne!(report.label_node, target);

    let first = projection_for(&host, "motor-callout");
    assert_eq!(first.node_handle, Some(target));
    assert!(first.visible);

    host.apply_patch_json(
        &serde_json::json!({
            "schema": VISUAL_PATCH_SCHEMA_V1,
            "labels": [{
                "id": "motor-callout",
                "target": {
                    "kind": "node",
                    "node": target,
                    "local_offset": [0.25, 0.0, 0.0]
                }
            }]
        })
        .to_string(),
    )
    .expect("label patch retargets the callout anchor");
    let result: VisualPatchResultV1 = serde_json::from_str(
        &host
            .apply_patch_json(
                &serde_json::json!({
                    "schema": VISUAL_PATCH_SCHEMA_V1,
                    "labels": [{
                        "id": "motor-callout",
                        "target": {
                            "kind": "node",
                            "node": target,
                            "local_offset": [0.25, 0.0, 0.0]
                        }
                    }]
                })
                .to_string(),
            )
            .expect("idempotent label patch applies"),
    )
    .expect("result decodes");
    assert_eq!(
        result.applied.labels, 0,
        "callouts must not introduce a parallel host annotation update path"
    );

    let moved = projection_for(&host, "motor-callout");
    assert!(
        moved.x > first.x,
        "retargeting through the labels patch channel moves the projected callout anchor"
    );

    assert!(host.clear_callout("motor-callout"));
    assert!(!host.clear_callout("motor-callout"));
    assert!(
        !projection_ids(&host).contains(&"motor-callout".to_owned()),
        "clearing a callout removes its annotation anchor"
    );
}

fn assert_label_text(scene: &Scene, label: LabelKey, expected: &str) {
    assert_eq!(scene.label(label).expect("label exists").text(), expected);
}

#[cfg(feature = "scene-host")]
fn projection_for(host: &SceneHostCore, id: &str) -> scena::AnnotationProjectionV1 {
    let report: AnnotationProjectionReportV1 =
        serde_json::from_str(&host.annotation_projections_json().expect("projection JSON"))
            .expect("projection report decodes");
    report
        .annotations
        .into_iter()
        .find(|projection| projection.id == id)
        .expect("callout projection appears")
}

#[cfg(feature = "scene-host")]
fn projection_ids(host: &SceneHostCore) -> Vec<String> {
    let report: AnnotationProjectionReportV1 =
        serde_json::from_str(&host.annotation_projections_json().expect("projection JSON"))
            .expect("projection report decodes");
    report
        .annotations
        .into_iter()
        .map(|projection| projection.id)
        .collect()
}
