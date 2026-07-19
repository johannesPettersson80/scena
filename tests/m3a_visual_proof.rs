#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::future::{Ready, ready};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, Color, CursorPosition, GeometryDesc,
    GeometryTopology, GeometryVertex, HitTarget, LabelBillboard, LabelDesc, MaterialDesc,
    OffscreenTarget, PerspectiveCamera, Primitive, Renderer, Scene, Transform, Vec3, Viewport,
};

#[path = "support/q03_visual_metrics.rs"]
mod q03_visual_metrics;
use q03_visual_metrics::{difference_metrics, foreground_metrics};

const CAMERA_DISTANCE_FOR_NDC_FIXTURES: f32 = 1.732_050_8;

fn ndc_fixture_camera_transform() -> Transform {
    Transform::at(Vec3::new(0.0, 0.0, CAMERA_DISTANCE_FOR_NDC_FIXTURES))
}

#[test]
fn m3a_headless_visual_artifacts_cover_import_interaction_instances_labels_and_readback() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    let import = render_glb_model_viewer();
    let interaction_base = render_picking_selection_path(false);
    let interaction_selected = render_picking_selection_path(true);
    let instances = render_instancing_path(&[-0.35, 0.35], "m3a-instancing");
    let removed_half = render_instancing_path(&[-0.35], "m3a-instancing-removed-half");
    let labels = render_label_path();
    let readback = render_offscreen_readback_path();

    let errors = evaluate_m3a_feature_specific_truth(
        &import,
        &interaction_base,
        &interaction_selected,
        &instances,
        &labels,
        &readback,
    );
    assert!(
        errors.is_empty(),
        "M3A feature-specific metrics failed {errors:?}: selection base={:?} selected={:?} delta={:?}; labels={:?}",
        foreground_metrics(
            &interaction_base.rgba,
            interaction_base.width,
            interaction_base.height,
        ),
        foreground_metrics(
            &interaction_selected.rgba,
            interaction_selected.width,
            interaction_selected.height,
        ),
        difference_metrics(
            &interaction_base.rgba,
            &interaction_selected.rgba,
            interaction_base.width,
            interaction_base.height,
            2,
        ),
        foreground_metrics(&labels.rgba, labels.width, labels.height),
    );
    assert!(
        nonblack_pixel_count(&removed_half.rgba) > 0,
        "the old nonblack oracle would accept the removed-half instance mutation"
    );
    assert!(
        evaluate_m3a_feature_specific_truth(
            &import,
            &interaction_base,
            &interaction_selected,
            &removed_half,
            &labels,
            &readback,
        )
        .contains(&"instance_component_count"),
        "feature-specific truth must reject removing half the instances"
    );

    for artifact in [
        import,
        interaction_base,
        interaction_selected,
        instances,
        removed_half,
        labels,
        readback,
    ] {
        assert!(
            nonblack_pixel_count(&artifact.rgba) > 0,
            "{} should have visible output",
            artifact.name
        );
        write_ppm_artifact(
            &artifact_dir,
            artifact.name,
            artifact.width,
            artifact.height,
            &artifact.rgba,
        );
    }
}

fn evaluate_m3a_feature_specific_truth(
    import: &VisualArtifact,
    interaction_base: &VisualArtifact,
    interaction_selected: &VisualArtifact,
    instances: &VisualArtifact,
    labels: &VisualArtifact,
    readback: &VisualArtifact,
) -> Vec<&'static str> {
    let mut errors = Vec::new();
    let import_metrics = foreground_metrics(&import.rgba, import.width, import.height);
    let import_rect = import_metrics.rect.expect("import frame is nonempty");
    if import_metrics.component_count != 1
        || import_rect.width() < 12
        || import_rect.height() < 12
        || !(8.0..=24.0).contains(&import_metrics.centroid_x)
        || !(8.0..=24.0).contains(&import_metrics.centroid_y)
    {
        errors.push("import_projected_region");
    }

    let base_metrics = foreground_metrics(
        &interaction_base.rgba,
        interaction_base.width,
        interaction_base.height,
    );
    let selected_metrics = foreground_metrics(
        &interaction_selected.rgba,
        interaction_selected.width,
        interaction_selected.height,
    );
    if base_metrics.component_count != 1
        || selected_metrics.component_count != 1
        || selected_metrics
            .rect
            .is_none_or(|rect| rect.width() < 12 || rect.height() < 12)
    {
        errors.push("selection_projected_region");
    }

    let instance_metrics = foreground_metrics(&instances.rgba, instances.width, instances.height);
    if instance_metrics.component_count != 2
        || instance_metrics
            .rect
            .is_none_or(|rect| rect.width() < 16 || rect.height() < 6)
    {
        errors.push("instance_component_count");
    }

    let label_metrics = foreground_metrics(&labels.rgba, labels.width, labels.height);
    if label_metrics.component_count != 1
        || !(64..=128).contains(&label_metrics.pixel_count)
        || label_metrics
            .rect
            .is_none_or(|rect| rect.width() < 8 || rect.height() < 4)
    {
        errors.push("label_projected_region");
    }

    let readback_metrics = foreground_metrics(&readback.rgba, readback.width, readback.height);
    if readback_metrics.component_count != 1
        || readback_metrics
            .rect
            .is_none_or(|rect| rect.width() < 12 || rect.height() < 12)
    {
        errors.push("readback_projected_region");
    }
    errors
}

fn render_glb_model_viewer() -> VisualArtifact {
    let assets = Assets::with_fetcher(BinaryFetcher::new(
        "memory://model-viewer.glb",
        minimal_glb_triangle_scene(),
    ));
    let scene_asset = pollster::block_on(assets.load_scene("memory://model-viewer.glb"))
        .expect("GLB scene loads for visual proof");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("GLB scene instantiates for visual proof");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform {
                translation: Vec3::new(0.0, 0.0, 3.0),
                ..Transform::default()
            },
        )
        .expect("camera inserts");
    scene
        .frame(
            camera,
            import.bounds_world(&scene).expect("import has bounds"),
        )
        .expect("camera frames GLB bounds");
    render_scene_with_assets("m3a-glb-model-viewer", scene, camera, &assets)
}

fn render_picking_selection_path(selected: bool) -> VisualArtifact {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    let node = scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("pickable renderable inserts");
    let hit = scene
        .pick(
            camera,
            CursorPosition::logical(16.0, 16.0),
            Viewport::new(32, 32, 1.0).expect("viewport is valid"),
        )
        .expect("pick path succeeds")
        .expect("pick path produces a hit");
    assert!(matches!(hit.target(), HitTarget::Node(hit_node) if hit_node == node));
    if selected {
        scene
            .interaction_mut()
            .set_hover(Some(HitTarget::Node(node)));
        scene
            .interaction_mut()
            .set_primary_selection(Some(HitTarget::Node(node)));
        assert!(matches!(
            scene.interaction().primary_selection(),
            Some(HitTarget::Node(selected_node)) if selected_node == node
        ));
    }
    render_scene(
        if selected {
            "m3a-picking-selection"
        } else {
            "m3a-picking-base"
        },
        scene,
        camera,
    )
}

fn render_instancing_path(instance_x: &[f32], name: &'static str) -> VisualArtifact {
    let assets = Assets::new();
    let geometry = assets.create_geometry(
        GeometryDesc::try_new(
            GeometryTopology::Triangles,
            vec![
                GeometryVertex {
                    position: Vec3::new(-0.25, -0.25, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                GeometryVertex {
                    position: Vec3::new(0.25, -0.25, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
                GeometryVertex {
                    position: Vec3::new(0.0, 0.25, 0.0),
                    normal: Vec3::new(0.0, 0.0, 1.0),
                },
            ],
            vec![0, 1, 2],
        )
        .expect("instance geometry is valid"),
    );
    let material =
        assets.create_material(MaterialDesc::unlit(Color::from_linear_rgb(0.0, 0.8, 1.0)));
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    for &x in instance_x {
        scene
            .push_instance(
                set,
                Transform {
                    translation: Vec3::new(x, 0.0, 0.0),
                    ..Transform::default()
                },
            )
            .expect("instance inserts");
    }
    render_scene_with_assets(name, scene, camera, &assets)
}

fn render_label_path() -> VisualArtifact {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::new("M3a")
                .with_color(Color::from_linear_rgb(0.0, 1.0, 0.0))
                .with_size(0.5)
                .with_billboard(LabelBillboard::ScreenAligned),
            Transform::default(),
        )
        .expect("label inserts");
    render_scene("m3a-labels", scene, camera)
}

fn render_offscreen_readback_path() -> VisualArtifact {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            ndc_fixture_camera_transform(),
        )
        .expect("camera inserts");
    scene
        .add_renderable(
            scene.root(),
            vec![Primitive::unlit_triangle()],
            Transform::default(),
        )
        .expect("readback renderable inserts");
    let mut renderer = Renderer::offscreen(OffscreenTarget::new(32, 32).expect("target validates"))
        .expect("offscreen renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    let surface_frame = renderer.frame_rgba8().to_vec();
    let readback = renderer.read_pixels();
    assert_eq!(
        readback.rgba8(),
        surface_frame.as_slice(),
        "offscreen readback must exactly preserve the renderer-owned surface frame"
    );
    VisualArtifact {
        name: "m3a-offscreen-readback",
        width: readback.width(),
        height: readback.height(),
        rgba: readback.into_rgba8(),
    }
}

fn render_scene(name: &'static str, mut scene: Scene, camera: scena::CameraKey) -> VisualArtifact {
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");
    renderer.prepare(&mut scene).expect("scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    VisualArtifact {
        name,
        width: 32,
        height: 32,
        rgba: renderer.frame_rgba8().to_vec(),
    }
}

fn render_scene_with_assets<F>(
    name: &'static str,
    mut scene: Scene,
    camera: scena::CameraKey,
    assets: &Assets<F>,
) -> VisualArtifact {
    let mut renderer = Renderer::headless(32, 32).expect("headless renderer builds");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("asset scene prepares");
    renderer.render(&scene, camera).expect("scene renders");
    VisualArtifact {
        name,
        width: 32,
        height: 32,
        rgba: renderer.frame_rgba8().to_vec(),
    }
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn write_ppm_artifact(dir: &Path, name: &str, width: u32, height: u32, rgba: &[u8]) {
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    fs::write(dir.join(format!("{name}.ppm")), ppm).expect("PPM artifact can be written");
    fs::write(
        dir.join(format!("{name}.toml")),
        format!(
            "[artifact]\nname = \"{name}\"\nformat = \"ppm\"\nencoding = \"srgb8\"\nwidth = {width}\nheight = {height}\n"
        ),
    )
    .expect("artifact metadata can be written");
}

fn artifact_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/m3a-visual")
}

struct VisualArtifact {
    name: &'static str,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[derive(Clone)]
struct BinaryFetcher {
    path: AssetPath,
    bytes: Arc<Vec<u8>>,
}

impl BinaryFetcher {
    fn new(path: impl Into<AssetPath>, bytes: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            bytes: Arc::new(bytes),
        }
    }
}

impl AssetFetcher for BinaryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        if path == &self.path {
            ready(Ok((*self.bytes).clone()))
        } else {
            ready(Err(AssetError::NotFound {
                path: path.as_str().to_string(),
            }))
        }
    }
}

fn minimal_glb_triangle_scene() -> Vec<u8> {
    let mut bin = Vec::new();
    for value in [-0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bin.extend_from_slice(&value.to_le_bytes());
    }
    let buffer_byte_length = bin.len();
    pad_to_four(&mut bin, 0);

    let json = format!(
        r#"{{
            "asset": {{ "version": "2.0" }},
            "buffers": [{{ "byteLength": {buffer_byte_length} }}],
            "bufferViews": [
                {{ "buffer": 0, "byteOffset": 0, "byteLength": 36 }},
                {{ "buffer": 0, "byteOffset": 36, "byteLength": 6 }}
            ],
            "accessors": [
                {{
                    "bufferView": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "min": [-0.5, -0.5, 0.0],
                    "max": [0.5, 0.5, 0.0]
                }},
                {{ "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" }}
            ],
            "materials": [
                {{ "pbrMetallicRoughness": {{ "baseColorFactor": [0.2, 0.8, 0.1, 1.0] }} }}
            ],
            "meshes": [
                {{ "primitives": [{{ "attributes": {{ "POSITION": 0 }}, "indices": 1, "material": 0 }}] }}
            ],
            "nodes": [{{ "name": "GlbTriangle", "mesh": 0 }}]
        }}"#
    );
    let mut json = json.into_bytes();
    pad_to_four(&mut json, b' ');

    let length = 12 + 8 + json.len() + 8 + bin.len();
    let mut glb = Vec::with_capacity(length);
    glb.extend_from_slice(&0x4654_6C67_u32.to_le_bytes());
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&(length as u32).to_le_bytes());
    glb.extend_from_slice(&(json.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes());
    glb.extend_from_slice(&json);
    glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    glb.extend_from_slice(&0x004E_4942_u32.to_le_bytes());
    glb.extend_from_slice(&bin);
    glb
}

fn pad_to_four(bytes: &mut Vec<u8>, pad: u8) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(pad);
    }
}
