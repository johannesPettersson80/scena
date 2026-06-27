#![cfg(all(feature = "scene-host", not(target_arch = "wasm32")))]

use scena::{
    AssetPath, Color, PRESENTATION_TIMELINE_SCHEMA_V1, PresentationTimelineActionKindV1,
    PresentationTimelineActionV1, PresentationTimelineCameraBookmarkV1, PresentationTimelineV1,
    SceneHostAnimationLoopMode, SceneHostAnimationPlayOptions, SceneHostCameraState, SceneHostCore,
    SceneHostVisualStateV1, SceneInspectionReportV1, Transform, Vec3, VisualPatchLabelTargetV1,
    VisualPatchLabelV1, VisualPatchResultV1, VisualPatchTintV1, VisualPatchTransformV1,
    VisualPatchV1, VisualPatchVisibilityV1,
};

#[test]
fn presentation_timeline_seeks_flattened_visual_patch_deterministically() {
    let mut host = SceneHostCore::headless(128, 96).expect("host builds");
    let node = host
        .add_empty(
            Some(host.root_handle()),
            Transform::IDENTITY,
            Some("timeline-node"),
        )
        .expect("node inserts");
    let camera = SceneHostCameraState {
        target: Vec3::new(1.0, 0.0, 0.0),
        distance: 4.0,
        yaw_radians: 0.4,
        pitch_radians: 0.2,
    };

    host.store_visual_state(SceneHostVisualStateV1::new(
        "service",
        VisualPatchV1 {
            visibility: vec![VisualPatchVisibilityV1 {
                node,
                visible: false,
            }],
            labels: vec![VisualPatchLabelV1 {
                id: "service-label".to_owned(),
                target: VisualPatchLabelTargetV1::Node {
                    node,
                    local_offset: [0.0, 0.0, 0.0],
                },
            }],
            ..VisualPatchV1::default()
        },
    ))
    .expect("visual state stores");

    let timeline = PresentationTimelineV1 {
        schema: PRESENTATION_TIMELINE_SCHEMA_V1.to_owned(),
        camera_bookmarks: vec![PresentationTimelineCameraBookmarkV1 {
            name: "overview".to_owned(),
            camera,
        }],
        actions: vec![
            PresentationTimelineActionV1 {
                at_seconds: 0.0,
                action: PresentationTimelineActionKindV1::apply_patch(VisualPatchV1 {
                    transforms: vec![VisualPatchTransformV1 {
                        node,
                        transform: Transform::at(Vec3::new(1.0, 0.0, 0.0)),
                    }],
                    tints: vec![VisualPatchTintV1 {
                        node,
                        tint: Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0)),
                    }],
                    ..VisualPatchV1::default()
                }),
            },
            PresentationTimelineActionV1 {
                at_seconds: 0.5,
                action: PresentationTimelineActionKindV1::apply_patch(VisualPatchV1 {
                    transforms: vec![VisualPatchTransformV1 {
                        node,
                        transform: Transform::at(Vec3::new(2.0, 0.0, 0.0)),
                    }],
                    tints: vec![VisualPatchTintV1 {
                        node,
                        tint: Some(Color::from_linear_rgba(0.0, 1.0, 0.0, 1.0)),
                    }],
                    ..VisualPatchV1::default()
                }),
            },
            PresentationTimelineActionV1 {
                at_seconds: 0.75,
                action: PresentationTimelineActionKindV1::apply_state("service"),
            },
            PresentationTimelineActionV1 {
                at_seconds: 1.0,
                action: PresentationTimelineActionKindV1::camera_bookmark("overview"),
            },
        ],
    };

    let patch = host
        .timeline_patch(&timeline, 1.0)
        .expect("timeline emits patch");
    assert_eq!(patch.transforms.len(), 1, "last transform wins");
    assert_eq!(patch.tints.len(), 1, "last tint wins");
    assert_eq!(patch.visibility.len(), 1);
    assert_eq!(patch.labels.len(), 1);
    assert_eq!(patch.camera, Some(camera));

    let result = host
        .seek_timeline(&timeline, 1.0)
        .expect("timeline applies");
    assert_eq!(result.applied.transforms, 1);
    assert_eq!(result.applied.tints, 1);
    assert_eq!(result.applied.visibility, 1);
    assert_eq!(result.applied.camera, 1);
    assert_eq!(result.applied.labels, 1);
    assert!(result.failed.is_empty());

    let inspection: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    let inspected = inspection
        .node_by_handle(node)
        .expect("timeline node remains inspectable");
    assert_eq!(
        inspected.local_transform.translation,
        Vec3::new(2.0, 0.0, 0.0)
    );
    assert!(!inspected.visible);
    assert_eq!(
        inspected.tint,
        Some(Color::from_linear_rgba(0.0, 1.0, 0.0, 1.0))
    );
    assert_eq!(host.camera_state(), camera);

    let replay = host
        .seek_timeline(&timeline, 1.0)
        .expect("timeline replay applies");
    assert_eq!(
        replay.applied.transforms, 0,
        "flattened replay should be a no-op"
    );
    assert_eq!(replay.applied.tints, 0);
    assert_eq!(replay.applied.visibility, 0);
    assert_eq!(replay.applied.camera, 0);
    assert_eq!(replay.applied.labels, 0);
}

#[test]
fn presentation_timeline_advance_samples_animation_clip_from_host_tick() {
    let mut host = SceneHostCore::headless(128, 96).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("animated glTF instantiates");
    let animated = host
        .node_handle_by_name(import, "AnimatedTriangle")
        .expect("animated node resolves");
    let start_translation = node_translation(&host, animated);
    let mixer = host
        .play_animation(
            import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions {
                loop_mode: SceneHostAnimationLoopMode::Once,
                speed: 1.0,
            },
        )
        .expect("animation mixer starts");
    host.pause_animation(mixer).expect("timeline owns sampling");

    let timeline = PresentationTimelineV1::new().at(
        0.0,
        PresentationTimelineActionKindV1::animation_clip(mixer, 0.0, 1.0, Some(1.0)),
    );

    let result = host
        .advance_timeline(&timeline, 0.0, 0.5)
        .expect("timeline advances by host tick");
    assert_eq!(result.applied.animation_time, 1);
    assert!(result.failed.is_empty());
    assert!(
        !node_translation(&host, animated).abs_diff_eq(start_translation, 1.0e-5),
        "timeline advance should seek the mixer to sampled clip time"
    );

    let result = host
        .advance_timeline_json(
            &serde_json::to_string(&timeline).expect("timeline serializes"),
            0.5,
            0.5,
        )
        .expect("timeline JSON advance applies");
    let result: VisualPatchResultV1 =
        serde_json::from_str(&result).expect("timeline JSON result decodes");
    assert_eq!(result.applied.animation_time, 1);
    assert!(result.failed.is_empty());
}

#[test]
fn presentation_timeline_golden_fixture_matches_live_schema_serialization() {
    let fixture =
        std::fs::read_to_string("tests/assets/stable-contracts/presentation_timeline.v1.json")
            .expect("presentation timeline fixture reads");
    let fixture_value: serde_json::Value =
        serde_json::from_str(&fixture).expect("presentation timeline fixture parses");
    let decoded: PresentationTimelineV1 =
        serde_json::from_str(&fixture).expect("presentation timeline fixture decodes");
    let encoded =
        serde_json::to_value(decoded).expect("presentation timeline fixture reserializes");
    assert_eq!(encoded, fixture_value);
}

fn node_translation(host: &SceneHostCore, node: u64) -> Vec3 {
    let inspection: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    inspection
        .node_by_handle(node)
        .expect("node remains inspectable")
        .local_transform
        .translation
}
