use crate::app::prelude::*;

pub(super) fn check_camera_control_kit(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "CAMERA-CONTROL-KIT",
        "src/controls.rs",
        &[
            "mod camera_kit;",
            "pub use camera_kit::{FlyControls, FollowControls};",
        ],
    );
    require_contains(
        root,
        findings,
        "CAMERA-CONTROL-KIT",
        "src/controls/camera_kit.rs",
        &[
            "pub struct FollowControls",
            "pub fn behind_and_above(",
            "pub fn with_target_offset(",
            "pub struct FlyControls",
            "pub fn with_yaw_pitch_degrees(",
            "pub fn move_local(",
            "pub fn look_delta(",
            "pub fn apply_to_scene(",
        ],
    );
    require_contains(
        root,
        findings,
        "CAMERA-CONTROL-KIT",
        "src/lib.rs",
        &["FollowControls", "FlyControls"],
    );
    require_contains(
        root,
        findings,
        "CAMERA-CONTROL-KIT",
        "tests/camera_control_kit.rs",
        &[
            "follow_controls_track_node_with_named_offset",
            "fly_controls_move_in_camera_local_axes_and_apply_to_scene",
        ],
    );
    require_contains(
        root,
        findings,
        "CAMERA-CONTROL-KIT",
        "docs/guides/easy-scene-setup.md",
        &[
            "FollowControls::behind_and_above",
            "FlyControls::new",
            "move_local",
            "with_yaw_pitch_degrees",
        ],
    );
    require_contains(
        root,
        findings,
        "CAMERA-CONTROL-KIT",
        "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
        &[
            "Follow/Fly",
            "library primitives",
            "**[shipped]**",
            "tests/camera_control_kit.rs",
            "CAMERA-CONTROL-KIT",
        ],
    );
}
