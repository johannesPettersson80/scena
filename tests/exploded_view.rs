#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use scena::{
    AssetPath, Assets, Color, ExplodedView, GeometryDesc, MaterialDesc, Renderer, Scene,
    SceneHostCore, Transform, Vec3, VisualPatchV1,
};

#[test]
fn exploded_view_factor_zero_identity_and_factor_one_separates_parts() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let assembly = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("assembly inserts");
    let left = scene
        .mesh(geometry, material)
        .parent(assembly)
        .transform(Transform::at(Vec3::new(-1.0, 0.0, 0.0)))
        .add()
        .expect("left part inserts");
    let right = scene
        .mesh(geometry, material)
        .parent(assembly)
        .transform(Transform::at(Vec3::new(1.0, 0.0, 0.0)))
        .add()
        .expect("right part inserts");

    let identity = ExplodedView::from_node(assembly)
        .factor(0.0)
        .distance(1.0)
        .transforms(&scene, &assets)
        .expect("identity plan computes");
    assert_eq!(identity.len(), 2);
    assert_eq!(
        transform_for(&identity, left).translation,
        Vec3::new(-1.0, 0.0, 0.0)
    );
    assert_eq!(
        transform_for(&identity, right).translation,
        Vec3::new(1.0, 0.0, 0.0)
    );

    let exploded = ExplodedView::from_node(assembly)
        .factor(1.0)
        .distance(1.0)
        .transforms(&scene, &assets)
        .expect("exploded plan computes");
    assert_eq!(
        transform_for(&exploded, left).translation,
        Vec3::new(-2.0, 0.0, 0.0)
    );
    assert_eq!(
        transform_for(&exploded, right).translation,
        Vec3::new(2.0, 0.0, 0.0)
    );

    scene
        .set_transforms(&exploded.as_transform_updates())
        .expect("exploded transforms apply");
    let separated = scene
        .world_distance(left, right)
        .expect("distance computes");
    assert!(
        separated > 3.9,
        "factor 1.0 should visibly separate the direct children, got {separated}"
    );

    scene
        .set_transforms(&identity.as_transform_updates())
        .expect("identity transforms restore");
    assert_eq!(
        scene
            .node(left)
            .expect("left exists")
            .transform()
            .translation,
        Vec3::new(-1.0, 0.0, 0.0)
    );
}

#[test]
fn exploded_view_supports_axis_and_hierarchy_depth_modes() {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::box_xyz(0.4, 0.4, 0.4));
    let material = assets.create_material(MaterialDesc::unlit(Color::WHITE));
    let mut scene = Scene::new();
    let assembly = scene
        .add_empty(scene.root(), Transform::IDENTITY)
        .expect("assembly inserts");
    let left = scene
        .mesh(geometry, material)
        .parent(assembly)
        .transform(Transform::at(Vec3::new(-1.0, 0.5, 0.0)))
        .add()
        .expect("left part inserts");
    let right = scene
        .mesh(geometry, material)
        .parent(assembly)
        .transform(Transform::at(Vec3::new(1.0, -0.5, 0.0)))
        .add()
        .expect("right part inserts");
    let nested = scene
        .mesh(geometry, material)
        .parent(right)
        .transform(Transform::at(Vec3::new(0.0, 0.5, 0.0)))
        .add()
        .expect("nested part inserts");

    let axis = ExplodedView::from_node(assembly)
        .along_axis(Vec3::X)
        .factor(1.0)
        .distance(0.5)
        .transforms(&scene, &assets)
        .expect("axis plan computes");
    assert!(transform_for(&axis, left).translation.x < -1.4);
    assert!(transform_for(&axis, right).translation.x > 1.4);
    assert_eq!(
        transform_for(&axis, left).translation.y,
        0.5,
        "axis mode must offset along the selected axis only"
    );

    let depth = ExplodedView::from_node(assembly)
        .by_hierarchy_depth()
        .factor(1.0)
        .distance(0.5)
        .transforms(&scene, &assets)
        .expect("depth plan computes");
    let right_delta = transform_for(&depth, right).translation.x - 1.0;
    let nested_delta = transform_for(&depth, nested).translation.x;
    assert!(
        nested_delta > right_delta,
        "hierarchy-depth mode should push deeper parts farther: nested {nested_delta}, right {right_delta}"
    );
}

#[test]
fn scene_host_exploded_view_emits_visual_patch_transform_channels() {
    let mut host = SceneHostCore::headless(128, 96).expect("host builds");
    let import = pollster::block_on(host.instantiate_url(AssetPath::from(
        "tests/assets/gltf/exploded_view_assembly.gltf",
    )))
    .expect("assembly imports");
    let assembly = host
        .node_handle(import, "Assembly")
        .expect("assembly handle resolves");

    let immediate: VisualPatchV1 = serde_json::from_str(
        &host
            .exploded_view_patch_json(
                assembly,
                r#"{"mode":"axis","axis":[1.0,0.0,0.0],"factor":1.0,"distance":0.35}"#,
            )
            .expect("immediate patch serializes"),
    )
    .expect("visual patch decodes");
    assert_eq!(immediate.schema, "scena.visual_patch.v1");
    assert_eq!(immediate.transforms.len(), 2);
    assert!(immediate.transforms_eased.is_empty());

    let eased: VisualPatchV1 = serde_json::from_str(
        &host
            .exploded_view_patch_json(
                assembly,
                r#"{"mode":"axis","axis":[1.0,0.0,0.0],"factor":1.0,"distance":0.35,"duration_seconds":0.25,"easing":"ease_in_out"}"#,
            )
            .expect("eased patch serializes"),
    )
    .expect("visual patch decodes");
    assert!(eased.transforms.is_empty());
    assert_eq!(eased.transforms_eased.len(), 2);
    assert_eq!(eased.transforms_eased[0].duration_seconds, 0.25);
}

#[test]
fn exploded_view_renders_imported_assembly_with_wider_part_span() {
    let assets = Assets::new();
    let scene_asset =
        pollster::block_on(assets.load_scene("tests/assets/gltf/exploded_view_assembly.gltf"))
            .expect("assembly glTF loads");
    let (mut scene, camera) = Scene::with_default_camera().expect("camera inserts");
    let import = scene
        .instantiate(&scene_asset)
        .expect("assembly instantiates");
    let assembly = import.node("Assembly").expect("assembly node resolves");
    let bounds = import.bounds_world(&scene).expect("import has bounds");
    scene.frame(camera, bounds).expect("assembled view frames");

    let assembled = render_frame(&mut scene, &assets);
    let assembled_rect = nonblack_pixel_rect(&assembled, 160, 120).expect("assembled visible");

    let exploded = ExplodedView::from_node(assembly)
        .along_axis(Vec3::X)
        .factor(1.0)
        .distance(0.35)
        .transforms(&scene, &assets)
        .expect("exploded plan computes");
    scene
        .set_transforms(&exploded.as_transform_updates())
        .expect("exploded transforms apply");

    let exploded_frame = render_frame(&mut scene, &assets);
    let exploded_rect = nonblack_pixel_rect(&exploded_frame, 160, 120).expect("exploded visible");
    assert!(
        exploded_rect.width() > assembled_rect.width() + 8,
        "exploded imported assembly should span more pixels: assembled={assembled_rect:?}, exploded={exploded_rect:?}"
    );

    let artifact_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/exploded-view");
    fs::create_dir_all(&artifact_dir).expect("artifact dir exists");
    write_ppm_artifact(&artifact_dir, "assembled", 160, 120, &assembled);
    write_ppm_artifact(&artifact_dir, "exploded", 160, 120, &exploded_frame);
}

fn transform_for(plan: &scena::ExplodedViewPlan, node: scena::NodeKey) -> Transform {
    plan.updates()
        .iter()
        .find(|update| update.node == node)
        .expect("node update appears")
        .transform
}

fn render_frame(scene: &mut Scene, assets: &Assets) -> Vec<u8> {
    let mut renderer = Renderer::headless(160, 120).expect("renderer builds");
    renderer
        .prepare_with_assets(scene, assets)
        .expect("scene prepares");
    renderer.render_active(scene).expect("scene renders");
    renderer.frame_rgba8().to_vec()
}

#[derive(Debug, Clone, Copy)]
struct PixelRect {
    min_x: u32,
    max_x: u32,
}

impl PixelRect {
    fn width(self) -> u32 {
        self.max_x - self.min_x + 1
    }
}

fn nonblack_pixel_rect(rgba: &[u8], width: u32, height: u32) -> Option<PixelRect> {
    let mut rect: Option<PixelRect> = None;
    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            let pixel = &rgba[index..index + 4];
            if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
                continue;
            }
            rect = Some(match rect {
                Some(existing) => PixelRect {
                    min_x: existing.min_x.min(x),
                    max_x: existing.max_x.max(x),
                },
                None => PixelRect { min_x: x, max_x: x },
            });
        }
    }
    rect
}

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{} {}\n255\n", width, height).into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact writes");
}
