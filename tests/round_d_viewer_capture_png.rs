#![cfg(not(target_arch = "wasm32"))]

use std::io::Cursor;
use std::path::PathBuf;

use scena::headless_gltf_viewer;

#[test]
fn viewer_capture_png_bytes_decode_to_current_frame() {
    let width = 48;
    let height = 32;
    let mut viewer = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(width, height)
            .with_default_light()
            .build(),
    )
    .expect("headless viewer builds");
    viewer.render_next_frame().expect("viewer renders a frame");

    let png_bytes = viewer.capture_png_bytes().expect("PNG capture encodes");
    let decoded = decode_png_rgba8(&png_bytes);

    assert_eq!(decoded.width, width);
    assert_eq!(decoded.height, height);
    assert_eq!(decoded.rgba8, viewer.snapshot_rgba8());
}

#[test]
fn viewer_capture_png_writes_reference_artifact() {
    let width = 48;
    let height = 32;
    let mut viewer = pollster::block_on(
        headless_gltf_viewer("tests/assets/gltf/mesh_material_vertex_color_scene.gltf")
            .size(width, height)
            .with_default_light()
            .build(),
    )
    .expect("headless viewer builds");
    viewer.render_next_frame().expect("viewer renders a frame");

    let artifact = artifact_path();
    viewer.capture_png(&artifact).expect("PNG capture writes");
    let file_bytes = std::fs::read(&artifact).expect("PNG artifact is readable");
    let bytes = viewer.capture_png_bytes().expect("PNG capture encodes");

    assert_eq!(file_bytes, bytes);
    assert_eq!(decode_png_rgba8(&file_bytes).rgba8, viewer.snapshot_rgba8());
}

#[derive(Debug)]
struct DecodedPng {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

fn decode_png_rgba8(bytes: &[u8]) -> DecodedPng {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("PNG header reads");
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("PNG output buffer size is known")
    ];
    let info = reader.next_frame(&mut buffer).expect("PNG payload reads");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    DecodedPng {
        width: info.width,
        height: info.height,
        rgba8: buffer[..info.buffer_size()].to_vec(),
    }
}

fn artifact_path() -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/gate-artifacts/viewer-capture");
    std::fs::create_dir_all(&dir).expect("viewer-capture artifact directory");
    dir.join("viewer-capture-png-reference.png")
}
