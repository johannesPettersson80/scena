use crate::SkinningMatrix;
use crate::animation::{
    AnimationChannel, AnimationClip, AnimationClipKey, AnimationInterpolation, AnimationOutput,
    AnimationTarget,
};
use crate::assets::Assets;
use crate::geometry::{GeometryDesc, GeometrySkin};
use crate::material::{Color, MaterialDesc};
use crate::scene::{Scene, SceneSkinBinding, Transform, Vec3};

use super::{PrepareTelemetry, Renderer};

impl Renderer {
    fn phase5_prepare_telemetry_for_test(&self) -> PrepareTelemetry {
        self.prepare_telemetry
    }

    fn phase5_dynamic_rejection_reason_for_test(
        &self,
        scene: &Scene,
        assets: &Assets,
    ) -> Option<&'static str> {
        let slots = super::prepare::collect_backend_material_slots(scene, Some(assets));
        let handles = slots.iter().map(|slot| slot.handle).collect::<Vec<_>>();
        self.dynamic_gpu_prepare_rejection_reason(scene, &handles)
    }
}

#[test]
fn transform_animation_gpu_prepare_uses_dynamic_path_without_recollecting_primitives() {
    let Ok(mut renderer) = Renderer::headless_gpu(48, 48) else {
        return;
    };
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.25, 0.25, 0.25));
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let mut scene = Scene::new();
    let camera = scene.add_default_camera().expect("camera inserts");
    let node = scene
        .mesh(geometry, material)
        .add()
        .expect("animated mesh inserts");
    let mixer = scene.insert_animation_mixer_for_test(translation_clip(node));

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    renderer
        .render(&scene, camera)
        .expect("initial GPU render succeeds");
    let first = renderer.phase5_prepare_telemetry_for_test();
    let first_dirty = scene.dirty_state();

    scene.play_animation(mixer).expect("mixer starts");
    for _ in 0..3 {
        let before = scene.dirty_state();
        scene
            .update_animation(mixer, 0.2)
            .expect("animation frame applies");
        let after = scene.dirty_state();
        assert_eq!(
            after.structure_revision, before.structure_revision,
            "transform animation frames must not dirty scene structure"
        );
        assert_eq!(
            after.transform_revision,
            before.transform_revision + 1,
            "each changed animation frame must bump transform revision once"
        );
        assert_eq!(
            renderer.phase5_dynamic_rejection_reason_for_test(&scene, &assets),
            None,
            "transform-only animation should satisfy dynamic GPU prepare preconditions"
        );
        renderer
            .prepare_with_assets(&mut scene, &assets)
            .expect("animated transform frame prepares dynamically");
        renderer
            .render(&scene, camera)
            .expect("animated transform frame renders");
    }

    let second = renderer.phase5_prepare_telemetry_for_test();
    let final_dirty = scene.dirty_state();
    assert_eq!(
        final_dirty.structure_revision, first_dirty.structure_revision,
        "transform-only playback must preserve structure revision across prepared frames"
    );
    assert_eq!(
        second.prepared_primitive_collections, first.prepared_primitive_collections,
        "transform-only animation prepares must skip canonical primitive collection"
    );
    assert_eq!(
        second.static_gpu_resource_rebuilds, first.static_gpu_resource_rebuilds,
        "transform-only animation prepares must not rebuild static GPU resources"
    );
    assert_eq!(
        second.draw_uniform_only_updates,
        first.draw_uniform_only_updates + 3,
        "each animated transform frame must ride the dynamic GPU draw state path"
    );
}

#[test]
fn skinned_joint_transform_rejects_dynamic_gpu_prepare_fast_path() {
    let mut renderer = Renderer::headless_gpu(48, 48).expect("HeadlessGpu renderer builds");
    let assets = Assets::new();
    let geometry = GeometryDesc::box_xyz(0.25, 0.25, 0.25)
        .with_skin(GeometrySkin::new(
            vec![[0, 0, 0, 0]; 24],
            vec![[1.0, 0.0, 0.0, 0.0]; 24],
        ))
        .expect("skinned geometry builds");
    let geometry = assets.create_geometry(geometry);
    let material =
        assets.create_material(MaterialDesc::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0));
    let mut scene = Scene::new();
    scene.add_default_camera().expect("camera inserts");
    let joint = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("joint inserts");
    let skinned = scene
        .mesh(geometry, material)
        .add()
        .expect("skinned mesh inserts");
    scene
        .set_skin_binding(
            skinned,
            SceneSkinBinding::new(vec![joint], vec![SkinningMatrix::IDENTITY]),
        )
        .expect("skin binding applies");

    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("initial GPU prepare succeeds");
    scene
        .set_transform(joint, Transform::at(Vec3::new(0.0, 0.2, 0.0)))
        .expect("joint transform updates");

    assert_eq!(
        renderer.phase5_dynamic_rejection_reason_for_test(&scene, &assets),
        Some("skinned joints may have moved"),
        "skinned joint motion must force a full GPU re-bake until skinning is shader-driven"
    );
}

#[cfg(feature = "scene-host")]
#[test]
fn eased_tint_transition_gpu_prepare_uses_dynamic_path_without_recollecting_primitives() {
    use crate::scene_host::{SceneHostCore, SceneHostEasing};
    use crate::{AssetPath, SurfaceViewport};

    let Ok(renderer) = Renderer::headless_gpu(48, 48) else {
        return;
    };
    let viewport = SurfaceViewport::new(48.0, 48.0, 1.0).expect("viewport is valid");
    let mut host =
        SceneHostCore::from_renderer(Assets::new(), renderer, viewport).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
    )))
    .expect("asset instantiates");
    let mesh = host
        .node_handle(import, "ColoredTriangle")
        .expect("mesh resolves");

    host.prepare().expect("initial prepare succeeds");
    let first = host.renderer().phase5_prepare_telemetry_for_test();

    host.set_node_tint_eased(
        mesh,
        Some(Color::from_linear_rgba(1.0, 0.0, 0.0, 1.0)),
        1.0,
        SceneHostEasing::Linear,
    )
    .expect("tint transition starts");
    host.advance(0.5).expect("tint transition advances");
    host.prepare()
        .expect("eased tint frame prepares dynamically");
    let second = host.renderer().phase5_prepare_telemetry_for_test();

    assert_eq!(
        second.prepared_primitive_collections, first.prepared_primitive_collections,
        "eased opaque tint prepares must skip canonical primitive collection"
    );
    assert_eq!(
        second.static_gpu_resource_rebuilds, first.static_gpu_resource_rebuilds,
        "eased opaque tint prepares must not rebuild static GPU resources"
    );
    assert_eq!(
        second.draw_uniform_only_updates,
        first.draw_uniform_only_updates + 1,
        "eased opaque tint frames must update retained draw uniforms"
    );
}

fn translation_clip(node: crate::scene::NodeKey) -> AnimationClip {
    AnimationClip::new(
        AnimationClipKey::fresh(),
        Some("MoveX".to_string()),
        vec![AnimationChannel::new(
            node,
            AnimationTarget::Translation,
            vec![0.0, 1.0],
            AnimationOutput::Vec3(vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)]),
            AnimationInterpolation::Linear,
        )],
        1.0,
    )
    .expect("test translation clip is valid")
}
