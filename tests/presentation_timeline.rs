#![cfg(all(feature = "scene-host", not(target_arch = "wasm32")))]

use std::future::{Ready, ready};
use std::sync::Arc;

use scena::{
    AssetError, AssetFetcher, AssetPath, Color, PRESENTATION_TIMELINE_SCHEMA_V1,
    PresentationTimelineActionKindV1, PresentationTimelineActionV1,
    PresentationTimelineCameraBookmarkV1, PresentationTimelineV1, SceneHostAnimationLoopMode,
    SceneHostAnimationPlayOptions, SceneHostCameraState, SceneHostCore, SceneHostErrorCode,
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
fn presentation_timeline_missing_end_clamps_once_clip_to_terminal_pose_without_failure() {
    let mut host = SceneHostCore::headless(128, 96).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("animated glTF instantiates");
    let animated = host
        .node_handle_by_name(import, "AnimatedTriangle")
        .expect("animated node resolves");
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
    host.seek_animation(mixer, 1.0)
        .expect("terminal clip pose resolves");
    let terminal = node_translation(&host, animated);
    host.seek_animation(mixer, 0.0)
        .expect("mixer resets before timeline proof");

    let timeline = PresentationTimelineV1::new().at(
        0.0,
        PresentationTimelineActionKindV1::animation_clip(mixer, 0.0, 1.0, None),
    );
    for seconds in [2.0, 3.0] {
        let result = host
            .seek_timeline(&timeline, seconds)
            .expect("timeline seek returns a patch result");

        assert!(
            result.failed.is_empty(),
            "a missing end must resolve to clip duration before the patch is applied: {:?}",
            result.failed
        );
        assert!(
            node_translation(&host, animated).abs_diff_eq(terminal, 1.0e-5),
            "a once segment must hold the exact terminal pose without repeated failed entries"
        );
    }
}

#[test]
fn presentation_timeline_validates_clip_bounds_before_any_action_is_due_or_applied() {
    let (mut host, animated, mixer) = animation_host(SceneHostAnimationLoopMode::Once);
    let before = node_translation(&host, animated);
    let invalid = PresentationTimelineV1::new().at(
        10.0,
        PresentationTimelineActionKindV1::animation_clip(mixer, 1.25, 1.0, None),
    );

    for _ in 0..2 {
        let error = host
            .seek_timeline(&invalid, 0.0)
            .expect_err("a future invalid segment must fail before patch application");
        assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
        assert!(error.message().contains("start_seconds"));
        assert!(error.message().contains("duration"));
        assert!(
            node_translation(&host, animated).abs_diff_eq(before, 1.0e-5),
            "construction-time validation must not mutate the animated node"
        );
    }

    let beyond_end = PresentationTimelineV1::new().at(
        0.0,
        PresentationTimelineActionKindV1::animation_clip(mixer, 0.25, 1.0, Some(10.0)),
    );
    let result = host
        .seek_timeline(&beyond_end, 20.0)
        .expect("an explicit end beyond duration is clamped");
    assert!(result.failed.is_empty());
    let actual = node_translation(&host, animated);
    host.seek_animation(mixer, 1.0)
        .expect("terminal reference pose resolves");
    let terminal = node_translation(&host, animated);
    assert!(actual.abs_diff_eq(terminal, 1.0e-5));
}

#[test]
fn presentation_timeline_once_and_repeat_segments_hold_or_wrap_at_stable_boundaries() {
    let (mut once_host, once_node, once_mixer) = animation_host(SceneHostAnimationLoopMode::Once);
    let once = PresentationTimelineV1::new().at(
        0.0,
        PresentationTimelineActionKindV1::animation_clip(once_mixer, 0.1, 1.0, Some(0.4)),
    );
    once_host
        .seek_animation(once_mixer, 0.4)
        .expect("once terminal reference resolves");
    let once_terminal = node_translation(&once_host, once_node);
    for seconds in [0.3, 0.1 + 0.2, 4.0] {
        let result = once_host
            .seek_timeline(&once, seconds)
            .expect("once boundary samples");
        assert!(result.failed.is_empty());
        assert!(
            node_translation(&once_host, once_node).abs_diff_eq(once_terminal, 1.0e-5),
            "once segment must hold its exact end pose at {seconds}"
        );
    }

    let (mut repeat_host, repeat_node, repeat_mixer) =
        animation_host(SceneHostAnimationLoopMode::Repeat);
    let repeat = PresentationTimelineV1::new().at(
        0.0,
        PresentationTimelineActionKindV1::animation_clip(repeat_mixer, 0.1, 1.0, Some(0.4)),
    );
    repeat_host
        .seek_animation(repeat_mixer, 0.1)
        .expect("repeat start reference resolves");
    let repeat_start = node_translation(&repeat_host, repeat_node);
    for seconds in [0.0, 0.3, 0.1 + 0.2, 0.6] {
        let result = repeat_host
            .seek_timeline(&repeat, seconds)
            .expect("repeat boundary samples");
        assert!(result.failed.is_empty());
        assert!(
            node_translation(&repeat_host, repeat_node).abs_diff_eq(repeat_start, 1.0e-5),
            "repeat segment must wrap to its start at stable boundary {seconds}"
        );
    }
}

#[test]
fn presentation_timeline_static_clip_samples_zero_and_rejects_positive_start() {
    let fetcher = StaticAnimationFetcher::new();
    let path = fetcher.path.clone();
    let mut host = SceneHostCore::headless_with_fetcher(fetcher, 64, 64).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(path)).expect("static clip instantiates");
    let animated = host
        .node_handle_by_name(import, "Animated")
        .expect("static animated node resolves");

    for loop_mode in [
        SceneHostAnimationLoopMode::Once,
        SceneHostAnimationLoopMode::Repeat,
    ] {
        let mixer = host
            .play_animation(
                import,
                "Probe",
                SceneHostAnimationPlayOptions {
                    loop_mode,
                    speed: 1.0,
                },
            )
            .expect("static mixer starts");
        host.pause_animation(mixer).expect("timeline owns sampling");
        for end_seconds in [None, Some(0.0), Some(10.0)] {
            let timeline = PresentationTimelineV1::new().at(
                0.0,
                PresentationTimelineActionKindV1::animation_clip(mixer, 0.0, 1.0, end_seconds),
            );
            let result = host
                .seek_timeline(&timeline, 100.0)
                .expect("static segment always samples time zero");
            assert!(result.failed.is_empty());
            assert!(
                node_translation(&host, animated).abs_diff_eq(Vec3::new(2.0, 0.0, 0.0), 1.0e-5)
            );
        }

        let invalid = PresentationTimelineV1::new().at(
            0.0,
            PresentationTimelineActionKindV1::animation_clip(mixer, 0.01, 1.0, None),
        );
        let error = host
            .timeline_patch(&invalid, 0.0)
            .expect_err("static clips reject positive starts");
        assert_eq!(error.code(), SceneHostErrorCode::InvalidInput);
    }
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

fn animation_host(loop_mode: SceneHostAnimationLoopMode) -> (SceneHostCore, u64, u64) {
    let mut host = SceneHostCore::headless(128, 96).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/animated_triangle_scene.glb",
    )))
    .expect("animated glTF instantiates");
    let animated = host
        .node_handle_by_name(import, "AnimatedTriangle")
        .expect("animated node resolves");
    let mixer = host
        .play_animation(
            import,
            "MoveTriangle",
            SceneHostAnimationPlayOptions {
                loop_mode,
                speed: 1.0,
            },
        )
        .expect("animation mixer starts");
    host.pause_animation(mixer).expect("timeline owns sampling");
    (host, animated, mixer)
}

fn node_translation<F: AssetFetcher>(host: &SceneHostCore<F>, node: u64) -> Vec3 {
    let inspection: SceneInspectionReportV1 =
        serde_json::from_str(&host.inspect_json().expect("inspection serializes"))
            .expect("inspection decodes");
    inspection
        .node_by_handle(node)
        .expect("node remains inspectable")
        .local_transform
        .translation
}

#[derive(Clone)]
struct StaticAnimationFetcher {
    path: AssetPath,
    bytes: Arc<Vec<u8>>,
}

impl StaticAnimationFetcher {
    fn new() -> Self {
        let path = AssetPath::from("memory://c08/static-animation.gltf");
        let document = br#"{
          "asset":{"version":"2.0"},
          "nodes":[{"name":"Animated"}],
          "animations":[{"name":"Probe","samplers":[{"input":0,"output":1}],"channels":[{"sampler":0,"target":{"node":0,"path":"translation"}}]}],
          "buffers":[{"byteLength":16,"uri":"data:application/octet-stream;base64,AAAAAAAAAEAAAAAAAAAAAA=="}],
          "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":4},{"buffer":0,"byteOffset":4,"byteLength":12}],
          "accessors":[{"bufferView":0,"componentType":5126,"count":1,"type":"SCALAR"},{"bufferView":1,"componentType":5126,"count":1,"type":"VEC3"}]
        }"#
        .to_vec();
        Self {
            path,
            bytes: Arc::new(document),
        }
    }
}

impl AssetFetcher for StaticAnimationFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        ready(if path == &self.path {
            Ok((*self.bytes).clone())
        } else {
            Err(AssetError::NotFound {
                path: path.as_str().to_owned(),
            })
        })
    }
}
