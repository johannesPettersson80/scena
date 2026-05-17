use crate::app::prelude::*;

pub(crate) fn check_m3b_animation_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/animation.rs",
        &[
            "pub struct AnimationMixerKey",
            "pub enum AnimationPlaybackState",
            "pub enum AnimationLoopMode",
            "pub enum AnimationTarget",
            "pub enum AnimationInterpolation",
            "pub struct AnimationClip",
            "pub struct AnimationSourceClip",
            "pub struct AnimationChannel",
            "pub struct AnimationSourceChannel",
            "pub struct AnimationMixer",
            "pub enum AnimationOutput",
            "pub fn rebind",
            "pub fn sample_vec3",
            "pub fn sample_quat",
            "pub fn sample_weights",
            "pub(crate) fn play",
            "pub(crate) fn pause",
            "pub(crate) fn stop",
            "pub(crate) fn seek",
            "pub(crate) fn advance",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/animation/sampling.rs",
        &[
            "sample_cubic_vec3",
            "sample_cubic_quat",
            "sample_cubic_weights",
            "slerp_quat",
            "cubic_scalar",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/animation.rs",
        &[
            "pub(super) fn parse_gltf_clips",
            "parse_channel",
            "GltfProperty::Translation",
            "GltfProperty::Rotation",
            "GltfProperty::Scale",
            "GltfProperty::MorphTargetWeights",
            "ReadOutputs::Translations",
            "ReadOutputs::Rotations",
            "ReadOutputs::Scales",
            "AnimationInterpolation::CubicSpline",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import.rs",
        &[
            "clip.clip().rebind",
            "resolve_import_skin_bindings",
            "SceneSkinBinding::new",
            "convert_animation_vec3",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/accessors.rs",
        &["pub(crate) fn live_flag"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/options.rs",
        &[
            "convert_animation_vec3",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/import/accessors.rs",
        &["pub fn channels(&self)", "pub const fn duration_seconds"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/mixers.rs",
        &[
            "pub fn create_animation_mixer",
            "pub fn animation_mixer",
            "pub fn play_animation",
            "pub fn pause_animation",
            "pub fn stop_animation",
            "pub fn seek_animation",
            "pub fn set_animation_speed",
            "pub fn set_animation_loop_mode",
            "pub fn update_animation",
            "AnimationError::StaleMixer",
            "AnimationTarget::Translation",
            "AnimationTarget::Rotation",
            "AnimationTarget::Scale",
            "AnimationTarget::Weights",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/skinning.rs",
        &[
            "pub struct SceneSkinBinding",
            "pub fn skin_binding",
            "pub fn skin_matrices",
            "set_initial_skin_binding",
            "world_transform",
            "SkinningMatrix::inverse_from_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/scene/morphs.rs",
        &[
            "pub fn morph_weights",
            "pub fn set_morph_weights",
            "set_initial_morph_weights",
            "set_morph_weights_unchecked",
            "structure_revision",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry.rs",
        &[
            "InvalidMorphTargetVertexCount",
            "InvalidSkinJointVertexCount",
            "InvalidSkinWeightVertexCount",
            "InvalidSkinJointIndex",
            "GeometryMorphTarget",
            "GeometrySkin",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry/morph.rs",
        &[
            "pub struct GeometryMorphTarget",
            "pub fn with_morph_targets",
            "pub fn morphed_vertices",
            "InvalidMorphTargetVertexCount",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/geometry/skinning.rs",
        &[
            "pub struct GeometrySkin",
            "pub struct SkinningMatrix",
            "pub fn with_skin",
            "pub fn skinned_vertices",
            "from_gltf_column_major",
            "inverse_from_transform",
            "pub fn then",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/meshes.rs",
        &[
            "read_morph_targets",
            "GeometryMorphTarget::new",
            "read_joints",
            "read_weights",
            "GeometrySkin::new",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/skins.rs",
        &[
            "read_inverse_bind_matrices",
            "SkinningMatrix::from_gltf_column_major",
            "skin.joints()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/scene_asset.rs",
        &["pub fn skins(&self)", "pub const fn skin(&self)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf.rs",
        &["pub use self::skins::SceneAssetSkin", "parse_skins"],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/assets/gltf/skins.rs",
        &[
            "pub struct SceneAssetSkin",
            "parse_skins",
            "inverseBindMatrices",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/render/prepare.rs",
        &[
            "scene.skin_matrices(node)",
            "skinned_vertices",
            "InvalidSkinGeometry",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "src/diagnostics.rs",
        &[
            "pub enum AnimationError",
            "StaleMixer",
            "InvalidSkinIndex",
            "InvalidSkinJointIndex",
            "InvalidSkinGeometry",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_gltf_animation.rs",
        &[
            "mixer_controls_rebind_translation_channels_to_import_local_nodes",
            "playing_paused_and_seek_animation_dirty_prepared_render_state",
            "replace_import_invalidates_animation_mixers_with_stale_error",
            "gltf_animation_supports_rotation_scale_weights_and_normalizes_quaternions",
            "morph_target_weights_channel_updates_scene_morph_weights",
            "skinning_rebinds_joints_and_deforms_vertices_from_skeleton_hierarchy",
            "combined_morph_and_skinning_deforms_morphed_vertices_through_joint_matrices",
            "interpolation_handles_step_cubic_spline_and_quaternion_slerp",
            "khronos_sample_assets_load_instantiate_and_cover_animation_contracts",
            "steady_animation_update_reprepare_keeps_resource_counts_stable",
            "AnimationLoopMode::Repeat",
            "AnimationPlaybackState::Playing",
            "AnimationError::StaleMixer",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/assets/gltf/khronos/manifest.toml",
        &[
            "https://github.com/KhronosGroup/glTF-Sample-Assets",
            "2bac6f8c57bf471df0d2a1e8a8ec023c7801dddf",
            "RiggedSimple",
            "SimpleSkin",
            "SimpleMorph",
            "MorphCube",
            "RiggedFigure",
            "BrainStem",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_visual_proof.rs",
        &[
            "m3b_headless_visual_artifacts_cover_khronos_skin_morph_and_animation",
            "m3b-khronos-simple-skin",
            "m3b-khronos-simple-morph",
            "m3b-khronos-rigged-simple",
            "write_ppm_artifact",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/m3b_browser_rendered_output.rs",
        &[
            "m3b_browser_wasm_renders_morph_animation_to_canvas",
            "browser_canvas_roundtrip",
            "render_morph_animation_frame",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-M3B-ANIMATION",
        "tests/visual/fixtures/m3b-headless-animation.toml",
        &[
            "m3b-headless-animation",
            "max_abs_diff = 0",
            "m3b-khronos-simple-skin",
            "m3b-khronos-simple-morph",
            "m3b-khronos-rigged-simple",
        ],
    );
}

pub(crate) fn check_material_desc_fields_private(root: &Path, findings: &mut Vec<Finding>) {
    let path = root.join("src/material.rs");
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };

    for field in public_fields_in_struct(&text, "MaterialDesc") {
        findings.push(Finding::new(
            "ARCH-ASSET-API",
            format!("src/material.rs MaterialDesc exposes public field '{field}'"),
        ));
    }
}
