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

#[test]
fn m3a_headless_visual_artifacts_cover_import_interaction_instances_labels_and_readback() {
    let artifact_dir = artifact_dir();
    fs::create_dir_all(&artifact_dir).expect("artifact directory can be created");

    for artifact in [
        render_glb_model_viewer(),
        render_picking_selection_path(),
        render_instancing_path(),
        render_label_path(),
        render_offscreen_readback_path(),
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

fn render_picking_selection_path() -> VisualArtifact {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
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
    scene
        .interaction_mut()
        .set_hover(Some(HitTarget::Node(node)));
    scene
        .interaction_mut()
        .set_primary_selection(Some(HitTarget::Node(node)));
    render_scene("m3a-picking-selection", scene, camera)
}

fn render_instancing_path() -> VisualArtifact {
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
            Transform::default(),
        )
        .expect("camera inserts");
    let set = scene
        .add_instance_set(scene.root(), geometry, material, Transform::default())
        .expect("instance set inserts");
    for x in [-0.35, 0.35] {
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
    render_scene_with_assets("m3a-instancing", scene, camera, &assets)
}

fn render_label_path() -> VisualArtifact {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene
        .add_label(
            scene.root(),
            LabelDesc::msdf("M3a")
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
            Transform::default(),
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
    let readback = renderer.read_pixels();
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
                {{ "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" }},
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
