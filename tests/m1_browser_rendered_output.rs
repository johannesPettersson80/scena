#![cfg(target_arch = "wasm32")]

use scena::{Color, PerspectiveCamera, Primitive, Renderer, Scene, Transform, Vec3, Vertex};
use wasm_bindgen::{Clamped, JsCast};
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn m1_browser_wasm_renders_color_and_alpha_to_canvas() {
    let (mut scene, camera) = scene_with_fullscreen_primitives(vec![
        fullscreen_triangle(Color::from_linear_rgba(0.0, 0.0, 1.0, 1.0)),
        fullscreen_triangle(Color::from_linear_rgba(1.0, 0.0, 0.0, 0.5)),
    ]);
    let mut renderer = Renderer::headless(4, 4).expect("headless renderer builds in wasm");

    renderer
        .prepare(&mut scene)
        .expect("scene prepares in wasm");
    renderer
        .render(&scene, camera)
        .expect("scene renders in wasm");
    assert_eq!(
        center_pixel(renderer.frame_rgba8(), 4, 4),
        [158, 0, 159, 255]
    );

    let canvas = browser_canvas(4, 4);
    let context = canvas_2d_context(&canvas);
    let image = ImageData::new_with_u8_clamped_array_and_sh(Clamped(renderer.frame_rgba8()), 4, 4)
        .expect("image data accepts renderer frame");
    context
        .put_image_data(&image, 0.0, 0.0)
        .expect("renderer frame writes to browser canvas");

    let readback = context
        .get_image_data(0.0, 0.0, 4.0, 4.0)
        .expect("browser canvas readback succeeds")
        .data()
        .to_vec();
    assert_eq!(center_pixel(&readback, 4, 4), [158, 0, 159, 255]);
}

fn scene_with_fullscreen_primitives(primitives: Vec<Primitive>) -> (Scene, scena::CameraKey) {
    let mut scene = Scene::new();
    let camera = scene
        .add_perspective_camera(
            scene.root(),
            PerspectiveCamera::default(),
            Transform::default(),
        )
        .expect("camera inserts");
    scene
        .set_active_camera(camera)
        .expect("camera can become active");
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("fullscreen primitives insert");
    (scene, camera)
}

fn fullscreen_triangle(color: Color) -> Primitive {
    Primitive::triangle([
        Vertex {
            position: Vec3::new(-2.0, -2.0, 0.0),
            color,
        },
        Vertex {
            position: Vec3::new(4.0, -2.0, 0.0),
            color,
        },
        Vertex {
            position: Vec3::new(-2.0, 4.0, 0.0),
            color,
        },
    ])
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
        .expect("document has body")
        .append_child(&canvas)
        .expect("canvas appends to document");
    canvas
}

fn canvas_2d_context(canvas: &HtmlCanvasElement) -> CanvasRenderingContext2d {
    canvas
        .get_context("2d")
        .expect("2d context lookup succeeds")
        .expect("2d context exists")
        .dyn_into::<CanvasRenderingContext2d>()
        .expect("context is CanvasRenderingContext2d")
}

fn center_pixel(frame: &[u8], width: u32, height: u32) -> [u8; 4] {
    let offset = (((height / 2) * width + (width / 2)) * 4) as usize;
    frame[offset..offset + 4]
        .try_into()
        .expect("pixel slice has four channels")
}
