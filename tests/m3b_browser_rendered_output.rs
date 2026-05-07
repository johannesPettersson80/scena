#![cfg(target_arch = "wasm32")]

use std::collections::BTreeMap;
use std::future::{Ready, ready};
use std::sync::Arc;

use scena::{
    AssetError, AssetFetcher, AssetPath, Assets, PerspectiveCamera, Renderer, Scene, Transform,
};
use wasm_bindgen::{Clamped, JsCast};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn m3b_browser_wasm_renders_morph_animation_to_canvas() {
    let frame = render_morph_animation_frame();
    assert!(nonblack_pixel_count(&frame) > 0);
    assert_eq!(browser_canvas_roundtrip(&frame, 32, 32), frame);
}

fn render_morph_animation_frame() -> Vec<u8> {
    let assets = Assets::with_fetcher(MemoryFetcher::new(vec![
        (
            AssetPath::from("memory://browser/morph.gltf"),
            morph_weight_gltf().into_bytes(),
        ),
        (
            AssetPath::from("memory://browser/morph.bin"),
            morph_weight_buffer(),
        ),
    ]));
    let scene_asset = pollster::block_on(assets.load_scene("memory://browser/morph.gltf"))
        .expect("morph scene loads in browser proof");
    let mut scene = Scene::new();
    let import = scene
        .instantiate(&scene_asset)
        .expect("morph scene instantiates in browser proof");
    let mixer = scene
        .create_animation_mixer(&import, "MorphWeight")
        .expect("morph mixer creates");
    scene
        .seek_animation(mixer, 1.0)
        .expect("morph mixer samples");
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    let mut renderer = Renderer::headless(32, 32).expect("renderer builds in wasm");
    renderer
        .prepare_with_assets(&mut scene, &assets)
        .expect("morph scene prepares in wasm");
    renderer
        .render(&scene, camera)
        .expect("morph scene renders in wasm");
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
struct MemoryFetcher {
    sources: Arc<BTreeMap<AssetPath, Vec<u8>>>,
}

impl MemoryFetcher {
    fn new(entries: Vec<(AssetPath, Vec<u8>)>) -> Self {
        Self {
            sources: Arc::new(entries.into_iter().collect()),
        }
    }
}

impl AssetFetcher for MemoryFetcher {
    type Future<'a> = Ready<Result<Vec<u8>, AssetError>>;

    fn fetch<'a>(&'a self, path: &'a AssetPath) -> Self::Future<'a> {
        if let Some(bytes) = self.sources.get(path) {
            ready(Ok(bytes.clone()))
        } else {
            ready(Err(AssetError::NotFound {
                path: path.as_str().to_string(),
            }))
        }
    }
}

fn morph_weight_gltf() -> String {
    r#"{
        "asset": { "version": "2.0" },
        "nodes": [
            { "name": "Morphing", "mesh": 0 }
        ],
        "meshes": [
            {
                "weights": [0.0],
                "primitives": [
                    {
                        "attributes": { "POSITION": 0 },
                        "indices": 1,
                        "targets": [
                            { "POSITION": 2 }
                        ]
                    }
                ]
            }
        ],
        "animations": [
            {
                "name": "MorphWeight",
                "samplers": [
                    { "input": 3, "output": 4, "interpolation": "LINEAR" }
                ],
                "channels": [
                    { "sampler": 0, "target": { "node": 0, "path": "weights" } }
                ]
            }
        ],
        "buffers": [
            { "byteLength": 94, "uri": "morph.bin" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 36, "byteLength": 6 },
            { "buffer": 0, "byteOffset": 42, "byteLength": 36 },
            { "buffer": 0, "byteOffset": 78, "byteLength": 8 },
            { "buffer": 0, "byteOffset": 86, "byteLength": 8 }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 1, "componentType": 5123, "count": 3, "type": "SCALAR" },
            { "bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC3" },
            { "bufferView": 3, "componentType": 5126, "count": 2, "type": "SCALAR" },
            { "bufferView": 4, "componentType": 5126, "count": 2, "type": "SCALAR" }
        ]
    }"#
    .to_string()
}

fn morph_weight_buffer() -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [-0.5_f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u16, 1, 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [
        0.0_f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.0, 1.0, 0.0, 1.0,
    ] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}
