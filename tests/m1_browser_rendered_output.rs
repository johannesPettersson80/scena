#![cfg(target_arch = "wasm32")]

use scena::{
    Assets, Color, GeometryDesc, GeometryTopology, MaterialDesc, PerspectiveCamera, Primitive,
    Renderer, Scene, Transform, Vec3, Vertex,
};
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

#[wasm_bindgen_test]
fn m1_browser_wasm_renders_technical_materials_to_canvas() {
    for frame in [
        render_line_material(),
        render_wireframe_material(),
        render_edge_material(),
    ] {
        assert!(nonblack_pixel_count(&frame) > 0);
        assert_eq!(browser_canvas_roundtrip(&frame, 16, 16), frame);
    }
}

fn render_line_material() -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(GeometryDesc::line(
        Vec3::new(-0.85, 0.0, 0.0),
        Vec3::new(0.85, 0.0, 0.0),
    ));
    let material = assets.create_material(MaterialDesc::line(Color::WHITE, 1.0));
    render_asset_mesh(&assets, geometry, material)
}

fn render_wireframe_material() -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(flat_square_geometry());
    let material = assets.create_material(MaterialDesc::wireframe(Color::WHITE, 1.0));
    render_asset_mesh(&assets, geometry, material)
}

fn render_edge_material() -> Vec<u8> {
    let assets = Assets::new();
    let geometry = assets.create_geometry(flat_square_geometry());
    let material = assets.create_material(MaterialDesc::edge(Color::WHITE, 1.0));
    render_asset_mesh(&assets, geometry, material)
}

fn render_asset_mesh(
    assets: &Assets,
    geometry: scena::GeometryHandle,
    material: scena::MaterialHandle,
) -> Vec<u8> {
    let (mut scene, camera) = scene_with_camera();
    scene
        .mesh(geometry, material)
        .add()
        .expect("asset mesh inserts");
    let mut renderer = Renderer::headless(16, 16).expect("headless renderer builds in wasm");
    renderer
        .prepare_with_assets(&mut scene, assets)
        .expect("asset mesh prepares in wasm");
    renderer
        .render(&scene, camera)
        .expect("asset mesh renders in wasm");
    renderer.frame_rgba8().to_vec()
}

fn scene_with_fullscreen_primitives(primitives: Vec<Primitive>) -> (Scene, scena::CameraKey) {
    let (mut scene, camera) = scene_with_camera();
    scene
        .add_renderable(scene.root(), primitives, Transform::default())
        .expect("fullscreen primitives insert");
    (scene, camera)
}

fn scene_with_camera() -> (Scene, scena::CameraKey) {
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

fn browser_canvas_roundtrip(frame: &[u8], width: u32, height: u32) -> Vec<u8> {
    let canvas = browser_canvas(width, height);
    let context = canvas_2d_context(&canvas);
    let image = ImageData::new_with_u8_clamped_array_and_sh(Clamped(frame), width, height)
        .expect("image data accepts renderer frame");
    context
        .put_image_data(&image, 0.0, 0.0)
        .expect("renderer frame writes to browser canvas");
    context
        .get_image_data(0.0, 0.0, f64::from(width), f64::from(height))
        .expect("browser canvas readback succeeds")
        .data()
        .to_vec()
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

fn nonblack_pixel_count(frame: &[u8]) -> usize {
    frame
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count()
}

fn flat_square_geometry() -> GeometryDesc {
    GeometryDesc::try_new(
        GeometryTopology::Triangles,
        vec![
            scena::GeometryVertex {
                position: Vec3::new(-0.75, -0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.75, -0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(0.75, 0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
            scena::GeometryVertex {
                position: Vec3::new(-0.75, 0.75, 0.0),
                normal: Vec3::new(0.0, 0.0, 1.0),
            },
        ],
        vec![0, 1, 2, 0, 2, 3],
    )
    .expect("flat square test geometry is valid")
}
