#![cfg(target_arch = "wasm32")]

use std::future::{Ready, ready};
use std::sync::Arc;

use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, CursorPosition, HitTarget, PerspectiveCamera,
    Primitive, Renderer, Scene, Transform, Vec3, Viewport,
};
use wasm_bindgen::{Clamped, JsCast};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn m3a_browser_wasm_renders_import_and_interaction_paths_to_canvas() {
    for frame in [render_glb_import_frame(), render_interaction_frame()] {
        assert!(nonblack_pixel_count(&frame) > 0);
        assert_eq!(browser_canvas_roundtrip(&frame, 32, 32), frame);
    }
}

fn render_glb_import_frame() -> Vec<u8> {
    let assets = Assets::with_fetcher(BinaryFetcher::new(
        "memory://browser-model.glb",
        minimal_glb_triangle_scene(),
    ));
    let scene_asset = pollster::block_on(assets.load_scene("memory://browser-model.glb"))
        .expect("GLB scene loads in browser proof");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("GLB scene instantiates in browser proof");
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
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds in wasm");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("GLB scene prepares in wasm");
    renderer
        .render(&scene, camera)
        .expect("GLB scene renders in wasm");
    renderer.frame_rgba8().to_vec()
}

fn render_interaction_frame() -> Vec<u8> {
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
        .expect("pickable node inserts");
    let hit = scene
        .pick(
            camera,
            CursorPosition::logical(16.0, 16.0),
            Viewport::new(32, 32, 1.0).expect("viewport is valid"),
        )
        .expect("pick succeeds")
        .expect("pick returns a hit");
    assert!(matches!(hit.target(), HitTarget::Node(hit_node) if hit_node == node));
    scene
        .interaction_mut()
        .set_hover(Some(HitTarget::Node(node)));
    scene
        .interaction_mut()
        .set_primary_selection(Some(HitTarget::Node(node)));

    let mut renderer = Renderer::headless(32, 32).expect("renderer builds in wasm");
    renderer
        .prepare(&mut scene)
        .expect("interaction scene prepares in wasm");
    renderer
        .render(&scene, camera)
        .expect("interaction scene renders in wasm");
    renderer.frame_rgba8().to_vec()
}

fn browser_canvas_roundtrip(frame: &[u8], width: u32, height: u32) -> Vec<u8> {
    let canvas = browser_canvas(width, height);
    let context = canvas_2d_context(&canvas);
    let image = ImageData::new_with_u8_clamped_array_and_sh(Clamped(frame), width, height)
        .expect("image data accepts renderer frame");
    context
        .put_image_data(&image, 0.0, 0.0)
        .expect("renderer frame writes to browser canvas");
    context
        .get_image_data(0.0, 0.0, width as f64, height as f64)
        .expect("browser canvas readback succeeds")
        .data()
        .to_vec()
}

fn browser_canvas(width: u32, height: u32) -> HtmlCanvasElement {
    let window = web_sys::window().expect("browser window exists");
    let document = window.document().expect("browser document exists");
    let canvas = document
        .create_element("canvas")
        .expect("canvas element can be created")
        .dyn_into::<HtmlCanvasElement>()
        .expect("element is a canvas");
    canvas.set_width(width);
    canvas.set_height(height);
    document
        .body()
        .expect("browser document has a body")
        .append_child(&canvas)
        .expect("canvas attaches to document");
    canvas
}

fn canvas_2d_context(canvas: &HtmlCanvasElement) -> CanvasRenderingContext2d {
    canvas
        .get_context("2d")
        .expect("2d context query succeeds")
        .expect("2d context exists")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context is CanvasRenderingContext2d")
}

fn nonblack_pixel_count(rgba: &[u8]) -> usize {
    rgba.chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
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
            "nodes": [{{ "name": "BrowserGlbTriangle", "mesh": 0 }}]
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
