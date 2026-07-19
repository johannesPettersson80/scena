use scena::{Assets, Callout, MeasurementOverlay, NodeKind, Scene, Transform, Vec3};

#[test]
fn removing_either_measurement_child_removes_the_complete_overlay() {
    for remove_label in [false, true] {
        let assets = Assets::new();
        let mut scene = Scene::new();
        let report = scene
            .add_measurement_overlay(
                &assets,
                MeasurementOverlay::distance("clearance", Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))
                    .with_label("clearance"),
            )
            .expect("measurement inserts");
        let label_node = scene
            .node(scene.root())
            .expect("root exists")
            .children()
            .iter()
            .copied()
            .find(|node| {
                matches!(
                    scene.node(*node).map(scena::Node::kind),
                    Some(NodeKind::Label(_))
                )
            })
            .expect("measurement label node exists");
        let removed = if remove_label {
            label_node
        } else {
            report.line_node
        };

        scene.remove_node(removed).expect("generated child removes");

        assert!(scene.node(report.line_node).is_none());
        assert!(scene.node(label_node).is_none());
        assert!(
            !scene.clear_measurement_overlay("clearance"),
            "direct child removal must also remove the overlay registry entry"
        );
    }
}

#[test]
fn removing_either_callout_child_closes_node_and_world_owned_state() {
    for world_anchor in [false, true] {
        for remove_label in [false, true] {
            let assets = Assets::new();
            let mut scene = Scene::new();
            let target = scene
                .add_empty(scene.root(), Transform::IDENTITY)
                .expect("target inserts");
            let callout = if world_anchor {
                Callout::world("note", Vec3::ZERO, "world note")
            } else {
                Callout::node("note", target, Vec3::ZERO, "node note")
            };
            let report = scene
                .add_callout(&assets, callout)
                .expect("callout inserts");
            let removed = if remove_label {
                report.label_node
            } else {
                report.leader_line_node
            };

            scene.remove_node(removed).expect("generated child removes");

            assert!(scene.node(report.leader_line_node).is_none());
            assert!(scene.node(report.label_node).is_none());
            assert!(scene.callout("note").is_none());
            assert!(scene.annotation_anchor("note").is_none());
            assert!(
                scene.node(target).is_some(),
                "the target is not owned output"
            );
        }
    }
}

#[cfg(feature = "scene-host")]
mod scene_host {
    use scena::{
        AnnotationProjectionReportV1, SceneHostCore, SceneHostErrorCode, SceneInspectionReportV1,
    };

    #[test]
    fn removing_either_callout_handle_invalidates_the_complete_owned_closure() {
        for remove_label in [false, true] {
            let mut host = SceneHostCore::headless(128, 96).expect("host builds");
            let report = host
                .add_world_callout("host-note", [0.0, 0.0, 0.0], [0.3, 0.2, 0.0], "note")
                .expect("callout inserts");
            let removed = if remove_label {
                report.label_node
            } else {
                report.leader_line_node
            };

            host.remove_node(removed).expect("generated child removes");

            assert_stale(&mut host, report.leader_line_node);
            assert_stale(&mut host, report.label_node);
            let projections: AnnotationProjectionReportV1 = serde_json::from_str(
                &host
                    .annotation_projections_json()
                    .expect("projections encode"),
            )
            .expect("projections decode");
            assert!(
                projections
                    .annotations
                    .iter()
                    .all(|annotation| annotation.id != "host-note")
            );
        }
    }

    #[test]
    fn removing_either_measurement_handle_invalidates_the_complete_owned_closure() {
        for remove_label in [false, true] {
            let mut host = SceneHostCore::headless(128, 96).expect("host builds");
            let report: serde_json::Value = serde_json::from_str(
                &host
                    .add_distance_measurement_json(
                        "host-clearance",
                        Vec3::ZERO,
                        Vec3::new(1.0, 0.0, 0.0),
                        Some("clearance"),
                        "m",
                        3,
                    )
                    .expect("measurement inserts"),
            )
            .expect("measurement report decodes");
            let line = report["line_node"].as_u64().expect("line handle");
            let inspection: SceneInspectionReportV1 =
                serde_json::from_str(&host.inspect_json().expect("inspection encodes"))
                    .expect("inspection decodes");
            let label = inspection
                .nodes
                .iter()
                .find(|node| node.kind == "Label")
                .map(|node| node.handle)
                .expect("label handle");
            let removed = if remove_label { label } else { line };

            host.remove_node(removed).expect("generated child removes");

            assert_stale(&mut host, line);
            assert_stale(&mut host, label);
        }
    }

    fn assert_stale(host: &mut SceneHostCore, handle: u64) {
        let error = host
            .remove_node(handle)
            .expect_err("owned child handle must be stale");
        assert_eq!(error.code(), SceneHostErrorCode::StaleNodeHandle);
    }

    use scena::Vec3;
}
