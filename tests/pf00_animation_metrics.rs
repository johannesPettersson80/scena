use scena::{
    AnimationChannel, AnimationClip, AnimationInterpolation, AnimationOutput, AnimationTarget,
    Scene, Transform, Vec3,
};

#[test]
fn pf00_animation_profiled_update_reports_zero_clip_clone_and_weight_work() {
    let mut scene = Scene::new();
    let node = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("animated node inserts");
    scene
        .set_morph_weights(node, [0.0, 0.0, 0.0])
        .expect("profiled morph weights initialize");
    let times = vec![0.0, 0.25, 0.5, 0.75, 1.0];
    let clip = AnimationClip::authored(
        Some("pf00-many-keyframes-and-weights".to_owned()),
        vec![
            AnimationChannel::new(
                node,
                AnimationTarget::Translation,
                times.clone(),
                AnimationOutput::Vec3(
                    times
                        .iter()
                        .copied()
                        .map(|time| Vec3::new(time, 0.0, 0.0))
                        .collect(),
                ),
                AnimationInterpolation::Linear,
            ),
            AnimationChannel::new(
                node,
                AnimationTarget::Weights,
                times,
                AnimationOutput::Weights(vec![
                    vec![0.0, 0.0, 0.0],
                    vec![0.25, 0.5, 0.75],
                    vec![0.5, 1.0, 0.5],
                    vec![0.75, 0.5, 0.25],
                    vec![1.0, 0.0, 0.0],
                ]),
                AnimationInterpolation::Linear,
            ),
        ],
        1.0,
    )
    .expect("profiled animation clip validates");
    let mixer = scene
        .play_authored_animation(clip)
        .expect("profiled animation starts");

    let metrics = scene
        .update_animation_profiled(mixer, 0.6)
        .expect("profiled animation advances");

    assert_eq!(metrics.channels_scanned, 2);
    assert_eq!(metrics.keyframe_intervals_tested, 6);
    assert_eq!(metrics.weight_values_written, 3);
    assert_eq!(
        metrics.weight_bytes_written,
        3 * std::mem::size_of::<f32>() as u64
    );
    assert_eq!(metrics.clip_clone_bytes, 0);
}
